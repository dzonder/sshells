//! Simple utility for picking OpenSSH shell on Windows after establishing SSH connection.

use cursive::align::HAlign;
use cursive::style::gradient::Linear;
use cursive::style::Rgb;
use cursive::utils::markup::gradient;
use cursive::view::Nameable;
use cursive::views::{Dialog, LinearLayout, SelectView, TextView};
use cursive::CursiveRunnable;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use serde::Deserialize;
use std::borrow::Cow;
use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Version of the sshells package (for informative purposes).
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Path to the directory where configuration is stored.
const PROGRAM_DATA: &str = "%SystemDrive%\\ProgramData\\dzonder\\SSHells";

/// Base name of the configuration file.
const CONFIG: &str = "config.json";

/// Index of the default shell in the list of shells.
const DEFAULT_SHELL_INDEX: usize = 0;

/// Timeout for executing the default shell.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);

/// Stores information about a shell (e.g. its name and how it should be executed).
#[derive(Deserialize)]
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
fn expand_env_vars(path: &str) -> Cow<'_, str> {
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

/// State of the countdown timer.
struct TimerState {
    end_time: Instant,
    active: bool,
}

/// Create a SelectView with the list of (existing) shells.
fn sshells_select(
    sshells: Arc<Vec<Sshell>>,
    timer_state: Arc<Mutex<TimerState>>,
) -> SelectView<usize> {
    let mut select_view = SelectView::new().autojump();
    for (i, sshell) in sshells.iter().enumerate() {
        if sshell.exists() {
            select_view.add_item(sshell.name.clone(), i);
        }
    }
    select_view.set_selection(0);
    let sshells_clone = sshells.clone();
    select_view.set_on_select(move |s, _| {
        let mut timer = timer_state.lock().unwrap();
        if timer.active {
            timer.active = false;
            set_shell_label(s, DEFAULT_SHELL_INDEX, &sshells[DEFAULT_SHELL_INDEX].name);
        }
    });
    select_view.set_on_submit(move |_, &index| sshells_clone[index].run());
    select_view
}

/// Handles the countdown timer tick.
fn handle_timer_tick(
    s: &mut cursive::Cursive,
    sshells: &[Sshell],
    timer_state: &Arc<Mutex<TimerState>>,
) {
    let mut timer = timer_state.lock().unwrap();
    if !timer.active {
        return;
    }

    let now = Instant::now();
    if now >= timer.end_time {
        timer.active = false;
        s.quit();
        sshells[DEFAULT_SHELL_INDEX].run();
    } else {
        let remaining = timer.end_time - now;
        let label = format!(
            "{} ({})",
            sshells[DEFAULT_SHELL_INDEX].name,
            remaining.as_secs() + 1
        );
        set_shell_label(s, DEFAULT_SHELL_INDEX, &label);
    }
}

/// Update the label of a shell in the SelectView.
fn set_shell_label(s: &mut cursive::Cursive, index: usize, label: &str) {
    if let Some(mut select) = s.find_name::<SelectView<usize>>("select") {
        if let Some((l, _)) = select.get_item_mut(index) {
            *l = label.into();
        }
    }
}

/// Set up the cursive TUI environment.
fn setup_tui(sshells: Arc<Vec<Sshell>>) -> CursiveRunnable {
    let timer_state = Arc::new(Mutex::new(TimerState {
        end_time: Instant::now() + DEFAULT_TIMEOUT,
        active: true,
    }));

    let mut siv = cursive::default();
    siv.set_autorefresh(true);

    let timer_clone = timer_state.clone();
    let sshells_clone = sshells.clone();
    siv.set_on_pre_event(cursive::event::Event::Refresh, move |s| {
        handle_timer_tick(s, &sshells_clone, &timer_clone);
    });

    let version_text = gradient::decorate_back(
        format!("SSHells {VERSION}"),
        Linear::simple(Rgb::yellow(), Rgb::cyan()),
    );
    siv.add_global_callback('q', |s| s.quit());
    siv.add_layer(
        LinearLayout::vertical()
            .child(TextView::new(version_text).h_align(HAlign::Center))
            .child(Dialog::around(
                sshells_select(sshells, timer_state).with_name("select"),
            )),
    );
    siv
}

/// Reads the configuration and sets up the select view in a TUI.
fn main() {
    let sshells: Arc<Vec<Sshell>> = Arc::new(read_config());
    if sshells.is_empty() {
        println!("No shells configured or configuration file not found.");
        return;
    }
    setup_tui(sshells).run();
}
