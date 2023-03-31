use crate::dir_pruner::dirp_state_thread_spawn;
use std::{
    collections::HashMap,
    hash::Hash,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};

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
    pub percent: u8,
    pub is_marked: bool,
}

#[derive(Debug, Clone, Hash)]
pub struct SymLink {
    pub path: PathBuf,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
}

#[derive(Debug, Clone, Hash)]
pub struct Dir {
    pub path: PathBuf,
    pub is_open: bool,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
    pub dir_obj_list: FSObjList,
}

#[derive(Debug, Clone, Hash)]
pub struct DirRef {
    pub path: PathBuf,
    pub is_open: bool,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
}

#[derive(Debug, Clone, Hash)]
pub enum ClientFSObj<'a> {
    ClientFile(ClientFile<'a>),
    ClientDir(ClientDir<'a>),
}

#[derive(Debug, Clone, Hash)]
pub struct ClientDir<'a> {
    pub name: String,
    pub is_open: bool,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
    pub dir_obj_list: Vec<ClientFSObj<'a>>,
    pub parent: Option<&'a ClientFSObj<'a>>,
}

#[derive(Debug, Clone, Hash)]
pub struct ClientFile<'a> {
    pub name: String,
    pub is_link: bool,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
    pub parent: Option<&'a ClientFSObj<'a>>,
}

pub trait Path {
    fn path(&self) -> String;
}

impl Path for ClientFSObj<'_> {
    fn path(&self) -> String {
        match self {
            ClientFSObj::ClientDir(dir) => self.path(),
            ClientFSObj::ClientFile(file) => self.path(),
        }
    }
}

impl ClientDir<'_> {
    pub fn path(&self) -> String {
        let mut path = self.name;
        if let Some(parent) = self.parent {
            if let ClientFSObj::ClientDir(parent) = parent {
                path = format!("{}/{}", parent.name, path);
            }
        }
        path
    }
}

impl ClientFile<'_> {
    pub fn path(&self) -> String {
        let mut path = self.name;
        if let Some(parent) = self.parent {
            if let ClientFSObj::ClientDir(parent) = parent {
                path = format!("{}/{}", parent.name, path);
            }
        }
        path
    }
}

impl From<Dir> for ClientDir<'_> {
    fn from(dir: Dir) -> Self {
        ClientDir {
            name: dir
                .path
                .file_name()
                .expect("No file name.")
                .to_string_lossy()
                .to_string(),
            is_open: dir.is_open,
            size_in_bytes: dir.size_in_bytes,
            percent: dir.percent,
            is_marked: dir.is_marked,
            dir_obj_list: Vec::new(),
            parent: None,
        }
    }
}

impl From<DirRef> for ClientDir<'_> {
    fn from(dir_ref: DirRef) -> Self {
        ClientDir {
            name: dir_ref
                .path
                .file_name()
                .expect("No file name.")
                .to_string_lossy()
                .to_string(),
            is_open: dir_ref.is_open,
            size_in_bytes: dir_ref.size_in_bytes,
            percent: dir_ref.percent,
            is_marked: dir_ref.is_marked,
            dir_obj_list: Vec::new(),
            parent: None,
        }
    }
}

impl From<File> for ClientFile<'_> {
    fn from(file: File) -> Self {
        ClientFile {
            name: file
                .path
                .file_name()
                .expect("No file name.")
                .to_string_lossy()
                .to_string(),
            is_link: false,
            size_in_bytes: file.size_in_bytes,
            percent: file.percent,
            is_marked: file.is_marked,
            parent: None,
        }
    }
}

impl From<SymLink> for ClientFile<'_> {
    fn from(sym_link: SymLink) -> Self {
        ClientFile {
            name: sym_link
                .path
                .file_name()
                .expect("No file name.")
                .to_string_lossy()
                .to_string(),
            is_link: true,
            size_in_bytes: sym_link.size_in_bytes,
            percent: sym_link.percent,
            is_marked: sym_link.is_marked,
            parent: None,
        }
    }
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

impl SizeInBytes for ClientFSObj<'_> {
    fn size_in_bytes(&self) -> u64 {
        match self {
            ClientFSObj::ClientDir(dir) => dir.size_in_bytes,
            ClientFSObj::ClientFile(file) => file.size_in_bytes,
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
pub enum UserMessage<'a> {
    GetStateResponse(GetStateResponse<'a>),
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
pub struct GetStateResponse<'a> {
    pub result_tree: ClientDir<'a>,
}

pub struct IntermediateState {
    pub ui_row: Vec<String>,
    pub is_marked: bool,
    pub name: String,
}

pub struct Args {
    pub path: PathBuf,
}

pub struct DirpState<'a> {
    pub user_receiver: Receiver<UserMessage<'a>>,
    pub user_sender: Sender<UserMessage<'a>>,
    dirp_state_sender: Sender<DirpStateMessage>,
    pub thread_handle: JoinHandle<()>,
}

impl DirpState<'_> {
    pub fn new<'a>(path: PathBuf) -> DirpState<'a> {
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
pub enum DirpError<'a> {
    StdIoError(std::io::Error),
    RecvError(std::sync::mpsc::RecvError),
    SendErrorDirpStateMessage(std::sync::mpsc::SendError<DirpStateMessage>),
    SendErrorUserMessage(std::sync::mpsc::SendError<UserMessage<'a>>),
    TrashError(trash::Error),
}

impl From<std::io::Error> for DirpError<'_> {
    fn from(error: std::io::Error) -> Self {
        DirpError::StdIoError(error)
    }
}

impl From<std::sync::mpsc::RecvError> for DirpError<'_> {
    fn from(error: std::sync::mpsc::RecvError) -> Self {
        DirpError::RecvError(error)
    }
}

impl From<std::sync::mpsc::SendError<DirpStateMessage>> for DirpError<'_> {
    fn from(error: std::sync::mpsc::SendError<DirpStateMessage>) -> Self {
        DirpError::SendErrorDirpStateMessage(error)
    }
}

impl From<std::sync::mpsc::SendError<UserMessage<'_>>> for DirpError<'_> {
    fn from(error: std::sync::mpsc::SendError<UserMessage>) -> Self {
        DirpError::SendErrorUserMessage(error)
    }
}

impl From<trash::Error> for DirpError<'_> {
    fn from(error: trash::Error) -> Self {
        DirpError::TrashError(error)
    }
}
