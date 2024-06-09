use crate::types::*;
use std::fs::DirEntry;
use std::str::FromStr;
use std::sync::mpsc::Sender;
use std::{fs, path::PathBuf};
use threadpool::ThreadPool;

#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
#[cfg(target_os = "macos")]
use std::os::macos::fs::MetadataExt;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;

pub fn scan_dir_path_in_threadpool(
    dir_path: String,
    is_open: bool,
    dirp_state_sender: Sender<DirpStateMessage>,
    threadpool: &ThreadPool,
) {
    threadpool.execute(move || {
        if let Err(_error) = scan_dir_path(dir_path.clone(), is_open, dirp_state_sender) {
            //            panic!("scan_dir_path path: '{}' error: {:#?}", dir_path, error);
            // ToDo: Log this error.
        }
    });
}

pub fn scan_dir_path(
    dir_path: String,
    is_open: bool,
    dirp_state_sender: Sender<DirpStateMessage>,
) -> Result<(), DirpError> {
    // Create a list containing a FSObj for each directory item in the
    // specified dir
    let mut fs_obj_list = FSObjList::new();

    match fs::read_dir(dir_path.clone()) {
        Ok(read_dir) => {
            for dir_entry in read_dir {
                let result =
                    |dir_entry: Result<DirEntry, std::io::Error>| -> Result<(), DirpError> {
                        let dir_entry = dir_entry?;
                        let obj_path = dir_entry.path();
                        let obj_path_string = obj_path.to_string_lossy().to_string();
                        let meta_data = obj_path.symlink_metadata()?;

                        if std::env::consts::OS == "macos" {
                            if dir_entry.file_name() == ".DS_Store" {
                                return Ok(());
                            } else if obj_path_string == "/Volumes" {
                                return Ok(());
                            } else if obj_path_string == "/System/Volumes" {
                                return Ok(());
                            }
                        }

                        if meta_data.is_symlink() {
                            fs_obj_list.push(FSObj::SymLink(SymLink {
                                path: obj_path_string,
                                size_in_bytes: meta_data.st_size(),
                                percent: 0,
                                is_marked: false,
                            }));
                        } else if meta_data.is_dir() {
                            fs_obj_list.push(FSObj::DirRef(DirRef {
                                path: obj_path_string,
                                is_open,
                                size_in_bytes: 0,
                                percent: 0,
                                is_marked: false,
                            }));
                        } else if meta_data.is_file() {
                            fs_obj_list.push(FSObj::File(File {
                                path: obj_path_string,
                                size_in_bytes: meta_data.st_size(),
                                percent: 0,
                                is_marked: false,
                            }));
                        }
                        Ok(())
                    }(dir_entry);
                if let Err(_error) = result {
                    // ToDo: Add logging of this error.
                }
            }
        }
        Err(_error) => {
            // ToDo: Log this error.
        }
    }

    // Sent it to the state managing thread.
    dirp_state_sender.send(DirpStateMessage::DirScanMessage(Dir {
        path: dir_path,
        is_open,
        size_in_bytes: 0,
        percent: 0,
        is_marked: false,
        dir_obj_list: fs_obj_list,
    }))?;

    Ok(())
}

pub fn indent_prefix_for_level(level: u32) -> String {
    let mut result = "".to_string();
    for _ in 1..level {
        result = result + " ";
    }
    return result;
}

pub fn human_readable_bytes(bytes: u64) -> String {
    if bytes < 1000 {
        return format!("{} bytes", bytes);
    } else if bytes < 1_000_000 {
        return format!("{:.2} KB", bytes as f64 / 1000.0);
    } else if bytes < 1_000_000_000 {
        return format!("{:.2} MB", bytes as f64 / 1_000_000.0);
    } else {
        return format!("{:.2} GB", bytes as f64 / 1_000_000_000.0);
    }
}

pub fn parent_file_path(file_path: &String) -> Option<String> {
    if let Ok(path) = PathBuf::from_str(&file_path) {
        if let Some(parent) = path.parent() {
            return Some(parent.to_string_lossy().to_string());
        }
    }
    None
}

pub fn file_name(file_path: &String) -> Option<String> {
    if file_path == "/" {
        return Some(file_path.clone());
    } else if let Ok(path) = PathBuf::from_str(file_path) {
        if let Some(file_name) = path.file_name() {
            return Some(file_name.to_string_lossy().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::mpsc::channel;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_scan_dir_path_task() -> Result<(), DirpError> {
        let threadpool = ThreadPool::new(30);
        let (sender, receiver) = channel();

        scan_dir_path_in_threadpool("./test/a".to_string(), true, sender.clone(), &threadpool);

        sleep(Duration::from_secs(1));

        let dirp_state_message = receiver.recv()?;
        if let DirpStateMessage::DirScanMessage(dir) = dirp_state_message {
            let expected_hash = 15929552558369993273 as u64;
            let mut hasher = DefaultHasher::new();
            dir.hash(&mut hasher);
            let hash = hasher.finish();

            println!("Dir: {:#?}", dir);
            println!("Hash: {}", hash);

            assert_eq!(hash, expected_hash, "Error: Unexpected result.");
        } else {
            assert!(false, "Error: Unexpected message type");
        }

        Ok(())
    }
}
