use crate::types::*;
use std::path::PathBuf;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct DirpState {
    dir_obj_list: Vec<FSObj>,
}

impl DirpState {
    pub async fn new(root_path: PathBuf) -> DirpState {
        let mut dirp_state = DirpState {
            dir_obj_list: Vec::<FSObj>::new(),
        };

        let (dirp_state_sender, dirp_state_receiver): (
            Sender<DirpStateMessage>,
            Receiver<DirpStateMessage>,
        ) = channel(1024);

        // Spawn a long running task to manage dirp state.
        dirp_state_loop_task(dirp_state_receiver);

        // ToDo: Open a notifier on 'root_path'.
        let notify_dir_path = root_path.clone();
        let notifier_task = tokio::spawn(async move {
            let dir_path = notify_dir_path;

            // ToDo: open a notifier on 'root_dir' recursivly.

            // ToDo: listen on notifications, and send them to dirp_state.
        });

        // ToDo: scan the root dir.
        let dir_path = root_path;
        scan_dir_path_task(&dir_path, dirp_state_sender.clone());

        dirp_state
    }
}

pub async fn dirp_state_loop_task(dirp_state_receiver: Receiver<DirpStateMessage>) {
    tokio::spawn(async move {
        let x = dirp_state_loop(dirp_state_receiver);
        // ToDo: Handle the error
    });
}

async fn dirp_state_loop(
    mut dirp_state_receiver: Receiver<DirpStateMessage>,
) -> Result<(), DirpError> {
    loop {
        let x = dirp_state_receiver.recv().await;
    }

    Ok(())
}
