use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use crossbeam_channel::Sender;

use crate::shared::message::Info;
use crate::shared::message::Message;
use crate::shared::npath::Abs;
use crate::shared::npath::Dir;
use crate::shared::npath::NPath;
use crate::shared::npath::Rel;
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

/// Task for backup the directories.
pub fn directory_backup_task(
    src_rel_dirs: Arc<Mutex<VecDeque<NPath<Rel, Dir>>>>,
    transferred_nodes: Arc<RwLock<TransferredNodes>>,
    backup_flags: MaskedFlags,
) -> impl Task {
    move |create_task_error_msg: &dyn TaskErrorFn,
          create_task_info_msg: &dyn TaskInfoFn,
          fs_conn: FSConnection,
          sender: Sender<Arc<dyn Message>>| {
        // Pop the first element.
        let src_rel_dirs_element = src_rel_dirs.lock().unwrap().pop_front();

        // Process if valid element.
        if let Some(src_rel_dir_path) = src_rel_dirs_element {
            // Make task messages with fixed path.
            let create_task_error_msg = |error: Arc<dyn Error + Send + Sync>| {
                create_task_error_msg(&src_rel_dir_path.clone().into(), error)
            };
            let create_task_info_msg = |info: Arc<dyn Info + Send + Sync>| {
                create_task_info_msg(&src_rel_dir_path.clone().into(), info)
            };

            // Task started.
            sender
                .send(create_task_info_msg(Arc::new(TaskInfo::Start)))
                .unwrap();

            // Create absolut path to the dest dir.
            let dest_abs_dir_path: NPath<Abs, Dir> =
                fs_conn.dest_mnt.abs_dir_path.add_rel_dir(&src_rel_dir_path);

            // Init with true.
            let mut create_dest_dir: bool = true;

            // Set transferred node flags to backup flags.
            let mut transferred_node_flags: Flags = backup_flags.flags();

            // If a transferred node exists ...
            if let Some(transferred_node) = transferred_nodes
                .read()
                .unwrap()
                .view::<Backup>()
                .get_node_for_src(&src_rel_dir_path.clone().into())
            {
                // ... and the flags match, ...
                if backup_flags.matches(transferred_node.flags) {
                    // ... we do not need to create the dir.
                    create_dest_dir = false;

                    // Update transferred node flags.
                    transferred_node_flags.insert(transferred_node.flags);

                    // Remove orphan flag.
                    transferred_node_flags.remove(Flags::ORPHAN);
                }
            }

            // Create dir at destination, if needed.
            if create_dest_dir {
                // Create directory.
                match fs_conn
                    .dest_mnt
                    .fs
                    .read()
                    .unwrap()
                    .mkdir(&dest_abs_dir_path)
                {
                    Ok(()) => {
                        sender
                            .send(create_task_info_msg(Arc::new(TaskInfo::Transferred)))
                            .unwrap();
                    }
                    Err(error) => {
                        // Maybe dir already exists?
                        match task_handle_error(
                            fs_conn
                                .dest_mnt
                                .fs
                                .read()
                                .unwrap()
                                .meta(&dest_abs_dir_path.into()),
                            &create_task_error_msg,
                            &sender,
                        ) {
                            Some(_meta) => {
                                // Dir exists.
                                sender
                                    .send(create_task_info_msg(Arc::new(TaskInfo::UpToDate)))
                                    .unwrap();
                            }
                            None => {
                                // Create dir failed.
                                sender.send(create_task_error_msg(Arc::new(error))).unwrap();

                                // Exit task and continue.
                                return exit_task_and_continue(&create_task_info_msg, &sender);
                            }
                        }
                    }
                }
            } else {
                // Update flags.
                transferred_nodes
                    .write()
                    .unwrap()
                    .view_mut::<Backup>()
                    .set_flags(&src_rel_dir_path.clone().into(), transferred_node_flags);

                // Dir is up to date.
                sender
                    .send(create_task_info_msg(Arc::new(TaskInfo::UpToDate)))
                    .unwrap();
            }

            // Set dir to transferred nodes.
            transferred_nodes
                .write()
                .unwrap()
                .view_mut::<Backup>()
                .set_transferred_node(
                    &src_rel_dir_path.clone().into(),
                    &TransferredNode::from_dir(&src_rel_dir_path, transferred_node_flags),
                );

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
