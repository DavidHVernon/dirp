use threadpool::ThreadPool;

use crate::types::*;
use crate::utils::*;
use chrono::Duration;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};
use timer::MessageTimer;

pub fn dirp_state_thread_spawn(
    path: PathBuf,
    user_sender: Sender<UserMessage>,
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
    user_sender: Sender<UserMessage>,
    dirp_state_sender: Sender<DirpStateMessage>,
    dirp_state_receiver: Receiver<DirpStateMessage>,
) -> Result<(), DirpError> {
    let mut dirp_state = DirHash::new();
    let threadpool = ThreadPool::new(30);
    let mut is_state_dirty = false;

    // Initialize dir scan.
    scan_dir_path_in_threadpool(path.clone(), dirp_state_sender.clone(), &threadpool);

    // Kick off timer.
    let message_timer = MessageTimer::new(dirp_state_sender.clone());
    let message_timer_guard =
        message_timer.schedule_repeating(Duration::milliseconds(500), DirpStateMessage::Timer);

    loop {
        match dirp_state_receiver.recv() {
            Ok(message) => match message {
                DirpStateMessage::DirScanMessage(dir) => {
                    process_dir_scan_message(dir, &mut dirp_state, &dirp_state_sender, &threadpool);
                    is_state_dirty = true;
                }
                DirpStateMessage::GetStateRequest => {
                    process_get_state_request(&path, &mut dirp_state, &user_sender)?;
                }
                DirpStateMessage::NoOp(no_op) => {}
                DirpStateMessage::Timer => {
                    if is_state_dirty {
                        is_state_dirty = false;
                        process_get_state_request(&path, &mut dirp_state, &user_sender)?;
                    }
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
                    &threadpool,
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
    let mut parent_path_opt = dir.path.parent();
    while let Some(parent_path) = parent_path_opt {
        if let Some(parent_dir) = dirp_state.get_mut(parent_path) {
            parent_dir.size_in_bytes += dir.size_in_bytes;
        }
        parent_path_opt = parent_path.parent();
    }

    // Update state.
    dirp_state.insert(dir.path.clone(), dir);
}

fn process_get_state_request(
    path: &PathBuf,
    dirp_state: &mut DirHash,
    user_sender: &Sender<UserMessage>,
) -> Result<(), DirpError> {
    let root_dir = dirp_state
        .get(path)
        .expect("internal error: could not find root element.");

    let result_dir = build_result_tree(&root_dir, &dirp_state);

    user_sender.send(UserMessage::GetStateResponse(GetStateResponse {
        dirp_state: result_dir,
    }))?;

    Ok(())
}

fn build_result_tree(dir: &Dir, dirp_state: &DirHash) -> Dir {
    let mut result_dir = Dir {
        path: dir.path.clone(),
        size_in_bytes: dir.size_in_bytes,
        dir_obj_list: FSObjList::new(),
    };

    for child_obj in &dir.dir_obj_list {
        match child_obj {
            FSObj::SymLink(sym_link) => result_dir
                .dir_obj_list
                .push(FSObj::SymLink((sym_link.clone()))),
            FSObj::File(file) => result_dir.dir_obj_list.push(FSObj::File(file.clone())),
            FSObj::Dir(dir) => assert!(false, "invalid state"),
            FSObj::DirRef(dir_ref) => {
                let dir = dirp_state
                    .get(&dir_ref.path)
                    .expect("internal error: could not find dir.");

                result_dir
                    .dir_obj_list
                    .push(FSObj::Dir(build_result_tree(dir, dirp_state)))
            }
        }
    }

    result_dir
}

struct DirpState {
    user_receiver: Receiver<UserMessage>,
    dirp_state_sender: Sender<DirpStateMessage>,
    thread_handle: JoinHandle<()>,
}

impl DirpState {
    pub fn new(path: PathBuf) -> DirpState {
        let (dirp_state_sender, dirp_state_receiver) = channel();
        let (user_sender, user_receiver) = channel();

        // Spawn a long running task to manage dirp state.
        let thread_handle = dirp_state_thread_spawn(
            path,
            user_sender.clone(),
            dirp_state_sender.clone(),
            dirp_state_receiver,
        );

        DirpState {
            user_receiver,
            dirp_state_sender,
            thread_handle,
        }
    }

    pub fn quit(self) {
        if let Err(error) = self.dirp_state_sender.send(DirpStateMessage::Quit) {
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
        if let Err(error) = self.dirp_state_sender.send(message) {
            panic!("DirpState.send(): error: {:#?}", error);
        }
    }

    pub fn recv(&self) -> UserMessage {
        match self.user_receiver.recv() {
            Ok(message) => {
                return message;
            }
            Err(error) => {
                panic!("DirpState.recv(): error: {:#?}", error);
            }
        }
    }

    pub fn request(&self, request: DirpStateMessage) -> UserMessage {
        self.send(request);
        self.recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{Hash, Hasher};
    use std::{collections::hash_map::DefaultHasher, thread::sleep, time::Duration};

    #[test]
    fn test_dirp_state_task() -> Result<(), DirpError> {
        let dirp_state = DirpState::new(PathBuf::from("./test"));
        let dirp_state_list = dirp_state.recv();
        if let UserMessage::GetStateResponse(dirp_state_list) = dirp_state_list {
            let expected_hash = 5662441951356583153 as u64;
            let mut hasher = DefaultHasher::new();
            dirp_state_list.hash(&mut hasher);
            let hash = hasher.finish();

            println!("{:#?}", dirp_state_list.dirp_state);
            println!("Hash: {}", hash);

            assert_eq!(expected_hash, hash, "Error: Unexpected result.");
        }

        dirp_state.quit();

        Ok(())
    }
}
