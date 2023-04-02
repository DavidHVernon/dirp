use crate::dir_pruner::dirp_state_thread_spawn;
use std::{
    collections::HashMap,
    hash::Hash,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};

pub type FSObjList = Vec<FSObj>;
pub type DirHash = HashMap<String, Dir>;

#[derive(Debug, Clone, Hash)]
pub enum FSObj {
    File(File),
    SymLink(SymLink),
    Dir(Dir),
    DirRef(DirRef),
}

#[derive(Debug, Clone, Hash)]
pub struct File {
    pub path: String,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
}

#[derive(Debug, Clone, Hash)]
pub struct SymLink {
    pub path: String,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
}

#[derive(Debug, Clone, Hash)]
pub struct Dir {
    pub path: String,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
    pub is_open: bool,
    pub dir_obj_list: FSObjList,
}

#[derive(Debug, Clone, Hash)]
pub struct DirRef {
    pub path: String,
    pub is_open: bool,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
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

#[derive(Debug, Clone)]
pub enum DirpStateMessage {
    DirScanMessage(Dir),
    GetStateRequest,
    OpenDir(String),
    CloseDir(String),
    ToggleDir(String),
    MarkPath(String),
    UnmarkPath(String),
    ToggleMarkPath(String),
    RemoveMarked,
    Timer,
    Quit,
}

#[derive(Debug, Hash)]
pub enum UserMessage {
    GetStateResponse(GetStateResponse),
    Next,
    Previous,
    CloseDir,
    OpenDir,
    ToggleDir,
    MarkPath,
    UnmarkPath,
    ToggleMarkPath,
    RemoveMarked,
    Quit,
}

#[derive(Debug, Hash)]
pub struct GetStateResponse {
    pub dirp_state: Dir,
}

pub struct IntermediateState {
    pub ui_row: Vec<String>,
    pub is_marked: bool,
    pub path: String,
}

pub struct Args {
    pub path: PathBuf,
}

pub struct DirpState {
    pub user_receiver: Receiver<UserMessage>,
    pub user_sender: Sender<UserMessage>,
    dirp_state_sender: Sender<DirpStateMessage>,
    pub thread_handle: JoinHandle<()>,
}

impl DirpState {
    pub fn new(path: String) -> DirpState {
        let (dirp_state_sender, dirp_state_receiver) = channel();
        let (user_sender, user_receiver) = channel();

        // Spawn a long running task to manage dirp state.
        let thread_handle = dirp_state_thread_spawn(
            path,
            user_sender.clone(),
            dirp_state_sender.clone(),
            dirp_state_receiver,
        );

        DirpState {
            user_receiver,
            user_sender,
            dirp_state_sender,
            thread_handle,
        }
    }

    pub fn quit(self) {
        if let Err(error) = self.dirp_state_sender.send(DirpStateMessage::Quit) {
            panic!(
                "DirpState.quit(): Could not send quit message. Error: {:#?}",
                error
            );
        }
        if let Err(error) = self.thread_handle.join() {
            panic!(
                "DirpState.quit(): Could not join thread handle: {:#?}",
                error
            );
        }
    }

    pub fn send(&self, message: DirpStateMessage) {
        if let Err(error) = self.dirp_state_sender.send(message) {
            panic!("DirpState.send(): error: {:#?}", error);
        }
    }

    pub fn recv(&self) -> UserMessage {
        match self.user_receiver.recv() {
            Ok(message) => {
                return message;
            }
            Err(error) => {
                panic!("DirpState.recv(): error: {:#?}", error);
            }
        }
    }

    pub fn request(&self, request: DirpStateMessage) -> UserMessage {
        self.send(request);
        self.recv()
    }
}

#[derive(Debug)]
pub enum DirpError {
    StdIoError(std::io::Error),
    RecvError(std::sync::mpsc::RecvError),
    SendErrorDirpStateMessage(std::sync::mpsc::SendError<DirpStateMessage>),
    SendErrorUserMessage(std::sync::mpsc::SendError<UserMessage>),
    TrashError(trash::Error),
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

impl From<trash::Error> for DirpError {
    fn from(error: trash::Error) -> Self {
        DirpError::TrashError(error)
    }
}
