use core::panic;
use std::env::{args, current_dir, home_dir};
use std::fs::canonicalize;
use std::path::PathBuf;
use std::process::exit;
use types::*;
use ui::ui_runloop;

mod dir_pruner;
mod tui_rs_boilerplate;
mod types;
mod ui;
mod utils;

fn main() {
    ui_runloop(parse_args());
}

fn parse_args() -> Args {
    let mut args = args();
    args.next();
    let args_list: Vec<String> = args.collect();

    if args_list.len() == 1 {
        // dirp <file-path>
        return Args {
            path: normalize_file_path(&args_list[0]),
        };
    } else {
        print_usage();
        exit(-1);
    }
}

fn normalize_file_path(file_path: &String) -> PathBuf {
    match file_path.as_str() {
        "~" => home_dir().expect("~ can not be resolved to a home directory."),
        "." => current_dir().expect(". can not be resolved to the current working directory."),
        _ => canonicalize(file_path.clone())
            .expect(&format!("Could not canonicalize file path: {}.", file_path)),
    }
}

fn print_usage() {
    println!("");
    println!("dirp - directory profiler.");
    println!("");
    println!("USAGE: dirp [directory path]");
    println!("");
}
