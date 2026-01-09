use std::{error::Error, sync::RwLock};

use cuba_lib::shared::{
    message::Info,
    msg_receiver::MsgHandler,
    npath::{Rel, UNPath},
};

use crate::{UpdateHandler, egui_widgets::ProgressState};

/// Defines a `TaskMessageType`.
#[derive(Clone, Copy)]
pub enum TaskMessageType {
    Info,
    Error,
}

/// Impl of `Default` for `TaskMessageType`.
impl Default for TaskMessageType {
    fn default() -> Self {
        TaskMessageType::Info
    }
}

/// Defines a TaskMessage
#[derive(Clone)]
pub struct TaskMessage {
    pub msg_type: TaskMessageType,
    pub path: String,
    pub message: String,
}

/// Methods of `TaskMessage`.
impl TaskMessage {
    /// Creates a new `TaskMessage`.
    pub fn new(msg_type: TaskMessageType, path: String, message: String) -> Self {
        Self {
            msg_type,
            path,
            message,
        }
    }
}

/// Impl of `Default` for `TaskMessage`.
impl Default for TaskMessage {
    fn default() -> Self {
        Self {
            msg_type: TaskMessageType::default(),
            path: String::default(),
            message: String::default(),
        }
    }
}

/// Defines a `TaskProgress`.
pub struct TaskProgress {
    transfer_threads: RwLock<usize>,
    task_progress: RwLock<Box<[RwLock<ProgressState>]>>,
    task_message: RwLock<Box<[RwLock<TaskMessage>]>>,
    total_progress: RwLock<ProgressState>,
    update_handler: UpdateHandler,
}

impl TaskProgress {
    /// Creates a new `TaskProgress`.
    pub fn new(update_handler: UpdateHandler) -> Self {
        Self {
            transfer_threads: RwLock::new(0),
            task_progress: RwLock::new(TaskProgress::init(0)),
            task_message: RwLock::new(TaskProgress::init(0)),
            total_progress: RwLock::new(ProgressState::default()),
            update_handler,
        }
    }

    /// Sets the transfer_threads.
    pub fn set_transfer_threads(&self, transfer_threads: usize) {
        *self.transfer_threads.write().unwrap() = transfer_threads;
        *self.task_progress.write().unwrap() = TaskProgress::init(transfer_threads);
        *self.task_message.write().unwrap() = TaskProgress::init(transfer_threads);
    }

    // Returns the transfer threads.
    pub fn transfer_threads(&self) -> usize {
        *self.transfer_threads.read().unwrap()
    }

    /// Returns the task progress.
    pub fn get_task_progress(&self, thread_number: usize) -> ProgressState {
        *self.task_progress.read().unwrap()[thread_number]
            .read()
            .unwrap()
    }

    /// Returns the task message.
    pub fn get_task_message(&self, thread_number: usize) -> TaskMessage {
        self.task_message.read().unwrap()[thread_number]
            .read()
            .unwrap()
            .clone()
    }

    /// Returns the total progress.
    pub fn get_total_progress(&self) -> ProgressState {
        *self.total_progress.read().unwrap()
    }

    /// Initializes a vector of `RwLock<T>` with a default value.
    fn init<T: Default>(size: usize) -> Box<[RwLock<T>]> {
        let mut vec = Vec::with_capacity(size);

        for _ in 0..size {
            vec.push(RwLock::new(T::default()));
        }

        vec.into_boxed_slice()
    }

    // Handles a task info.
    fn handle_task_info(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        *self.task_message.read().unwrap()[thread_number]
            .write()
            .unwrap() = TaskMessage::new(
            TaskMessageType::Info,
            rel_path.compact_unicode(),
            info.to_string(),
        );
        self.update_handler.update();
    }

    /// Handles a task error.
    fn handle_task_error(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        error: &(dyn Error + Send + Sync),
    ) {
        *self.task_message.read().unwrap()[thread_number]
            .write()
            .unwrap() = TaskMessage::new(
            TaskMessageType::Error,
            rel_path.compact_unicode(),
            error.to_string(),
        );
        self.update_handler.update();
    }

    /// Handles a clean info.
    fn handle_clean_info(&self, rel_path: &UNPath<Rel>, info: &(dyn Info + Send + Sync)) {
        *self.task_message.read().unwrap()[0].write().unwrap() = TaskMessage::new(
            TaskMessageType::Info,
            rel_path.compact_unicode(),
            info.to_string(),
        );

        self.task_progress.read().unwrap()[0]
            .write()
            .unwrap()
            .advance_one();

        self.update_handler.update();
    }

    /// Handles a clean error.
    fn handle_clean_error(&self, rel_path: &UNPath<Rel>, error: &(dyn Error + Send + Sync)) {
        *self.task_message.read().unwrap()[0].write().unwrap() = TaskMessage::new(
            TaskMessageType::Error,
            rel_path.compact_unicode(),
            error.to_string(),
        );

        self.task_progress.read().unwrap()[0]
            .write()
            .unwrap()
            .advance_one();

        self.update_handler.update();
    }
}

/// Impl of `MsgHandler` for `TaskProgress`.
impl MsgHandler for TaskProgress {
    /// Called when the `MsgHandler` has started.
    fn started(&self) {
        self.total_progress.write().unwrap().clear();

        for thread_number in 0..*self.transfer_threads.read().unwrap() {
            *self.task_message.read().unwrap()[thread_number]
                .write()
                .unwrap() = TaskMessage::new(TaskMessageType::Info, String::new(), String::new());
        }
    }

    /// Handles a `TaskInfo::Start` message.
    fn task_start(
        &self,
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        self.task_progress.read().unwrap()[thread_number]
            .write()
            .unwrap()
            .clear();
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
        self.task_progress.read().unwrap()[thread_number]
            .write()
            .unwrap()
            .clear();
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
        self.task_progress.read().unwrap()[thread_number]
            .write()
            .unwrap()
            .advance_one();
        self.update_handler.update();
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

    /// Handles a `ProgressInfo::Ticks` message.
    fn progress_ticks(&self, ticks: u64, _info: &(dyn Info + Send + Sync)) {
        self.total_progress.write().unwrap().advance_ticks(ticks);
        self.update_handler.update();
    }

    /// Handles a `ProgressInfo::Duration` message.
    fn progress_duration(&self, ticks: u64, _info: &(dyn Info + Send + Sync)) {
        self.total_progress.write().unwrap().set_duration(ticks);
        self.update_handler.update();
    }
}
