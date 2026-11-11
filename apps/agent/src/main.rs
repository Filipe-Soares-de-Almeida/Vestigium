#![windows_subsystem = "windows"]

mod app;
mod autostart;
mod clock;
mod dumps;
mod fall;
mod health;
mod icon;
mod import;
mod instance;
mod monitor;
mod orphan;
mod paths;
mod power;
mod readings;
mod sampler;
mod sensitive;
mod service;
mod shutdown;
#[cfg(test)]
mod testing;
mod tray;

use vestigium_i18n::{Catalog, Lang};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

fn main() {
    // SAFETY: a plain query with no arguments.
    let system = Lang::from_langid(unsafe { GetUserDefaultUILanguage() });
    match app::run(system, app::Options::parse(std::env::args().skip(1))) {
        Ok(()) | Err(app::StartError::AlreadyRunning) => {}
        Err(app::StartError::PipeTaken(owner)) => {
            app::show_pipe_taken(&Catalog::load(system), owner.as_deref());
            std::process::exit(1);
        }
        Err(app::StartError::Failed(reason)) => {
            app::show_error(&Catalog::load(system), &reason);
            std::process::exit(1);
        }
    }
}
