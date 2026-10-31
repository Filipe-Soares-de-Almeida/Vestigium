use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use argos_core::{GateState, RunGate};
use argos_i18n::Catalog;
use argos_ipc::AgentStatus;
use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

use crate::{autostart, icon};

/// What the message loop does after a menu click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    Exit,
}

/// The tray icon and its menu; lives on the thread that runs the message loop.
pub struct Tray {
    icon: TrayIcon,
    open: MenuItem,
    pause: MenuItem,
    autostart: CheckMenuItem,
    exit: MenuItem,
    exe: PathBuf,
    ui_exe: Option<PathBuf>,
    text: Texts,
}

struct Texts {
    watching: String,
    paused: String,
    pause: String,
    resume: String,
}

impl Tray {
    pub fn new(catalog: &Catalog, exe: &Path, ui_exe: Option<PathBuf>) -> Result<Tray, String> {
        let text = Texts {
            watching: catalog.get("tray.tooltip.watching").to_string(),
            paused: catalog.get("tray.tooltip.paused").to_string(),
            pause: catalog.get("tray.pause").to_string(),
            resume: catalog.get("tray.resume").to_string(),
        };
        let open = MenuItem::new(catalog.get("tray.open"), ui_exe.is_some(), None);
        let pause = MenuItem::new(&text.pause, true, None);
        let autostart = CheckMenuItem::new(
            catalog.get("tray.autostart"),
            true,
            autostart::is_enabled(exe),
            None,
        );
        let exit = MenuItem::new(catalog.get("tray.exit"), true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &open,
            &pause,
            &PredefinedMenuItem::separator(),
            &autostart,
            &PredefinedMenuItem::separator(),
            &exit,
        ])
        .map_err(|err| err.to_string())?;
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(&text.watching)
            .with_icon(icon::placeholder())
            .build()
            .map_err(|err| err.to_string())?;
        Ok(Tray {
            icon,
            open,
            pause,
            autostart,
            exit,
            exe: exe.to_path_buf(),
            ui_exe,
            text,
        })
    }

    pub fn on_menu(&self, id: &MenuId, gate: &RunGate, status: &Mutex<AgentStatus>) -> Action {
        if id == self.open.id() {
            if let Some(ui) = &self.ui_exe {
                let _ = Command::new(ui).spawn();
            }
        } else if id == self.pause.id() {
            let paused = gate.state() == GateState::Running;
            gate.set(if paused {
                GateState::Paused
            } else {
                GateState::Running
            });
            self.show_paused(paused);
            status
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .paused = paused;
        } else if id == self.autostart.id() {
            let _ = if self.autostart.is_checked() {
                autostart::enable(&self.exe)
            } else {
                autostart::disable()
            };
            self.autostart.set_checked(autostart::is_enabled(&self.exe));
        } else if id == self.exit.id() {
            return Action::Exit;
        }
        Action::Continue
    }

    fn show_paused(&self, paused: bool) {
        let (tooltip, item) = if paused {
            (&self.text.paused, &self.text.resume)
        } else {
            (&self.text.watching, &self.text.pause)
        };
        let _ = self.icon.set_tooltip(Some(tooltip));
        self.pause.set_text(item);
    }
}
