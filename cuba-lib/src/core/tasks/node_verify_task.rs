use crossbeam_channel::Sender;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use crate::shared::message::Info;
use crate::shared::message::Message;
use crate::shared::npath::Abs;
use crate::shared::npath::File;
use crate::shared::npath::NPath;
use crate::shared::npath::Rel;
use crate::shared::npath::UNPath;
use crate::shared::task_message::TaskError;
use crate::shared::task_message::TaskInfo;
use crate::shared::task_message::TaskMessage;

use super::super::fs::fs_base::FSConnection;
use super::super::password_cache::PasswordCache;
use super::super::process_data::age_procs::age_decrypt_proc;
use super::super::process_data::data_processor::DataProcessor;
use super::super::process_data::gz_procs::gz_decode_proc;
use super::super::process_data::signature_proc::signature_proc;
use super::super::transferred_node::Flags;
use super::super::transferred_node::MaskedFlags;
use super::super::transferred_node::Restore;
use super::super::transferred_node::TransferredNodes;
use super::super::transferred_node::sig_valid_and_match;

use super::task_helpers::exit_task_and_continue;
use super::task_helpers::task_transfer_file;
use super::task_worker::Task;
use super::task_worker::TaskErrorFn;
use super::task_worker::TaskInfoFn;

/// Set verified.
fn set_verified_ok(
    ok: bool,
    src_rel_path: &UNPath<Rel>,
    mut flags: Flags,
    transferred_nodes: &Arc<RwLock<TransferredNodes>>,
    create_task_info_msg: &dyn Fn(Arc<dyn Info + Send + Sync>) -> Arc<TaskMessage>,
    create_task_error_msg: &dyn Fn(Arc<dyn Error + Send + Sync>) -> Arc<TaskMessage>,
    sender: &Sender<Arc<dyn Message>>,
) {
    if ok {
        // Set flags.
        flags.insert(Flags::VERIFIED);
        flags.remove(Flags::VERIFY_ERROR);

        sender
            .send(create_task_info_msg(Arc::new(TaskInfo::Verified)))
            .unwrap();
    } else {
        // Set flags.
        flags.insert(Flags::VERIFIED);
        flags.insert(Flags::VERIFY_ERROR);

        sender
            .send(create_task_error_msg(Arc::new(TaskError::VerifiedFailed)))
            .unwrap();
    }

    // Set flags.
    transferred_nodes
        .write()
        .unwrap()
        .view_mut::<Restore>()
        .set_flags(src_rel_path, flags);
}

