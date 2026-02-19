use crossbeam_channel::Sender;
use flate2::Compression;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use crate::core::tasks::task_helpers::task_handle_error;
use crate::shared::message::Info;
use crate::shared::message::Message;
use crate::shared::npath::Abs;
use crate::shared::npath::File;
use crate::shared::npath::NPath;
use crate::shared::npath::Rel;
use crate::shared::task_message::TaskError;
use crate::shared::task_message::TaskInfo;

use super::super::fs::fs_base::FSConnection;
use super::super::password_cache::PasswordCache;
use super::super::process_data::age_procs::age_encrypt_proc;
use super::super::process_data::data_processor::DataProcessor;
use super::super::process_data::gz_procs::gz_encode_proc;
use super::super::transferred_node::Backup;
use super::super::transferred_node::Flags;
use super::super::transferred_node::MaskedFlags;
use super::super::transferred_node::TransferredNode;
use super::super::transferred_node::TransferredNodes;
use super::super::transferred_node::sig_valid_and_match;

use super::task_helpers::exit_task_and_continue;
use super::task_helpers::task_read_signature;
use super::task_helpers::task_transfer_file;
use super::task_helpers::task_transfer_successful;
use super::task_worker::Task;
use super::task_worker::TaskErrorFn;
use super::task_worker::TaskInfoFn;

