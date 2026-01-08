use crossbeam_channel::{Receiver, Sender, select, unbounded};
use std::error::Error;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::shared::clean_message::{CleanInfo, CleanMessage};
use crate::shared::message::Message;
use crate::shared::message::{ErrorMessage, WarnMessage};
use crate::shared::message::{Info, InfoMessage};
use crate::shared::npath::{Rel, UNPath};
use crate::shared::progress_message::{ProgressInfo, ProgressMessage};
use crate::shared::task_message::{TaskInfo, TaskMessage};

/// Trace error.
pub fn trace_error(err: &dyn std::error::Error) -> String {
    let mut msg = format!("{}", err);
    let mut source = err.source();

    while let Some(err) = source {
        msg.push_str(&format!("\nCaused by: {}", err));
        source = err.source();
    }

    msg
}

/// Defines a `MsgHandler`.
pub trait MsgHandler {
    /// Called when the `MsgHandler` has started.
    fn started(&self) {}

    /// Called after the `MsgReceiver` has stopped.
    fn stopped(&self) {}

    /// Handles a `TaskInfo::Start` message.
    fn task_start(
        &self,
        _thread_number: usize,
        _rel_path: &UNPath<Rel>,
        _info: &(dyn Info + Send + Sync),
    ) {
    }

    /// Handles a `TaskInfo::Transferring` message.
    fn task_transferring(
        &self,
        _thread_number: usize,
        _rel_path: &UNPath<Rel>,
        _info: &(dyn Info + Send + Sync),
    ) {
    }

    /// Handles a `TaskInfo::Finished` message.
    fn task_finished(
        &self,
        _thread_number: usize,
        _rel_path: &UNPath<Rel>,
        _info: &(dyn Info + Send + Sync),
    ) {
    }

    /// Handles a `TaskInfo::Transferred` message.
    fn task_transferred(
        &self,
        _thread_number: usize,
        _rel_path: &UNPath<Rel>,
        _info: &(dyn Info + Send + Sync),
    ) {
    }

    /// Handles a `TaskInfo::Tick` message.
    fn task_tick(
        &self,
        _thread_number: usize,
        _rel_path: &UNPath<Rel>,
        _info: &(dyn Info + Send + Sync),
    ) {
    }

    /// Handles a `TaskInfo::UpToDate` message.
    fn task_up_to_date(
        &self,
        _thread_number: usize,
        _rel_path: &UNPath<Rel>,
        _info: &(dyn Info + Send + Sync),
    ) {
    }

    /// Handles a `TaskInfo::Verified` message.
    fn task_verified(
        &self,
        _thread_number: usize,
        _rel_path: &UNPath<Rel>,
        _info: &(dyn Info + Send + Sync),
    ) {
    }

    /// Handles a `TaskMessage` with error.
    fn task_error(
        &self,
        _thread_number: usize,
        _rel_path: &UNPath<Rel>,
        _error: &(dyn Error + Send + Sync),
    ) {
    }

    /// Handles a `ProgressInfo::Ticks` message.
    fn progress_ticks(&self, _ticks: u64, _info: &(dyn Info + Send + Sync)) {}

    /// Handles a `ProgressInfo::Duration` message.
    fn progress_duration(&self, _ticks: u64, _info: &(dyn Info + Send + Sync)) {}

    /// Handles a `CleanInfo::Ok` message.
    fn clean_ok(&self, _rel_path: &UNPath<Rel>, _info: &(dyn Info + Send + Sync)) {}

    /// Handles a `CleanInfo::Removed` message.
    fn clean_removed(&self, _rel_path: &UNPath<Rel>, _info: &(dyn Info + Send + Sync)) {}

    /// Handles a `CleanMessage` with error.
    fn clean_error(&self, _rel_path: &UNPath<Rel>, _error: &(dyn Error + Send + Sync)) {}

    /// Handles a `InfoMessage`.
    fn info(&self, _info: &(dyn Info + Send + Sync)) {}

    /// Handles a `WarnMessage`.
    fn warn(&self, _warning: &(dyn Info + Send + Sync)) {}

    /// Handles a `ErrorMessage`.
    fn error(&self, _error: &(dyn Error + Send + Sync)) {}
}

/// Defines a `MsgReceiver`.
///
/// The `MsgReceiver` can be used to handle messages.
pub struct MsgReceiver {
    receiver: Receiver<Arc<dyn Message>>,
    shutdown_sender: Option<Sender<()>>,
    thread_handle: Option<JoinHandle<()>>,
    msg_handler: Arc<dyn MsgHandler + Sync + Send>,
}

