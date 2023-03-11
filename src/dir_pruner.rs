use threadpool::ThreadPool;

use crate::types::*;
use crate::utils::*;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

pub fn dirp_state_thread_spawn(
    path: PathBuf,
    user_sender: Sender<DirpStateMessage>,
    dirp_state_sender: Sender<DirpStateMessage>,
    dirp_state_receiver: Receiver<DirpStateMessage>,
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
    dirp_state_receiver: Receiver<DirpStateMessage>,
) -> Result<(), DirpError> {
    let threadpool = ThreadPool::new(30);
    let mut dirp_state = DirHash::new();

    scan_dir_path_in_threadpool(path, dirp_state_sender.clone(), threadpool.clone());

    loop {
        match dirp_state_receiver.recv() {
            Ok(message) => match message {
                DirpStateMessage::DirScanMessage(dir) => {
                    process_dir_scan_message(dir, &mut dirp_state, &dirp_state_sender, &threadpool);
                }
                DirpStateMessage::GetStateRequest => {
                    user_sender.send(DirpStateMessage::GetStateResponse(GetStateResponse {
                        dirp_state: dirp_state.clone(),
                    }))?;
                }
                DirpStateMessage::GetStateResponse(_state_response) => {
                    assert!(false, "Invalid message.");
                }
                DirpStateMessage::Quit => break,
            },
            Err(_error) => {
                // Error means connection closed, so exit.
                break;
            }
        }
    }
    Ok(())
}

fn process_dir_scan_message(
    mut dir: Dir,
    dirp_state: &mut DirHash,
    dirp_state_sender: &Sender<DirpStateMessage>,
    threadpool: &ThreadPool,
) {
    dir.size_in_bytes = 0;
    for fs_obj in &dir.dir_obj_list {
        match fs_obj {
            FSObj::Dir(_) => {
                assert!(false, "Invalid state.");
            }
            FSObj::DirRef(dir_ref) => {
                // Recurse
                scan_dir_path_in_threadpool(
                    dir_ref.path.clone(),
                    dirp_state_sender.clone(),
                    threadpool.clone(),
                );
            }
            FSObj::SymLink(_) => {
                // Ignore
            }
            FSObj::File(file) => {
                // Size the directory
                dir.size_in_bytes += file.size_in_bytes;
            }
        }
    }

    // Resize parent dirs.
    while let Some(parent_path) = dir.path.parent() {
        if let Some(parent_dir) = dirp_state.get_mut(parent_path) {
            parent_dir.size_in_bytes += dir.size_in_bytes;
        }
    }

    // Update state.
    dirp_state.insert(dir.path.clone(), dir);
}

pub struct DirpState {
    sender: Sender<DirpStateMessage>,
    receiver: Receiver<DirpStateMessage>,
    thread_handle: JoinHandle<()>,
}

impl DirpState {
    pub fn new() -> DirpState {
        let (dirp_state_sender, dirp_state_receiver) = channel();
        let (sender, receiver) = channel();

        // Spawn a long running task to manage dirp state.
        let thread_handle = dirp_state_thread_spawn(
            PathBuf::from("./test"),
            sender.clone(),
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
