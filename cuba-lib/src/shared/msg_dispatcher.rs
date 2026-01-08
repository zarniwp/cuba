use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use crossbeam_channel::{Receiver, Sender, unbounded};

/// Defines a `MsgDispatcher`.
///
/// Sends messages from a source to all subscribers.
pub struct MsgDispatcher<T: Send + Sync + Clone + 'static> {
    source: Receiver<T>,
    receivers: Arc<Mutex<Vec<Sender<T>>>>,
    shutdown_sender: Option<Sender<()>>,
    thread_handle: Option<JoinHandle<()>>,
}

/// Methods of `MsgDispatcher`.
impl<T: Send + Sync + Clone + 'static> MsgDispatcher<T> {
    /// Creates a `MsgDispatcher`.
    /// Receives messages from source and sends them to the
    /// subscribed receivers.
    pub fn new(source: Receiver<T>) -> Self {
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
        let source = self.source.clone();

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
                                lock.retain(|sender| sender.send(value.clone()).is_ok());
                            }
                            Err(_) => break, // Source closed.
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
            // Signal shutdown.
            let _ = sender.send(());
        }

        if let Some(handle) = self.thread_handle.take() {
            // Wait for thread to finish.
            let _ = handle.join();
        }
    }
}
