use std::sync::mpsc::{RecvError, SendError};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone)]
pub enum FSObj {
    File(File),
    Link(Link),
    Dir(Dir),
}

#[derive(Debug, Clone)]
pub struct File {
    pub name: PathBuf,
    pub size_in_bytes: u64,
}
#[derive(Debug, Clone)]
pub struct Link {
    pub name: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Dir {
    pub name: PathBuf,
    pub dir_obj_list: Vec<FSObj>,
}

#[derive(Debug)]
pub enum DirpError {
    StdIoError(std::io::Error),
    RecvError(std::sync::mpsc::RecvError),
    SendErrorDirpStateMessage(std::sync::mpsc::SendError<DirpStateMessage>),
}

#[derive(Debug)]
pub enum DirpStateMessage {
    DirScanMessage(DirScanMessage),
    FSCreateMessage(FSCreateMessage),
    FSDeleteMessage(FSDeleteMessage),
    FSMoveMessage(FSMoveMessage),
    GetStateRequest,
    GetStateResponse(GetStateResponse),
    Quit,
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

#[derive(Debug)]
pub struct GetStateResponse {
    pub dirp_state: HashMap<PathBuf, Vec<FSObj>>,
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

impl From<std::sync::mpsc::RecvError> for DirpError {
    fn from(error: std::sync::mpsc::RecvError) -> Self {
        DirpError::RecvError(error)
    }
}

impl From<std::sync::mpsc::SendError<DirpStateMessage>> for DirpError {
    fn from(error: std::sync::mpsc::SendError<DirpStateMessage>) -> Self {
        DirpError::SendErrorDirpStateMessage(error)
    }
}
