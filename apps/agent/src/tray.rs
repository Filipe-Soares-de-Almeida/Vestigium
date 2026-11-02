use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::process::Command;

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};
use vestigium_brand::Badge;
use vestigium_collect::wall_ms;
use vestigium_core::{ColorTheme, GateState, RecentState};
use vestigium_i18n::{Catalog, Lang};
use vestigium_ipc::TrayState;

use crate::app::{Shared, lock};
use crate::icon::Taskbar;
use crate::{autostart, icon, shutdown};

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
    lang: Cell<Lang>,
    text: RefCell<Texts>,
    /// Read again only when Windows reports a changed setting or display.
    taskbar: Cell<Taskbar>,
    /// What the icon and menu show; `None` until the first refresh and after a relabel.
    shown: Cell<Option<Shown>>,
}

struct Texts {
    watching: String,
    paused: String,
    sensitive: String,
    /// Templates with a `{count}` of the last 24 h.
    attention: String,
    problem: String,
    health: String,
    /// The extra tooltip line that points to the deep capture.
    suggest_capture: String,
    pause: String,
    resume: String,
}

impl Texts {
    fn load(catalog: &Catalog) -> Texts {
        Texts {
            watching: catalog.get("tray.tooltip.watching").to_string(),
            paused: catalog.get("tray.tooltip.paused").to_string(),
            sensitive: catalog.get("tray.tooltip.sensitive").to_string(),
            attention: catalog.get("tray.tooltip.attention").to_string(),
            problem: catalog.get("tray.tooltip.problem").to_string(),
            health: catalog.get("tray.tooltip.health").to_string(),
            suggest_capture: catalog.get("tray.tooltip.suggest_capture").to_string(),
            pause: catalog.get("tray.pause").to_string(),
            resume: catalog.get("tray.resume").to_string(),
        }
    }

    fn tooltip(&self, now: Shown) -> String {
        let mut tooltip = if now.paused {
            self.paused.clone()
        } else if now.sensitive {
            self.sensitive.clone()
        } else {
            match now.recent {
                Level::Normal => self.watching.clone(),
                Level::Attention(n) => self.attention.replace("{count}", &n.to_string()),
                Level::Health(n) => self.health.replace("{count}", &n.to_string()),
                Level::Problem(n) => self.problem.replace("{count}", &n.to_string()),
            }
        };
        if now.suggest_capture {
            tooltip.push('\n');
            tooltip.push_str(&self.suggest_capture);
        }
        tooltip
    }
}

/// The state the icon and menu currently show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shown {
    paused: bool,
    sensitive: bool,
    suggest_capture: bool,
    recent: Level,
    theme: ColorTheme,
    taskbar: Taskbar,
}

impl Shown {
    /// One badge, in the order the tooltip follows: paused, sensitive mode, then the last 24 h.
    fn badge(self) -> Badge {
        if self.paused {
            Badge::Paused
        } else if self.sensitive {
            Badge::Sensitive
        } else {
            match self.recent {
                Level::Normal => Badge::None,
                Level::Attention(_) | Level::Health(_) => Badge::Attention,
                Level::Problem(_) => Badge::Problem,
            }
        }
    }

    /// What the drawn icon depends on.
    fn look(self) -> (Badge, ColorTheme, Taskbar) {
        (self.badge(), self.theme, self.taskbar)
    }
}

/// What the last 24 h held: nothing, slowdowns, or a severe freeze; or, when calm, a critical
/// health finding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Level {
    #[default]
    Normal,
    Attention(u32),
    Problem(u32),
    Health(u32),
}

impl Level {
    pub fn of(recent: &RecentState) -> Level {
        if recent.severe > 0 {
            Level::Problem(recent.severe)
        } else if recent.occurrences + recent.episodes > 0 {
            Level::Attention(recent.occurrences + recent.episodes)
        } else if recent.critical_findings > 0 {
            Level::Health(recent.critical_findings)
        } else {
            Level::Normal
        }
    }

    pub fn state(self) -> TrayState {
        match self {
            Level::Normal => TrayState::Normal,
            Level::Attention(_) | Level::Health(_) => TrayState::Attention,
            Level::Problem(_) => TrayState::Problem,
        }
    }
}

