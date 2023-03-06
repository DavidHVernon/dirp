use crate::types::*;
use crate::utils::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc::{channel, Receiver};

type FSObjHash = HashMap<PathBuf, Vec<FSObj>>;

pub async fn dirp_state_task(
    mut dirp_state_receiver: Receiver<DirpStateMessage>,
) -> Result<FSObjHash, DirpError> {
    tokio::spawn(async move {
        let mut dirp_state = FSObjHash::new();

        loop {
            let message = dirp_state_receiver.recv().await;

            match message {
                Some(message) => match message {
                    DirpStateMessage::DirScanMessage(dir_scan_message) => {
                        process_dir_scan_message(&mut dirp_state, dir_scan_message)?
                    }
                    DirpStateMessage::FSCreateMessage(_fs_create_message) => {}
                    DirpStateMessage::FSMoveMessage(_fs_move_message) => {}
                    DirpStateMessage::FSDeleteMessage(_fs_delete_message) => {}
                    DirpStateMessage::Quit => break,
                },
                None => break,
            }
        }

        Ok(dirp_state)
    })
    .await?
}

fn process_dir_scan_message(
    dirp_state: &mut FSObjHash,
    dir_scan_message: DirScanMessage,
) -> Result<(), DirpError> {
    dirp_state.insert(dir_scan_message.dir_path, dir_scan_message.fs_obj_list);
    Ok(())
}

pub struct DirpState {}

impl DirpState {
    pub async fn new() -> DirpState {
        DirpState {}
    }

    pub async fn run(root_path: PathBuf) {
        let (dirp_state_sender, dirp_state_receiver) = channel(1024);

        // Spawn a long running task to manage dirp state.
        let dirp_state_task_handle = dirp_state_task(dirp_state_receiver);

        scan_dir_path_task(root_path, dirp_state_sender.clone(), true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_dirp_state_task() {
        let (dirp_state_sender, dirp_state_receiver) = channel(1024);

        // Spawn a long running task to manage dirp state.
        let dirp_state_task_handle = dirp_state_task(dirp_state_receiver);

        // Tick off the initialization process.
        scan_dir_path_task(PathBuf::from("./test/a/"), dirp_state_sender.clone(), true);

        // Sleep long enough for the test dir to be scanned recursivly.
        sleep(Duration::from_secs(3)).await;

        // Shut down the dirp state task.
        dirp_state_sender.send(DirpStateMessage::Quit).await;

        // Join the dirp state task.
        match dirp_state_task_handle.await {
            Ok(dirp_state) => {
                println!("{:#?}", dirp_state);
            }
            Err(error) => {
                println!("{:#?}", error);
            }
        }
    }
}
