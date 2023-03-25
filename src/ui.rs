use std::{io, path::PathBuf};
use tui::{
    backend::Backend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame, Terminal,
};

pub struct App<'a> {
    pub path: PathBuf,
    state: TableState,
    items: Vec<Vec<&'a str>>,
}

impl<'a> App<'a> {
    pub fn new(path: PathBuf, items: Vec<Vec<&'a str>>) -> App<'a> {
        App {
            path,
            state: TableState::default(),
            items,
        }
    }

    pub fn selected(&mut self) -> usize {
        self.state.selected().expect("We always want a selection.")
    }

    pub fn set_selected(&mut self, state: usize) {
        self.state.select(Some(state));
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    i
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub fn step_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    terminal.draw(|f| ui(f, &mut app))?;
    Ok(())
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let rects = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .margin(5)
        .split(f.size());

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let normal_style = Style::default().bg(Color::Blue);
    let header_cells = ["File", "%", "Size"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::LightGreen)));
    let header = Row::new(header_cells)
        .style(normal_style)
        .height(1)
        .bottom_margin(1);
    let rows = app.items.iter().map(|item| {
        let height = item
            .iter()
            .map(|content| content.chars().filter(|c| *c == '\n').count())
            .max()
            .unwrap_or(0)
            + 1;
        let cells = item.iter().map(|c| Cell::from(*c));
        Row::new(cells).height(height as u16).bottom_margin(1)
    });

    let path = app.path.to_string_lossy();
    let path = format!(" {} ", path);

    let t = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(path.as_str()))
        .highlight_style(selected_style)
        .highlight_symbol("   ")
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Length(30),
            Constraint::Min(10),
        ]);
    f.render_stateful_widget(t, rects[0], &mut app.state);
}
