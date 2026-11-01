#![windows_subsystem = "windows"]

mod app;
mod autostart;
mod icon;
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
    let catalog = Catalog::load(Lang::from_langid(unsafe { GetUserDefaultUILanguage() }));
    match app::run(&catalog, app::Options::parse(std::env::args().skip(1))) {
        Ok(()) | Err(app::StartError::AlreadyRunning) => {}
        Err(app::StartError::Failed(reason)) => {
            app::show_error(&catalog, &reason);
            std::process::exit(1);
        }
    }
}
