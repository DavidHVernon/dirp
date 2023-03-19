use crate::types::*;
use crate::utils::*;
use crate::UserMessage::GetStateResponse;
use dir_pruner::DirpState;
use std::str::FromStr;
use std::{
    cmp::Ordering,
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
                KeyCode::Left => user_sender.send(UserMessage::UserInputCloseDir)?,
                KeyCode::Right => user_sender.send(UserMessage::UserInputOpenDir)?,
                KeyCode::Char('p') => user_sender.send(UserMessage::UserInputPrevious)?,
                KeyCode::Char('n') => user_sender.send(UserMessage::UserInputNext)?,
                KeyCode::Char('f') => user_sender.send(UserMessage::UserInputToggleDir)?,
                _ => {}
            },
            _ => { /* Ignore all other forms of input. */ }
        }
    }
}

fn dirp_state_to_i_state(
    fs_obj: &mut FSObj,
    level: u32,
    i_state: &mut Vec<IntermediateState>,
) -> Option<()> {
    match fs_obj {
        FSObj::Dir(dir) => {
            let flipper = match dir.is_open {
                true => "v",
                false => ">",
            };
            let name = dir.path.file_name()?.to_string_lossy();
            let name = format!("{}{} {}", indent_to_level(level), flipper, name);
            let size = human_readable_bytes(dir.size_in_bytes);
            let file_size = human_readable_bytes(dir.size_in_bytes);
            let percent = format!("{}%", dir.percent);

            i_state.push(IntermediateState {
                ui_row: vec![name, size, percent],
                path: dir.path.clone(),
            });

            dir.dir_obj_list
                .sort_by(|a, b| b.size_in_bytes().cmp(&a.size_in_bytes()));

            for child_obj in &mut dir.dir_obj_list {
                dirp_state_to_i_state(child_obj, level + 1, i_state);
            }
        }
        FSObj::DirRef(dir_ref) => {
            let name = dir_ref.path.file_name()?.to_string_lossy();
            let name = format!("{}> {}", indent_to_level(level), name);
            let size = human_readable_bytes(dir_ref.size_in_bytes);
            let file_size = human_readable_bytes(dir_ref.size_in_bytes);
            let percent = format!("{}%", dir_ref.percent);

            i_state.push(IntermediateState {
                ui_row: vec![name, size, percent],
                path: dir_ref.path.clone(),
            });
        }
        FSObj::File(file) => {
            let name = file.path.file_name()?.to_string_lossy();
            let name = format!("{}  {}", indent_to_level(level), name);
            let size = human_readable_bytes(file.size_in_bytes);
            let file_size = human_readable_bytes(file.size_in_bytes);
            let percent = format!("{}%", file.percent);

            i_state.push(IntermediateState {
                ui_row: vec![name, size, percent],
                path: file.path.clone(),
            });
        }
        FSObj::SymLink(sym_link) => {
            let name = sym_link.path.file_name()?.to_string_lossy();
            let name = format!("{}  {}", indent_to_level(level), name);
            let size = human_readable_bytes(sym_link.size_in_bytes);
            let file_size = human_readable_bytes(sym_link.size_in_bytes);
            let percent = format!("{}%", sym_link.percent);

            i_state.push(IntermediateState {
                ui_row: vec![name, size, percent],
                path: sym_link.path.clone(),
            });
        }
    };

    Some(())
}

fn i_state_to_app_state<'a>(i_state: &'a Vec<IntermediateState>) -> Vec<Vec<&'a str>> {
    let mut result = Vec::new();

    for item in i_state {
        result.push(vec![
            item.ui_row[0].as_str(),
            item.ui_row[1].as_str(),
            item.ui_row[2].as_str(),
        ]);
    }

    result
}

struct IntermediateState {
    ui_row: Vec<String>,
    path: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let path = PathBuf::from("./test");
    let dirp_state = DirpState::new(path.clone());

    input_thread_spawn(dirp_state.user_sender.clone());

    let mut i_state = Vec::new();
    let mut state = 0;

    loop {
        let mut do_next = false;
        let mut do_prev = false;

        match dirp_state.user_receiver.recv() {
            Ok(user_message) => match user_message {
                UserMessage::GetStateResponse(user_message) => {
                    // create app and run it
                    i_state.clear();
                    dirp_state_to_i_state(
                        &mut FSObj::Dir(user_message.dirp_state),
                        1,
                        &mut i_state,
                    )
                    .expect("err");
                }
                UserMessage::UserInputNext => {
                    do_next = true;
                }
                UserMessage::UserInputPrevious => {
                    do_prev = true;
                }
                UserMessage::UserInputOpenDir => {
                    dirp_state.send(DirpStateMessage::OpenDir(i_state[state].path.clone()));
                }
                UserMessage::UserInputCloseDir => {
                    dirp_state.send(DirpStateMessage::CloseDir(i_state[state].path.clone()));
                }
                UserMessage::UserInputToggleDir => {
                    dirp_state.send(DirpStateMessage::ToggleDir(i_state[state].path.clone()));
                }
                UserMessage::UserInputQuit => break,
            },
            Err(error) => {
                panic!("recv() error: {}", error);
            }
        }

        let app_state = i_state_to_app_state(&i_state);
        let mut app = App::new(path.clone(), app_state);

        app.set_selected(state);
        if do_next {
            app.next();
        }
        if do_prev {
            app.previous();
        }
        state = app.selected();

        let res = step_app(&mut terminal, app);

        do_next = false;
        do_prev = false;
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
