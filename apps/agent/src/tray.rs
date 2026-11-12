use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::process::Command;

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use vestigium_brand::Badge;
use vestigium_collect::{interrupt_ms, local_hour_minute, wall_ms};
use vestigium_core::{ColorTheme, GateState, RecentState};
use vestigium_i18n::{Catalog, Lang};
use vestigium_ipc::{AgentStatus, MonitorSource, MonitorState, TrayState};

use crate::app::{Shared, lock, set_paused};
use crate::icon::Taskbar;
use crate::{autostart, icon, monitor, shutdown};

/// The longest tooltip Windows shows, in characters.
const TOOLTIP_MAX: usize = 127;

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

#[derive(Default)]
struct Texts {
    watching: String,
    paused: String,
    sensitive: String,
    /// Templates with `{time}`, and `{source}` for the partial one.
    no_reading: String,
    partial: String,
    unverified: String,
    /// The name of each source, in `MonitorSource` order, and the name of several, with `{count}`.
    sources: [String; 5],
    several: String,
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
            no_reading: catalog.get("tray.tooltip.no_reading").to_string(),
            partial: catalog.get("tray.tooltip.degraded").to_string(),
            unverified: catalog.get("tray.tooltip.unverified").to_string(),
            sources: [
                "ui.monitor.source.memory",
                "ui.monitor.source.cpu",
                "ui.monitor.source.counters",
                "ui.monitor.source.processes",
                "ui.monitor.source.timer",
            ]
            .map(|key| catalog.get(key).to_string()),
            several: catalog.get("ui.monitor.source.several").to_string(),
            attention: catalog.get("tray.tooltip.attention").to_string(),
            problem: catalog.get("tray.tooltip.problem").to_string(),
            health: catalog.get("tray.tooltip.health").to_string(),
            suggest_capture: catalog.get("tray.tooltip.suggest_capture").to_string(),
            pause: catalog.get("tray.pause").to_string(),
            resume: catalog.get("tray.resume").to_string(),
        }
    }

    fn level(&self, level: Level) -> String {
        match level {
            Level::Normal => self.watching.clone(),
            Level::Attention(n) => self.attention.replace("{count}", &n.to_string()),
            Level::Health(n) => self.health.replace("{count}", &n.to_string()),
            Level::Problem(n) => self.problem.replace("{count}", &n.to_string()),
        }
    }

    /// The lines of the tooltip, most important first: what the monitor says, then the last 24 h.
    fn lines(&self, now: Shown) -> Vec<String> {
        if now.paused {
            return vec![self.paused.clone()];
        }
        if now.sensitive {
            return vec![self.sensitive.clone()];
        }
        let mut lines = Vec::new();
        match now.monitor {
            Monitor::Reading => lines.push(self.level(now.recent)),
            Monitor::NoReading { since_min } => {
                lines.push(self.no_reading.replace("{time}", &clock(since_min)));
            }
            Monitor::Partial {
                first,
                count,
                since_min,
            } => {
                let source = if count > 1 {
                    self.several.replace("{count}", &count.to_string())
                } else {
                    self.sources[first as usize].clone()
                };
                lines.push(
                    self.partial
                        .replace("{source}", &source)
                        .replace("{time}", &clock(since_min)),
                );
            }
            Monitor::Unverified { since_min } => {
                let unverified = self.unverified.replace("{time}", &clock(since_min));
                if now.recent != Level::Normal {
                    lines.push(self.level(now.recent));
                }
                lines.push(unverified);
            }
        }
        if matches!(
            now.monitor,
            Monitor::NoReading { .. } | Monitor::Partial { .. }
        ) && now.recent != Level::Normal
        {
            lines.push(self.level(now.recent));
        }
        lines
    }

    /// The lines that fit in what Windows shows; the capture hint goes first when there is no room.
    fn tooltip(&self, now: Shown) -> String {
        let mut lines = self.lines(now);
        if now.suggest_capture && !now.paused {
            lines.push(self.suggest_capture.clone());
        }
        let mut tooltip = lines.remove(0);
        for line in lines {
            if tooltip.chars().count() + 1 + line.chars().count() > TOOLTIP_MAX {
                break;
            }
            tooltip.push('\n');
            tooltip.push_str(&line);
        }
        tooltip
    }
}

