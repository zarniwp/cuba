use std::sync::atomic::{AtomicBool, Ordering};

/// Defines the `RunState`.
pub struct RunState {
    canceled: AtomicBool,
    running: AtomicBool,
}

/// Methods of `RunState`.
impl RunState {
    /// Creates a new `RunState`.
    pub fn new() -> Self {
        Self {
            canceled: AtomicBool::new(false),
            running: AtomicBool::new(false),
        }
    }

    /// Starts the a run.
    pub fn start(&self) {
        self.canceled.store(false, Ordering::SeqCst);
        self.running.store(true, Ordering::SeqCst);
    }

    /// Stops a run.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst)
    }

    /// Requests a cancel.
    pub fn request_cancel(&self) {
        self.canceled.store(true, Ordering::SeqCst);
    }

    /// Returns true if a cancel was requested.
    pub fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::SeqCst)
    }

    /// Returns true if a run is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Impl of `Default` for `RunState`.
impl Default for RunState {
    fn default() -> Self {
        Self::new()
    }
}
