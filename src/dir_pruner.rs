use threadpool::ThreadPool;

use crate::types::*;
use crate::utils::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

type FSObjHash = HashMap<PathBuf, Vec<FSObj>>;

pub fn dirp_state_thread_spawn(
    path: PathBuf,
    user_sender: Sender<DirpStateMessage>,
    dirp_state_sender: Sender<DirpStateMessage>,
    mut dirp_state_receiver: Receiver<DirpStateMessage>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        if let Err(error) =
            dirp_state_loop(path, user_sender, dirp_state_sender, dirp_state_receiver)
        {
            panic!("dirp_state_loop error: {:#?}", error);
        }
    })
}

pub fn dirp_state_loop(
    path: PathBuf,
    user_sender: Sender<DirpStateMessage>,
    dirp_state_sender: Sender<DirpStateMessage>,
    mut dirp_state_receiver: Receiver<DirpStateMessage>,
) -> Result<(), DirpError> {
    let threadpool = ThreadPool::new(30);
    let mut dirp_state = FSObjHash::new();

    scan_dir_path_in_threadpool(path, dirp_state_sender.clone(), threadpool.clone());

    loop {
        match dirp_state_receiver.recv() {
            Ok(message) => match message {
                DirpStateMessage::DirScanMessage(dir_scan_message) => {
                    // Recursively scan dirs.
                    for fs_obj in &dir_scan_message.fs_obj_list {
                        if let FSObj::Dir(dir_obj) = fs_obj {
                            scan_dir_path_in_threadpool(
                                dir_obj.name.clone(),
                                dirp_state_sender.clone(),
                                threadpool.clone(),
                            );
                        }
                    }
                    // Update state.
                    dirp_state.insert(dir_scan_message.dir_path, dir_scan_message.fs_obj_list);
                }
                DirpStateMessage::FSCreateMessage(_fs_create_message) => {}
                DirpStateMessage::FSMoveMessage(_fs_move_message) => {}
                DirpStateMessage::FSDeleteMessage(_fs_delete_message) => {}
                DirpStateMessage::GetStateRequest => {
                    user_sender.send(DirpStateMessage::GetStateResponse(GetStateResponse {
                        dirp_state: dirp_state.clone(),
                    }))?;
                }
                DirpStateMessage::GetStateResponse(_state_response) => {}
                DirpStateMessage::Quit => break,
            },
            Err(error) => {
                // Error means connection closed, so exit.
                break;
            }
        }
    }
    Ok(())
}

pub struct DirpState {
    sender: Sender<DirpStateMessage>,
    receiver: Receiver<DirpStateMessage>,
    thread_handle: JoinHandle<()>,
}

impl DirpState {
    pub fn new() -> DirpState {
        let (dirp_state_sender, dirp_state_receiver) = channel();
        let (sender, mut receiver) = channel();

        // Spawn a long running task to manage dirp state.
        let thread_handle = dirp_state_thread_spawn(
            PathBuf::from("./test"),
            sender,
            dirp_state_sender.clone(),
            dirp_state_receiver,
        );

        DirpState {
            sender: dirp_state_sender,
            receiver,
            thread_handle,
        }
    }

    pub fn quit(self) {
        if let Err(error) = self.sender.send(DirpStateMessage::Quit) {
            panic!(
                "DirpState.quit(): Could not send quit message. Error: {:#?}",
                error
            );
        }
        if let Err(error) = self.thread_handle.join() {
            panic!(
                "DirpState.quit(): Could not join thread handle: {:#?}",
                error
            );
        }
    }

    pub fn send(&self, message: DirpStateMessage) {
        if let Err(error) = self.sender.send(message) {
            panic!("DirpState.send(): error: {:#?}", error);
        }
    }

    pub fn recv(&self) -> DirpStateMessage {
        match self.receiver.recv() {
            Ok(message) => {
                return message;
            }
            Err(error) => {
                panic!("DirpState.recv(): error: {:#?}", error);
            }
        }
    }

    pub fn make_request(&self, request: DirpStateMessage) -> DirpStateMessage {
        self.send(request);
        return self.recv();
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;

    #[test]
    fn test_dirp_state_task() -> Result<(), DirpError> {
        let dirp_state = DirpState::new();
        sleep(Duration::from_secs(3));
        let dirp_state_list = dirp_state.make_request(DirpStateMessage::GetStateRequest);
        println!("{:#?}", dirp_state_list);
        dirp_state.quit();

        Ok(())
    }
}
