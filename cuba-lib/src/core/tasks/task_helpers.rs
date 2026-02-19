use crossbeam_channel::Sender;
use std::error::Error;
use std::fmt::Display;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;

use crate::shared::message::Info;
use crate::shared::message::Message;
use crate::shared::npath::Abs;
use crate::shared::npath::File;
use crate::shared::npath::NPath;
use crate::shared::npath::Rel;
use crate::shared::progress_message::ProgressInfo;
use crate::shared::progress_message::ProgressMessage;
use crate::shared::task_message::TaskInfo;
use crate::shared::task_message::TaskMessage;

use super::super::fs::fs_base::FSBlockSize;
use super::super::fs::fs_base::FSConnection;
use super::super::fs::fs_base::FSMount;
use super::super::process_data::data_processor::DataProcessor;
use super::super::process_data::signature_proc::signature_proc;

/// Exit task.
pub fn exit_task_and_continue(
    create_task_info_msg: &dyn Fn(Arc<dyn Info + Send + Sync>) -> Arc<TaskMessage>,
    sender: &Sender<Arc<dyn Message>>,
) -> bool {
    // Progress tick.
    sender
        .send(Arc::new(ProgressMessage::new(
            Arc::new(ProgressInfo::Ticks),
            1,
        )))
        .unwrap();

    // Task finished.
    sender
        .send(create_task_info_msg(Arc::new(TaskInfo::Finished)))
        .unwrap();

    true
}

/// A function that checks if the task transfer was successful.
pub fn task_transfer_successful(
    dest_mnt: &FSMount,
    dest_rel_file_path: &NPath<Rel, File>,
    task_transfer_result: Option<usize>,
    create_task_error_msg: &dyn Fn(Arc<dyn Error + Send + Sync>) -> Arc<TaskMessage>,
    sender: &Sender<Arc<dyn Message>>,
) -> bool {
    match task_transfer_result {
        None => false,
        Some(transferred_bytes) => {
            match task_handle_error(
                dest_mnt.fs.read().unwrap().meta(
                    &dest_mnt
                        .abs_dir_path
                        .add_rel_file(dest_rel_file_path)
                        .into(),
                ),
                &create_task_error_msg,
                sender,
            ) {
                Some(dest_file_meta) => match dest_file_meta.size {
                    Some(size) => size == transferred_bytes as u64,
                    None => false,
                },
                None => false,
            }
        }
    }
}

/// Handle a task error.
pub fn task_handle_error<T, E, TFn>(
    result: Result<T, E>,
    create_task_error_msg: &TFn,
    sender: &Sender<Arc<dyn Message>>,
) -> Option<T>
where
    E: Error + Send + Sync + Display + 'static,
    TFn: Fn(Arc<dyn Error + Send + Sync>) -> Arc<TaskMessage>,
{
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            let task_error_message = create_task_error_msg(Arc::new(error));
            if sender.send(task_error_message).is_err() {
                eprintln!("Warning: Failed to send error message.");
            }
            None
        }
    }
}

/// Transfers a file from fs_conn.src to fs_conn.dest.
pub fn task_transfer_file(
    fs_conn: &FSConnection,
    src_abs_file_path: &NPath<Abs, File>,
    dest_rel_file_path: &mut NPath<Rel, File>,
    data_procs: &Vec<DataProcessor>,
    create_task_info_msg: Option<&dyn Fn(Arc<dyn Info + Send + Sync>) -> Arc<TaskMessage>>,
    create_task_error_msg: &dyn Fn(Arc<dyn Error + Send + Sync>) -> Arc<TaskMessage>,
    sender: &Sender<Arc<dyn Message>>,
) -> Option<usize> {
    // Open the src_file for reading.
    let src_reader: Box<dyn Read + Send> = task_handle_error(
        fs_conn
            .src_mnt
            .fs
            .read()
            .unwrap()
            .read_data(src_abs_file_path),
        &create_task_error_msg,
        sender,
    )?;

    // Create buf reader.
    let mut data: Box<dyn Read + Send> = Box::new(BufReader::new(src_reader));

    // Apply data processors.
    for proc in data_procs.iter() {
        data = proc(
            sender.clone(),
            Box::new(BufReader::new(data)),
            Some(dest_rel_file_path),
        );
    }

    // The read buffer size.
    let data_buffer_size = FSBlockSize::choose(
        &fs_conn.src_mnt.fs.read().unwrap().block_size(),
        &fs_conn.dest_mnt.fs.read().unwrap().block_size(),
    );

    // Init bytes of the transfer.
    let mut transferred_bytes = 0;

    // Write data.
    match fs_conn.dest_mnt.fs.read().unwrap().write_data(
        &fs_conn
            .dest_mnt
            .abs_dir_path
            .add_rel_file(dest_rel_file_path),
    ) {
        Ok(mut write) => {
            // The buffer.
            let mut data_buffer = vec![0u8; data_buffer_size];

            // Read loop.
            loop {
                match task_handle_error(data.read(&mut data_buffer), &create_task_error_msg, sender)
                {
                    Some(bytes_read) => {
                        if bytes_read == 0 {
                            break; // EOR
                        }

                        transferred_bytes += bytes_read;

                        task_handle_error(
                            write.write_all(&data_buffer[..bytes_read]),
                            &create_task_error_msg,
                            sender,
                        )?;
                    }
                    None => return None,
                };

                // Send tick.
                if let Some(create_task_info_msg) = create_task_info_msg {
                    sender
                        .send(create_task_info_msg(Arc::new(TaskInfo::Tick)))
                        .unwrap();
                }
            }

            // Finish write.
            write.finish();
        }
        Err(error) => {
            // Error
            sender.send(create_task_error_msg(Arc::new(error))).unwrap();
            return None;
        }
    }

    Some(transferred_bytes)
}

/// Read the signature of a file.
pub fn task_read_signature(
    fs_mnt: &FSMount,
    abs_file_path: &NPath<Abs, File>,
    create_task_error_msg: &dyn Fn(Arc<dyn Error + Send + Sync>) -> Arc<TaskMessage>,
    sender: &Sender<Arc<dyn Message>>,
) -> Option<[u8; 32]> {
    // Create fs_conn.
    let fs_conn = FSConnection::new(fs_mnt.clone(), FSMount::dev_null());

    // Connect dev_null fs.
    if let Err(error) = fs_conn.dest_mnt.fs.write().unwrap().connect() {
        sender.send(create_task_error_msg(Arc::new(error))).unwrap();
        return None;
    }

    // Init signature.
    let signature = Arc::new(Mutex::new([0u8; 32]));

    // Init data_procs with signature proc.
    let data_procs = vec![signature_proc(signature.clone())];

    // Transfer to destination.
    task_transfer_file(
        &fs_conn,
        abs_file_path,
        &mut NPath::default(),
        &data_procs,
        None,
        &create_task_error_msg,
        sender,
    );

    // Disconnect dev_null fs.
    if let Err(error) = fs_conn.dest_mnt.fs.write().unwrap().disconnect() {
        sender.send(create_task_error_msg(Arc::new(error))).unwrap();
        return None;
    }

    Some(*signature.lock().unwrap())
}
