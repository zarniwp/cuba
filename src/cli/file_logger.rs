use crossbeam_channel::{Receiver, Sender, select, unbounded};
use flexi_logger::writers::LogWriter;
use flexi_logger::{DeferredNow, LoggerHandle};
use flexi_logger::{Logger, WriteMode};
use log::{LevelFilter, Record};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::shared::clean_message::{CleanInfo, CleanMessage};
use crate::shared::message::InfoMessage;
use crate::shared::message::Message;
use crate::shared::message::{ErrorMessage, WarnMessage};
use crate::shared::task_message::{TaskInfo, TaskMessage};

/// Trace error.
fn trace_error(err: &dyn std::error::Error) -> String {
    let mut msg = format!("{}", err);
    let mut source = err.source();

    while let Some(err) = source {
        msg.push_str(&format!("\nCaused by: {}", err));
        source = err.source();
    }

    msg
}

/// Defines a `LogFile`
struct LogFile {
    file: Mutex<std::fs::File>,
    log_levels: Vec<log::Level>,
}

/// Methods of `LogFile`.
impl LogFile {
    /// Creates a new `LogFile`.
    pub fn new(file_name: &str, log_levels: Vec<log::Level>) -> Self {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_name)
            .unwrap();

        LogFile {
            file: Mutex::new(file),
            log_levels,
        }
    }

    /// Check if the log file accepts the log level.
    pub fn accepts_level(&self, level: log::Level) -> bool {
        self.log_levels.contains(&level)
    }

    /// Write a message to the log file.
    pub fn write(&self, msg: &str) -> std::io::Result<()> {
        self.file.lock().unwrap().write_all(msg.as_bytes())
    }

    /// Flush the log file.
    pub fn flush(&self) -> std::io::Result<()> {
        self.file.lock().unwrap().flush()
    }
}

/// Defines a `LevelFileLogWriter`.
struct LevelFileLogWriter {
    log_files: Vec<LogFile>,
}

/// Defines methods of `LevelFileLogWriter`.
impl LevelFileLogWriter {
    /// Creates a new `LevelFileLogWriter`.
    pub fn new() -> Self {
        LevelFileLogWriter {
            log_files: Vec::new(),
        }
    }

    /// Adds a log file with accepted levels.
    pub fn add_log_file(&mut self, file_name: &str, log_levels: Vec<log::Level>) {
        self.log_files.push(LogFile::new(file_name, log_levels));
    }
}

impl LogWriter for LevelFileLogWriter {
    /// Write the log record.
    fn write(&self, _now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        let log_msg = format!("{} {}\n", record.level(), record.args());

        for log_file in self
            .log_files
            .iter()
            .filter(|log_file| log_file.accepts_level(record.level()))
        {
            log_file.write(log_msg.as_str())?;
        }

        Ok(())
    }

    /// Flush the log files.
    fn flush(&self) -> std::io::Result<()> {
        for log_file in self.log_files.iter() {
            let _ = log_file.flush();
        }

        Ok(())
    }
}

/// Defines a `LogBuilder`.
///
/// Prepares a logger that logs messages to files based on their levels.
pub struct LogBuilder {
    receiver: Arc<Receiver<Arc<dyn Message>>>,
    log_writer: LevelFileLogWriter,
}

/// Methods of `LogBuilder`.
impl LogBuilder {
    /// Creates a new `LogBuilder` instance.
    pub fn new(receiver: Arc<Receiver<Arc<dyn Message>>>) -> Self {
        LogBuilder {
            receiver,
            log_writer: LevelFileLogWriter::new(),
        }
    }

    /// Adds a log file with accepted levels.
    pub fn add_log_file(mut self, accept: Vec<log::Level>, file_name: &str) -> Self {
        self.log_writer.add_log_file(file_name, accept);
        self
    }

    /// Creates a logger instance.
    pub fn build(self) -> Log {
        Log {
            receiver: self.receiver,
            shutdown_sender: None,
            thread_handle: None,
            logger_handle: Some(
                Logger::with(LevelFilter::Debug)
                    .log_to_writer(Box::new(self.log_writer))
                    .write_mode(WriteMode::Direct)
                    .start()
                    .unwrap(),
            ),
        }
    }
}

/// Defines a `Log`.
///
/// A logger that logs messages to files based on their levels.
pub struct Log {
    receiver: Arc<Receiver<Arc<dyn Message>>>,
    shutdown_sender: Option<Sender<()>>,
    thread_handle: Option<JoinHandle<()>>,
    logger_handle: Option<LoggerHandle>,
}

/// Methods of `Log`.
impl Log {
    /// Initializes a new Log instance.
    pub fn start(&mut self) {
        let receiver = Arc::clone(&self.receiver);
        let (shutdown_sender, shutdown_receiver) = unbounded();
        self.shutdown_sender = Some(shutdown_sender);

        // Handle messages in a separate thread
        self.thread_handle = Some(thread::spawn(move || {
            loop {
                select! {
                    recv(receiver) -> message => {
                        if let Ok(message) = message {
                            if let Some(task_message) = message.as_ref().as_any().downcast_ref::<TaskMessage>() {
                                if let Some(info) = task_message.info() {
                                    if let Some(task_info) = info.as_any().downcast_ref::<TaskInfo>()
                                        && (task_info == &TaskInfo::Transferred || task_info == &TaskInfo::Verified) {
                                            log::info!("{:?} : {}", task_message.rel_path, task_info);
                                        }
                                }
                                else if let Some(err) = task_message.err() {
                                    log::error!("{:?} : {}", task_message.rel_path, trace_error(err));
                                }
                            }
                            else if let Some(clean_message) = message.as_ref().as_any().downcast_ref::<CleanMessage>() {
                                if let Some(info) = clean_message.info() {
                                    if let Some(clean_info) = info.as_any().downcast_ref::<CleanInfo>()
                                        && clean_info == &CleanInfo::Removed  {
                                            log::info!("{:?} : {}", clean_message.rel_path, info);
                                        }
                                }
                                else if let Some(err) = clean_message.err() {
                                    log::error!("{:?} : {}", clean_message.rel_path, trace_error(err));
                                }
                            }
                            else if let Some(info_message) = message.as_ref().as_any().downcast_ref::<InfoMessage>()
                                && let Some(info) = info_message.info() {
                                    log::info!("{}", info);
                                }
                            else if let Some(warn_message) = message.as_ref().as_any().downcast_ref::<WarnMessage>()
                                && let Some(info) = warn_message.info() {
                                    log::warn!("{}", info);
                                }
                            else if let Some(error_message) = message.as_ref().as_any().downcast_ref::<ErrorMessage>()
                                && let Some(err) = error_message.err() {
                                    log::error!("{}",  trace_error(err));
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

    /// Stops the logger.
    pub fn stop(&mut self) {
        if let Some(sender) = self.shutdown_sender.take() {
            thread::sleep(Duration::from_millis(100)); // Lets wait a little bit to receiver pending msgs.
            let _ = sender.send(()); // signal shutdown
        }

        if let Some(handle) = self.thread_handle.take() {
            // Wait for the thread to finish
            handle.join().unwrap();
        }

        if let Some(logger_handle) = self.logger_handle.take() {
            logger_handle.flush();
        }
    }
}
