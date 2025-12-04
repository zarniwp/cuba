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

/// Defines a `CleanError`.
#[derive(Error, Debug)]
pub enum CleanError {
    /// Can used by cli or gui to show that the removal of a file or directory was not successful.   
    #[error("Remove failed")]
    RemoveFailed,
}

/// Defines a `CleanInfo`.
#[derive(Display, Debug, PartialEq)]
pub enum CleanInfo {
    /// Can used by cli or gui to show that a file or directory is indexed and no orphan.   
    #[strum(to_string = "Ok")]
    Ok,

    /// Can used by cli or gui to show that a file or directory was removed.   
    #[strum(to_string = "Removed")]
    Removed,
}

/// Impl of `Info` for `CleanInfo`.
impl Info for CleanInfo {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Defines a `CleanMessage`.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use std::path::Path;
/// use cuba::shared::message::StringError;
/// use cuba::shared::clean_message::{CleanInfo, CleanError, CleanMessage};
/// use cuba::shared::npath::{NPath, Rel, File};
///
/// let rel_file_path = NPath::<Rel, File>::try_from("file.zip").unwrap();
/// let clean_error = CleanMessage::new(&rel_file_path.clone().into(), Some(Arc::new(CleanError::RemoveFailed)), None);
/// let clean_info = CleanMessage::new(&rel_file_path.into(), None, Some(Arc::new(CleanInfo::Removed)));
/// ```
pub struct CleanMessage {
    /// The path.
    pub rel_path: UNPath<Rel>,

    /// Error (if any).
    error: Option<Arc<dyn Error + Send + Sync>>,

    /// Info (if any).
    info: Option<Arc<dyn Info + Send + Sync>>,
}

/// Methods of `CleanMessage`.
impl CleanMessage {
    /// Creates a new `CleanMessage`.
    pub fn new(
        rel_path: &UNPath<Rel>,
        error: Option<Arc<dyn Error + Send + Sync>>,
        info: Option<Arc<dyn Info + Send + Sync>>,
    ) -> Self {
        CleanMessage {
            rel_path: rel_path.clone(),
            error,
            info,
        }
    }
}

/// Impl of `Message` for `CleanMessage`.
impl Message for CleanMessage {
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

/// Impl of `Display` for `CleanMessage`.
impl Display for CleanMessage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(err) = self.err() {
            write!(formatter, "Error : {:?} : {}", self.rel_path, err)
        } else if let Some(info) = self.info() {
            write!(formatter, "Info : {:?} : {}", self.rel_path, info)
        } else {
            write!(formatter, "No Message : {:?}", self.rel_path)
        }
    }
}
