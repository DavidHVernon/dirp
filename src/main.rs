use cli::parse_args;
use terminal_ui::ui_runloop;

mod cli;
mod dirp_state;
mod terminal_ui;
mod tui_rs_boilerplate;
mod types;
mod utils;

fn main() {
    let _ = ui_runloop(parse_args());
}
