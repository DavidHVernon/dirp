use crate::types::*;
use crate::utils::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc::{channel, Receiver};

type FSObjHash = HashMap<PathBuf, Vec<FSObj>>;

pub async fn dirp_state_task(mut dirp_state_receiver: Receiver<DirpStateMessage>)
//-> Result<FSObjHash, DirpError>
{
    // let x = tokio::spawn(async move {
    //     let mut dirp_state = FSObjHash::new();

    //     loop {
    //         let message = dirp_state_receiver.recv().await;
    //         match message {
    //             Some(message) => match message {
    //                 DirpStateMessage::DirScanMessage(dir_scan_message) => {
    //                     dirp_state = process_dir_scan_message(dirp_state, dir_scan_message)
    //                 }
    //                 DirpStateMessage::FSCreateMessage(fs_create_message) => {
    //                     dirp_state = process_fs_create_message(dirp_state, fs_create_message);
    //                 }
    //                 DirpStateMessage::FSMoveMessage(fs_move_message) => {
    //                     dirp_state = process_fs_move_message(dirp_state, fs_move_message);
    //                 }
    //                 DirpStateMessage::FSDeleteMessage(fs_delete_message) => {
    //                     dirp_state = process_fs_delete_message(dirp_state, fs_delete_message);
    //                 }
    //             },
    //             None => break,
    //         }
    //     }

    //     Ok(dirp_state)
    // })
    // .await;
}

fn process_dir_scan_message(
    mut dirp_state: FSObjHash,
    dir_scan_message: DirScanMessage,
) -> FSObjHash {
    dirp_state.insert(dir_scan_message.dir_path, dir_scan_message.fs_obj_list);
    dirp_state
}

fn process_fs_create_message(
    mut dirp_state: FSObjHash,
    fs_create_message: FSCreateMessage,
) -> FSObjHash {
    // ToDo: Add support for fs monitoring.
    dirp_state
}

fn process_fs_move_message(mut dirp_state: FSObjHash, fs_move_message: FSMoveMessage) -> FSObjHash {
    // ToDo: Add support for fs monitoring.
    dirp_state
}

fn process_fs_delete_message(
    mut dirp_state: FSObjHash,
    process_fs_delete_message: FSDeleteMessage,
) -> FSObjHash {
    // ToDo: Add support for fs monitoring.
    dirp_state
}

pub struct DirpState {}

impl DirpState {
    pub async fn new() -> DirpState {
        DirpState {}
    }

    pub async fn run(root_path: PathBuf) {
        // Create a channel to communicate to the dirp_state_task with.
        let (dirp_state_sender, dirp_state_receiver) = channel(1024);

        // Spawn a long running task to manage dirp state.
        let x = dirp_state_task(dirp_state_receiver);

        // ToDo: Open a notifier on 'root_path'.
        let notify_dir_path = root_path.clone();
        let notifier_task = tokio::spawn(async move {
            let dir_path = notify_dir_path;

            // ToDo: open a notifier on 'root_dir' recursivly.

            // ToDo: listen on notifications, and send them to dirp_state.
        });

        scan_dir_path_task(root_path, dirp_state_sender.clone(), true);
    }
}
