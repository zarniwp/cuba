use console::Style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::error::Error;
use std::sync::{Arc, Mutex};

use cuba_lib::shared::message::Info;
use cuba_lib::shared::msg_receiver::MsgHandler;
use cuba_lib::shared::npath::{Rel, UNPath};

/// Visualizes messages as progress bars.
pub struct ProgressBars {
    threads: usize,
    _multi_progress: MultiProgress,
    progress_bars: Arc<Vec<Mutex<ProgressBar>>>,
    error_occurred: Arc<Vec<Mutex<bool>>>,
    progress_bar_index: Mutex<usize>,
    green: Style,
    red: Style,
}

/// Methods of `ProgressBars`.
impl ProgressBars {
    /// Creates a new `ProgressBars`.  
    pub fn new(threads: usize) -> Self {
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
            threads,
            _multi_progress: multi_progress,
            progress_bars: Arc::new(progress_bars),
            error_occurred: Arc::new(error_occurred),
            progress_bar_index: Mutex::new(0),
            green: Style::new().green().bold(),
            red: Style::new().red().bold(),
        }
    }

    // Handles a task info.
    fn handle_task_info(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        if let Some(bar_mutex) = self.progress_bars.get(thread_number) {
            let bar = bar_mutex.lock().unwrap();

            if let Some(err_mutex) = self.error_occurred.get(thread_number) {
                let error_flag = err_mutex.lock().unwrap();

                if !*error_flag {
                    bar.set_message(format!("{:?} : {}", rel_path, self.green.apply_to(info)));
                }
            }
        }
    }

    /// Handles a task error.
    fn handle_task_error(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        error: &(dyn Error + Send + Sync),
    ) {
        if let Some(bar_mutex) = self.progress_bars.get(thread_number) {
            let bar = bar_mutex.lock().unwrap();

            if let Some(err_mutex) = self.error_occurred.get(thread_number) {
                let mut error_flag = err_mutex.lock().unwrap();

                if !*error_flag {
                    bar.set_message(format!("{:?} : {}", rel_path, self.red.apply_to(error)));
                    *error_flag = true;
                }
            }
        }
    }

    /// Handles a clean info.
    fn handle_clean_info(&self, rel_path: &UNPath<Rel>, info: &(dyn Info + Send + Sync)) {
        let pb_index: usize;

        {
            let mut index = self.progress_bar_index.lock().unwrap();
            pb_index = *index;
            *index = (*index + 1) % self.threads;
        }

        if let Some(bar_mutex) = self.progress_bars.get(pb_index) {
            let bar = bar_mutex.lock().unwrap();
            bar.set_message(format!("{:?} : {}", rel_path, self.green.apply_to(info)));
        }
    }

    /// Handles a clean error.
    fn handle_clean_error(&self, rel_path: &UNPath<Rel>, error: &(dyn Error + Send + Sync)) {
        let pb_index: usize;

        {
            let mut index = self.progress_bar_index.lock().unwrap();
            pb_index = *index;
            *index = (*index + 1) % self.threads;
        }

        if let Some(bar_mutex) = self.progress_bars.get(pb_index) {
            let bar = bar_mutex.lock().unwrap();
            bar.set_message(format!("{:?} : {}", rel_path, self.red.apply_to(error)));
        }
    }
}

/// Impl of `MsgHandler` for `ProgressBars`.
impl MsgHandler for ProgressBars {
    /// Called when the `MsgHandler` has started.
    fn started(&self) {
        let mut index = self.progress_bar_index.lock().unwrap();
        *index = 0;
    }

    /// Called after the `MsgReceiver` has stopped.
    fn stopped(&self) {
        for bar_mutex in self.progress_bars.iter() {
            bar_mutex.lock().unwrap().finish();
        }
    }

    /// Handles a `TaskInfo::Start` message.
    fn task_start(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        self.handle_task_info(thread_number, rel_path, info);
    }

    /// Handles a `TaskInfo::Transferring` message.
    fn task_transferring(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        self.handle_task_info(thread_number, rel_path, info);
    }

    /// Handles a `TaskInfo::Finished` message.
    fn task_finished(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        self.handle_task_info(thread_number, rel_path, info);
    }

    /// Handles a `TaskInfo::Transferred` message.
    fn task_transferred(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        self.handle_task_info(thread_number, rel_path, info);
    }

    /// Handles a `TaskInfo::Tick` message.
    fn task_tick(
        &self,
        thread_number: usize,
        _rel_path: &UNPath<Rel>,
        _info: &(dyn Info + Send + Sync),
    ) {
        if let Some(bar_mutex) = self.progress_bars.get(thread_number) {
            bar_mutex.lock().unwrap().tick();
        }
    }

    /// Handles a `TaskInfo::UpToDate` message.
    fn task_up_to_date(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        self.handle_task_info(thread_number, rel_path, info);
    }

    /// Handles a `TaskInfo::Verified` message.
    fn task_verified(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        self.handle_task_info(thread_number, rel_path, info);
    }

    /// Handles a `TaskMessage` with error.
    fn task_error(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        error: &(dyn Error + Send + Sync),
    ) {
        self.handle_task_error(thread_number, rel_path, error);
    }

    /// Handles a `ProgressInfo::Ticks` message.
    fn progress_ticks(&self, ticks: u64, _info: &(dyn Info + Send + Sync)) {
        if let Some(total_bar_mutex) = self.progress_bars.get(self.threads) {
            total_bar_mutex.lock().unwrap().inc(ticks);
        }
    }

    /// Handles a `ProgressInfo::Duration` message.
    fn progress_duration(&self, ticks: u64, _info: &(dyn Info + Send + Sync)) {
        if let Some(total_bar_mutex) = self.progress_bars.get(self.threads) {
            total_bar_mutex.lock().unwrap().set_length(ticks);
        }
    }

    /// Handles a `CleanInfo::Ok` message.
    fn clean_ok(&self, rel_path: &UNPath<Rel>, info: &(dyn Info + Send + Sync)) {
        self.handle_clean_info(rel_path, info);
    }

    /// Handles a `CleanInfo::Removed` message.
    fn clean_removed(&self, rel_path: &UNPath<Rel>, info: &(dyn Info + Send + Sync)) {
        self.handle_clean_info(rel_path, info);
    }

    /// Handles a `CleanMessage` with error.
    fn clean_error(&self, rel_path: &UNPath<Rel>, error: &(dyn Error + Send + Sync)) {
        self.handle_clean_error(rel_path, error);
    }

    /// Handles a `InfoMessage`.
    fn info(&self, _info: &(dyn Info + Send + Sync)) {}

    /// Handles a `WarnMessage`.
    fn warn(&self, _warning: &(dyn Info + Send + Sync)) {}

    /// Handles a `ErrorMessage`.
    fn error(&self, _error: &(dyn Error + Send + Sync)) {}
}
