use lazy_static::lazy_static;
use regex::*;
use serde::Deserialize;
use std::borrow::Cow;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PROGRAM_DATA: &str = "%SystemDrive%\\ProgramData\\dzonder\\SSHells";
const CONFIG: &str = "config.json";

#[derive(Debug, Deserialize)]
struct Sshell {
    name: String,
    path: String,
}

/// Expand environmental variables (e.g. `%SystemRoot%`) in a path string.
fn expand_env_vars(path: &str) -> Cow<str> {
    lazy_static! {
        static ref ENV_VAR_REGEX: Regex = Regex::new("%([[:word:]]+)%").unwrap();
    }
    ENV_VAR_REGEX.replace_all(path, |c: &Captures| {
        env::var(&c[1]).expect("invalid environmental variable")
    })
}

/// Print an invalid choice message.
fn invalid_choice(sshells: &Vec<Sshell>) {
    eprintln!("Choose a shell from 1 to {}", sshells.len());
}

/// Print shell selection prompt and parse the input.
fn choose_sshell(sshells: &Vec<Sshell>) {
    print!("\nYour choice: ");
    io::stdout().flush().unwrap();
    let mut choice = String::new();
    io::stdin()
        .read_line(&mut choice)
        .expect("failed to read from stdin");
    match choice.trim().parse::<usize>() {
        Ok(i) => run_sshell(sshells, i),
        Err(..) => invalid_choice(sshells),
    };
}

/// List shells and loop the prompt until a valid selection.
fn list_sshells(sshells: &Vec<Sshell>) {
    println!("Please choose your shell:\n");
    for (i, sshell) in sshells.iter().enumerate() {
        println!("{}) {}", i + 1, sshell.name);
    }
    loop {
        choose_sshell(&sshells);
    }
}

/// Run the selected shell (index `i` from configuration).
fn run_sshell(sshells: &Vec<Sshell>, i: usize) {
    if i < 1 || i > sshells.len() {
        invalid_choice(sshells);
        return;
    }
    // Clear the terminal screen and move cursor.
    print!("\x1B[2J\x1B[1;1H");
    let path: String = expand_env_vars(sshells[i - 1].path.as_str()).into();
    Command::new(path).spawn().expect("shell failed to start");
    std::process::exit(0);
}

/// Read and parse list of shells from a configuration file.
fn read_config() -> Vec<Sshell> {
    let program_data: String = expand_env_vars(PROGRAM_DATA).into();
    let cfg_dir = Path::new(&program_data);
    let cfg_path = cfg_dir.join(CONFIG);
    // Write default config if none exists.
    if !cfg_path.exists() {
        let _ = fs::create_dir_all(cfg_dir);
        fs::write(&cfg_path, include_str!("config.json")).expect("failed to write default config");
    }
    let cfg = File::open(cfg_path).expect("failed to open config file");
    return serde_json::from_reader(cfg).expect("failed to parse config file");
}

fn main() {
    println!("SSHells {VERSION}\n");
    list_sshells(&read_config());
}
