use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use crossbeam_channel::Sender;

use crate::shared::message::Info;
use crate::shared::message::Message;
use crate::shared::npath::Abs;
use crate::shared::npath::NPath;
use crate::shared::npath::Rel;
use crate::shared::npath::Symlink;
use crate::shared::task_message::TaskInfo;

use super::super::fs::fs_base::FSConnection;
use super::super::transferred_node::Backup;
use super::super::transferred_node::Flags;
use super::super::transferred_node::MaskedFlags;
use super::super::transferred_node::TransferredNode;
use super::super::transferred_node::TransferredNodes;

use super::task_helpers::exit_task_and_continue;
use super::task_helpers::task_handle_error;
use super::task_worker::Task;
use super::task_worker::TaskErrorFn;
use super::task_worker::TaskInfoFn;

/// Task for backup the symlinks.
pub fn symlink_backup_task(
    src_rel_symlinks: Arc<Mutex<VecDeque<NPath<Rel, Symlink>>>>,
    transferred_nodes: Arc<RwLock<TransferredNodes>>,
    backup_flags: MaskedFlags,
) -> impl Task {
    move |create_task_error_msg: &dyn TaskErrorFn,
          create_task_info_msg: &dyn TaskInfoFn,
          fs_conn: FSConnection,
          sender: Sender<Arc<dyn Message>>| {
        // Pop the first element.
        let src_rel_symlink_element = src_rel_symlinks.lock().unwrap().pop_front();

        // Process if valid element.
        if let Some(src_rel_sym_path) = src_rel_symlink_element {
            // Make task messages with fixed path.
            let create_task_error_msg = |error: Arc<dyn Error + Send + Sync>| {
                create_task_error_msg(&src_rel_sym_path.clone().into(), error)
            };
            let create_task_info_msg = |info: Arc<dyn Info + Send + Sync>| {
                create_task_info_msg(&src_rel_sym_path.clone().into(), info)
            };

            // Task started.
            sender
                .send(create_task_info_msg(Arc::new(TaskInfo::Start)))
                .unwrap();

            // Create absolut path to the src symlink.
            let src_abs_sym_path: NPath<Abs, Symlink> = fs_conn
                .src_mnt
                .abs_dir_path
                .add_rel_symlink(&src_rel_sym_path);

            // Retrieve metadata for the src symlink.
            let src_sym_metadata = match task_handle_error(
                fs_conn
                    .src_mnt
                    .fs
                    .read()
                    .unwrap()
                    .meta(&src_abs_sym_path.into()),
                &create_task_error_msg,
                &sender,
            ) {
                Some(metadata) => metadata,
                None => {
                    // Exit task and continue.
                    return exit_task_and_continue(&create_task_info_msg, &sender);
                }
            };

            // Set symlink_up_to_date to false.
            let mut symlink_up_to_date = false;

            // Set transferred node flags to backup flags.
            let mut transferred_node_flags: Flags = backup_flags.flags();

            // If a transferred node exists ...
            if let Some(transferred_node) = transferred_nodes
                .read()
                .unwrap()
                .view::<Backup>()
                .get_node_for_src(&src_rel_sym_path.clone().into())
            {
                // ... and the flags match, ...
                if backup_flags.matches(transferred_node.flags) {
                    //... and symlink meta is the same, ...
                    if src_sym_metadata.symlink_meta == transferred_node.src_symlink_meta {
                        // ... symlink is up to date.
                        symlink_up_to_date = true;

                        // Update transferred node flags.
                        transferred_node_flags.insert(transferred_node.flags);

                        // Remove orphan flag.
                        transferred_node_flags.remove(Flags::ORPHAN);
                    }
                }
            }

            // Symlink is up to date
            if symlink_up_to_date {
                // Update flags.
                transferred_nodes
                    .write()
                    .unwrap()
                    .view_mut::<Backup>()
                    .set_flags(&src_rel_sym_path.clone().into(), transferred_node_flags);

                sender
                    .send(create_task_info_msg(Arc::new(TaskInfo::UpToDate)))
                    .unwrap();
            } else {
                // Set symlink to transferred nodes.
                transferred_nodes
                    .write()
                    .unwrap()
                    .view_mut::<Backup>()
                    .set_transferred_node(
                        &src_rel_sym_path.clone().into(),
                        &TransferredNode::from_symlink(
                            &src_rel_sym_path,
                            transferred_node_flags,
                            &src_sym_metadata,
                        ),
                    );

                sender
                    .send(create_task_info_msg(Arc::new(TaskInfo::Transferred)))
                    .unwrap();
            }

            // Task finished
            sender
                .send(create_task_info_msg(Arc::new(TaskInfo::Finished)))
                .unwrap();

            // Exit task and continue.
            return exit_task_and_continue(&create_task_info_msg, &sender);
        }

        // Exit task.
        false
    }
}