/// Hour and minute of a minute counted from the epoch, on the local clock.
fn clock(minutes: i64) -> String {
    local_hour_minute(minutes.saturating_mul(60_000)).map_or_else(String::new, |(hour, minute)| {
        format!("{hour:02}:{minute:02}")
    })
}

/// What the monitor says about itself, as the icon and tooltip show it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Monitor {
    #[default]
    Reading,
    /// Minutes since the epoch, so a tooltip that shows hours and minutes never redraws for less.
    NoReading {
        since_min: i64,
    },
    Partial {
        first: MonitorSource,
        count: usize,
        since_min: i64,
    },
    Unverified {
        since_min: i64,
    },
}

impl Monitor {
    fn of(status: &AgentStatus, now_ms: i64) -> Monitor {
        let view = monitor::view(status, now_ms);
        let minute = |ms: i64| ms.div_euclid(60_000);
        match view.state {
            MonitorState::NoReading => Monitor::NoReading {
                since_min: minute(view.no_reading_since_ms.unwrap_or(now_ms)),
            },
            MonitorState::Degraded if !view.failing.is_empty() => Monitor::Partial {
                first: view.failing[0].source,
                count: view.failing.len(),
                since_min: minute(
                    view.failing
                        .iter()
                        .map(|f| f.since_ms)
                        .min()
                        .unwrap_or(now_ms),
                ),
            },
            _ => monitor::unverified_since(status, now_ms)
                .filter(|_| !status.paused && !status.suspended)
                .map_or(Monitor::Reading, |since| Monitor::Unverified {
                    since_min: minute(since),
                }),
        }
    }
}

/// The state the icon and menu currently show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shown {
    paused: bool,
    sensitive: bool,
    suggest_capture: bool,
    recent: Level,
    monitor: Monitor,
    theme: ColorTheme,
    taskbar: Taskbar,
}

impl Shown {
    /// One badge, in the order the tooltip follows: paused, sensitive mode, a monitor that does not
    /// read, then the last 24 h, and last a health check that has not finished.
    fn badge(self) -> Badge {
        if self.paused {
            return Badge::Paused;
        }
        if self.sensitive {
            return Badge::Sensitive;
        }
        match (self.monitor, self.recent) {
            (Monitor::NoReading { .. }, _) => Badge::NoReading,
            (Monitor::Partial { .. }, _) => Badge::Partial,
            (_, Level::Problem(_)) => Badge::Problem,
            (_, Level::Attention(_) | Level::Health(_)) => Badge::Attention,
            (Monitor::Unverified { .. }, Level::Normal) => Badge::Unverified,
            (_, Level::Normal) => Badge::None,
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
            set_paused(shared, shared.gate.state() == GateState::Running, None);
        } else if id == self.sensitive.id() {
            shared
                .sensitive
                .set(self.sensitive.is_checked(), interrupt_ms());
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
        let (suggest_capture, recent, theme, monitor) = read_shared(shared);
        let now = Shown {
            paused: shared.gate.state() == GateState::Paused,
            sensitive: shared.sensitive.is_on(),
            suggest_capture,
            recent,
            monitor,
            theme,
            taskbar: self.taskbar.get(),
        };
        let text = self.text.borrow();
        if !show(&self.icon, &self.shown, now, &text) {
            return;
        }
        self.pause.set_text(if now.paused {
            &text.resume
        } else {
            &text.pause
        });
        self.sensitive.set_checked(now.sensitive);
    }
}

/// What the notification area accepts: a new icon and a new tooltip.
trait Face {
    fn set_icon(&self, icon: Icon) -> Result<(), String>;
    fn set_tooltip(&self, text: &str) -> Result<(), String>;
}

impl Face for TrayIcon {
    fn set_icon(&self, icon: Icon) -> Result<(), String> {
        TrayIcon::set_icon(self, Some(icon)).map_err(|err| err.to_string())
    }

