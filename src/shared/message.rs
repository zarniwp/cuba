use std::{
    any::Any,
    error::Error,
    fmt::{self, Display, Formatter},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use crossbeam_channel::{Receiver, Sender, unbounded};

/// Defines a trait for an `Info`.
pub trait Info: fmt::Debug + fmt::Display + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

/// Defines a `StringInfo`.
///
/// # Example
/// ```
/// use cuba::shared::message::StringInfo;
///
/// let str_info = StringInfo::new("My Info".to_string());
/// ```
#[derive(Debug)]
pub struct StringInfo {
    message: String,
}

/// Methods of `StringInfo`.
impl StringInfo {
    /// Creates a new instance of `StringInfo`.    
    pub fn new(message: String) -> Self {
        StringInfo { message }
    }
}

/// Impl of `Info` for `StringInfo`.
impl Info for StringInfo {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Impl of `Display` for `StringInfo`.
impl Display for StringInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Defines a trait for a `Message`.
pub trait Message: fmt::Display + Send + Sync {
    fn err(&self) -> Option<&(dyn Error + Send + Sync)>;
    fn info(&self) -> Option<&(dyn Info + Send + Sync)>;
    fn as_any(&self) -> &dyn Any;
}

/// Defines an `InfoMessage`.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use cuba::shared::message::{StringInfo, InfoMessage};
///
/// let str_info = StringInfo::new("My Info".to_string());
/// let info_message = InfoMessage::new(Arc::new(str_info));
/// ```
pub struct InfoMessage {
    info: Arc<dyn Info + Send + Sync>,
}

/// Methods of `InfoMessage`.
impl InfoMessage {
    /// Creates a new instance of `InfoMessage`.
    #[allow(dead_code)]
    pub fn new(info: Arc<dyn Info>) -> Self {
        InfoMessage { info }
    }
}

/// Impl of `Message` for `InfoMessage`.
impl Message for InfoMessage {
    fn err(&self) -> Option<&(dyn Error + Send + Sync)> {
        None
    }

    fn info(&self) -> Option<&(dyn Info + Send + Sync)> {
        Some(&*self.info)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Impl of `Display` for `InfoMessage`.
impl Display for InfoMessage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "Info : {}", self.info)
    }
}

/// Defines an `WarnMessage`.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use cuba::shared::message::{StringInfo, WarnMessage};
///
/// let str_info = StringInfo::new("My Warning".to_string());
/// let info_message = WarnMessage::new(Arc::new(str_info));
/// ```
pub struct WarnMessage {
    warning: Arc<dyn Info + Send + Sync>,
}

/// Methods of `WarnMessage`.
impl WarnMessage {
    /// Creates a new instance of `WarnMessage`.
    #[allow(dead_code)]
    pub fn new(warning: Arc<dyn Info>) -> Self {
        WarnMessage { warning }
    }
}

/// Impl of `Message` for `WarnMessage`.
impl Message for WarnMessage {
    fn err(&self) -> Option<&(dyn Error + Send + Sync)> {
        None
    }

    fn info(&self) -> Option<&(dyn Info + Send + Sync)> {
        Some(&*self.warning)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Impl of `Display` for `WarnMessage`.
impl Display for WarnMessage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "Warning : {}", self.warning)
    }
}

/// Defines a `StringError`.
///
/// # Example
/// ```
/// use cuba::shared::message::StringError;
///
/// let str_err = StringError::new("My Error".to_string());
/// ```
#[derive(Debug, Clone)]
pub struct StringError {
    message: String,
}

/// Methods of `StringError`.
impl StringError {
    /// Creates a new instance of `StringError`.
    pub fn new(message: String) -> Self {
        StringError { message }
    }
}

/// Impl of `Error` for `StringError`.
impl Error for StringError {}

/// Impl of `Display` for `StringError`.
impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Defines an `ErrorMessage`.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use std::io;
/// use cuba::shared::message::{StringError, ErrorMessage};
///
/// let io_err = io::Error::new(io::ErrorKind::Other, "Disk full");
/// let str_err = StringError::new("My Error".to_string());
///
/// let io_err_message = ErrorMessage::new(Arc::new(io_err));
/// let str_err_message = ErrorMessage::new(Arc::new(str_err));
/// ```
pub struct ErrorMessage {
    error: Arc<dyn Error + Send + Sync>,
}

