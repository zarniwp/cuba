use crossbeam_channel::Sender;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use crate::core::run_state::RunState;
use crate::send_error;
use crate::shared::message::Message;
use crate::shared::npath::Rel;
use crate::shared::npath::UNPath;
use crate::shared::progress_message::ProgressInfo;
use crate::shared::progress_message::ProgressMessage;

use super::cuba_json::read_cuba_json;
use super::cuba_json::write_cuba_json;
use super::fs::fs_base::FSConnection;
use super::fs::fs_base::FSMount;
use super::password_cache::PasswordCache;
use super::tasks::node_verify_task::node_verify_task;
use super::tasks::task_worker::TaskWorker;
use super::transferred_node::Flags;
use super::transferred_node::MaskedFlags;
use super::transferred_node::MatchMode;
use super::transferred_node::Restore;

/// Runs the verify process.
pub fn run_verify(
    run_state: Arc<RunState>,
    threads: usize,
    fs_mnt: FSMount,
    verify_all: bool,
    sender: Sender<Arc<dyn Message>>,
) {
    // Set running to true.
    run_state.start();

    // Create connection.
    let fs_conn = FSConnection {
        src_mnt: fs_mnt,
        dest_mnt: FSMount::dev_null(),
    };

    // Open connection.
    if let Err(err) = fs_conn.open() {
        send_error!(sender, err);
        return;
    }

    // Read cuba json.
    let transferred_nodes = match read_cuba_json(&fs_conn.src_mnt, &sender) {
        Some(nodes) => nodes,
        None => return,
    };

    // Collect source directories and files.
    let mut src_rel_nodes: VecDeque<UNPath<Rel>> = VecDeque::new();

    for src_rel_path in transferred_nodes.view::<Restore>().iter_src_nodes() {
        src_rel_nodes.push_back(src_rel_path.clone());
    }

    // Create password cache.
    let password_cache = PasswordCache::new();

    // Create arcs for tasks.
    let arc_mutex_src_rel_nodes = Arc::new(Mutex::new(src_rel_nodes));
    let arc_rwlock_transferred_nodes = Arc::new(RwLock::new(transferred_nodes));
    let arc_mutex_password_cache = Arc::new(Mutex::new(password_cache));

    // Init task worker.
    let task_worker = TaskWorker::new(fs_conn.clone(), sender.clone());

    // Progress duration.
    let items = arc_mutex_src_rel_nodes.lock().unwrap().len();
    sender
        .send(Arc::new(ProgressMessage::new(
            Arc::new(ProgressInfo::Duration),
            items as u64,
        )))
        .unwrap();

    // Init verify flags.
    let mut verify_flags: MaskedFlags = MaskedFlags::new();

    if !verify_all {
        verify_flags = verify_flags
            .with_mode(MatchMode::Uq)
            .with_flags(Flags::VERIFIED)
            .with_mask(Flags::VERIFIED | Flags::VERIFY_ERROR);
    }

    // Run file verfiy.
    task_worker.run(
        run_state.clone(),
        threads,
        Arc::new(node_verify_task(
            arc_mutex_src_rel_nodes,
            arc_rwlock_transferred_nodes.clone(),
            verify_flags,
            arc_mutex_password_cache.clone(),
        )),
    );

    // Drop task worker.
    drop(task_worker);

    if !run_state.is_canceled() {
        // Write cuba json.
        write_cuba_json(
            &fs_conn.src_mnt,
            &arc_rwlock_transferred_nodes.read().unwrap(),
            &sender,
        );
    }

    // Close connection.
    if let Err(err) = fs_conn.close() {
        send_error!(sender, err);
    }

    // Set running to false.
    run_state.stop();
}