    fn set_tooltip(&self, text: &str) -> Result<(), String> {
        TrayIcon::set_tooltip(self, Some(text)).map_err(|err| err.to_string())
    }
}

/// Draws `now` on the icon and tooltip and records it in `shown` only when the shell took both.
/// Returns false when `now` is already shown.
fn show(face: &impl Face, shown: &Cell<Option<Shown>>, now: Shown, text: &Texts) -> bool {
    let before = shown.get();
    if before == Some(now) {
        return false;
    }
    let icon_ok = before.map(Shown::look) == Some(now.look())
        || face
            .set_icon(icon::tray(now.badge(), now.theme, now.taskbar))
            .is_ok();
    if icon_ok && face.set_tooltip(&text.tooltip(now)).is_ok() {
        shown.set(Some(now));
    }
    true
}

/// Reads what the icon shows from the shared state: the capture hint, the level, the theme and
/// the monitor's own state. One lock is held at a time.
pub(crate) fn read_shared(shared: &Shared) -> (bool, Level, ColorTheme, Monitor) {
    let status = lock(&shared.status).clone();
    let recent = Level::of(&lock(&shared.recent));
    let theme = lock(&shared.settings).color_theme;
    (
        status.suggest_capture,
        recent,
        theme,
        Monitor::of(&status, wall_ms()),
    )
}

#[cfg(test)]
mod tests {
    use vestigium_brand::Badge;
    use vestigium_core::{ColorTheme, RecentState};

    use std::cell::{Cell, RefCell};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use tray_icon::Icon;
    use vestigium_ipc::MonitorSource;

    use super::{Face, Level, Monitor, Shown, Texts, clock, read_shared, show};
    use crate::app::lock;
    use crate::app::tests::{both_finish, shared};
    use crate::icon::Taskbar;

