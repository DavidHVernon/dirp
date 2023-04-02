use crate::types::*;
use crate::utils::*;
use chrono::format::format;
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

pub fn dirp_state_loop(
    root_path: String,
    user_sender: Sender<UserMessage>,
    dirp_state_sender: Sender<DirpStateMessage>,
    dirp_state_receiver: Receiver<DirpStateMessage>,
) -> Result<(), DirpError> {
    let mut dirp_state = DirpState::new();
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
        message_timer.schedule_repeating(Duration::milliseconds(100), DirpStateMessage::Timer);

    // Event Loop
    loop {
        match dirp_state_receiver.recv() {
            Ok(message) => match message {
                DirpStateMessage::DirScanMessage(dir) => {
                    process_dir_scan_message(dir, &mut dirp_state, &dirp_state_sender, &threadpool);
                    is_state_dirty = true;
                }
                DirpStateMessage::GetStateRequest => {
                    user_sender.send(UserMessage::GetStateResponse(GetStateResponse {
                        dirp_state: build_result_tree(&root_path, false, &dirp_state),
                    }))?;
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
                    let dir = dirp_state.get_dir_ref_mut_by_path(&path);
                    dir.is_open = true;
                    is_state_dirty = true;
                }
                DirpStateMessage::CloseDir(path) => {
                    let dir = dirp_state.get_dir_ref_mut_by_path(&path);
                    dir.is_open = false;
                    is_state_dirty = true;
                }
                DirpStateMessage::ToggleDir(path) => {
                    let dir = dirp_state.get_dir_ref_mut_by_path(&path);
                    dir.is_open = !dir.is_open;
                    is_state_dirty = true;
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

fn process_dir_scan_message(
    mut dir: Dir,
    dirp_state: &mut DirpState,
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
                    dir_ref.path(dirp_state),
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
    let mut parent_path_opt = parent_file_path(&dir.path(&dirp_state));
    while let Some(parent_path) = parent_path_opt {
        let parent_dir = dirp_state.get_dir_ref_mut_by_path(&parent_path);
        parent_dir.size_in_bytes += dir.size_in_bytes;
        parent_path_opt = parent_file_path(&parent_path);
    }

    // Update state.
    dirp_state.insert(dir.path(&dirp_state), dir);
}

fn process_remove_marked(root_path: &String, dirp_state: &DirpState) -> Result<(), DirpError> {
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

fn build_result_tree(path: &String, include_all: bool, dirp_state: &DirpState) -> Dir {
    let root_dir = dirp_state.get_dir_ref_by_path(path);
    _build_result_tree(path, include_all, dirp_state, root_dir.size_in_bytes as f64)
}

fn _build_result_tree(
    path: &String,
    include_all: bool,
    dirp_state: &DirpState,
    total_bytes: f64,
) -> Dir {
    let mut result_dir = dirp_state.get_dir_ref_by_path(path).clone();
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
                        &dir_ref.path(dirp_state),
                        include_all,
                        dirp_state,
                        total_bytes,
                    )));
                }
                FSObj::File(fs_obj) => {
                    let mut fs_obj = fs_obj.clone();
                    fs_obj.percent = ((fs_obj.size_in_bytes as f64 / total_bytes) * 100.0) as u8;
                    new_dir_obj_list.push(FSObj::File(fs_obj));
                }
                FSObj::SymLink(fs_obj) => {
                    let mut fs_obj = fs_obj.clone();
                    fs_obj.percent = ((fs_obj.size_in_bytes as f64 / total_bytes) * 100.0) as u8;
                    new_dir_obj_list.push(FSObj::SymLink(fs_obj));
                }
            }
        }
    }
    result_dir.dir_obj_list = new_dir_obj_list;

    result_dir
}

fn is_path_marked(path: &String, dirp_state: &DirpState) -> Option<bool> {
    if dirp_state.path_exists(path) {
        let dir = dirp_state.get_dir_ref_by_path(path);
        Some(dir.is_marked)
    } else {
        let parent_dir = dirp_state.get_dir_ref_by_path(&parent_file_path(path)?);
        for child in &parent_dir.dir_obj_list {
            match child {
                FSObj::Dir(obj) => {
                    if obj.path(dirp_state) == *path {
                        return Some(obj.is_marked);
                    }
                }
                FSObj::DirRef(obj) => {
                    if obj.path(dirp_state) == *path {
                        return Some(obj.is_marked);
                    }
                }
                FSObj::File(obj) => {
                    if obj.path(dirp_state) == *path {
                        return Some(obj.is_marked);
                    }
                }
                FSObj::SymLink(obj) => {
                    if obj.path(dirp_state) == *path {
                        return Some(obj.is_marked);
                    }
                }
            }
        }
        None
    }
}