/// Methods of `MsgReceiver`.
impl MsgReceiver {
    /// Creates a new `MsgReceiver`.
    pub fn new(
        receiver: Receiver<Arc<dyn Message>>,
        msg_handler: Arc<dyn MsgHandler + Sync + Send>,
    ) -> Self {
        Self {
            receiver,
            shutdown_sender: None,
            thread_handle: None,
            msg_handler,
        }
    }

    /// Starts the `MsgReceiver`.
    pub fn start(&mut self) {
        let receiver = self.receiver.clone();
        let (shutdown_sender, shutdown_receiver) = unbounded();
        self.shutdown_sender = Some(shutdown_sender);

        self.msg_handler.started();

        let msg_handler = Arc::clone(&self.msg_handler);

        // Handle messages in a separate thread.
        self.thread_handle = Some(thread::spawn(move || {
            loop {
                select! {
                    recv(receiver) -> message => {
                        if let Ok(message) = message {
                            if let Some(task_message) = message.as_ref().as_any().downcast_ref::<TaskMessage>() {
                                if let Some(info) = task_message.info() {
                                    if let Some(task_info) = info.as_any().downcast_ref::<TaskInfo>() {
                                        match task_info {
                                            TaskInfo::Start => msg_handler.task_start(task_message.thread_number, &task_message.rel_path, info),
                                            TaskInfo::Transferring => msg_handler.task_transferring(task_message.thread_number, &task_message.rel_path, info),
                                            TaskInfo::Finished => msg_handler.task_finished(task_message.thread_number, &task_message.rel_path, info),
                                            TaskInfo::Transferred => msg_handler.task_transferred(task_message.thread_number, &task_message.rel_path, info),
                                            TaskInfo::Tick => msg_handler.task_tick(task_message.thread_number, &task_message.rel_path, info),
                                            TaskInfo::UpToDate => msg_handler.task_up_to_date(task_message.thread_number, &task_message.rel_path, info),
                                            TaskInfo::Verified => msg_handler.task_verified(task_message.thread_number, &task_message.rel_path, info)
                                        }
                                    }
                                }
                                else if let Some(err) = task_message.err() {
                                    msg_handler.task_error(task_message.thread_number, &task_message.rel_path, err);
                                }
                            }
                            else if let Some(progress_message) = message.as_ref().as_any().downcast_ref::<ProgressMessage>()
                                && let Some(info) = progress_message.info() {
                                    if let Some(progress_info) = info.as_any().downcast_ref::<ProgressInfo>() {
                                        match progress_info {
                                            ProgressInfo::Ticks => msg_handler.progress_ticks(progress_message.ticks, info),
                                            ProgressInfo::Duration => msg_handler.progress_duration(progress_message.ticks, info)
                                        }
                                    }
                                }
                            else if let Some(clean_message) = message.as_ref().as_any().downcast_ref::<CleanMessage>() {
                                if let Some(info) = clean_message.info() {
                                    if let Some(clean_info) = info.as_any().downcast_ref::<CleanInfo>() {
                                        match clean_info {
                                            CleanInfo::Ok => msg_handler.clean_ok(&clean_message.rel_path, info),
                                            CleanInfo::Removed => msg_handler.clean_removed(&clean_message.rel_path, info)
                                        }
                                    }
                                }
                                else if let Some(err) = clean_message.err() {
                                    msg_handler.clean_error(&clean_message.rel_path, err);
                                }
                            }
                            else if let Some(info_message) = message.as_ref().as_any().downcast_ref::<InfoMessage>()
                                && let Some(info) = info_message.info() {
                                    msg_handler.info(info);
                                }
                            else if let Some(warn_message) = message.as_ref().as_any().downcast_ref::<WarnMessage>()
                                && let Some(info) = warn_message.info() {
                                    msg_handler.warn(info);
                                }
                            else if let Some(error_message) = message.as_ref().as_any().downcast_ref::<ErrorMessage>()
                                && let Some(err) = error_message.err() {
                                    msg_handler.error(err);
                                }
                        }
                    },
                    recv(shutdown_receiver) -> _ => {
                        break;
                    },
                }
            }
        }));
    }

    /// Stops the `MsgReceiver`.
    pub fn stop(&mut self) {
        if let Some(sender) = self.shutdown_sender.take() {
            // Lets wait a little bit to receiver pending messages.
            thread::sleep(Duration::from_millis(100));

            // Signal shutdown.
            let _ = sender.send(());
        }

        if let Some(handle) = self.thread_handle.take() {
            // Wait for the thread to finish.
            handle.join().unwrap();
        }

        self.msg_handler.stopped();
    }
}
