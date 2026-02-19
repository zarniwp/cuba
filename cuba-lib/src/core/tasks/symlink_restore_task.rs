use crossbeam_channel::Sender;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;

use crate::shared::message::Info;
use crate::shared::message::Message;
use crate::shared::npath::Abs;
use crate::shared::npath::NPath;
use crate::shared::npath::Rel;
use crate::shared::npath::Symlink;
use crate::shared::task_message::TaskInfo;

use super::super::fs::fs_base::FSConnection;

use super::task_helpers::exit_task_and_continue;
use super::task_worker::Task;
use super::task_worker::TaskErrorFn;
use super::task_worker::TaskInfoFn;

/// Task for restore the directories.
pub fn symlink_restore_task(
    src_rel_symlinks: Arc<Mutex<VecDeque<NPath<Rel, Symlink>>>>,
) -> impl Task {
    move |create_task_error_msg: &dyn TaskErrorFn,
          create_task_info_msg: &dyn TaskInfoFn,
          fs_conn: FSConnection,
          sender: Sender<Arc<dyn Message>>| {
        // Pop the first element.
        let src_rel_symlinks_element = src_rel_symlinks.lock().unwrap().pop_front();

        // Process if valid element.
        if let Some(src_rel_sym_path) = src_rel_symlinks_element {
            // Make task messages with fixed path.
            let _create_task_error_msg = |error: Arc<dyn Error + Send + Sync>| {
                create_task_error_msg(&src_rel_sym_path.clone().into(), error)
            };
            let create_task_info_msg = |info: Arc<dyn Info + Send + Sync>| {
                create_task_info_msg(&src_rel_sym_path.clone().into(), info)
            };

            // Task started.
            sender
                .send(create_task_info_msg(Arc::new(TaskInfo::Start)))
                .unwrap();

            // Create absolut path to the dest symlink.
            let _dest_abs_sym_path: NPath<Abs, Symlink> = fs_conn
                .dest_mnt
                .abs_dir_path
                .add_rel_symlink(&src_rel_sym_path);

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