fn do_mark_deep(path: &String, is_marked: bool, dirp_state: &mut DirpState) {
    if let None = _do_mark_deep(path, is_marked, dirp_state) {
        assert!(false, "Internal error in do_mark_deep");
    }
}

fn _do_mark_deep(path: &String, is_marked: bool, dirp_state: &mut DirpState) -> Option<()> {
    // Mark all objects as 'is_marked' from 'path' all the way down the tree.

    // Find 'path' in 'dirp_state'.
    if dirp_state.path_exists(path) {
        // path resolves to a dir.
        let dir = dirp_state.get_dir_ref_mut_by_path(path);
        dir.is_marked = is_marked;
        let mut child_path_list = Vec::new();
        for child in &mut dir.dir_obj_list {
            match child {
                FSObj::Dir(dir) => {
                    child_path_list.push(format!("{}/{}", path, dir.name));
                }
                FSObj::DirRef(dir_ref) => {
                    child_path_list.push(format!("{}/{}", path, dir_ref.name));
                }
                FSObj::File(file) => {
                    child_path_list.push(format!("{}/{}", path, file.name));
                }
                FSObj::SymLink(sym_link) => {
                    child_path_list.push(format!("{}/{}", path, sym_link.name));
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

        let parent_path = parent_file_path(path)?;
        let parent_dir = dirp_state.get_dir_ref_mut_by_path(&parent_path);
        for child in &mut parent_dir.dir_obj_list {
            match child {
                FSObj::File(file) => {
                    let file_path = format!("{}/{}", parent_path, file.name);
                    if file_path == *path {
                        // 'path' is a file.
                        file.is_marked = is_marked;

                        return Some(());
                    }
                }
                FSObj::SymLink(sym_link) => {
                    let sym_link_path = format!("{}/{}", parent_path, sym_link.name);
                    if sym_link_path == *path {
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

fn marked_files_list(path: &String, dirp_state: &DirpState) -> Vec<String> {
    let mut marked_files_list = Vec::new();

    _marked_files_list(
        &build_result_tree(&path, true, dirp_state),
        dirp_state,
        &mut marked_files_list,
    );
    marked_files_list.sort();

    marked_files_list
}

fn _marked_files_list(dir: &Dir, dirp_state: &DirpState, marked_files_list: &mut Vec<String>) {
    if dir.is_marked {
        marked_files_list.push(dir.path(dirp_state));
    } else {
        for child in &dir.dir_obj_list {
            match child {
                FSObj::Dir(child_dir) => {
                    _marked_files_list(&child_dir, dirp_state, marked_files_list);
                }
                FSObj::DirRef(_dir_ref) => {
                    panic!("Internal Error");
                }
                FSObj::File(file) => {
                    if file.is_marked {
                        marked_files_list.push(file.path(dirp_state));
                    }
                }
                FSObj::SymLink(sym_link) => {
                    if sym_link.is_marked {
                        marked_files_list.push(sym_link.path(dirp_state));
                    }
                }
            }
        }
    }
}

fn remove_marked_files(path: String, dirp_state: &DirpState) -> Result<(), DirpError> {
    _remove_marked_files(
        FSObj::Dir(build_result_tree(&path, true, dirp_state)),
        dirp_state,
    )
}

fn _remove_marked_files(obj: FSObj, dirp_state: &DirpState) -> Result<(), DirpError> {
    match obj {
        FSObj::Dir(dir) => {
            if dir.is_marked {
                trash::delete(&dir.path(dirp_state))?;
            } else {
                for child in dir.dir_obj_list {
                    _remove_marked_files(child, dirp_state)?;
                }
            }
        }
        FSObj::DirRef(_dir_ref) => {
            panic!("Internal Error");
        }
        FSObj::File(file) => {
            if file.is_marked {
                trash::delete(&file.path(dirp_state))?;
            }
        }
        FSObj::SymLink(sym_link) => {
            if sym_link.is_marked {
                trash::delete(&sym_link.path(dirp_state))?;
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
        let dirp_state = DirpStateThread::new("./test".to_string());

        // Test initial dirp state.
        println!("Test initial dirp state.");
        if let UserMessage::GetStateResponse(state_response) = dirp_state.recv() {
            let expected_hash = 1916964086373338034 as u64;
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
        dirp_state.send(DirpStateMessage::OpenDir("./test/e".to_string()));
        dirp_state.send(DirpStateMessage::OpenDir("./test/e/f".to_string()));
        dirp_state.send(DirpStateMessage::MarkPath("./test/e".to_string()));
        if let UserMessage::GetStateResponse(state_response) = dirp_state.recv() {
            let expected_hash = 1716204932036142087 as u64;
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