/// Task for verify the nodes.
pub fn node_verify_task(
    src_rel_nodes: Arc<Mutex<VecDeque<UNPath<Rel>>>>,
    transferred_nodes: Arc<RwLock<TransferredNodes>>,
    verify_flags: MaskedFlags,
    password_cache: Arc<Mutex<PasswordCache>>,
) -> impl Task {
    move |create_task_error_msg: &dyn TaskErrorFn,
          create_task_info_msg: &dyn TaskInfoFn,
          fs_conn: FSConnection,
          sender: Sender<Arc<dyn Message>>| {
        // Pop the first element.
        let src_rel_nodes_element = src_rel_nodes.lock().unwrap().pop_front();

        // Process if valid element.
        if let Some(src_rel_path) = src_rel_nodes_element {
            // Make task messages with fixed path.
            let create_task_error_msg =
                |error: Arc<dyn Error + Send + Sync>| create_task_error_msg(&src_rel_path, error);
            let create_task_info_msg =
                |info: Arc<dyn Info + Send + Sync>| create_task_info_msg(&src_rel_path, info);

            // Task started.
            sender
                .send(create_task_info_msg(Arc::new(TaskInfo::Start)))
                .unwrap();

            // Get transferred node.
            let transferred_node_opt = {
                let guard = transferred_nodes.read().unwrap();
                guard
                    .view::<Restore>()
                    .get_node_for_src(&src_rel_path)
                    .cloned()
            }; // lock released

            // Check if a transferred node exists.
            if let Some(transferred_node) = transferred_node_opt {
                // If verify flags match, verify ...
                if verify_flags.matches(transferred_node.flags) {
                    // Type?
                    match src_rel_path {
                        UNPath::Dir(ref src_rel_dir_path) => {
                            // Directory exists?
                            let ok = fs_conn
                                .src_mnt
                                .fs
                                .read()
                                .unwrap()
                                .meta(
                                    &fs_conn
                                        .src_mnt
                                        .abs_dir_path
                                        .add_rel_dir(src_rel_dir_path)
                                        .into(),
                                )
                                .is_ok();

                            set_verified_ok(
                                ok,
                                &src_rel_path,
                                transferred_node.flags,
                                &transferred_nodes,
                                &create_task_info_msg,
                                &create_task_error_msg,
                                &sender,
                            );
                        }
                        UNPath::File(ref src_rel_file_path) => {
                            // Create absolut path to the src file.
                            let src_abs_file_path: NPath<Abs, File> =
                                fs_conn.src_mnt.abs_dir_path.add_rel_file(src_rel_file_path);

                            // Init transfer file signature.
                            let transfer_file_signature = Arc::new(Mutex::new([0u8; 32]));

                            // Make data procs vector.
                            let mut data_procs: Vec<DataProcessor> = Vec::new();

                            // Is encypted?
                            if transferred_node.flags.contains(Flags::ENCRYPTED) {
                                // Get password id.
                                match &transferred_node.password_id {
                                    Some(password_id) => {
                                        // Get password.
                                        match password_cache
                                            .lock()
                                            .unwrap()
                                            .get_password(password_id)
                                        {
                                            Ok(password) => {
                                                // Add decryptor.
                                                data_procs.push(age_decrypt_proc(password.clone()));
                                            }
                                            Err(err) => {
                                                // No password found.
                                                sender
                                                    .send(create_task_error_msg(Arc::new(err)))
                                                    .unwrap();

                                                // Exit task and continue.
                                                return exit_task_and_continue(
                                                    &create_task_info_msg,
                                                    &sender,
                                                );
                                            }
                                        }
                                    }
                                    None => {
                                        // No password id.
                                        sender
                                            .send(create_task_error_msg(Arc::new(
                                                TaskError::NoPasswordId,
                                            )))
                                            .unwrap();

                                        // Exit task and continue.
                                        return exit_task_and_continue(
                                            &create_task_info_msg,
                                            &sender,
                                        );
                                    }
                                }
                            }

                            // Is compressed?
                            if transferred_node.flags.contains(Flags::COMPRESSED) {
                                data_procs.push(gz_decode_proc());
                            }

                            // Add signature processor.
                            data_procs.push(signature_proc(transfer_file_signature.clone()));

                            // Transfer file.
                            task_transfer_file(
                                &fs_conn,
                                &src_abs_file_path,
                                &mut NPath::<Rel, File>::default(),
                                &data_procs,
                                Some(&create_task_info_msg),
                                &create_task_error_msg,
                                &sender,
                            );

                            // Note: signature_proc writes the signature when being dropped.
                            // This is working here, because task_transfer_file gets ownership of
                            // data_procs - which is dropped when leaving task_transfer_file.
                            // If task_transfer_file borrows data_procs, signature_proc must be dropped
                            // expicit before the call of sig_valid_and_match.

                            // Check if signatures are equal.
                            let ok = sig_valid_and_match(
                                transferred_node.src_signature,
                                Some(*transfer_file_signature.lock().unwrap()),
                            );

                            set_verified_ok(
                                ok,
                                &src_rel_path,
                                transferred_node.flags,
                                &transferred_nodes,
                                &create_task_info_msg,
                                &create_task_error_msg,
                                &sender,
                            );
                        }
                        UNPath::Symlink(ref _src_rel_sym_path) => {
                            // Symlinks do not exist as backuped files or directories.
                            // So no verification is needed.

                            set_verified_ok(
                                true,
                                &src_rel_path,
                                transferred_node.flags,
                                &transferred_nodes,
                                &create_task_info_msg,
                                &create_task_error_msg,
                                &sender,
                            );
                        }
                    }
                }
            } else {
                // No transferred node found.
                sender
                    .send(create_task_error_msg(Arc::new(
                        TaskError::NoTransferredNode,
                    )))
                    .unwrap();
            }

            // Exit task and continue.
            return exit_task_and_continue(&create_task_info_msg, &sender);
        }

        // Exit task.
        false
    }
}
