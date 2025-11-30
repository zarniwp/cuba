use console::Style;
use crossbeam_channel::{Receiver, Sender, select, unbounded};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::shared::clean_message::CleanMessage;
use crate::shared::message::{ErrorMessage, InfoMessage, Message, WarnMessage};
use crate::shared::task_message::{TaskInfo, TaskMessage};

/// Prints messages to the console.
pub struct ConsoleOut {
    receiver: Arc<Receiver<Arc<dyn Message>>>,
    shutdown_sender: Option<Sender<()>>,
    thread_handle: Option<JoinHandle<()>>,
}

impl ConsoleOut {
    /// Creates a new console. Takes a message receiver.
    pub fn new(receiver: Arc<Receiver<Arc<dyn Message>>>) -> Self {
        Self {
            receiver,
            shutdown_sender: None,
            thread_handle: None,
        }
    }

    /// Starts a thread that listens for messages and prints them to the console.
    pub fn start(&mut self) {
        let receiver = Arc::clone(&self.receiver);
        let (shutdown_sender, shutdown_receiver) = unbounded();
        self.shutdown_sender = Some(shutdown_sender);

        self.thread_handle = Some(thread::spawn(move || {
            let green = Style::new().green().bold();
            let yellow = Style::new().yellow().bold();
            let red = Style::new().red().bold();

            loop {
                select! {
                    recv(receiver) -> msg => {
                        match msg {
                            Ok(message) => {
                                if let Some(task_message) = message.as_ref().as_any().downcast_ref::<TaskMessage>() {
                                    if let Some(info) = task_message.info() {
                                        if let Some(task_info) = info.as_any().downcast_ref::<TaskInfo>() {
                                            if task_info != &TaskInfo::Tick {
                                                println!("{:?} : {}", task_message.rel_path, green.apply_to(info));
                                            }
                                        } else {
                                            println!("{:?} : {}", task_message.rel_path, green.apply_to(info));
                                        }
                                    }
                                    else if let Some(err) = task_message.err() {
                                        println!("{:?} : {}", task_message.rel_path, red.apply_to(err));
                                    }
                                }
                                else if let Some(clean_message) = message.as_ref().as_any().downcast_ref::<CleanMessage>() {
                                    if let Some(info) = clean_message.info() {
                                        println!("{:?} : {}", clean_message.rel_path, green.apply_to(info));
                                    }
                                    else if let Some(err) = clean_message.err() {
                                        println!("{:?} : {}", clean_message.rel_path, red.apply_to(err));
                                    }
                                }
                                else if let Some(info_message) = message.as_ref().as_any().downcast_ref::<InfoMessage>() {
                                    if let Some(info) = info_message.info() {
                                        println!("{}", green.apply_to(info));
                                    }
                                }
                                else if let Some(warn_message) = message.as_ref().as_any().downcast_ref::<WarnMessage>() {
                                    if let Some(info) = warn_message.info() {
                                        println!("{}", yellow.apply_to(info));
                                    }
                                }
                                else if let Some(error_message) = message.as_ref().as_any().downcast_ref::<ErrorMessage>() {
                                    if let Some(err) = error_message.err() {
                                        println!("{}", red.apply_to(err));
                                    }
                                }
                            }
                            Err(_) => break, // All senders dropped.
                        }
                    }
                    recv(shutdown_receiver) -> _ => break, // Received shutdown signal.
                }
            }
        }));
    }

    /// Signal the thread to stop and wait for it to finish.
    pub fn stop(&mut self) {
        if let Some(sender) = self.shutdown_sender.take() {
            thread::sleep(Duration::from_millis(100)); // Lets wait a little bit to receiver pending msgs.
            let _ = sender.send(()); // Signal shutdown.
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}
