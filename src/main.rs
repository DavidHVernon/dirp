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
use ui::{step_app, App};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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
        let result = input_thread(user_sender);
        if let Err(error) = result {
            panic!("input_thread: {:?}.", error);
        }
    });
}

fn input_thread(user_sender: Sender<UserMessage>) -> Result<(), DirpError> {
    loop {
        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => {
                    user_sender.send(UserMessage::UserInputQuit)?;
                    return Ok(());
                }
                KeyCode::Down => user_sender.send(UserMessage::UserInputNext)?,
                KeyCode::Up => user_sender.send(UserMessage::UserInputPrevious)?,
                KeyCode::Char('n') => user_sender.send(UserMessage::UserInputPrevious)?,
                KeyCode::Char('p') => user_sender.send(UserMessage::UserInputNext)?,
                _ => {}
            },
            _ => { /* Ignore all other forms of input. */ }
        }
    }
}
fn indent_to_level(level: u32) -> String {
    let mut result = "".to_string();
    for _ in 0..level {
        result = result + "    ";
    }
    return result;
}

fn human_readable_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{} bytes", bytes);
    } else if bytes < 1024 * 1024 {
        return format!("{:.2} KB", bytes as f64 / 1024.0);
    } else if bytes < 1024 * 1024 * 1024 {
        return format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0));
    } else {
        return format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0));
    }
}

fn dirp_state_to_intermediate_state(
    fs_obj: FSObj,
    level: u32,
    i_state: &mut Vec<Vec<String>>,
) -> Option<()> {
    let row = match fs_obj {
        FSObj::Dir(dir) => {
            let name = dir.path.file_name()?.to_string_lossy();
            let name = format!("{}{}", indent_to_level(level), name);
            let size = human_readable_bytes(dir.size_in_bytes);
            let file_size = human_readable_bytes(dir.size_in_bytes);

            for sub_dir in dir.dir_obj_list {
                dirp_state_to_intermediate_state(sub_dir, level, i_state);
            }

            vec![name, size, "".to_string()]
        }
        FSObj::DirRef(dir_ref) => {
            let name = dir_ref.path.file_name()?.to_string_lossy();
            let name = format!("{}{}", indent_to_level(level), name);
            let size = human_readable_bytes(dir_ref.size_in_bytes);
            let file_size = human_readable_bytes(dir_ref.size_in_bytes);

            vec![name, size, "".to_string()]
        }
        FSObj::File(file) => {
            let name = file.path.file_name()?.to_string_lossy();
            let name = format!("{}{}", indent_to_level(level), name);
            let size = human_readable_bytes(file.size_in_bytes);
            let file_size = human_readable_bytes(file.size_in_bytes);

            vec![name, size, "".to_string()]
        }
        FSObj::SymLink(sym_link) => {
            let name = sym_link.path.file_name()?.to_string_lossy();
            let name = format!("{}{}", indent_to_level(level), name);
            let size = human_readable_bytes(sym_link.size_in_bytes);
            let file_size = human_readable_bytes(sym_link.size_in_bytes);

            vec![name, size, "".to_string()]
        }
    };
    i_state.push(row);

    Some(())
}

fn i_state_to_app_state<'a>(i_state: &'a Vec<Vec<String>>) -> Vec<Vec<&'a str>> {
    let mut result = Vec::new();

    for item in i_state {
        result.push(vec![item[0].as_str(), item[1].as_str(), ""]);
    }

    result
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let dirp_state = DirpState::new(PathBuf::from("./test"));

    input_thread_spawn(dirp_state.user_sender);

    loop {
        match dirp_state.user_receiver.recv() {
            Ok(user_message) => match user_message {
                UserMessage::GetStateResponse(user_message) => {
                    // create app and run it
                    let mut i_state = Vec::new();
                    dirp_state_to_intermediate_state(
                        FSObj::Dir(user_message.dirp_state),
                        0,
                        &mut i_state,
                    )
                    .expect("err");
                    let app_state = i_state_to_app_state(&i_state);

                    let app = App::new(app_state);
                    let res = step_app(&mut terminal, app);
                }
                UserMessage::UserInputNext => {}
                UserMessage::UserInputPrevious => {}
                UserMessage::UserInputQuit => break,
            },
            Err(error) => {
                panic!("recv() error: {}", error);
            }
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
