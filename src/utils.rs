use crate::types::*;
use std::fs::DirEntry;
use std::os::unix::thread;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{fs, os::macos::fs::MetadataExt, path::PathBuf};
use threadpool::ThreadPool;

pub fn scan_dir_path_in_threadpool(
    dir_path: PathBuf,
    dirp_state_sender: Sender<DirpStateMessage>,
    threadpool: ThreadPool,
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
    // specified dir and return it.
    let mut fs_obj_list = Vec::<FSObj>::new();

    for dir_entry in fs::read_dir(dir_path.clone())? {
        let dir_entry = dir_entry?;
        let obj_path = dir_entry.path();
        let meta_data = dir_entry.metadata()?;

        if obj_path.is_dir() {
            fs_obj_list.push(FSObj::Dir(Dir::new(obj_path.clone())));
        } else if obj_path.is_file() {
            fs_obj_list.push(FSObj::File(File::new(obj_path, meta_data.st_size())))
        } else if obj_path.is_symlink() {
            fs_obj_list.push(FSObj::Link(Link::new(obj_path)))
        }
    }

    dirp_state_sender.send(DirpStateMessage::DirScanMessage(DirScanMessage {
        dir_path,
        fs_obj_list,
    }))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_dir_path_task() -> Result<(), DirpError> {
        let threadpool = ThreadPool::new(30);
        let (sender, mut receiver) = channel();

        scan_dir_path_in_threadpool(PathBuf::from("./test/a"), sender.clone(), threadpool);

        let dirp_state_message = receiver.recv()?;
        if let DirpStateMessage::DirScanMessage(dir_scan_message) = dirp_state_message {
            let fs_obj_list = dir_scan_message.fs_obj_list;
            let expected_result = r#"[File(File{name:"./test/a/3.txt",size_in_bytes:1010,},),File(File{name:"./test/a/2.txt",size_in_bytes:1010,},),File(File{name:"./test/a/1.txt",size_in_bytes:1010,},),]"#;
            let result = format!("{:#?}", fs_obj_list)
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>();

            assert_eq!(result, expected_result, "Error: Unexpected result.");
        } else {
            assert!(false, "Error: Unexpected message type");
        }

        Ok(())
    }
}
