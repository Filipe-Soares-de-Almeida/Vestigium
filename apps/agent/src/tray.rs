use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};
use vestigium_collect::wall_ms;
use vestigium_core::{GateState, RecentState, RunGate};
use vestigium_i18n::Catalog;
use vestigium_ipc::AgentStatus;

use crate::app::lock;
use crate::sensitive::SensitiveMode;
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
    sensitive: CheckMenuItem,
    autostart: CheckMenuItem,
    exit: MenuItem,
    exe: PathBuf,
    ui_exe: Option<PathBuf>,
    text: Texts,
    shown: Cell<Shown>,
}

struct Texts {
    watching: String,
    paused: String,
    sensitive: String,
    /// Templates with a `{count}` of the last 24 h.
    attention: String,
    problem: String,
    pause: String,
    resume: String,
}

/// The state the icon and menu currently show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Shown {
    paused: bool,
    sensitive: bool,
    recent: Level,
}

/// What the last 24 h held: nothing, slowdowns, or a severe freeze.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Level {
    #[default]
    Normal,
    Attention(u32),
    Problem(u32),
}

impl Level {
    pub fn of(recent: &RecentState) -> Level {
        if recent.severe > 0 {
            Level::Problem(recent.severe)
        } else if recent.occurrences + recent.episodes > 0 {
            Level::Attention(recent.occurrences + recent.episodes)
        } else {
            Level::Normal
        }
    }
}

impl Tray {
    pub fn new(catalog: &Catalog, exe: &Path, ui_exe: Option<PathBuf>) -> Result<Tray, String> {
        let text = Texts {
            watching: catalog.get("tray.tooltip.watching").to_string(),
            paused: catalog.get("tray.tooltip.paused").to_string(),
            sensitive: catalog.get("tray.tooltip.sensitive").to_string(),
            attention: catalog.get("tray.tooltip.attention").to_string(),
            problem: catalog.get("tray.tooltip.problem").to_string(),
            pause: catalog.get("tray.pause").to_string(),
            resume: catalog.get("tray.resume").to_string(),
        };
        let open = MenuItem::new(catalog.get("tray.open"), ui_exe.is_some(), None);
        let pause = MenuItem::new(&text.pause, true, None);
        let sensitive = CheckMenuItem::new(catalog.get("tray.sensitive"), true, false, None);
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
            &sensitive,
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
            sensitive,
            autostart,
            exit,
            exe: exe.to_path_buf(),
            ui_exe,
            text,
            shown: Cell::new(Shown::default()),
        })
    }

    pub fn on_menu(
        &self,
        id: &MenuId,
        gate: &RunGate,
        status: &Mutex<AgentStatus>,
        sensitive: &SensitiveMode,
        recent: &RecentState,
    ) -> Action {
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
            lock(status).paused = paused;
        } else if id == self.sensitive.id() {
            sensitive.set(self.sensitive.is_checked(), wall_ms());
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
        self.refresh(gate, sensitive, recent);
        Action::Continue
    }

    /// Brings the tooltip and the menu in line with the watcher, when they differ.
    pub fn refresh(&self, gate: &RunGate, sensitive: &SensitiveMode, recent: &RecentState) {
        let now = Shown {
            paused: gate.state() == GateState::Paused,
            sensitive: sensitive.is_on(),
            recent: Level::of(recent),
        };
        if now == self.shown.get() {
            return;
        }
        self.shown.set(now);
        let tooltip = if now.paused {
            self.text.paused.clone()
        } else if now.sensitive {
            self.text.sensitive.clone()
        } else {
            match now.recent {
                Level::Normal => self.text.watching.clone(),
                Level::Attention(n) => self.text.attention.replace("{count}", &n.to_string()),
                Level::Problem(n) => self.text.problem.replace("{count}", &n.to_string()),
            }
        };
        let _ = self.icon.set_tooltip(Some(&tooltip));
        self.pause.set_text(if now.paused {
            &self.text.resume
        } else {
            &self.text.pause
        });
        self.sensitive.set_checked(now.sensitive);
    }
}

#[cfg(test)]
mod tests {
    use vestigium_core::RecentState;

    use super::Level;

    #[test]
    fn level_follows_the_last_day() {
        let quiet = RecentState::default();
        let slow = RecentState {
            occurrences: 2,
            severe: 0,
            episodes: 1,
        };
        let severe = RecentState { severe: 1, ..slow };

        assert_eq!(Level::of(&quiet), Level::Normal);
        assert_eq!(Level::of(&slow), Level::Attention(3));
        assert_eq!(Level::of(&severe), Level::Problem(1));
    }
}
