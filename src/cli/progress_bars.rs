use console::Style;
use crossbeam_channel::{Receiver, Sender, select, unbounded};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::shared::clean_message::CleanMessage;
use crate::shared::message::Message;
use crate::shared::progress_message::{ProgressInfo, ProgressMessage};
use crate::shared::task_message::{TaskInfo, TaskMessage};

/// Visualizes messages as progress bars.
pub struct ProgressBars {
    receiver: Arc<Receiver<Arc<dyn Message>>>,
    shutdown_sender: Option<Sender<()>>,
    threads: usize,
    _multi_progress: MultiProgress,
    progress_bars: Arc<Vec<Mutex<ProgressBar>>>,
    error_occurred: Arc<Vec<Mutex<bool>>>,
    thread_handle: Option<JoinHandle<()>>,
}

/// Methods of `ProgressBars`.
impl ProgressBars {
    /// Creates a new `ProgressBars`. Takes a message receiver.    
    pub fn new(receiver: Arc<Receiver<Arc<dyn Message>>>, threads: usize) -> Self {
        let mut progress_bars = Vec::new();
        let mut error_occurred = Vec::new();
        let multi_progress = MultiProgress::new();

        let thread_style =
            ProgressStyle::with_template("{prefix:.bold.dim} {spinner:.green} {wide_msg}").unwrap();
        let total_style =
            ProgressStyle::with_template("{prefix:.bold.dim} [{wide_bar:.green}] {percent}%")
                .unwrap()
                .progress_chars(". ");

        for i in 0..threads {
            let bar = multi_progress.add(ProgressBar::new(0));
            bar.set_style(thread_style.clone());
            bar.set_prefix(format!("[{}]", i));
            progress_bars.push(Mutex::new(bar));
            error_occurred.push(Mutex::new(false));
        }

        // Add total progress bar
        let total_bar = multi_progress.add(ProgressBar::new_spinner());
        total_bar.set_style(total_style.clone());
        total_bar.set_prefix("[Progress]".to_string());
        progress_bars.push(Mutex::new(total_bar));

        Self {
            receiver,
            shutdown_sender: None,
            threads,
            _multi_progress: multi_progress,
            progress_bars: Arc::new(progress_bars),
            error_occurred: Arc::new(error_occurred),
            thread_handle: None,
        }
    }

    /// Starts a thread that listens for messages and visualizes them as progress bars.
    pub fn start(&mut self) {
        let progress_bars = Arc::clone(&self.progress_bars);
        let error_occurred = Arc::clone(&self.error_occurred);
        let total_index = self.threads;

        let receiver = Arc::clone(&self.receiver);
        let (shutdown_sender, shutdown_receiver) = unbounded();
        self.shutdown_sender = Some(shutdown_sender);

        let progress_bar_index: Mutex<usize> = Mutex::new(0);

        self.thread_handle = Some(thread::spawn(move || {
            let red = Style::new().red().bold();
            let green = Style::new().green().bold();

            loop {
                select! {
                    recv(receiver) -> msg => {
                        match msg {
                            Ok(message) => {
                                if let Some(task_message) = message.as_ref().as_any().downcast_ref::<TaskMessage>() {
                                    if let Some(bar_mutex) = progress_bars.get(task_message.thread_number) {
                                        let bar = bar_mutex.lock().unwrap();
                                        if let Some(err_mutex) = error_occurred.get(task_message.thread_number) {
                                            let mut error_flag = err_mutex.lock().unwrap();

                                            if !*error_flag {
                                                if let Some(err) = task_message.err() {
                                                    bar.set_message(format!("{:?} : {}", task_message.rel_path, red.apply_to(err)));
                                                    *error_flag = true;
                                                }

                                                if let Some(info) = task_message.info() {
                                                    if let Some(task_info) = info.as_any().downcast_ref::<TaskInfo>() {
                                                        match task_info {
                                                            TaskInfo::Tick => bar.tick(),
                                                            TaskInfo::Finished => {
                                                                bar.set_message(format!("{:?} : {}", task_message.rel_path, green.apply_to(info)));
                                                            },
                                                            _ => {
                                                                bar.set_message(format!("{:?} : {}", task_message.rel_path, green.apply_to(info)));
                                                            }
                                                        }
                                                    } else {
                                                        bar.set_message(format!("{:?} : {}", task_message.rel_path, green.apply_to(info)));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                else if let Some(clean_message) = message.as_ref().as_any().downcast_ref::<CleanMessage>() {
                                    let pb_index: usize;

                                    {
                                        let mut index = progress_bar_index.lock().unwrap();
                                        pb_index = *index;
                                        *index = (*index + 1) % total_index;
                                    }

                                    if let Some(bar_mutex) = progress_bars.get(pb_index) {
                                        let bar = bar_mutex.lock().unwrap();
                                        if let Some(err) = clean_message.err() {
                                            bar.set_message(format!("{:?} : {}", clean_message.rel_path, red.apply_to(err)));
                                        }
                                        if let Some(info) = clean_message.info() {
                                            bar.set_message(format!("{:?} : {}", clean_message.rel_path, green.apply_to(info)));
                                        }
                                    }
                                }
                                else if let Some(progress_message) = message.as_ref().as_any().downcast_ref::<ProgressMessage>()
                                    && let Some(total_bar_mutex) = progress_bars.get(total_index)
                                        && let Some(info) = progress_message.info()
                                            && let Some(progress_info) = info.as_any().downcast_ref::<ProgressInfo>() {
                                                match progress_info {
                                                    ProgressInfo::Ticks => {
                                                        total_bar_mutex.lock().unwrap().inc(progress_message.ticks);
                                                    }
                                                    ProgressInfo::Duration => {
                                                        total_bar_mutex.lock().unwrap().set_length(progress_message.ticks);
                                                    }
                                                }
                                            }
                            },
                            Err(_) => break, // channel closed
                        }
                    },
                    recv(shutdown_receiver) -> _ => break, // shutdown requested
                }
            }
        }));
    }

    /// Signal the thread to stop and wait for it to finish.
    pub fn stop(&mut self) {
        if let Some(sender) = self.shutdown_sender.take() {
            thread::sleep(Duration::from_millis(100)); // Lets wait a little bit to receiver pending messages.
            let _ = sender.send(()); // signal shutdown
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        for bar_mutex in self.progress_bars.iter() {
            bar_mutex.lock().unwrap().finish();
        }
    }
}
