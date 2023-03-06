use std::path::PathBuf;

#[derive(Debug)]

pub enum FSObj {
    File(File),
    Link(Link),
    Dir(Dir),
}

#[derive(Debug)]
pub struct File {
    name: PathBuf,
    size_in_bytes: u64,
}
#[derive(Debug)]
pub struct Link {
    name: PathBuf,
}

#[derive(Debug)]
pub struct Dir {
    name: PathBuf,
    dir_obj_list: Vec<FSObj>,
}

#[derive(Debug)]
pub enum DirpError {
    StdIoError(std::io::Error),
    JoinError(tokio::task::JoinError),
}

#[derive(Debug)]
pub enum DirpStateMessage {
    DirScanMessage(DirScanMessage),
    FSCreateMessage(FSCreateMessage),
    FSDeleteMessage(FSDeleteMessage),
    FSMoveMessage(FSMoveMessage),
}

#[derive(Debug)]
pub struct DirScanMessage {
    pub dir_path: PathBuf,
    pub fs_obj_list: Vec<FSObj>,
}

#[derive(Debug)]
pub struct FSCreateMessage {
    pub dir_path: PathBuf,
}

#[derive(Debug)]
pub struct FSDeleteMessage {
    pub dir_path: PathBuf,
}

#[derive(Debug)]
pub struct FSMoveMessage {
    pub from_dir_path: PathBuf,
    pub to_dir_path: PathBuf,
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

impl From<tokio::task::JoinError> for DirpError {
    fn from(error: tokio::task::JoinError) -> Self {
        DirpError::JoinError(error)
    }
}
