use crossbeam_channel::Sender;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

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
use super::cuba_json::write_cuba_json;
use super::fs::fs_base::FSConnection;
use super::glob_matcher::ExcludeMatcher;
use super::glob_matcher::GlobMatcher;
use super::glob_matcher::IncludeMatcher;
use super::password_cache::PasswordCache;
use super::tasks::directory_backup_task::directory_backup_task;
use super::tasks::file_backup_task::file_backup_task;
use super::tasks::symlink_backup_task::symlink_backup_task;
use super::tasks::task_worker::TaskWorker;
use super::transferred_node::Flags;
use super::transferred_node::MaskedFlags;
use super::util::move_rel_npaths;

#[allow(clippy::too_many_arguments)]
/// Runs the backup process.
pub fn run_backup(
    run_state: Arc<RunState>,
    threads: usize,
    compression: bool,
    encrypt: bool,
    password_id: &Option<String>,
    include_patterns: &Option<Vec<String>>,
    exclude_patterns: &Option<Vec<String>>,
    fs_conn: &FSConnection,
    sender: Sender<Arc<dyn Message>>,
) {
    // Set running to true.
    run_state.start();

    let mut include_matcher: Option<IncludeMatcher> = None;
    let mut exclude_matcher: Option<ExcludeMatcher> = None;

    // Create include matcher.
    if let Some(include_patterns) = include_patterns {
        include_matcher = match GlobMatcher::new(include_patterns) {
            // Note: a include matcher does include all predecessor directories of a glob statement.
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
    let mut transferred_nodes = read_cuba_json(&fs_conn.dest_mnt, &sender).unwrap_or_default();

    // Collect source files, directories and symlinks.
    let mut src_rel_files: VecDeque<NPath<Rel, File>> = VecDeque::new();
    let mut src_rel_directories: VecDeque<NPath<Rel, Dir>> = VecDeque::new();
    let mut src_rel_symlinks: VecDeque<NPath<Rel, Symlink>> = VecDeque::new();

    fs_conn
        .src_mnt
        .fs
        .read()
        .unwrap()
        .walk_dir_rec(
            &fs_conn.src_mnt.abs_dir_path,
            &mut |abs_path| {
                let mut included = true;
                let mut excluded = false;

                match abs_path.sub_abs_dir(&fs_conn.src_mnt.abs_dir_path) {
                    Ok(rel_path) => {
                        if let Some(ref matcher) = include_matcher {
                            included = matcher.is_match(&rel_path);
                        }

                        if let Some(ref matcher) = exclude_matcher {
                            excluded = matcher.is_match(&rel_path);
                        }

                        if included && !excluded {
                            match &rel_path {
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
                    Err(err) => {
                        send_error!(sender, err);
                    }
                }

                included && !excluded
            },
            &|err| send_error!(sender, err),
        )
        .unwrap();

    // Before backup, set all nodes to be an orphan.
    transferred_nodes.insert_flags(Flags::ORPHAN);

    // Create password cache.
    let password_cache = PasswordCache::new();

    let arc_mutex_src_rel_files = Arc::new(Mutex::new(src_rel_files));
    let arc_mutex_src_rel_symlinks = Arc::new(Mutex::new(src_rel_symlinks));
    let arc_rwlock_transferred_nodes = Arc::new(RwLock::new(transferred_nodes));
    let arc_mutex_password_cache = Arc::new(Mutex::new(password_cache));

    // Init task worker.
    let task_worker = TaskWorker::new(fs_conn.clone(), sender.clone());

    // Init dir backup flags.
    let dir_backup_flags: MaskedFlags = MaskedFlags::new().with_mask(Flags::VERIFY_ERROR);

    // Init file backup flags.
    let mut file_backup_flags: MaskedFlags =
        MaskedFlags::new().with_mask(Flags::COMPRESSED | Flags::ENCRYPTED | Flags::VERIFY_ERROR);

    // Init symlink backup flags.
    let sym_backup_flags: MaskedFlags = MaskedFlags::new().with_mask(Flags::VERIFY_ERROR);

    // Is compression true?
    if compression {
        // Set flag.
        file_backup_flags.insert(Flags::COMPRESSED);
    }

    // Is encryption is true?
    if encrypt {
        // Set flag.
        file_backup_flags.insert(Flags::ENCRYPTED);
    }

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

    // We cannot process dir list parallel, because if dir A is subdir of dir B: B must be processed before A.
    // But we can process all dirs of the same depth parallel.
    let mut depth = 1;

    while !src_rel_directories.is_empty() {
        let mut depth_src_rel_dirs: VecDeque<NPath<Rel, Dir>> = VecDeque::new();

        move_rel_npaths(&mut src_rel_directories, &mut depth_src_rel_dirs, depth);
        let depth_threads: usize = std::cmp::min(threads, depth_src_rel_dirs.len());

        if !depth_src_rel_dirs.is_empty() {
            let arc_mutex_depth_src_rel_dirs = Arc::new(Mutex::new(depth_src_rel_dirs));

            // Run directory backup.
            task_worker.run(
                run_state.clone(),
                depth_threads,
                Arc::new(directory_backup_task(
                    arc_mutex_depth_src_rel_dirs,
                    arc_rwlock_transferred_nodes.clone(),
                    dir_backup_flags,
                )),
            );
        }

        depth += 1;
    }

    // Run file backup.
    task_worker.run(
        run_state.clone(),
        threads,
        Arc::new(file_backup_task(
            arc_mutex_src_rel_files,
            arc_rwlock_transferred_nodes.clone(),
            file_backup_flags,
            arc_mutex_password_cache.clone(),
            password_id.clone(),
        )),
    );

    // Run symlink backup.
    task_worker.run(
        run_state.clone(),
        threads,
        Arc::new(symlink_backup_task(
            arc_mutex_src_rel_symlinks,
            arc_rwlock_transferred_nodes.clone(),
            sym_backup_flags,
        )),
    );

    // Drop task worker.
    drop(task_worker);

    if !run_state.is_canceled() {
        // Write cuba json.
        write_cuba_json(
            &fs_conn.dest_mnt,
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
