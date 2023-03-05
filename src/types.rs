use std::path::PathBuf;
use tokio::sync::mpsc::{self, Receiver, Sender};

pub enum FSObj {
    File(File),
    Link(Link),
    Dir(Dir),
}

pub struct File {
    name: PathBuf,
    size_in_bytes: u64,
}
pub struct Link {
    name: PathBuf,
}

pub struct Dir {
    name: PathBuf,
    dir_obj_list: Vec<FSObj>,
}

pub enum DirpError {
    StdIoError(std::io::Error),
}

pub enum DirpStateMessage {
    DirScanMessage(DirScanMessage),
    FSCreateMessage(FSCreateMessage),
    FSDeleteMessage(FSDeleteMessage),
    FSMoveMessage(FSMoveMessage),
}

pub struct DirScanMessage {
    dir_path: PathBuf,
    fs_obj_list: Vec<FSObj>,
}

pub struct FSCreateMessage {
    dir_path: PathBuf,
}

pub struct FSDeleteMessage {
    dir_path: PathBuf,
}

pub struct FSMoveMessage {
    from_dir_path: PathBuf,
    to_dir_path: PathBuf,
}

impl File {
    pub fn new(name: PathBuf, size_in_bytes: u64) -> File {
        File {
            name,
            size_in_bytes,
        }
    }
}

impl Link {
    pub fn new(name: PathBuf) -> Link {
        Link { name }
    }
}

impl Dir {
    pub fn new(name: PathBuf) -> Dir {
        Dir {
            name,
            dir_obj_list: Vec::<FSObj>::new(),
        }
    }
}

impl From<std::io::Error> for DirpError {
    fn from(error: std::io::Error) -> Self {
        DirpError::StdIoError(error)
    }
}
