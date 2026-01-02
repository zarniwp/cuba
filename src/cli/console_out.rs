use crate::shared::message::Info;
use crate::shared::msg_receiver::MsgHandler;
use crate::shared::npath::{Rel, UNPath};
use console::Style;
use std::error::Error;

/// Defines a `ConsoleOut`.
///
/// Prints messages to the console.
pub struct ConsoleOut {
    green: Style,
    yellow: Style,
    red: Style,
}

/// Methods of `ConsoleOut`.
impl ConsoleOut {
    /// Creates a new `ConsoleOut`.
    pub fn new() -> Self {
        let green = Style::new().green().bold();
        let yellow = Style::new().yellow().bold();
        let red = Style::new().red().bold();

        Self { green, yellow, red }
    }
}

/// Impl `Default` for `ConsoleOut`.
impl Default for ConsoleOut {
    fn default() -> Self {
        Self::new()
    }
}

/// Impls `MsgHandler` for `ConsoleOut`.
impl MsgHandler for ConsoleOut {
    /// Handles a `TaskInfo::Start` message.
    fn task_start(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        println!("{:?} : {}", rel_path, self.green.apply_to(info));
    }

    /// Handles a `TaskInfo::Transferring` message.
    fn task_transferring(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        println!("{:?} : {}", rel_path, self.green.apply_to(info));
    }

    /// Handles a `TaskInfo::Finished` message.
    fn task_finished(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        println!("{:?} : {}", rel_path, self.green.apply_to(info));
    }

    /// Handles a `TaskInfo::Transferred` message.
    fn task_transferred(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        println!("{:?} : {}", rel_path, self.green.apply_to(info));
    }

    /// Handles a `TaskInfo::UpToDate` message.
    fn task_up_to_date(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        println!("{:?} : {}", rel_path, self.green.apply_to(info));
    }

    /// Handles a `TaskInfo::Verified` message.
    fn task_verified(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        println!("{:?} : {}", rel_path, self.green.apply_to(info));
    }

    /// Handles a `TaskMessage` with error.
    fn task_error(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        error: &(dyn Error + Send + Sync),
    ) {
        println!("{:?} : {}", rel_path, self.red.apply_to(error));
    }

    /// Handles a `CleanInfo::Ok` message.
    fn clean_ok(&self, rel_path: &UNPath<Rel>, info: &(dyn Info + Send + Sync)) {
        println!("{:?} : {}", rel_path, self.green.apply_to(info));
    }

    /// Handles a `CleanInfo::Removed` message.
    fn clean_removed(&self, rel_path: &UNPath<Rel>, info: &(dyn Info + Send + Sync)) {
        println!("{:?} : {}", rel_path, self.green.apply_to(info));
    }

    /// Handles a `CleanMessage` with error.
    fn clean_error(&self, rel_path: &UNPath<Rel>, error: &(dyn Error + Send + Sync)) {
        println!("{:?} : {}", rel_path, self.red.apply_to(error));
    }

    /// Handles a `InfoMessage`.
    fn info(&self, info: &(dyn Info + Send + Sync)) {
        println!("{}", self.green.apply_to(info));
    }

    /// Handles a `WarnMessage`.
    fn warn(&self, warning: &(dyn Info + Send + Sync)) {
        println!("{}", self.yellow.apply_to(warning));
    }

    /// Handles a `ErrorMessage`.
    fn error(&self, error: &(dyn Error + Send + Sync)) {
        println!("{}", self.red.apply_to(error));
    }
}
