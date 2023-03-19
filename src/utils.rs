use crate::types::*;
use std::sync::mpsc::Sender;
use std::{fs, os::macos::fs::MetadataExt, path::PathBuf};
use threadpool::ThreadPool;

pub fn scan_dir_path_in_threadpool(
    dir_path: PathBuf,
    dirp_state_sender: Sender<DirpStateMessage>,
    threadpool: &ThreadPool,
) {
    threadpool.execute(move || {
        if let Err(error) = scan_dir_path(dir_path, dirp_state_sender) {
            panic!("scan_dir_path error: {:#?}", error);
        }
    });
}

pub fn scan_dir_path(
    dir_path: PathBuf,
    dirp_state_sender: Sender<DirpStateMessage>,
) -> Result<(), DirpError> {
    // Create a list containing a FSObj for each directory item in the
    // specified dir
    let mut fs_obj_list = FSObjList::new();

    match fs::read_dir(dir_path.clone()) {
        Ok(read_dir) => {
            for dir_entry in read_dir {
                let dir_entry = dir_entry?;
                let obj_path = dir_entry.path();
                let meta_data = dir_entry.metadata()?;

                if obj_path.to_string_lossy() == "/Users/davidvernon/." {
                    let brk = true;
                }
                if dir_entry.file_name() == ".DS_Store" {
                    continue;
                }

                if obj_path.is_dir() {
                    fs_obj_list.push(FSObj::DirRef(DirRef {
                        path: obj_path.clone(),
                        is_open: true,
                        size_in_bytes: 0,
                        percent: 0,
                    }));
                } else if obj_path.is_symlink() {
                    // NOTE: Symlink needs to be checked before file because symlinks
                    // are files.
                    fs_obj_list.push(FSObj::SymLink(SymLink {
                        path: obj_path,
                        size_in_bytes: 0,
                        percent: 0,
                    }));
                } else if obj_path.is_file() {
                    fs_obj_list.push(FSObj::File(File {
                        path: obj_path,
                        size_in_bytes: meta_data.st_size(),
                        percent: 0,
                    }));
                }
            }
        }
        Err(error) => {
            println!(
                "Cannot open file (ignoring): {}.",
                dir_path.to_string_lossy()
            );
        }
    }

    // Sent it to the state managing thread.
    dirp_state_sender.send(DirpStateMessage::DirScanMessage(Dir {
        path: dir_path,
        is_open: true,
        size_in_bytes: 0,
        percent: 0,
        dir_obj_list: fs_obj_list,
    }))?;

    Ok(())
}

pub fn indent_to_level(level: u32) -> String {
    let mut result = "".to_string();
    for _ in 1..level {
        result = result + "   ";
    }
    return result;
}

pub fn human_readable_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{} bytes", bytes);
    } else if bytes < 1024 * 1024 {
        return format!("{:.2} KB", bytes as f64 / 1024.0);
    } else if bytes < 1024 * 1024 * 1024 {
        return format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0));
    } else {
        return format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashMap;
    use std::hash::{self, Hash, Hasher};
    use std::sync::mpsc::channel;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_scan_dir_path_task() -> Result<(), DirpError> {
        let threadpool = ThreadPool::new(30);
        let (sender, receiver) = channel();

        scan_dir_path_in_threadpool(PathBuf::from("./test/a"), sender.clone(), &threadpool);

        sleep(Duration::from_secs(1));

        let dirp_state_message = receiver.recv()?;
        if let DirpStateMessage::DirScanMessage(dir) = dirp_state_message {
            let expected_hash = 69190488897742781 as u64;
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