/// Defines methods of `ErrorMessage`.
impl ErrorMessage {
    /// Creates a new instance of `ErrorMessage`.
    pub fn new(error: Arc<dyn Error + Send + Sync>) -> Self {
        ErrorMessage { error }
    }
}

/// Impl of `Message` for `ErrorMessage`.
impl Message for ErrorMessage {
    fn err(&self) -> Option<&(dyn Error + Send + Sync)> {
        Some(&*self.error)
    }

    fn info(&self) -> Option<&(dyn Info + Send + Sync)> {
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Impl of `Display` for `ErrorMessage`.
impl Display for ErrorMessage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "Error : {}", self.error)
    }
}

/// A macro for sending a string info.
#[macro_export]
macro_rules! send_info {
    ($sender:expr, $($arg:tt)*) => {{
        use std::sync::Arc;
        use $crate::shared::message::{InfoMessage, StringInfo};
        let info = Arc::new(StringInfo::new(format!($($arg)*)));
        let msg = Arc::new(InfoMessage::new(info));
        $sender.send(msg).unwrap();
    }};
}

/// A macro for sending a warning.
#[macro_export]
macro_rules! send_warn {
    ($sender:expr, $($arg:tt)*) => {{
        use std::sync::Arc;
        use $crate::shared::message::{WarnMessage, StringInfo};
        let info = Arc::new(StringInfo::new(format!($($arg)*)));
        let msg = Arc::new(WarnMessage::new(info));
        $sender.send(msg).unwrap();
    }};
}

/// A macro for sending warnings.
#[macro_export]
macro_rules! send_warns {
    ($sender:expr, $vec:expr) => {{
        for warning in &$vec {
            $crate::send_warn!($sender, "{}", warning);
        }
    }};
}

/// A macro for sending an error.
#[macro_export]
macro_rules! send_error {
    ($sender:expr, $err:expr) => {{
        use std::sync::Arc;
        use $crate::shared::message::ErrorMessage; // Adjust path if needed
        let msg = Arc::new(ErrorMessage::new(Arc::new($err)));
        $sender.send(msg).unwrap();
    }};
}

/// Defines a `MsgDispatcher`.
///
/// Sends messages from a source to all subscribers.
pub struct MsgDispatcher<T: Send + Sync + Clone + 'static> {
    source: Arc<Receiver<T>>,
    receivers: Arc<Mutex<Vec<Sender<T>>>>,
    shutdown_sender: Option<Sender<()>>,
    thread_handle: Option<JoinHandle<()>>,
}

/// Methods of `MsgDispatcher`.
impl<T: Send + Sync + Clone + 'static> MsgDispatcher<T> {
    /// Creates a `MsgDispatcher`. Receives messages from source and sends them to the
    /// subscribed receivers.
    pub fn new(source: Arc<Receiver<T>>) -> Self {
        Self {
            source,
            receivers: Arc::new(Mutex::new(Vec::new())),
            shutdown_sender: None,
            thread_handle: None,
        }
    }

    /// Returns a subscribed message receiver.
    pub fn subscribe(&self) -> Receiver<T> {
        let (sender, receiver) = unbounded();
        self.receivers.lock().unwrap().push(sender);
        receiver
    }

    /// Starts the `MsgDispatcher`.
    pub fn start(&mut self) {
        let source = Arc::clone(&self.source);

        let receivers = Arc::clone(&self.receivers);
        let (shutdown_sender, shutdown_receiver) = unbounded();
        self.shutdown_sender = Some(shutdown_sender);

        self.thread_handle = Some(thread::spawn(move || {
            loop {
                crossbeam_channel::select! {
                    recv(source) -> msg => {
                        match msg {
                            Ok(value) => {
                                let mut lock = receivers.lock().unwrap();
                                lock.retain(|s| s.send(value.clone()).is_ok());
                            }
                            Err(_) => break, // source closed.
                        }
                    }
                    recv(shutdown_receiver) -> _ => break,
                }
            }
        }));
    }

    /// Stops the `MsgDispatcher`.
    pub fn stop(&mut self) {
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(()); // signal shutdown.
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join(); // Wait for thread to finish.
        }
    }
}