impl Tray {
    pub fn new(
        catalog: &Catalog,
        lang: Lang,
        exe: &Path,
        ui_exe: Option<PathBuf>,
    ) -> Result<Tray, String> {
        let text = Texts::load(catalog);
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
        let taskbar = Taskbar::read();
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(&text.watching)
            .with_icon(icon::tray(Badge::None, ColorTheme::default(), taskbar))
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
            lang: Cell::new(lang),
            text: RefCell::new(text),
            taskbar: Cell::new(taskbar),
            shown: Cell::new(None),
        })
    }

    /// The language the menu is written in.
    pub fn lang(&self) -> Lang {
        self.lang.get()
    }

    /// Rewrites the menu and the tooltip in another language.
    pub fn relabel(&self, catalog: &Catalog, lang: Lang) {
        self.open.set_text(catalog.get("tray.open"));
        self.sensitive.set_text(catalog.get("tray.sensitive"));
        self.autostart.set_text(catalog.get("tray.autostart"));
        self.exit.set_text(catalog.get("tray.exit"));
        *self.text.borrow_mut() = Texts::load(catalog);
        self.lang.set(lang);
        self.shown.set(None);
    }

    pub fn on_menu(&self, id: &MenuId, shared: &Shared) -> Action {
        if id == self.open.id() {
            if let Some(ui) = &self.ui_exe {
                let _ = Command::new(ui).spawn();
            }
        } else if id == self.pause.id() {
            let paused = shared.gate.state() == GateState::Running;
            shared.gate.set(if paused {
                GateState::Paused
            } else {
                GateState::Running
            });
            lock(&shared.status).paused = paused;
        } else if id == self.sensitive.id() {
            shared.sensitive.set(self.sensitive.is_checked(), wall_ms());
        } else if id == self.autostart.id() {
            let _ = if self.autostart.is_checked() {
                autostart::enable(&self.exe)
            } else {
                autostart::disable()
            };
        } else if id == self.exit.id() {
            return Action::Exit;
        }
        self.refresh(shared);
        Action::Continue
    }

    /// Brings the icon, the tooltip and the menu in line with the watcher, when they differ.
    pub fn refresh(&self, shared: &Shared) {
        self.autostart.set_checked(autostart::is_enabled(&self.exe));
        if shutdown::display_changed() {
            self.taskbar.set(Taskbar::read());
        }
        let now = Shown {
            paused: shared.gate.state() == GateState::Paused,
            sensitive: shared.sensitive.is_on(),
            suggest_capture: lock(&shared.status).suggest_capture,
            recent: Level::of(&lock(&shared.recent)),
            theme: lock(&shared.settings).color_theme,
            taskbar: self.taskbar.get(),
        };
        let before = self.shown.replace(Some(now));
        if before == Some(now) {
            return;
        }
        if before.map(Shown::look) != Some(now.look()) {
            let _ = self
                .icon
                .set_icon(Some(icon::tray(now.badge(), now.theme, now.taskbar)));
        }
        let text = self.text.borrow();
        let _ = self.icon.set_tooltip(Some(&text.tooltip(now)));
        self.pause.set_text(if now.paused {
            &text.resume
        } else {
            &text.pause
        });
        self.sensitive.set_checked(now.sensitive);
    }
}

#[cfg(test)]
mod tests {
    use vestigium_brand::Badge;
    use vestigium_core::{ColorTheme, RecentState};

    use super::{Level, Shown, Texts};
    use crate::icon::Taskbar;

    fn shown(paused: bool, sensitive: bool, recent: Level) -> Shown {
        Shown {
            paused,
            sensitive,
            suggest_capture: false,
            recent,
            theme: ColorTheme::Red,
            taskbar: Taskbar {
                dark: true,
                size: 16,
            },
        }
    }

    #[test]
    fn the_badge_follows_the_tooltip_order() {
        let problem = Level::Problem(1);

        assert_eq!(shown(true, true, problem).badge(), Badge::Paused);
        assert_eq!(shown(false, true, problem).badge(), Badge::Sensitive);
        assert_eq!(shown(false, false, problem).badge(), Badge::Problem);
        assert_eq!(
            shown(false, false, Level::Attention(2)).badge(),
            Badge::Attention
        );
        assert_eq!(shown(false, false, Level::Normal).badge(), Badge::None);
        assert_eq!(
            shown(false, false, Level::Health(1)).badge(),
            Badge::Attention
        );
    }

    #[test]
    fn the_tooltip_adds_a_line_when_a_capture_is_suggested() {
        let text = Texts {
            watching: "watching".to_string(),
            paused: "paused".to_string(),
            sensitive: "closely".to_string(),
            attention: "slow {count}".to_string(),
            problem: "severe {count}".to_string(),
            health: "health {count}".to_string(),
            suggest_capture: "look into it".to_string(),
            pause: String::new(),
            resume: String::new(),
        };
        let quiet = shown(false, true, Level::Normal);
        let suggesting = Shown {
            suggest_capture: true,
            ..quiet
        };

        assert_eq!(text.tooltip(quiet), "closely");
        assert_eq!(text.tooltip(suggesting), "closely\nlook into it");
    }

    #[test]
    fn a_new_count_keeps_the_drawn_icon() {
        let before = shown(false, false, Level::Attention(2));
        let after = shown(false, false, Level::Attention(3));

        assert_ne!(before, after);
        assert_eq!(before.look(), after.look());
    }

    #[test]
    fn level_follows_the_last_day() {
        let quiet = RecentState::default();
        let slow = RecentState {
            occurrences: 2,
            severe: 0,
            episodes: 1,
            critical_findings: 0,
        };
        let severe = RecentState { severe: 1, ..slow };
        let unhealthy = RecentState {
            critical_findings: 2,
            ..quiet
        };

        assert_eq!(Level::of(&quiet), Level::Normal);
        assert_eq!(Level::of(&slow), Level::Attention(3));
        assert_eq!(Level::of(&severe), Level::Problem(1));
        assert_eq!(Level::of(&unhealthy), Level::Health(2));
        assert_eq!(
            Level::of(&unhealthy).state(),
            vestigium_ipc::TrayState::Attention
        );
        assert_eq!(
            Level::of(&RecentState {
                critical_findings: 2,
                ..slow
            }),
            Level::Attention(3),
            "slowdowns speak first"
        );
    }
}
