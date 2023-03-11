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

#[derive(Debug)]
pub enum DirpError {
    StdIoError(std::io::Error),
    RecvError(std::sync::mpsc::RecvError),
    SendErrorDirpStateMessage(std::sync::mpsc::SendError<DirpStateMessage>),
    SendErrorUserMessage(std::sync::mpsc::SendError<UserMessage>),
}

#[derive(Debug)]
pub enum DirpStateMessage {
    DirScanMessage(Dir),
    GetStateRequest,
    NoOp(bool),
    Quit,
}

#[derive(Debug)]
pub enum UserMessage {
    GetStateResponse(GetStateResponse),
}

#[derive(Debug)]
pub struct GetStateResponse {
    pub dirp_state: DirHash,
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
