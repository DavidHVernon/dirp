use crate::tui_rs_boilerplate::AppRow;
use crate::tui_rs_boilerplate::{step_app, App};
use crate::types::*;
use crate::utils::*;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dialoguer::console::{self};
use std::{error::Error, io};
use std::{sync::mpsc::Sender, thread};
use tui::{backend::CrosstermBackend, Terminal};

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
                KeyCode::Down => user_sender.send(UserMessage::Next)?,
                KeyCode::Up => user_sender.send(UserMessage::Previous)?,
                KeyCode::Left => user_sender.send(UserMessage::CloseDir)?,
                KeyCode::Right => user_sender.send(UserMessage::OpenDir)?,
                KeyCode::Delete => user_sender.send(UserMessage::ToggleMarkPath)?,
                KeyCode::Backspace => user_sender.send(UserMessage::ToggleMarkPath)?,

                KeyCode::Char('p') => user_sender.send(UserMessage::Previous)?,
                KeyCode::Char('n') => user_sender.send(UserMessage::Next)?,
                KeyCode::Char('f') => user_sender.send(UserMessage::ToggleDir)?,
                KeyCode::Char('d') => user_sender.send(UserMessage::MarkPath)?,
                KeyCode::Char('u') => user_sender.send(UserMessage::UnmarkPath)?,

                KeyCode::Char('x') => {
                    user_sender.send(UserMessage::RemoveMarked)?;
                    return Ok(());
                }

                KeyCode::Char('q') => {
                    user_sender.send(UserMessage::Quit)?;
                    return Ok(());
                }

                _ => {}
            },
            _ => { /* Ignore all other forms of input. */ }
        }
    }
}

fn result_tree_to_i_state(
    result_tree: &mut ClientFSObj,
    level: u32,
    i_state: &mut Vec<IntermediateState>,
) -> Option<()> {
    match result_tree {
        ClientFSObj::ClientDir(dir) => {
            let flipper = match dir.is_open {
                true => "⏷",
                false => "⏵",
            };
            let name = dir.name;
            let name = format!("{}{} {}", indent_to_level(level), flipper, name);
            let size = human_readable_bytes(dir.size_in_bytes);
            let percent = format!("{}%", dir.percent);

            i_state.push(IntermediateState {
                ui_row: vec![name, percent, size],
                is_marked: dir.is_marked,
                name: dir.name,
            });

            dir.dir_obj_list
                .sort_by(|a, b| b.size_in_bytes().cmp(&a.size_in_bytes()));

            if dir.is_open {
                for child_obj in &mut dir.dir_obj_list {
                    result_tree_to_i_state(child_obj, level + 1, i_state);
                }
            }
        }
        ClientFSObj::ClientFile(file) => {
            let name = file.name;
            let name = format!("{}  {}", indent_to_level(level), name);
            let size = human_readable_bytes(file.size_in_bytes);
            let percent = format!("{}%", file.percent);

            i_state.push(IntermediateState {
                ui_row: vec![name, percent, size],
                is_marked: file.is_marked,
                name: file.name,
            });
        }
    };

    Some(())
}

fn i_state_to_app_state<'a>(i_state: &'a Vec<IntermediateState>) -> Vec<AppRow<'a>> {
    let mut result = Vec::new();

    for item in i_state {
        result.push(AppRow {
            display_data: vec![
                item.ui_row[0].as_str(),
                item.ui_row[1].as_str(),
                item.ui_row[2].as_str(),
            ],
            is_marked: item.is_marked,
        });
    }

    result
}

pub fn ui_runloop(args: Args) -> Result<(), Box<dyn Error>> {
    let path = args.path;

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let dirp_state = DirpState::new(path.clone());

    input_thread_spawn(dirp_state.user_sender.clone());

    let mut i_state = Vec::new();
    let mut state = 0;

    let mut do_remove_marked = false;

    let app_state = i_state_to_app_state(&i_state);
    let mut app = App::new(path.clone(), app_state);

    let _ = step_app(&mut terminal, app);

    loop {
        let mut do_next = false;
        let mut do_prev = false;

        match dirp_state.user_receiver.recv() {
            Ok(user_message) => match user_message {
                UserMessage::GetStateResponse(user_message) => {
                    // create app and run it
                    i_state.clear();
                    result_tree_to_i_state(
                        &mut ClientFSObj::ClientDir(user_message.result_tree),
                        1,
                        &mut i_state,
                    )
                    .expect("err");
                }
                UserMessage::Next => {
                    do_next = true;
                }
                UserMessage::Previous => {
                    do_prev = true;
                }
                UserMessage::OpenDir => {
                    dirp_state.send(DirpStateMessage::OpenDir(i_state[state].name));
                }
                UserMessage::CloseDir => {
                    dirp_state.send(DirpStateMessage::CloseDir(i_state[state].name));
                }
                UserMessage::ToggleDir => {
                    dirp_state.send(DirpStateMessage::ToggleDir(i_state[state].name));
                }
                UserMessage::MarkPath => {
                    dirp_state.send(DirpStateMessage::MarkPath(i_state[state].name));
                }
                UserMessage::UnmarkPath => {
                    dirp_state.send(DirpStateMessage::UnmarkPath(i_state[state].name));
                }
                UserMessage::ToggleMarkPath => {
                    dirp_state.send(DirpStateMessage::ToggleMarkPath(
                        i_state[state].path.clone(),
                    ));
                }
                UserMessage::RemoveMarked => {
                    do_remove_marked = true;
                    break;
                }
                UserMessage::Quit => break,
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

        let _ = step_app(&mut terminal, app);
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if do_remove_marked {
        dirp_state.send(DirpStateMessage::RemoveMarked);
        let _ = dirp_state.thread_handle.join();
    }

    Ok(())
}