#![cfg_attr(windows, windows_subsystem = "windows")]

mod core;
#[cfg(windows)]
mod gui;
#[cfg(windows)]
mod win;

fn main() {
    #[cfg(windows)]
    win::entry();

    #[cfg(not(windows))]
    eprintln!("Audio Shortcut Switch runs on Windows.");
}
