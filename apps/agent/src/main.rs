#![windows_subsystem = "windows"]

mod app;
mod autostart;
mod health;
mod icon;
mod import;
mod paths;
mod power;
mod sampler;
mod sensitive;
mod shutdown;
mod tray;

use vestigium_i18n::{Catalog, Lang};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

fn main() {
    // SAFETY: a plain query with no arguments.
    let system = Lang::from_langid(unsafe { GetUserDefaultUILanguage() });
    match app::run(system, app::Options::parse(std::env::args().skip(1))) {
        Ok(()) | Err(app::StartError::AlreadyRunning) => {}
        Err(app::StartError::Failed(reason)) => {
            app::show_error(&Catalog::load(system), &reason);
            std::process::exit(1);
        }
    }
}
