use std::{
    any::Any,
    error::Error,
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use strum_macros::Display;

use super::message::{Info, Message};

/// Defines a `ProgressInfo`.
#[derive(Display, Debug, PartialEq)]
pub enum ProgressInfo {
    /// Can used by cli or gui to show the the progress continues n ticks.
    #[strum(to_string = "Ticks")]
    Ticks,

    /// Can used by cli or gui to show that the progress total duration is n ticks.
    #[strum(to_string = "Duration")]
    Duration,
}

/// Impl of `Info` for `ProgressInfo`.
impl Info for ProgressInfo {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Defines a `ProgressMessage`.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use cuba_lib::shared::progress_message::{ProgressInfo, ProgressMessage};
///
/// let progress_tick = ProgressMessage::new(Arc::new(ProgressInfo::Ticks), 1);
/// ```
pub struct ProgressMessage {
    /// Info
    info: Arc<dyn Info + Send + Sync>,

    /// The ticks.
    pub ticks: u64,
}

/// Methods of `ProgressMessage`.
impl ProgressMessage {
    /// Creates a new `ProgressMessage`.
    pub fn new(info: Arc<dyn Info + Send + Sync>, ticks: u64) -> Self {
        ProgressMessage { info, ticks }
    }
}

/// Impl of `Message` for `ProgressMessage`.
impl Message for ProgressMessage {
    fn err(&self) -> Option<&(dyn Error + Send + Sync)> {
        None
    }

    fn info(&self) -> Option<&(dyn Info + Send + Sync)> {
        Some(self.info.as_ref())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Impl of `Display` for `ProgressMessage`.
impl Display for ProgressMessage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "Info : {:?} : {}", self.info, self.ticks)
    }
}
