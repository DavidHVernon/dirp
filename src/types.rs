use uuid::Uuid;

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
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
}

#[derive(Debug, Clone, Hash)]
pub struct SymLink {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
}

#[derive(Debug, Clone, Hash)]
pub struct Dir {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
    pub is_open: bool,
    pub dir_obj_list: FSObjList,
}

#[derive(Debug, Clone, Hash)]
pub struct DirRef {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub is_open: bool,
    pub size_in_bytes: u64,
    pub percent: u8,
    pub is_marked: bool,
}

impl Dir {
    pub fn path(&self, dirp_state: &DirpState) -> String {
        let mut path = self.name.clone();
        let mut parent_id_opt = self.parent_id;
        while let Some(parent_id) = &parent_id_opt {
            let parent = dirp_state.get_dir_ref_by_uuid(parent_id);
            path = format!("{}/{}", parent.name, path);
            parent_id_opt = parent.parent_id;
        }

        path
    }
}

impl DirRef {
    pub fn path(&self, dirp_state: &DirpState) -> String {
        let mut path = self.name.clone();
        let mut parent_id_opt = self.parent_id;
        while let Some(parent_id) = &parent_id_opt {
            let parent = dirp_state.get_dir_ref_by_uuid(parent_id);
            path = format!("{}/{}", parent.name, path);
            parent_id_opt = parent.parent_id;
        }

        path
    }
}

impl File {
    pub fn path(&self, dirp_state: &DirpState) -> String {
        let mut path = self.name.clone();
        if let Some(parent_id) = &self.parent_id {
            let parent_path = dirp_state.get_dir_ref_by_uuid(parent_id).path(&dirp_state);
            path = parent_path + &path;
        }
        path
    }
}

impl SymLink {
    pub fn path(&self, dirp_state: &DirpState) -> String {
        let mut path = self.name.clone();
        if let Some(parent_id) = &self.parent_id {
            let parent_path = dirp_state.get_dir_ref_by_uuid(parent_id).path(&dirp_state);
            path = parent_path + &path;
        }
        path
    }
}

pub trait Path {
    fn path(&self, dirp_state: &DirpState) -> String;
}

impl Path for FSObj {
    fn path(&self, dirp_state: &DirpState) -> String {
        match self {
            FSObj::Dir(o) => o.path(dirp_state),
            FSObj::DirRef(o) => o.path(dirp_state),
            FSObj::File(o) => o.path(dirp_state),
            FSObj::SymLink(o) => o.path(dirp_state),
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
    pub name: String,
}

pub struct Args {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct DirpState {
    uuid_to_dir: HashMap<Uuid, Dir>,
    path_to_uuid: HashMap<String, Uuid>,
}

impl DirpState {
    pub fn new() -> DirpState {
        DirpState {
            uuid_to_dir: HashMap::new(),
            path_to_uuid: HashMap::new(),
        }
    }

    pub fn insert(&mut self, path: String, dir: Dir) -> Option<Dir> {
        let uuid = Uuid::new_v4();
        self.path_to_uuid.insert(path, uuid.clone());
        self.uuid_to_dir.insert(uuid, dir)
    }

    pub fn path_exists(&self, path: &String) -> bool {
        match self.path_to_uuid.get(path) {
            Some(uuid) => true,
            None => false,
        }
    }

    pub fn get_dir_ref_by_uuid(&self, uuid: &Uuid) -> &Dir {
        match self.uuid_to_dir.get(uuid) {
            Some(dir) => dir,
            None => panic!("Invalid call to dir_ref_by_uuid"),
        }
    }

    pub fn get_dir_ref_mut_by_uuid(&mut self, uuid: &Uuid) -> &mut Dir {
        match self.uuid_to_dir.get_mut(uuid) {
            Some(dir) => dir,
            None => panic!("Invalid call to dir_ref_by_uuid"),
        }
    }

    pub fn get_dir_ref_by_path(&self, path: &String) -> &Dir {
        match self.path_to_uuid.get(path) {
            Some(uuid) => match self.uuid_to_dir.get(uuid) {
                Some(dir) => dir,
                None => panic!("Invalid call to dir_ref_by_path"),
            },
            None => panic!("Invalid call to dir_ref_by_path"),
        }
    }

    pub fn get_dir_ref_mut_by_path(&mut self, path: &String) -> &mut Dir {
        match self.path_to_uuid.get(path) {
            Some(uuid) => match self.uuid_to_dir.get_mut(uuid) {
                Some(dir) => dir,
                None => panic!("Invalid call to dir_ref_by_path"),
            },
            None => panic!("Invalid call to dir_ref_by_path"),
        }
    }
}

pub struct DirpStateThread {
    pub user_receiver: Receiver<UserMessage>,
    pub user_sender: Sender<UserMessage>,
    dirp_state_sender: Sender<DirpStateMessage>,
    pub thread_handle: JoinHandle<()>,
}

impl DirpStateThread {
    pub fn new(path: String) -> DirpStateThread {
        let (dirp_state_sender, dirp_state_receiver) = channel();
        let (user_sender, user_receiver) = channel();

        // Spawn a long running task to manage dirp state.
        let thread_handle = dirp_state_thread_spawn(
            path,
            user_sender.clone(),
            dirp_state_sender.clone(),
            dirp_state_receiver,
        );

        DirpStateThread {
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
