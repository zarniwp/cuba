use crossbeam_channel::Sender;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use crate::core::run_state::RunState;
use crate::send_error;
use crate::shared::message::Message;
use crate::shared::npath::Dir;
use crate::shared::npath::File;
use crate::shared::npath::NPath;
use crate::shared::npath::Rel;
use crate::shared::npath::Symlink;
use crate::shared::npath::UNPath;
use crate::shared::progress_message::ProgressInfo;
use crate::shared::progress_message::ProgressMessage;

use super::cuba_json::read_cuba_json;
use super::fs::fs_base::FSConnection;
use super::glob_matcher::ExcludeMatcher;
use super::glob_matcher::GlobMatcher;
use super::glob_matcher::IncludeMatcher;
use super::password_cache::PasswordCache;
use super::tasks::directory_restore_task::directory_restore_task;
use super::tasks::file_restore_task::file_restore_task;
use super::tasks::symlink_restore_task::symlink_restore_task;
use super::tasks::task_worker::TaskWorker;
use super::transferred_node::Restore;
use super::util::move_rel_npaths;

#[allow(clippy::too_many_arguments)]
pub fn run_restore(
    run_state: Arc<RunState>,
    threads: usize,
    include_patterns: &Option<Vec<String>>,
    exclude_patterns: &Option<Vec<String>>,
    fs_conn: FSConnection,
    sender: Sender<Arc<dyn Message>>,
) {
    // Set running to true.
    run_state.start();

    let mut include_matcher: Option<IncludeMatcher> = None;
    let mut exclude_matcher: Option<ExcludeMatcher> = None;

    // Create include matcher.
    if let Some(include_patterns) = include_patterns {
        include_matcher = match GlobMatcher::new(include_patterns) {
            Ok(matcher) => Some(matcher.include_matcher()),
            Err(err) => {
                send_error!(sender, err);
                return;
            }
        }
    };

    // Create exclude matcher.
    if let Some(exclude_patterns) = exclude_patterns {
        exclude_matcher = match GlobMatcher::new(exclude_patterns) {
            Ok(matcher) => Some(matcher.exclude_matcher()),
            Err(err) => {
                send_error!(sender, err);
                return;
            }
        }
    };

    // Open connection.
    if let Err(err) = fs_conn.open() {
        send_error!(sender, err);
        return;
    }

    // Read cuba json.
    let transferred_nodes_read = match read_cuba_json(&fs_conn.src_mnt, &sender) {
        Some(nodes) => nodes,
        None => return,
    };

    // Collect source files, directories and symlinks.
    let mut src_rel_files: VecDeque<NPath<Rel, File>> = VecDeque::new();
    let mut src_rel_directories: VecDeque<NPath<Rel, Dir>> = VecDeque::new();
    let mut src_rel_symlinks: VecDeque<NPath<Rel, Symlink>> = VecDeque::new();

    for src_rel_path in transferred_nodes_read.view::<Restore>().iter_src_nodes() {
        let mut included = true;
        let mut excluded = false;

        if let Some(ref matcher) = include_matcher {
            // Note: a include matcher does include all predecessor directories of a glob statement.
            included = matcher.is_match(src_rel_path);
        }

        if let Some(ref matcher) = exclude_matcher {
            excluded = matcher.is_match(src_rel_path);
        }

        if included && !excluded {
            match &src_rel_path {
                UNPath::File(rel_file_path) => {
                    src_rel_files.push_back(rel_file_path.clone());
                }
                UNPath::Dir(rel_dir_path) => {
                    src_rel_directories.push_back(rel_dir_path.clone());
                }
                UNPath::Symlink(rel_sym_path) => {
                    src_rel_symlinks.push_back(rel_sym_path.clone());
                }
            }
        }
    }

    // Create password cache.
    let password_cache = PasswordCache::new();

    // Create arcs for tasks.
    let arc_mutex_src_rel_files = Arc::new(Mutex::new(src_rel_files));
    let arc_mutex_src_rel_symlinks = Arc::new(Mutex::new(src_rel_symlinks));
    let arc_transferred_nodes_read = Arc::new(transferred_nodes_read);
    let arc_mutex_password_cache = Arc::new(Mutex::new(password_cache));

    // Init task worker.
    let task_worker = TaskWorker::new(fs_conn.clone(), sender.clone());

    // Progress duration.
    let items = src_rel_directories.len()
        + arc_mutex_src_rel_files.lock().unwrap().len()
        + arc_mutex_src_rel_symlinks.lock().unwrap().len();
    sender
        .send(Arc::new(ProgressMessage::new(
            Arc::new(ProgressInfo::Duration),
            items as u64,
        )))
        .unwrap();

    // We can not process dir list parallel, because if dir A is subdir of dir B: B must be processed before A.
    // But we can process all dirs of the same depth parallel.
    let mut depth = 1;

    while !src_rel_directories.is_empty() {
        let mut depth_src_rel_dirs: VecDeque<NPath<Rel, Dir>> = VecDeque::new();

        move_rel_npaths(&mut src_rel_directories, &mut depth_src_rel_dirs, depth);
        let depth_threads: usize = std::cmp::min(threads, depth_src_rel_dirs.len());

        if !depth_src_rel_dirs.is_empty() {
            let arc_mutex_depth_src_rel_dirs = Arc::new(Mutex::new(depth_src_rel_dirs));

            // Run directory restore.
            task_worker.run(
                run_state.clone(),
                depth_threads,
                Arc::new(directory_restore_task(arc_mutex_depth_src_rel_dirs)),
            );
        }

        depth += 1;
    }

    // Run file restore.
    task_worker.run(
        run_state.clone(),
        threads,
        Arc::new(file_restore_task(
            arc_mutex_src_rel_files,
            arc_transferred_nodes_read.clone(),
            arc_mutex_password_cache.clone(),
        )),
    );

    // Run symlink restore.
    task_worker.run(
        run_state.clone(),
        threads,
        Arc::new(symlink_restore_task(
            arc_mutex_src_rel_symlinks,
            arc_transferred_nodes_read.clone(),
        )),
    );

    // Drop task worker.
    drop(task_worker);

    // Close connection.
    if let Err(err) = fs_conn.close() {
        send_error!(sender, err);
    }

    // Set running to false.
    run_state.stop();
}
