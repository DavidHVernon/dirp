use crate::types::*;
use home::home_dir;
use std::env::{args, current_dir};
use std::fs::canonicalize;
use std::path::PathBuf;
use std::process::exit;

pub fn parse_args() -> Args {
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
    let mut file_path = file_path.clone();
    if file_path.chars().collect::<Vec<char>>()[0] == '~' {
        file_path = home_dir()
            .expect("~ can not be resolved to a home directory.")
            .to_string_lossy()
            .to_string();
    }
    if file_path.chars().collect::<Vec<char>>()[0] == '.' {
        file_path = current_dir()
            .expect(". can not be resolved to the current working directory.")
            .to_string_lossy()
            .to_string();
    }

    canonicalize(PathBuf::from(file_path)).expect("")
}

fn print_usage() {
    println!("");
    println!("A directory profiler.");
    println!("");
    println!("USAGE: dirp [directory path]");
    println!("");
    println!("Key Bindings:");
    println!("");
    println!("    Up Arrow, p          - Move selection up.");
    println!("    Down Arrow, n        - Move selection down.");
    println!("    ");
    println!("    Left Arrow           - Show directory contents.");
    println!("    Right Arrow          - Hide directory contents.");
    println!("    f                    - Toggle directory contents.");
    println!("    ");
    println!("    d                    - Mark/unmark selection for removal.");
    println!("    Delete, Backspace    - Toggle selection for removal.");
    println!("    ");
    println!("    x                    - Remove marked files, and exit program.");
    println!("    q                    - Exit program.");
    println!("    ");
}
