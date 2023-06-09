use crate::types::*;
use crate::utils::*;
use chrono::Duration;
use console::Term;
use dialoguer::{console, theme::ColorfulTheme, FuzzySelect};
use std::{
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
};
use threadpool::ThreadPool;
use timer::MessageTimer;
use trash;

pub fn dirp_state_thread_spawn(
    path: String,
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

//
// This thread manages the enter state of this program. It has an object on the stack
// called 'dirp_state'. That object is a hash from String (a file path) to a Dir object
// (Directory meta data gleaned from the file system).
//
// You interact with this thread (and it's state) by sending it messages on a channel.
//
// This thread runs a timer. Every time the timer kicks off a 'dirty state' vairable is checked ('is_state_dirty').
// If that var is true then the thread cooks up a representation of what state is currently being displayed to the
// user (a proper subset of the sate in 'dirp_state') and sends it along.
//
pub fn dirp_state_loop(
    root_path: String,
    user_sender: Sender<UserMessage>,
    dirp_state_sender: Sender<DirpStateMessage>,
    dirp_state_receiver: Receiver<DirpStateMessage>,
) -> Result<(), DirpError> {
    let mut dirp_state = DirHash::new();
    let threadpool = ThreadPool::new(30);
    let mut is_state_dirty = false;

    // Initialize dir scan.
    scan_dir_path_in_threadpool(
        root_path.clone(),
        true,
        dirp_state_sender.clone(),
        &threadpool,
    );

    // Kick off timer.
    let message_timer = MessageTimer::new(dirp_state_sender.clone());
    let _message_timer_guard =
        message_timer.schedule_repeating(Duration::milliseconds(200), DirpStateMessage::Timer);

    // Event Loop
    loop {
        match dirp_state_receiver.recv() {
            Ok(message) => match message {
                DirpStateMessage::DirScanMessage(dir) => {
                    process_dir_scan_message(dir, &mut dirp_state, &dirp_state_sender, &threadpool);
                    is_state_dirty = true;
                }
                DirpStateMessage::Timer => {
                    if is_state_dirty {
                        is_state_dirty = false;
                        user_sender.send(UserMessage::GetStateResponse(GetStateResponse {
                            dirp_state: build_result_tree(&root_path, false, &dirp_state),
                        }))?;
                    }
                }
                DirpStateMessage::OpenDir(path) => {
                    if let Some(dir) = dirp_state.get_mut(&path) {
                        dir.is_open = true;
                        is_state_dirty = true;
                    }
                }
                DirpStateMessage::CloseDir(path) => {
                    if let Some(dir) = dirp_state.get_mut(&path) {
                        dir.is_open = false;
                        is_state_dirty = true;
                    }
                }
                DirpStateMessage::ToggleDir(path) => {
                    if let Some(dir) = dirp_state.get_mut(&path) {
                        dir.is_open = !dir.is_open;
                        is_state_dirty = true;
                    }
                }
                DirpStateMessage::MarkPath(path) => {
                    do_mark_deep(&path, true, &mut dirp_state);
                    is_state_dirty = true;
                }
                DirpStateMessage::UnmarkPath(path) => {
                    do_mark_deep(&path, false, &mut dirp_state);
                    is_state_dirty = true;
                }
                DirpStateMessage::ToggleMarkPath(path) => {
                    if let Some(is_path_marked) = is_path_marked(&path, &dirp_state) {
                        do_mark_deep(&path, !is_path_marked, &mut dirp_state);
                        is_state_dirty = true;
                    } else {
                        panic!("shit");
                    }
                }
                DirpStateMessage::RemoveMarked => {
                    process_remove_marked(&root_path, &dirp_state)?;
                    break;
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
    // A dir scan has been completed in the thread pool. 'dir' is the result of that work.

    // Post process the dir.
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
                    false,
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
    let mut parent_path_opt = parent_file_path(&dir.path);
    while let Some(parent_path) = parent_path_opt {
        if let Some(parent_dir) = dirp_state.get_mut(&parent_path) {
            parent_dir.size_in_bytes += dir.size_in_bytes;
        }
        parent_path_opt = parent_file_path(&parent_path);
    }

    // Update state.
    dirp_state.insert(dir.path.clone(), dir);
}

fn process_remove_marked(root_path: &String, dirp_state: &DirHash) -> Result<(), DirpError> {
    println!("");
    for marked_file in marked_files_list(&root_path, &dirp_state) {
        println!("{}", marked_file);
    }
    println!("");
    println!("Move these files to the Trash?");

    let items = vec!["No", "Yes"];
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .interact_on_opt(&Term::stdout())?;

    match selection {
        Some(index) => {
            if index == 1 {
                if let Err(error) = remove_marked_files(root_path.clone(), &dirp_state) {
                    panic!("Error Removing Files: {:#?}", error);
                }
            }
        }
        None => {}
    }
    Ok(())
}

fn build_result_tree(path: &String, include_all: bool, dirp_state: &DirHash) -> Dir {
    let root_dir = dirp_state.get(path).expect("internal error");
    _build_result_tree(path, include_all, dirp_state, root_dir.size_in_bytes as f64)
}

fn _build_result_tree(
    path: &String,
    include_all: bool,
    dirp_state: &DirHash,
    total_bytes: f64,
) -> Dir {
    // dirp_state holds all of the dirs in a hash (by path). This code will convert that
    // into a tree structure that the client code expect.

    let result_dir = dirp_state.get(path);
    if let Some(result_dir) = result_dir {
        let mut result_dir = result_dir.clone();
        result_dir.percent = ((result_dir.size_in_bytes as f64 / total_bytes) * 100.0) as u8;

        let mut new_dir_obj_list = Vec::<FSObj>::new();
        if result_dir.is_open || include_all {
            for child_obj in &result_dir.dir_obj_list {
                match child_obj {
                    FSObj::Dir(_) => {
                        assert!(false, "Internal Error");
                    }
                    FSObj::DirRef(dir_ref) => {
                        new_dir_obj_list.push(FSObj::Dir(_build_result_tree(
                            &dir_ref.path,
                            include_all,
                            dirp_state,
                            total_bytes,
                        )));
                    }
                    FSObj::File(fs_obj) => {
                        let mut fs_obj = fs_obj.clone();
                        fs_obj.percent =
                            ((fs_obj.size_in_bytes as f64 / total_bytes) * 100.0) as u8;
                        new_dir_obj_list.push(FSObj::File(fs_obj));
                    }
                    FSObj::SymLink(fs_obj) => {
                        let mut fs_obj = fs_obj.clone();
                        fs_obj.percent =
                            ((fs_obj.size_in_bytes as f64 / total_bytes) * 100.0) as u8;
                        new_dir_obj_list.push(FSObj::SymLink(fs_obj));
                    }
                }
            }
        }
        result_dir.dir_obj_list = new_dir_obj_list;

        result_dir
    } else {
        Dir {
            path: path.clone(),
            size_in_bytes: 0,
            percent: 0,
            is_marked: false,
            is_open: false,
            dir_obj_list: FSObjList::new(),
        }
    }
}

fn is_path_marked(path: &String, dirp_state: &DirHash) -> Option<bool> {
    if let Some(dir) = dirp_state.get(path) {
        Some(dir.is_marked)
    } else {
        let parent_dir = dirp_state.get(&parent_file_path(path)?)?;
        for child in &parent_dir.dir_obj_list {
            match child {
                FSObj::Dir(obj) => {
                    if obj.path == *path {
                        return Some(obj.is_marked);
                    }
                }
                FSObj::DirRef(obj) => {
                    if obj.path == *path {
                        return Some(obj.is_marked);
                    }
                }
                FSObj::File(obj) => {
                    if obj.path == *path {
                        return Some(obj.is_marked);
                    }
                }
                FSObj::SymLink(obj) => {
                    if obj.path == *path {
                        return Some(obj.is_marked);
                    }
                }
            }
        }
        None
    }
}

fn do_mark_deep(path: &String, is_marked: bool, dirp_state: &mut DirHash) {
    if let None = _do_mark_deep(path, is_marked, dirp_state) {
        assert!(false, "Internal error in do_mark_deep");
    }
}

fn _do_mark_deep(path: &String, is_marked: bool, dirp_state: &mut DirHash) -> Option<()> {
    // Mark all objects as 'is_marked' from 'path' all the way down the tree.

    // Find 'path' in 'dirp_state'.
    if let Some(dir) = dirp_state.get_mut(path) {
        // path resolves to a dir.

        dir.is_marked = is_marked;
        let mut child_path_list = Vec::new();
        for child in &mut dir.dir_obj_list {
            match child {
                FSObj::Dir(dir) => {
                    child_path_list.push(dir.path.clone());
                }
                FSObj::DirRef(dir_ref) => {
                    child_path_list.push(dir_ref.path.clone());
                }
                FSObj::File(file) => {
                    child_path_list.push(file.path.clone());
                }
                FSObj::SymLink(sym_link) => {
                    child_path_list.push(sym_link.path.clone());
                }
            }
        }
        for child_path in child_path_list {
            _do_mark_deep(&child_path, is_marked, dirp_state)?;
        }

        Some(())
    } else {
        // path must be a file or a sym_link.
        // Look for parent, then search parent.

        let parent_dir = dirp_state.get_mut(&parent_file_path(path)?)?;
        for child in &mut parent_dir.dir_obj_list {
            match child {
                FSObj::File(file) => {
                    if file.path == *path {
                        // 'path' is a file.
                        file.is_marked = is_marked;

                        return Some(());
                    }
                }
                FSObj::SymLink(sym_link) => {
                    if sym_link.path == *path {
                        // 'path' is a sym link.
                        sym_link.is_marked = is_marked;

                        return Some(());
                    }
                }
                _ => {
                    // Do nothing.
                }
            }
        }

        assert!(false, "path {:#?} not found in dirp_state.", &path,);
        None
    }
}

fn marked_files_list(path: &String, dirp_state: &DirHash) -> Vec<String> {
    let mut marked_files_list = Vec::new();

    _marked_files_list(
        &build_result_tree(&path, true, dirp_state),
        &mut marked_files_list,
    );
    marked_files_list.sort();

    marked_files_list
}

fn _marked_files_list(dir: &Dir, marked_files_list: &mut Vec<String>) {
    if dir.is_marked {
        marked_files_list.push(dir.path.clone());
    } else {
        for child in &dir.dir_obj_list {
            match child {
                FSObj::Dir(child_dir) => {
                    _marked_files_list(&child_dir, marked_files_list);
                }
                FSObj::DirRef(_dir_ref) => {
                    panic!("Internal Error");
                }
                FSObj::File(file) => {
                    if file.is_marked {
                        marked_files_list.push(file.path.clone());
                    }
                }
                FSObj::SymLink(sym_link) => {
                    if sym_link.is_marked {
                        marked_files_list.push(sym_link.path.clone());
                    }
                }
            }
        }
    }
}

fn remove_marked_files(path: String, dirp_state: &DirHash) -> Result<(), DirpError> {
    let mut marked_files_list = Vec::new();

    _remove_marked_files(
        FSObj::Dir(build_result_tree(&path, true, dirp_state)),
        &mut marked_files_list,
    )?;

    trash::delete_all(marked_files_list)?;

    Ok(())
}

fn _remove_marked_files(obj: FSObj, marked_files_list: &mut Vec<String>) -> Result<(), DirpError> {
    match obj {
        FSObj::Dir(dir) => {
            if dir.is_marked {
                trash::delete(&dir.path)?;
            } else {
                for child in dir.dir_obj_list {
                    _remove_marked_files(child, marked_files_list)?;
                }
            }
        }
        FSObj::DirRef(_dir_ref) => {
            panic!("Internal Error");
        }
        FSObj::File(file) => {
            if file.is_marked {
                marked_files_list.push(file.path);
            }
        }
        FSObj::SymLink(sym_link) => {
            if sym_link.is_marked {
                marked_files_list.push(sym_link.path)
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    #[test]
    fn test_dirp_state_task() -> Result<(), DirpError> {
        let dirp_state = DirpState::new(String::from("./test"));

        // Test initial dirp state.
        println!("Test initial dirp state.");
        if let UserMessage::GetStateResponse(state_response) = dirp_state.recv() {
            let expected_hash = 5806868005216161309 as u64;
            let mut hasher = DefaultHasher::new();
            let dir = state_response.dirp_state;
            dir.hash(&mut hasher);
            let hash = hasher.finish();

            println!("{:#?}", dir);
            println!("Hash: {}", hash);

            assert_eq!(expected_hash, hash, "Error: Unexpected result.");
        } else {
            assert!(false, "Unexpected user message.");
        }

        // Toggle ./test/e and ./test/e/f open, then mark ./test/e and test result.
        println!("Toggle ./test/e and ./test/e/f open, then mark ./test/e and test result.");
        dirp_state.send(DirpStateMessage::OpenDir(String::from("./test/e")));
        dirp_state.send(DirpStateMessage::OpenDir(String::from("./test/e/f")));
        dirp_state.send(DirpStateMessage::MarkPath(String::from("./test/e")));
        if let UserMessage::GetStateResponse(state_response) = dirp_state.recv() {
            let expected_hash = 3663658119498061311 as u64;
            let mut hasher = DefaultHasher::new();
            let dir = state_response.dirp_state;
            dir.hash(&mut hasher);
            let hash = hasher.finish();

            println!("{:#?}", dir);
            println!("Hash: {}", hash);

            assert_eq!(expected_hash, hash, "Error: Unexpected result.");
        } else {
            assert!(false, "Unexpected user message 2.");
        }

        dirp_state.quit();

        Ok(())
    }
}
