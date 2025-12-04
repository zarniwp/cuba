use crossbeam_channel::Sender;
use std::error::Error;
use std::sync::Arc;
use std::thread;
use trait_set::trait_set;

use crate::shared::message::Info;
use crate::shared::message::Message;
use crate::shared::npath::Rel;
use crate::shared::npath::UNPath;
use crate::shared::task_message::TaskMessage;

use super::super::fs::fs_base::FSConnection;

trait_set! {
    pub trait TaskErrorFn = Fn(&UNPath<Rel>, Arc<dyn Error + Send + Sync>) -> Arc<TaskMessage>;
    pub trait TaskInfoFn = Fn(&UNPath<Rel>, Arc<dyn Info + Send + Sync>) -> Arc<TaskMessage>;
    pub trait Task =
    Fn(
        &dyn TaskErrorFn,
        &dyn TaskInfoFn,
        FSConnection,
        Sender<Arc<dyn Message>>
    ) -> bool
    + Send
    + Sync
    + 'static
}

/// Defines the `TaskWorker`.
///
/// A struct representing the task worker.
pub struct TaskWorker {
    fs_conn: FSConnection,
    sender: Sender<Arc<dyn Message>>,
}

/// Methods of `TaskWorker`.
impl TaskWorker {
    /// Creates a new instance of `TaskWorker`.
    pub fn new(fs_conn: FSConnection, sender: Sender<Arc<dyn Message>>) -> Self {
        Self { fs_conn, sender }
    }

    /// Run function.
    pub fn run(&self, threads: usize, task: Arc<dyn Task>) {
        let mut handles: Vec<thread::JoinHandle<()>> = vec![];

        for thread_number in 0..threads {
            let fs = self.fs_conn.clone();
            let sender: Sender<Arc<dyn Message>> = self.sender.clone();
            let task: Arc<dyn Task> = Arc::clone(&task);

            let handle: thread::JoinHandle<()> = thread::spawn(move || {
                let mut processing: bool = true;

                let create_task_error_message =
                    move |rel_path: &UNPath<Rel>, error: Arc<dyn Error + Send + Sync>| {
                        Arc::new(TaskMessage::new(thread_number, rel_path, Some(error), None))
                    };

                let create_task_info_message =
                    move |rel_path: &UNPath<Rel>, info: Arc<dyn Info + Send + Sync>| {
                        Arc::new(TaskMessage::new(thread_number, rel_path, None, Some(info)))
                    };

                while processing {
                    processing = task(
                        &create_task_error_message,
                        &create_task_info_message,
                        fs.clone(),
                        sender.clone(),
                    );
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
