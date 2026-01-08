use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use crossbeam_channel::Sender;
use cuba_lib::{
    core::cuba::{Cuba, RunHandle},
    send_info,
    shared::{message::Message, msg_dispatcher::MsgDispatcher, msg_receiver::MsgReceiver},
};

use crate::task_progress::TaskProgress;

type CubaArc = Arc<RwLock<Cuba>>;

type RunFn = dyn Fn(CubaArc, RunHandle, String) + Send + 'static;

/// Creates a cuba runner.
#[allow(clippy::too_many_arguments)]
pub fn make_cuba_runner(
    run_handle: RunHandle,
    sender: Sender<Arc<dyn Message>>,
    cuba: Arc<RwLock<Cuba>>,
    selected_profiles: HashSet<String>,
    msg_dispatcher: Arc<MsgDispatcher<Arc<dyn Message>>>,
    task_progress: Arc<TaskProgress>,
) -> impl Fn(String, Box<RunFn>) {
    move |name, call_run| {
        cuba_run(
            name,
            run_handle.clone(),
            sender.clone(),
            cuba.clone(),
            selected_profiles.clone(),
            msg_dispatcher.clone(),
            task_progress.clone(),
            call_run,
        )
    }
}

/// Runs a cuba command on the selected profiles.
#[allow(clippy::too_many_arguments)]
pub fn cuba_run<RunFunc>(
    name: String,
    run_handle: RunHandle,
    sender: Sender<Arc<dyn Message>>,
    cuba: Arc<RwLock<Cuba>>,
    selected_profiles: HashSet<String>,
    msg_dispatcher: Arc<MsgDispatcher<Arc<dyn Message>>>,
    task_progress: Arc<TaskProgress>,
    call_run: RunFunc,
) where
    RunFunc: Fn(Arc<RwLock<Cuba>>, RunHandle, String) + Send + 'static,
{
    std::thread::spawn(move || {
        let mut msg_receiver = MsgReceiver::new(msg_dispatcher.subscribe(), task_progress.clone());

        msg_receiver.start();

        for profile in selected_profiles {
            send_info!(sender, "Start {} of {}", name.to_lowercase(), profile);

            call_run(cuba.clone(), run_handle.clone(), profile);

            send_info!(sender, "{} finished", name);
        }

        msg_receiver.stop();
    });
}
