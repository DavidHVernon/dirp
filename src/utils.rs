use crate::types::*;
use std::{fs, os::macos::fs::MetadataExt, path::PathBuf};
use tokio::sync::mpsc::Sender;

pub fn scan_dir_path_task(dir_path: &PathBuf, dirp_state_sender: Sender<DirpStateMessage>) {
    let dir_path = dir_path.clone();
    tokio::spawn(async move {
        let x = scan_dir_path(dir_path, dirp_state_sender);
        // ToDo: Handle the error (log to file?)
    });
}

pub fn scan_dir_path(
    dir_path: PathBuf,
    dirp_state_sender: Sender<DirpStateMessage>,
) -> Result<(), DirpError> {
    // Create a list containing a FSObj for each directory item in the
    // specified dir and return it.
    let mut dir_obj_list = Vec::<FSObj>::new();
    for dir_entry in fs::read_dir(dir_path.clone())? {
        let dir_entry = dir_entry?;
        let fs_obj_path = dir_entry.path();
        let fs_obj_meta_data = dir_entry.metadata()?;

        if fs_obj_path.is_dir() {
            dir_obj_list.push(FSObj::Dir(Dir::new(fs_obj_path.clone())));
            scan_dir_path_task(&fs_obj_path, dirp_state_sender.clone());
        } else if fs_obj_path.is_file() {
            dir_obj_list.push(FSObj::File(File::new(
                fs_obj_path,
                fs_obj_meta_data.st_size(),
            )))
        } else if fs_obj_path.is_symlink() {
            dir_obj_list.push(FSObj::Link(Link::new(fs_obj_path)))
        }
    }
    // ToDo: Send dir_obj_list to dirp state.
    Ok(())
}
