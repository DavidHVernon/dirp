use crate::types::*;
use crate::DirpStateMessage::NoOp;
use crate::UserMessage::GetStateResponse;
use dir_pruner::DirpState;
use std::{
    path::PathBuf,
    rc::Rc,
    sync::{mpsc::Sender, Arc, Mutex},
    thread,
};
use ui::{run_app, App};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{backend::CrosstermBackend, Terminal};

mod dir_pruner;
mod types;
mod ui;
mod utils;

fn input_thread_spawn(user_sender: Sender<UserMessage>) {
    thread::spawn(move || {
        
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let dirp_state = DirpState::new(PathBuf::from("./test"));

    input_thread_spawn(dirp_state.user_sender);

    match dirp_state.user_receiver.recv() {
        Ok(user_message) => match user_message {
            UserMessage::GetStateResponse(dirp_state) => {
                // create app and run it
                let app = App::new();
                let res = run_app(&mut terminal, app);
            }
            UserMessage::NoOp(no_op) => {}
        },
        Err(error) => {
            panic!("recv() error: {}", error);
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
