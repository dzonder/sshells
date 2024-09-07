use cursive::align::HAlign;
use cursive::views::{Dialog, LinearLayout, SelectView, TextView};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use serde::Deserialize;
use std::borrow::Cow;
use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::Command;

/// Version of the sshells package (for informative purposes).
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Path to the directory where configuration is stored.
const PROGRAM_DATA: &str = "%SystemDrive%\\ProgramData\\dzonder\\SSHells";

/// Base name of the configuration file.
const CONFIG: &str = "config.json";

/// Stores information about a shell (e.g. its name and how it should be executed).
#[derive(Debug, Deserialize)]
struct Sshell {
    name: String,
    path: String,

    #[serde(default)]
    args: Vec<String>,

    #[serde(skip)]
    expanded_path: String,
}

impl Sshell {
    /// Run the selected shell. Exits after shell terminates.
    fn run(&self) {
        // Reset colors, clear the terminal screen and move cursor.
        print!("\x1B[0m\x1B[?25h\x1B[2J\x1B[1;1H");
        Command::new(&self.expanded_path)
            .args(&self.args)
            .spawn()
            .expect("shell failed to start");
        std::process::exit(0);
    }

    /// Checks if this shell exists in the system.
    fn exists(&self) -> bool {
        Path::new(&self.expanded_path).exists()
    }
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
    let mut sshells: Vec<Sshell> =
        serde_json::from_reader(cfg).expect("failed to parse config file");
    // Expand environmental variables in all paths.
    for sshell in sshells.iter_mut() {
        sshell.expanded_path = expand_env_vars(&sshell.path).into();
    }
    sshells
}

/// Create a SelectView with the list of (existing) shells.
fn sshells_select(sshells: Vec<Sshell>) -> SelectView<Sshell> {
    let mut select_view = SelectView::new().autojump();
    for sshell in sshells {
        if sshell.exists() {
            select_view.add_item(sshell.name.clone(), sshell);
        }
    }
    select_view.on_submit(|_, sshell| {
        sshell.run();
    })
}

fn main() {
    let sshells = read_config();
    let mut siv = cursive::default();
    siv.add_global_callback('q', |s| s.quit());
    siv.add_layer(
        LinearLayout::vertical()
            .child(TextView::new(format!("SSHells {VERSION}")).h_align(HAlign::Center))
            .child(Dialog::around(sshells_select(sshells))),
    );
    siv.run();
}
