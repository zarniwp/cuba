use std::{
    any::Any,
    error::Error,
    fmt::{self, Display, Formatter},
    sync::Arc,
};
use strum_macros::Display;
use thiserror::Error;

use super::message::{Info, Message};
use super::npath::{Rel, UNPath};

#[derive(Error, Debug)]
pub enum TaskError {
    /// Can used by cli or gui to show that the transfer of a file or directory was not successful.   
    #[error("Transfer failed")]
    TransferFailed,

    /// Can used by cli or gui to show that the verification of a file or directory was not successful.   
    #[error("Verified failed")]
    VerifiedFailed,

    /// Can used by cli or gui to show that a transferred node was not found.
    #[error("No transferred node found")]
    NoTransferredNode,

    /// Can used by cli or gui to show that password id is missing.
    #[error("No password id available")]
    NoPasswordId,
}

/// A Task is a thread that transfers a file or directory. After finishing,
/// the task looks for more work to do.
/// For simplicity, the messages for files and directories are the same — even though
/// transferring a directory technically means creating it (mkdir), and an up-to-date
/// directory simply means it already exists.
#[derive(Display, Debug, PartialEq)]
pub enum TaskInfo {
    /// Can used by cli or gui to show that the task is going to start another work.
    #[strum(to_string = "Start process ...")]
    Start,

    /// Can used by cli or gui to show that the task is transferring a file or directory.
    #[strum(to_string = "Transferring ...")]
    Transferring,

    /// Can used by cli or gui to show that the task has finished doing its current work.
    #[strum(to_string = "Finished!")]
    Finished,

    /// Can used by cli or gui to show that the task has finished transferring a file or directory.
    #[strum(to_string = "Transferred")]
    Transferred,

    /// Can used by cli or gui to show a progress indication of the working task.
    #[strum(to_string = "Tick")]
    Tick,

    /// Can used by cli or gui to show that a file or directory is up to date and no transfer is needed.
    #[strum(to_string = "Up to date")]
    UpToDate,

    /// Can used by cli or gui to show that a file or directory was successfully verified.   
    #[strum(to_string = "Verified")]
    Verified,
}

impl Info for TaskInfo {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Defines an `TaskMessage`.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use std::path::Path;
/// use cuba::shared::message::StringError;
/// use cuba::shared::task_message::{TaskInfo, TaskError, TaskMessage};
/// use cuba::shared::npath::{NPath, Rel, File};
///
/// let rel_file_path = NPath::<Rel, File>::try_from("file.zip").unwrap();
/// let task_error = TaskMessage::new(5, &rel_file_path.clone().into(), Some(Arc::new(TaskError::VerifiedFailed)), None);
/// let task_info = TaskMessage::new(5, &rel_file_path.into(), None, Some(Arc::new(TaskInfo::Transferred)));
/// ```
pub struct TaskMessage {
    /// The thread number.
    pub thread_number: usize,

    /// The path.
    pub rel_path: UNPath<Rel>,

    /// Error (if any).
    error: Option<Arc<dyn Error + Send + Sync>>,

    /// Info (if any).
    info: Option<Arc<dyn Info + Send + Sync>>,
}

impl TaskMessage {
    /// Creates a new instance of `TaskMessage`.
    pub fn new(
        thread_number: usize,
        rel_path: &UNPath<Rel>,
        error: Option<Arc<dyn Error + Send + Sync>>,
        info: Option<Arc<dyn Info + Send + Sync>>,
    ) -> Self {
        TaskMessage {
            thread_number,
            rel_path: rel_path.clone(),
            error,
            info,
        }
    }
}

/// Implementation of Message for TaskMessage.
impl Message for TaskMessage {
    fn err(&self) -> Option<&(dyn Error + Send + Sync)> {
        self.error.as_deref()
    }

    fn info(&self) -> Option<&(dyn Info + Send + Sync)> {
        self.info.as_deref()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Implementation of Display for TaskMessage.
impl Display for TaskMessage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(err) = self.err() {
            write!(
                formatter,
                "Thread: {} : Error : {:?} : {}",
                self.thread_number, self.rel_path, err
            )
        } else if let Some(info) = self.info() {
            write!(
                formatter,
                "Thread: {} : Info : {:?} : {}",
                self.thread_number, self.rel_path, info
            )
        } else {
            write!(
                formatter,
                "Thread: {} : No Message : {:?}",
                self.thread_number, self.rel_path
            )
        }
    }
}
