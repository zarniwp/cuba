use crossbeam_channel::Sender;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;

use crate::shared::message::Info;
use crate::shared::message::Message;
use crate::shared::npath::Abs;
use crate::shared::npath::File;
use crate::shared::npath::NPath;
use crate::shared::npath::Rel;
use crate::shared::npath::UNPath;
use crate::shared::task_message::TaskError;
use crate::shared::task_message::TaskInfo;

use super::super::fs::fs_base::FSConnection;
use super::super::password_cache::PasswordCache;
use super::super::process_data::age_procs::age_decrypt_proc;
use super::super::process_data::data_processor::DataProcessor;
use super::super::process_data::gz_procs::gz_decode_proc;
use super::super::transferred_node::Flags;
use super::super::transferred_node::Restore;
use super::super::transferred_node::TransferredNodes;
use super::super::transferred_node::sig_valid_and_match;

use super::task_helpers::exit_task_and_continue;
use super::task_helpers::task_read_signature;
use super::task_helpers::task_transfer_file;
use super::task_helpers::task_transfer_successful;
use super::task_worker::Task;
use super::task_worker::TaskErrorFn;
use super::task_worker::TaskInfoFn;

/// Task for restore the files.
pub fn file_restore_task(
    src_rel_files: Arc<Mutex<VecDeque<NPath<Rel, File>>>>,
    transferred_nodes_read: Arc<TransferredNodes>,
    password_cache: Arc<Mutex<PasswordCache>>,
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

            // Task started.
            sender
                .send(create_task_info_msg(Arc::new(TaskInfo::Start)))
                .unwrap();

            // Check if a transferred node exists.
            if let Some(transferred_node) = transferred_nodes_read
                .view::<Restore>()
                .get_node_for_src(&src_rel_file_path.clone().into())
            {
                // Create absolut path to the src file.
                let src_abs_file_path: NPath<Abs, File> = fs_conn
                    .src_mnt
                    .abs_dir_path
                    .add_rel_file(&src_rel_file_path);

                // Set dest rel file path.
                if let Some(UNPath::<Rel>::File(dest_rel_file_path)) = transferred_nodes_read
                    .view::<Restore>()
                    .get_dest_rel_path(transferred_node)
                {
                    // Create absolut path to the dest file.
                    let dest_abs_file_path: NPath<Abs, File> = fs_conn
                        .dest_mnt
                        .abs_dir_path
                        .add_rel_file(&dest_rel_file_path);

                    // Init dest file signature.
                    let mut dest_file_signature: Option<[u8; 32]> = None;

                    // Check if dest file exists.
                    if fs_conn
                        .dest_mnt
                        .fs
                        .read()
                        .unwrap()
                        .meta(&dest_abs_file_path.clone().into())
                        .is_ok()
                    {
                        // Read dest file signature.
                        dest_file_signature = task_read_signature(
                            &fs_conn.dest_mnt,
                            &dest_abs_file_path.clone(),
                            &create_task_error_msg,
                            &sender,
                        );
                    }

                    // Check if signatures are equal.
                    if sig_valid_and_match(transferred_node.src_signature, dest_file_signature) {
                        // No transfer needed.
                        sender
                            .send(create_task_info_msg(Arc::new(TaskInfo::UpToDate)))
                            .unwrap();

                        // Task finished.
                        sender
                            .send(create_task_info_msg(Arc::new(TaskInfo::Finished)))
                            .unwrap();

                        // Exit task and continue.
                        return exit_task_and_continue(&create_task_info_msg, &sender);
                    }
                }

                // Set dest rel file path.
                let mut dest_rel_file_path = src_rel_file_path.clone();

                // Start transferring.
                sender
                    .send(create_task_info_msg(Arc::new(TaskInfo::Transferring)))
                    .unwrap();

                // Make data procs vector.
                let mut data_procs: Vec<DataProcessor> = Vec::new();

                // Is encypted?
                if transferred_node.flags.contains(Flags::ENCRYPTED) {
                    // Get password id.
                    match &transferred_node.password_id {
                        Some(password_id) => {
                            // Get password.
                            match password_cache.lock().unwrap().get_password(password_id) {
                                Ok(password) => {
                                    // Add decryptor.
                                    data_procs.push(age_decrypt_proc(password.clone()));
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

                // Is compressed?
                if transferred_node.flags.contains(Flags::COMPRESSED) {
                    data_procs.push(gz_decode_proc());
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

                // Read dest file signature.
                let dest_file_signature = task_read_signature(
                    &fs_conn.dest_mnt,
                    &fs_conn
                        .dest_mnt
                        .abs_dir_path
                        .add_rel_file(&dest_rel_file_path),
                    &create_task_error_msg,
                    &sender,
                );

                // Check if signatures are equal.
                if sig_valid_and_match(transferred_node.src_signature, dest_file_signature) {
                    sender
                        .send(create_task_info_msg(Arc::new(TaskInfo::Verified)))
                        .unwrap();
                } else {
                    sender
                        .send(create_task_error_msg(Arc::new(TaskError::VerifiedFailed)))
                        .unwrap();
                }
            } else {
                // No transferred node found.
                sender
                    .send(create_task_error_msg(Arc::new(
                        TaskError::NoTransferredNode,
                    )))
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