    fn shown(paused: bool, sensitive: bool, recent: Level) -> Shown {
        Shown {
            paused,
            sensitive,
            suggest_capture: false,
            recent,
            monitor: Monitor::Reading,
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

    const MIN: i64 = 29_000_000;

    fn with(monitor: Monitor, recent: Level) -> Shown {
        Shown {
            monitor,
            ..shown(false, false, recent)
        }
    }

    fn monitor_texts() -> Texts {
        Texts {
            no_reading: "no reading since {time}".to_string(),
            partial: "partial: {source} since {time}".to_string(),
            unverified: "unverified since {time}".to_string(),
            sources: ["memory", "cpu", "counters", "processes", "timer"].map(String::from),
            several: "{count} sources".to_string(),
            ..texts()
        }
    }

    #[test]
    fn no_reading_outranks_the_24_hour_level() {
        let none = Monitor::NoReading { since_min: MIN };
        let partial = Monitor::Partial {
            first: MonitorSource::Counters,
            count: 1,
            since_min: MIN,
        };

        assert_eq!(with(none, Level::Problem(2)).badge(), Badge::NoReading);
        assert_eq!(with(none, Level::Normal).badge(), Badge::NoReading);
        assert_eq!(with(partial, Level::Problem(2)).badge(), Badge::Partial);
        assert_eq!(
            Shown {
                paused: true,
                ..with(none, Level::Normal)
            }
            .badge(),
            Badge::Paused
        );
        assert_eq!(
            Shown {
                sensitive: true,
                ..with(none, Level::Normal)
            }
            .badge(),
            Badge::Sensitive
        );
    }

    #[test]
    fn an_unverified_health_check_shows_only_when_nothing_else_asks_for_a_badge() {
        let unverified = Monitor::Unverified { since_min: MIN };

        assert_eq!(with(unverified, Level::Normal).badge(), Badge::Unverified);
        assert_eq!(with(unverified, Level::Problem(1)).badge(), Badge::Problem);
        assert_eq!(
            with(unverified, Level::Attention(1)).badge(),
            Badge::Attention
        );
    }

    #[test]
    fn the_tooltip_names_the_monitor_state_and_keeps_the_24_hour_level() {
        let text = monitor_texts();
        let time = clock(MIN);
        let none = Monitor::NoReading { since_min: MIN };
        let partial = Monitor::Partial {
            first: MonitorSource::Counters,
            count: 1,
            since_min: MIN,
        };
        let several = Monitor::Partial {
            first: MonitorSource::Memory,
            count: 3,
            since_min: MIN,
        };
        let unverified = Monitor::Unverified { since_min: MIN };

        assert_eq!(
            text.tooltip(with(none, Level::Normal)),
            format!("no reading since {time}")
        );
        assert_eq!(
            text.tooltip(with(none, Level::Problem(2))),
            format!(
                "no reading since {time}
severe 2"
            )
        );
        assert_eq!(
            text.tooltip(with(partial, Level::Normal)),
            format!("partial: counters since {time}")
        );
        assert_eq!(
            text.tooltip(with(several, Level::Normal)),
            format!("partial: 3 sources since {time}")
        );
        assert_eq!(
            text.tooltip(with(unverified, Level::Normal)),
            format!("unverified since {time}")
        );
        assert_eq!(
            text.tooltip(with(unverified, Level::Attention(4))),
            format!(
                "slow 4
unverified since {time}"
            )
        );
    }

    #[test]
    fn every_monitor_text_exists_in_both_languages_and_fits_a_tooltip() {
        use vestigium_i18n::{Catalog, Lang};

        for lang in [Lang::En, Lang::PtBr] {
            let text = Texts::load(&Catalog::load(lang));
            let all = [
                &text.no_reading,
                &text.partial,
                &text.unverified,
                &text.several,
            ]
            .into_iter()
            .chain(&text.sources);
            for line in all {
                assert!(
                    !line.starts_with("tray.") && !line.starts_with("ui."),
                    "{line}"
                );
            }
            let longest = Shown {
                suggest_capture: false,
                ..with(
                    Monitor::Partial {
                        first: MonitorSource::Counters,
                        count: 1,
                        since_min: MIN,
                    },
                    Level::Normal,
                )
            };
            assert!(text.tooltip(longest).chars().count() <= 127);
        }
    }

    #[test]
    fn the_tooltip_leaves_out_lines_that_do_not_fit() {
        let text = Texts {
            no_reading: "x".repeat(100),
            suggest_capture: "y".repeat(40),
            ..monitor_texts()
        };
        let now = Shown {
            suggest_capture: true,
            ..with(Monitor::NoReading { since_min: MIN }, Level::Normal)
        };

        assert_eq!(text.tooltip(now), "x".repeat(100));
    }

    #[test]
    fn the_monitor_face_follows_the_status() {
        use vestigium_ipc::{AgentStatus, SourceFailure};

        let now = 10_000_000;
        let mut status = AgentStatus {
            started_at_ms: now - 600_000,
            last_sample_ms: Some(now - 1_000),
            ..AgentStatus::default()
        };
        assert_eq!(Monitor::of(&status, now), Monitor::Reading);

        status.monitor.failing.push(SourceFailure {
            source: MonitorSource::Memory,
            since_ms: now - 120_000,
            error: "x".to_string(),
        });
        assert_eq!(
            Monitor::of(&status, now),
            Monitor::Partial {
                first: MonitorSource::Memory,
                count: 1,
                since_min: (now - 120_000) / 60_000
            }
        );

        status.last_sample_ms = Some(now - 30_000);
        assert_eq!(
            Monitor::of(&status, now),
            Monitor::NoReading {
                since_min: (now - 30_000) / 60_000
            }
        );

        status.paused = true;
        assert_eq!(Monitor::of(&status, now), Monitor::Reading);
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
            ..Texts::default()
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

    #[test]
    fn tray_state_read_never_deadlocks_with_a_recent_then_status_writer() {
        let shared = Arc::new(shared());
        let reading = Arc::clone(&shared);
        let writing = Arc::clone(&shared);

        both_finish(
            move || {
                for _ in 0..3_000 {
                    read_shared(&reading);
                }
            },
            move || {
                for _ in 0..300 {
                    let recent = lock(&writing.recent);
                    thread::sleep(Duration::from_millis(1));
                    let status = lock(&writing.status);
                    drop((recent, status));
                }
            },
        );
    }

    /// A notification area that rejects the first `reject_icons` icons and `reject_tooltips` tooltips.
    #[derive(Default)]
    struct Fake {
        reject_icons: Cell<u32>,
        reject_tooltips: Cell<u32>,
        icons: Cell<u32>,
        tooltips: RefCell<Vec<String>>,
    }

    impl Face for Fake {
        fn set_icon(&self, _icon: Icon) -> Result<(), String> {
            self.icons.set(self.icons.get() + 1);
            take_rejection(&self.reject_icons)
        }

        fn set_tooltip(&self, text: &str) -> Result<(), String> {
            self.tooltips.borrow_mut().push(text.to_string());
            take_rejection(&self.reject_tooltips)
        }
    }

    fn take_rejection(left: &Cell<u32>) -> Result<(), String> {
        if left.get() == 0 {
            return Ok(());
        }
        left.set(left.get() - 1);
        Err("the shell refused".to_string())
    }

    fn texts() -> Texts {
        Texts {
            watching: "watching".to_string(),
            paused: "paused".to_string(),
            sensitive: "closely".to_string(),
            attention: "slow {count}".to_string(),
            problem: "severe {count}".to_string(),
            health: "health {count}".to_string(),
            suggest_capture: "look".to_string(),
            ..Texts::default()
        }
    }

    #[test]
    fn a_rejected_icon_update_is_retried_on_the_next_refresh() {
        let face = Fake::default();
        face.reject_icons.set(1);
        let shown_now = Cell::new(None);
        let now = shown(false, false, Level::Problem(1));

        assert!(show(&face, &shown_now, now, &texts()));
        assert_eq!(shown_now.get(), None);
        assert!(show(&face, &shown_now, now, &texts()));

        assert_eq!(face.icons.get(), 2);
        assert_eq!(shown_now.get(), Some(now));
        assert!(
            !show(&face, &shown_now, now, &texts()),
            "an accepted state is not drawn again"
        );
        assert_eq!(face.icons.get(), 2);
    }

    #[test]
    fn a_rejected_tooltip_update_is_retried() {
        let face = Fake::default();
        face.reject_tooltips.set(1);
        let shown_now = Cell::new(None);
        let now = shown(false, false, Level::Normal);

        show(&face, &shown_now, now, &texts());
        assert_eq!(shown_now.get(), None);
        show(&face, &shown_now, now, &texts());

        assert_eq!(face.tooltips.borrow().len(), 2);
        assert_eq!(shown_now.get(), Some(now));
    }

    #[test]
    fn a_count_change_updates_the_tooltip_without_redrawing_the_icon() {
        let face = Fake::default();
        let shown_now = Cell::new(None);
        let first = shown(false, false, Level::Attention(2));
        let second = shown(false, false, Level::Attention(3));

        show(&face, &shown_now, first, &texts());
        show(&face, &shown_now, second, &texts());

        assert_eq!(face.icons.get(), 1);
        assert_eq!(
            face.tooltips.borrow().last().map(String::as_str),
            Some("slow 3")
        );
    }
}
