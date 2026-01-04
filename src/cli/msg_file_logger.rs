use flexi_logger::DeferredNow;
use flexi_logger::writers::LogWriter;
use flexi_logger::{Logger, WriteMode};
use log::{LevelFilter, Record};
use std::error::Error;
use std::io::Write;
use std::sync::Mutex;

use crate::shared::message::Info;
use crate::shared::msg_receiver::{MsgHandler, trace_error};
use crate::shared::npath::{Rel, UNPath};

/// Defines a `MsgLogFile`
struct MsgLogFile {
    file: Mutex<std::fs::File>,
    log_levels: Vec<log::Level>,
}

/// Methods of `MsgLogFile`.
impl MsgLogFile {
    /// Creates a new `MsgLogFile`.
    pub fn new(file_name: &str, log_levels: Vec<log::Level>) -> Self {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_name)
            .unwrap();

        MsgLogFile {
            file: Mutex::new(file),
            log_levels,
        }
    }

    /// Check if the `MsgLogFile` accepts the log level.
    pub fn accepts_level(&self, level: log::Level) -> bool {
        self.log_levels.contains(&level)
    }

    /// Write a message to `MsgLogFile`.
    pub fn write(&self, message: &str) -> std::io::Result<()> {
        self.file.lock().unwrap().write_all(message.as_bytes())
    }

    /// Flush the `MsgLogFile`.
    pub fn flush(&self) -> std::io::Result<()> {
        self.file.lock().unwrap().flush()
    }
}

/// Defines a `MsgLogFileWriter`.
struct MsgLogFileWriter {
    msg_log_files: Vec<MsgLogFile>,
}

/// Methods of `MsgLogFileWriter`.
impl MsgLogFileWriter {
    /// Creates a new `MsgLogFileWriter`.
    pub fn new() -> Self {
        MsgLogFileWriter {
            msg_log_files: Vec::new(),
        }
    }

    /// Adds a log file with accepted levels.
    pub fn add_log_file(&mut self, file_name: &str, log_levels: Vec<log::Level>) {
        self.msg_log_files
            .push(MsgLogFile::new(file_name, log_levels));
    }
}

/// Impl of `LogWriter` for `MsgLogFileWriter`.
impl LogWriter for MsgLogFileWriter {
    /// Write the log record to the log files.
    fn write(&self, _now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        let message = format!("{} {}\n", record.level(), record.args());

        for msg_log_file in self
            .msg_log_files
            .iter()
            .filter(|msg_log_file| msg_log_file.accepts_level(record.level()))
        {
            msg_log_file.write(message.as_str())?;
        }

        Ok(())
    }

    /// Flush the log files.
    fn flush(&self) -> std::io::Result<()> {
        for msg_log_file in self.msg_log_files.iter() {
            let _ = msg_log_file.flush();
        }

        Ok(())
    }
}

/// Defines a `MsgFileLoggerBuilder`.
///
/// Prepares a message logger that logs messages to files based on their levels.
pub struct MsgFileLoggerBuilder {
    log_writer: MsgLogFileWriter,
}

/// Methods of `MsgFileLoggerBuilder`.
impl MsgFileLoggerBuilder {
    /// Creates a new `MsgFileLoggerBuilder` instance.
    pub fn new() -> Self {
        MsgFileLoggerBuilder {
            log_writer: MsgLogFileWriter::new(),
        }
    }

    /// Adds a log file with accepted levels.
    pub fn add_log_file(mut self, accept: Vec<log::Level>, file_name: &str) -> Self {
        self.log_writer.add_log_file(file_name, accept);
        self
    }

    /// Creates a `MsgFileLogger` instance.
    pub fn build(self) -> MsgFileLogger {
        Logger::with(LevelFilter::Debug)
            .log_to_writer(Box::new(self.log_writer))
            .write_mode(WriteMode::Direct)
            .start()
            .unwrap();

        MsgFileLogger {}
    }
}

/// Impl of `Default` for `MsgFileLoggerBuilder`.
impl Default for MsgFileLoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Defines a `MsgFileLogger`.
///
/// A logger that logs messages to files based on their levels.
///
/// We don't want to log all messages, because it will flood the log file.
/// Only interesting messages should be logged.
pub struct MsgFileLogger {}

/// Impl of `MsgHandler` for `MsgFileLogger`.
impl MsgHandler for MsgFileLogger {
    /// Handles a `TaskInfo::Transferred` message.
    fn task_transferred(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        log::info!("{:?} : {}", rel_path, info);
    }

    /// Handles a `TaskInfo::Verified` message.
    fn task_verified(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        log::info!("{:?} : {}", rel_path, info);
    }

    /// Handles a `TaskMessage` with error.
    fn task_error(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        error: &(dyn Error + Send + Sync),
    ) {
        log::error!("{:?} : {}", rel_path, trace_error(error));
    }

    /// Handles a `CleanInfo::Removed` message.
    fn clean_removed(&self, rel_path: &UNPath<Rel>, info: &(dyn Info + Send + Sync)) {
        log::info!("{:?} : {}", rel_path, info);
    }

    /// Handles a `CleanMessage` with error.
    fn clean_error(&self, rel_path: &UNPath<Rel>, error: &(dyn Error + Send + Sync)) {
        log::error!("{:?} : {}", rel_path, trace_error(error));
    }

    /// Handles a `InfoMessage`.
    fn info(&self, info: &(dyn Info + Send + Sync)) {
        log::info!("{}", info);
    }

    /// Handles a `WarnMessage`.
    fn warn(&self, warning: &(dyn Info + Send + Sync)) {
        log::info!("{}", warning);
    }

    /// Handles a `ErrorMessage`.
    fn error(&self, error: &(dyn Error + Send + Sync)) {
        log::error!("{}", trace_error(error));
    }
}
