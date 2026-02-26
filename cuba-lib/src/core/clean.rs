use crossbeam_channel::Sender;
use std::sync::Arc;

use crate::core::run_state::RunState;
use crate::core::transferred_node::Backup;
use crate::send_error;
use crate::shared::clean_message::{CleanError, CleanInfo, CleanMessage};
use crate::shared::message::Message;
use crate::shared::npath::{Abs, Rel, UNPath};
use crate::shared::progress_message::ProgressInfo;
use crate::shared::progress_message::ProgressMessage;

use super::cuba_json::CUBA_JSON_REL_PATH;
use super::cuba_json::read_cuba_json;
use super::cuba_json::write_cuba_json;
use super::fs::fs_base::FSMount;
use super::transferred_node::{Flags, MaskedFlags, Restore, TransferredNodes};

/// Runs the clean process.
/// 
/// Clean means to synchronize the backup with the source, this means in detail:
/// - Files/directories that are not in the backup index are deleted from the backup
/// - Files/directories/symlinks that are marked as ophans (not in the source anymore) are 
///   deleted from the backup
pub fn run_clean(run_state: Arc<RunState>, fs_mnt: FSMount, sender: Sender<Arc<dyn Message>>) {
    // Set running to true.
    run_state.start();

    // Connect fs.
    if let Err(err) = fs_mnt.fs.write().unwrap().connect() {
        send_error!(sender, err);
        return;
    }

    // Read cuba json.
    let transferred_nodes_read = match read_cuba_json(&fs_mnt, &sender) {
        Some(nodes) => nodes,
        None => return,
    };

    // Create the transferred nodes write
    let mut transferred_nodes_write = TransferredNodes::new();

    // Make clean flags.
    let clean_flags: MaskedFlags = MaskedFlags::new()
        .with_flags(Flags::ORPHAN)
        .with_mask(Flags::ORPHAN);

    // Progress duration.
    sender
        .send(Arc::new(ProgressMessage::new(
            Arc::new(ProgressInfo::Duration),
            transferred_nodes_read.node_count() as u64,
        )))
        .unwrap();

    // Symlinks do not exist as backup files, so we have to treat them in a different way.
    for (src_rel_path, transferred_node) in transferred_nodes_read.iter() {
        // If symlink and clean flags do not match, keep the symlink.
        if transferred_node.src_symlink_meta.is_some()
            && !clean_flags.matches(transferred_node.flags)
        {
            transferred_nodes_write
                .view_mut::<Backup>()
                .set_transferred_node(src_rel_path, transferred_node);

            // Progress tick.
            sender
                .send(Arc::new(ProgressMessage::new(
                    Arc::new(ProgressInfo::Ticks),
                    1,
                )))
                .unwrap();
        }
    }

    fs_mnt
        .fs
        .read()
        .unwrap()
        .walk_dir_rec(
            &fs_mnt.abs_dir_path,
            &mut |abs_path| {
                // Progress tick.
                sender
                    .send(Arc::new(ProgressMessage::new(
                        Arc::new(ProgressInfo::Ticks),
                        1,
                    )))
                    .unwrap();

                if run_state.is_canceled() {
                    false
                } else {
                    match abs_path.sub_abs_dir(&fs_mnt.abs_dir_path) {
                        Ok(node_rel_path) => {
                            if let Some(transferred_node) = transferred_nodes_read
                                .view::<Restore>()
                                .get_node_for_src(&node_rel_path)
                            {
                                if clean_flags.matches(transferred_node.flags) {
                                    // If flags match (ophan flag) remove the node.
                                    return remove_node(
                                        &abs_path,
                                        &node_rel_path,
                                        fs_mnt.clone(),
                                        sender.clone(),
                                    );
                                } else {
                                    sender
                                        .send(Arc::new(CleanMessage::new(
                                            &node_rel_path,
                                            None,
                                            Some(Arc::new(CleanInfo::Ok)),
                                        )))
                                        .unwrap();

                                    if let Some(dest_rel_path) = transferred_nodes_read
                                        .view::<Restore>()
                                        .get_dest_rel_path(transferred_node)
                                    {
                                        transferred_nodes_write
                                            .view_mut::<Restore>()
                                            .set_transferred_node(&dest_rel_path, transferred_node);
                                    }

                                    return true;
                                }
                            } else {
                                // If node not in backup index, remove node.
                                return remove_node(
                                    &abs_path,
                                    &node_rel_path,
                                    fs_mnt.clone(),
                                    sender.clone(),
                                );
                            }
                        }
                        Err(err) => {
                            send_error!(sender, err);
                        }
                    }

                    true
                }
            },
            &|err| send_error!(sender, err),
        )
        .unwrap();

    if !run_state.is_canceled() {
        // Write cuba json.
        write_cuba_json(&fs_mnt, &transferred_nodes_write, &sender);
    }

    // Disconnect fs.
    if let Err(err) = fs_mnt.fs.write().unwrap().disconnect() {
        send_error!(sender, err);
    }

    // Set running to false.
    run_state.stop();
}

/// Removes a node.
fn remove_node(
    abs_path: &UNPath<Abs>,
    rel_path: &UNPath<Rel>,
    fs_mnt: FSMount,
    sender: Sender<Arc<dyn Message>>,
) -> bool {
    match abs_path {
        UNPath::File(abs_file_path) => {
            if !abs_file_path.ends_with(&CUBA_JSON_REL_PATH.clone()) {
                if fs_mnt.fs.read().unwrap().remove_file(abs_file_path).is_ok() {
                    sender
                        .send(Arc::new(CleanMessage::new(
                            rel_path,
                            None,
                            Some(Arc::new(CleanInfo::Removed)),
                        )))
                        .unwrap();
                } else {
                    sender
                        .send(Arc::new(CleanMessage::new(
                            rel_path,
                            Some(Arc::new(CleanError::RemoveFailed)),
                            None,
                        )))
                        .unwrap();
                }
            }

            true
        }
        UNPath::Dir(abs_dir_path) => {
            if fs_mnt.fs.read().unwrap().remove_dir(abs_dir_path).is_ok() {
                sender
                    .send(Arc::new(CleanMessage::new(
                        rel_path,
                        None,
                        Some(Arc::new(CleanInfo::Removed)),
                    )))
                    .unwrap();

                // Do not walk into the directory.
                false
            } else {
                sender
                    .send(Arc::new(CleanMessage::new(
                        rel_path,
                        Some(Arc::new(CleanError::RemoveFailed)),
                        None,
                    )))
                    .unwrap();

                // Do not walk into the directory.
                false
            }
        }
        UNPath::Symlink(_abs_sym_path) => true,
    }
}