/// Task for backup the files.
pub fn file_backup_task(
    src_rel_files: Arc<Mutex<VecDeque<NPath<Rel, File>>>>,
    transferred_nodes: Arc<RwLock<TransferredNodes>>,
    backup_flags: MaskedFlags,
    password_cache: Arc<Mutex<PasswordCache>>,
    password_id: Option<String>,
) -> impl Task {
    move |create_task_error_msg: &dyn TaskErrorFn,
          create_task_info_msg: &dyn TaskInfoFn,
          fs_conn: FSConnection,
          sender: Sender<Arc<dyn Message>>| {
        // Pop the first element.
        let src_rel_files_element = src_rel_files.lock().unwrap().pop_front();

        // Process if valid element.
        if let Some(src_rel_file_path) = src_rel_files_element {
            // Make task messages with fixed path.
            let create_task_error_msg = |error: Arc<dyn Error + Send + Sync>| {
                create_task_error_msg(&src_rel_file_path.clone().into(), error)
            };
            let create_task_info_msg = |info: Arc<dyn Info + Send + Sync>| {
                create_task_info_msg(&src_rel_file_path.clone().into(), info)
            };

            // Task started
            sender
                .send(create_task_info_msg(Arc::new(TaskInfo::Start)))
                .unwrap();

            // Create absolut path to the src file.
            let src_abs_file_path: NPath<Abs, File> = fs_conn
                .src_mnt
                .abs_dir_path
                .add_rel_file(&src_rel_file_path);

            // Retrieve metadata for the src file.
            let src_file_metadata = match task_handle_error(
                fs_conn
                    .src_mnt
                    .fs
                    .read()
                    .unwrap()
                    .meta(&src_abs_file_path.clone().into()),
                &create_task_error_msg,
                &sender,
            ) {
                Some(metadata) => metadata,
                None => {
                    // Exit task and continue.
                    return exit_task_and_continue(&create_task_info_msg, &sender);
                }
            };

            // Read src file signature.
            let src_file_signature = match task_read_signature(
                &fs_conn.src_mnt,
                &src_abs_file_path,
                &create_task_error_msg,
                &sender,
            ) {
                Some(file_signature) => file_signature,
                None => {
                    // Reading signature failed.

                    // Exit task and continue.
                    return exit_task_and_continue(&create_task_info_msg, &sender);
                }
            };

            // Set transfer_src to true.
            let mut transfer_src = true;

            // Set transferred node flags to backup_flags.
            let mut transferred_node_flags: Flags = backup_flags.flags();

            // If a transferred node exists, ...
            if let Some(transferred_node) = transferred_nodes
                .read()
                .unwrap()
                .view::<Backup>()
                .get_node_for_src(&src_rel_file_path.clone().into())
            {
                // ... the flags match ...
                if backup_flags.matches(transferred_node.flags) {
                    // ... the password_id match ...
                    if password_id == transferred_node.password_id {
                        // ... and the signature is the same as the src signature, ...
                        if sig_valid_and_match(
                            transferred_node.src_signature,
                            Some(src_file_signature),
                        ) {
                            // ... then we don't need to transfer the src.
                            transfer_src = false;

                            // Update transferred node flags.
                            transferred_node_flags.insert(transferred_node.flags);

                            // Remove orphan flag.
                            transferred_node_flags.remove(Flags::ORPHAN);
                        }
                    }
                }
            }

            // Transfer source to destination - if needed.
            if transfer_src {
                // Set dest rel file path.
                let mut dest_rel_file_path = src_rel_file_path.clone();

                // Start transferring.
                sender
                    .send(create_task_info_msg(Arc::new(TaskInfo::Transferring)))
                    .unwrap();

                // Make data procs vector.
                let mut data_procs: Vec<DataProcessor> = Vec::new();

                // Should be compressed?
                if backup_flags.contains(Flags::COMPRESSED) {
                    data_procs.push(gz_encode_proc(Compression::default()));
                }

                // Should be encypted?
                if backup_flags.contains(Flags::ENCRYPTED) {
                    // Get password id.
                    match &password_id {
                        Some(password_id) => {
                            // Get password.
                            match password_cache.lock().unwrap().get_password(password_id) {
                                Ok(password) => {
                                    // Add encryptor.
                                    data_procs.push(age_encrypt_proc(password.clone()));
                                }
                                Err(err) => {
                                    // No password found.
                                    sender.send(create_task_error_msg(Arc::new(err))).unwrap();

                                    // Exit task and continue.
                                    return exit_task_and_continue(&create_task_info_msg, &sender);
                                }
                            }
                        }
                        None => {
                            // No password id.
                            sender
                                .send(create_task_error_msg(Arc::new(TaskError::NoPasswordId)))
                                .unwrap();

                            // Exit task and continue.
                            return exit_task_and_continue(&create_task_info_msg, &sender);
                        }
                    }
                }

                // Transfer file.
                let task_transfer_result = task_transfer_file(
                    &fs_conn,
                    &src_abs_file_path,
                    &mut dest_rel_file_path,
                    &data_procs,
                    Some(&create_task_info_msg),
                    &create_task_error_msg,
                    &sender,
                );

                // Check if transfer was successful.
                if task_transfer_successful(
                    &fs_conn.dest_mnt,
                    &dest_rel_file_path,
                    task_transfer_result,
                    &create_task_error_msg,
                    &sender,
                ) {
                    // Set transferred file to transferred nodes.
                    transferred_nodes
                        .write()
                        .unwrap()
                        .view_mut::<Backup>()
                        .set_transferred_node(
                            &src_rel_file_path.clone().into(),
                            &TransferredNode::from_file(
                                &dest_rel_file_path,
                                transferred_node_flags,
                                password_id.clone(),
                                &src_file_signature,
                                &src_file_metadata,
                            ),
                        );

                    // Transfer was successful.
                    sender
                        .send(create_task_info_msg(Arc::new(TaskInfo::Transferred)))
                        .unwrap();
                } else {
                    // Transfer failed.
                    sender
                        .send(create_task_error_msg(Arc::new(TaskError::TransferFailed)))
                        .unwrap();

                    // Exit task and continue.
                    return exit_task_and_continue(&create_task_info_msg, &sender);
                }
            } else {
                // Update flags.
                transferred_nodes
                    .write()
                    .unwrap()
                    .view_mut::<Backup>()
                    .set_flags(&src_rel_file_path.clone().into(), transferred_node_flags);

                // No transfer needed.
                sender
                    .send(create_task_info_msg(Arc::new(TaskInfo::UpToDate)))
                    .unwrap();
            }

            // Task finished.
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
