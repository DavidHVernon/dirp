use std::{collections::HashMap, hash::Hash, path::PathBuf};

pub type FSObjList = Vec<FSObj>;
pub type DirHash = HashMap<PathBuf, Dir>;

#[derive(Debug, Clone, Hash)]
pub enum FSObj {
    File(File),
    SymLink(SymLink),
    Dir(Dir),
    DirRef(DirRef),
}

#[derive(Debug, Clone, Hash)]
pub struct File {
    pub path: PathBuf,
    pub size_in_bytes: u64,
}
#[derive(Debug, Clone, Hash)]
pub struct SymLink {
    pub path: PathBuf,
    pub size_in_bytes: u64,
}

#[derive(Debug, Clone, Hash)]
pub struct Dir {
    pub path: PathBuf,
    pub size_in_bytes: u64,
    pub dir_obj_list: FSObjList,
}

#[derive(Debug, Clone, Hash)]
pub struct DirRef {
    pub path: PathBuf,
    pub size_in_bytes: u64,
}

pub trait SizeInBytes {
    fn size_in_bytes(&self) -> u64;
}

impl SizeInBytes for FSObj {
    fn size_in_bytes(&self) -> u64 {
        match self {
            FSObj::Dir(dir) => dir.size_in_bytes,
            FSObj::DirRef(dir_ref) => dir_ref.size_in_bytes,
            FSObj::File(file) => file.size_in_bytes,
            FSObj::SymLink(sym_link) => sym_link.size_in_bytes,
        }
    }
}

#[derive(Debug)]
pub enum DirpError {
    StdIoError(std::io::Error),
    RecvError(std::sync::mpsc::RecvError),
    SendErrorDirpStateMessage(std::sync::mpsc::SendError<DirpStateMessage>),
    SendErrorUserMessage(std::sync::mpsc::SendError<UserMessage>),
}

#[derive(Debug, Clone)]
pub enum DirpStateMessage {
    DirScanMessage(Dir),
    GetStateRequest,
    NoOp(bool),
    Timer,
    Quit,
}

#[derive(Debug, Hash)]
pub enum UserMessage {
    GetStateResponse(GetStateResponse),
    UserInputNext,
    UserInputPrevious,
    UserInputQuit,
}

#[derive(Debug, Hash)]
pub struct GetStateResponse {
    pub dirp_state: Dir,
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

impl From<std::sync::mpsc::SendError<UserMessage>> for DirpError {
    fn from(error: std::sync::mpsc::SendError<UserMessage>) -> Self {
        DirpError::SendErrorUserMessage(error)
    }
}
