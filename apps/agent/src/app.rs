use std::any::Any;
use std::collections::VecDeque;
use std::fs;
use std::mem;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread;
use std::time::Duration;

use tray_icon::menu::MenuEvent;
use vestigium_collect::{
    EventSubscription, ProbeConfig, Wake, WindowsSettings, spawn_probe, wall_ms,
};
use vestigium_core::{
    Applier, DEFAULT_PROBE_PERIOD, GateState, LanguagePref, PROGRAM_FRAMES_MAX, PlanItem,
    PressureSpan, ProbeHiccup, ProgramRing, RING_MS, Reading, RecentState, Ring, RunGate,
    SENSITIVE_FOR, SettingStore, Settings, Timing, carry_out, check_item,
};
use vestigium_i18n::{Catalog, Lang};
use vestigium_ipc::{
    AgentStatus, ErrorCode, IpcError, NowFrame, PROTOCOL_VERSION, PipeServer, Request, Response,
    current_user_sid, pipe_name, pipe_owner,
};
use vestigium_store::Store;
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
use windows::core::HSTRING;

use crate::import::{import_captures, import_dumps, import_scans, import_tests, import_tweaks};
use crate::instance::Instance;
use crate::orphan::release_orphaned_capture;
use crate::readings::windows_source_set;
use crate::sampler::{Wiring, run_sampler};
use crate::sensitive::SensitiveMode;
use crate::tray::{Action, Level, Tray};
use crate::{autostart, paths, power, shutdown};

/// How often the history is written to the database.
const DEFAULT_FLUSH: Duration = Duration::from_secs(300);
/// How long Exit and logoff wait for the last batch to be written.
const STOP_TIMEOUT: Duration = Duration::from_secs(2);
/// How far back the first run reads the curated Windows events.
const EVENT_BACKFILL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// Ten minutes of readings at 1 Hz, with room for late ticks.
const READINGS_KEPT: usize = 660;

/// Command-line options, for tests and measurements.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub data_dir: Option<PathBuf>,
    pub flush: Duration,
    pub sensitive_for: Duration,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Options {
        let mut options = Options {
            data_dir: None,
            flush: DEFAULT_FLUSH,
            sensitive_for: SENSITIVE_FOR,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--data-dir" => options.data_dir = args.next().map(PathBuf::from),
                "--flush-secs" => {
                    if let Some(secs) = args.next().and_then(|s| s.parse().ok()) {
                        options.flush = Duration::from_secs(secs);
                    }
                }
                "--sensitive-secs" => {
                    if let Some(secs) = args.next().and_then(|s| s.parse().ok()) {
                        options.sensitive_for = Duration::from_secs(secs);
                    }
                }
                _ => {}
            }
        }
        options
    }
}

/// Why the agent did not start.
#[derive(Debug)]
pub enum StartError {
    AlreadyRunning,
    /// Another program owns the pipe name; its image, when it could be read.
    PipeTaken(Option<PathBuf>),
    Failed(String),
}

fn failed(err: impl std::fmt::Display) -> StartError {
    StartError::Failed(err.to_string())
}

/// What the pipe answers from and acts on, shared with the tray and the sampler.
/// No code holds two of these mutexes at once.
pub struct Shared {
    pub gate: Arc<RunGate>,
    pub status: Arc<Mutex<AgentStatus>>,
    pub sensitive: Arc<SensitiveMode>,
    pub recent: Arc<Mutex<RecentState>>,
    pub now: Arc<Mutex<NowFrame>>,
    /// The last ten minutes, one reading a second.
    pub readings: Arc<Mutex<Ring<Reading>>>,
    /// Each program's use at every ranking of the last ten minutes.
    pub programs: Arc<Mutex<ProgramRing>>,
    /// The pressure episodes of the last ten minutes, ended or under way.
    pub pressure: Arc<Mutex<Vec<PressureSpan>>>,
    pub settings: Arc<Mutex<Settings>>,
    /// Set when the history was cleared; the sampler drops what it still holds in memory.
    pub cleared: Arc<AtomicBool>,
    /// Late probe wakes of the last fifteen minutes, kept while sensitive mode is on.
    pub hiccups: Arc<Mutex<VecDeque<ProbeHiccup>>>,
    /// Where the helper leaves finished deep captures.
    pub captures_dir: PathBuf,
    /// Where the helper leaves finished health scans.
    pub scans_dir: PathBuf,
    /// Where the helper leaves dump reads and deletion results.
    pub dumps_dir: PathBuf,
    /// Where the helper leaves the results of elevated tweaks.
    pub tweaks_dir: PathBuf,
    /// Where the window and the helper leave the results of hardware tests.
    pub tests_dir: PathBuf,
    /// Set after a scan was stored; the sampler runs a health pass on its next tick.
    pub health_due: Arc<AtomicBool>,
    /// Set when the pipe could not be bound again; the agent then exits.
    pub pipe_lost: Arc<AtomicBool>,
    /// This executable, as the Start with Windows entry names it.
    pub exe: PathBuf,
}

/// The language the tray speaks: the stored choice, or Windows' own.
pub fn lang_for(pref: LanguagePref, system: Lang) -> Lang {
    match pref {
        LanguagePref::System => system,
        LanguagePref::En => Lang::En,
        LanguagePref::PtBr => Lang::PtBr,
    }
}

/// Carries out a tweak of the user's own settings and keeps it, written or not, in the history.
fn apply_user_tweak(
    item: &PlanItem,
    store: &mut Store,
    settings: &mut impl SettingStore,
    now_ms: i64,
) -> Response {
    if let Err(why) = check_item(item, Applier::User) {
        return error(ErrorCode::InvalidSetting, why);
    }
    let change = carry_out(item, settings);
    match store.record_tweak_change(&change, now_ms, Applier::User) {
        Ok(id) => Response::TweakApplied { id, change },
        Err(err) => error(ErrorCode::Failed, err),
    }
}

/// Pauses or resumes watching; a pause with `until_ms` ends by itself at that time.
pub fn set_paused(shared: &Shared, paused: bool, until_ms: Option<i64>) {
    {
        let mut status = lock(&shared.status);
        status.paused = paused;
        status.paused_until_ms = until_ms.filter(|_| paused);
    }
    shared.gate.set(if paused {
        GateState::Paused
    } else {
        GateState::Running
    });
}

fn error(code: ErrorCode, message: impl std::fmt::Display) -> Response {
    Response::Error {
        code,
        message: message.to_string(),
    }
}

/// Answers one request from the interface; `store` is where settings are written.
pub fn handle(request: Request, shared: &Shared, store: Option<&mut Store>) -> Response {
    match request {
        Request::Hello { protocol } if protocol == PROTOCOL_VERSION => Response::Hello {
            protocol: PROTOCOL_VERSION,
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        Request::Hello { protocol } => error(
            ErrorCode::Protocol,
            format!("protocol {protocol} is not supported; this agent speaks {PROTOCOL_VERSION}"),
        ),
        Request::Status => {
            let state = Level::of(&lock(&shared.recent)).state();
            let base = lock(&shared.status).clone();
            Response::Status(AgentStatus {
                sensitive_until_ms: shared.sensitive.until_ms(),
                state,
                ..base
            })
        }
        Request::Now => Response::Now(Box::new(lock(&shared.now).clone())),
        Request::Recent => {
            let since_ms = wall_ms() - RING_MS;
            let readings = lock(&shared.readings)
                .iter()
                .filter(|r| r.ts_ms >= since_ms)
                .copied()
                .collect();
            let pressure = lock(&shared.pressure)
                .iter()
                .filter(|span| span.end_ms.is_none_or(|end| end >= since_ms))
                .cloned()
                .collect();
            Response::Recent { readings, pressure }
        }
        Request::ProgramRecent { name } => Response::ProgramRecent {
            points: lock(&shared.programs).history(&name),
        },
        Request::SetPaused { paused, until_ms } => {
            set_paused(shared, paused, until_ms);
            shutdown::wake();
            Response::Done
        }
        Request::SetSensitive { on } => {
            shared.sensitive.set(on, wall_ms());
            shutdown::wake();
            Response::Done
        }
        Request::Settings => Response::Settings {
            settings: *lock(&shared.settings),
            autostart: autostart::is_enabled(&shared.exe),
        },
        Request::SetSettings(settings) => {
            if let Err(err) = settings.validate() {
                return error(ErrorCode::InvalidSetting, err);
            }
            let Some(store) = store else {
                return error(ErrorCode::Failed, "the history database is not open");
            };
            if let Err(err) = store.set_settings(&settings) {
                return error(ErrorCode::Failed, err);
            }
            *lock(&shared.settings) = settings;
            shutdown::wake();
            Response::Done
        }
        Request::ClearHistory => {
            let Some(store) = store else {
                return error(ErrorCode::Failed, "the history database is not open");
            };
            if let Err(err) = store.clear_history() {
                return error(ErrorCode::Failed, err);
            }
            shared.cleared.store(true, Ordering::Release);
            lock(&shared.readings).clear();
            lock(&shared.programs).clear();
            lock(&shared.pressure).clear();
            lock(&shared.now).latest = None;
            *lock(&shared.recent) = RecentState::default();
            shutdown::wake();
            Response::Done
        }
        Request::ImportResults => {
            let Some(store) = store else {
                return error(ErrorCode::Failed, "the history database is not open");
            };
            let hiccups: Vec<ProbeHiccup> = lock(&shared.hiccups).iter().copied().collect();
            let captures = import_captures(&shared.captures_dir, &hiccups, store);
            let scans = import_scans(&shared.scans_dir, store);
            if !scans.is_empty() {
                shared.health_due.store(true, Ordering::Release);
            }
            let (dumps, deleted) = import_dumps(&shared.dumps_dir, store);
            let tweaks = import_tweaks(&shared.tweaks_dir, store, &mut WindowsSettings);
            let tests = import_tests(&shared.tests_dir, store);
            if !tests.is_empty() {
                shared.health_due.store(true, Ordering::Release);
            }
            Response::Imported {
                captures,
                scans,
                dumps,
                deleted,
                tweaks,
                tests,
            }
        }
        Request::ApplyUserTweak { item } => {
            let Some(store) = store else {
                return error(ErrorCode::Failed, "the history database is not open");
            };
            apply_user_tweak(&item, store, &mut WindowsSettings, wall_ms())
        }
        Request::SetAutostart { on } => {
            let changed = if on {
                autostart::enable(&shared.exe)
            } else {
                autostart::disable()
            };
            shutdown::wake();
            match changed {
                Ok(()) => Response::Done,
                Err(err) => error(ErrorCode::Failed, err),
            }
        }
    }
}

/// Binds that failed in a row before the agent gives up on its pipe.
const PIPE_BIND_TRIES: u32 = 5;
/// Pause before the pipe is bound again.
const PIPE_RETRY_WAIT: Duration = Duration::from_secs(1);

/// Why one run of the pipe ended.
#[derive(Debug)]
enum PipeFailure {
    Bind(IpcError),
    Serve(String),
}

/// The pipe's own connection to the history, opened again when a request finds it closed.
struct SecondConnection {
    db: PathBuf,
    store: Option<Store>,
    /// The results waiting from before the start are imported once, on the first open.
    imported: bool,
}

impl SecondConnection {
    fn new(db: PathBuf) -> SecondConnection {
        SecondConnection {
            db,
            store: None,
            imported: false,
        }
    }

    /// The open connection; a closed one is opened now, once per call, and the first open also
    /// imports the results that were waiting.
    fn store(&mut self, shared: &Shared) -> Option<&mut Store> {
        if self.store.is_none() {
            match Store::open(&self.db) {
                Ok(store) => self.store = Some(store),
                Err(err) => lock(&shared.status).last_error = Some(err.to_string()),
            }
        }
        let store = self.store.as_mut()?;
        if !mem::replace(&mut self.imported, true) {
            import_captures(&shared.captures_dir, &[], store);
            import_scans(&shared.scans_dir, store);
            import_dumps(&shared.dumps_dir, store);
            import_tweaks(&shared.tweaks_dir, store, &mut WindowsSettings);
            import_tests(&shared.tests_dir, store);
        }
        Some(store)
    }
}

/// True for the requests that write to the history through the pipe's own connection.
fn needs_store(request: &Request) -> bool {
    matches!(
        request,
        Request::SetSettings(_)
            | Request::ClearHistory
            | Request::ImportResults
            | Request::ApplyUserTweak { .. }
    )
}

/// Answers a request, opening the pipe's connection first when the request needs it.
fn serve(request: Request, shared: &Shared, second: &mut SecondConnection) -> Response {
    let store = if needs_store(&request) {
        second.store(shared)
    } else {
        None
    };
    handle(request, shared, store)
}

/// Runs the pipe until `run_once` returns `Ok`; a run that fails or panics is recorded and started
/// again, and after five binds in a row that fail the agent is told to exit.
fn keep_pipe_serving(
    shared: &Shared,
    wait: Duration,
    mut run_once: impl FnMut() -> Result<(), PipeFailure>,
) {
    let mut failed_binds = 0;
    loop {
        let failure = match panic::catch_unwind(AssertUnwindSafe(&mut run_once)) {
            Ok(Ok(())) => return,
            Ok(Err(failure)) => failure,
            Err(payload) => PipeFailure::Serve(panic_text(payload.as_ref())),
        };
        let reason = match failure {
            PipeFailure::Bind(err) => {
                failed_binds += 1;
                err.to_string()
            }
            PipeFailure::Serve(reason) => {
                failed_binds = 0;
                reason
            }
        };
        lock(&shared.status).last_error = Some(format!("the pipe stopped: {reason}"));
        if failed_binds >= PIPE_BIND_TRIES {
            shared.pipe_lost.store(true, Ordering::Relaxed);
            shutdown::wake();
            return;
        }
        thread::sleep(wait);
    }
}

fn panic_text(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| text.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the pipe thread panicked".to_string())
}

/// Starts the agent and runs it until Exit or the end of the session.
pub fn run(system: Lang, options: Options) -> Result<(), StartError> {
    let _instance = Instance::acquire()?;
    let pipe = pipe_name(&current_user_sid().map_err(failed)?);
    let server = match PipeServer::bind(&pipe) {
        Err(IpcError::AlreadyRunning) => {
            let this = std::env::current_exe().map_err(failed)?;
            return Err(pipe_taken(pipe_owner(&pipe), &this));
        }
        bound => bound.map_err(failed)?,
    };
    let data_dir = match options.data_dir {
        Some(dir) => dir,
        None => paths::data_dir().ok_or_else(|| failed("the LOCALAPPDATA folder is not set"))?,
    };
    let db = data_dir.join("vestigium.db");
    let (store, set_aside) = Store::open_recovering(&db).map_err(failed)?;
    let shared = Arc::new(Shared {
        gate: RunGate::new(),
        status: Arc::new(Mutex::new(AgentStatus {
            started_at_ms: wall_ms(),
            ..AgentStatus::default()
        })),
        sensitive: SensitiveMode::new(
            ProbeConfig::new(DEFAULT_PROBE_PERIOD),
            options.sensitive_for,
        ),
        recent: Arc::new(Mutex::new(RecentState::default())),
        now: Arc::new(Mutex::new(NowFrame::default())),
        readings: Arc::new(Mutex::new(Ring::with_capacity(READINGS_KEPT))),
        programs: Arc::new(Mutex::new(ProgramRing::new(RING_MS, PROGRAM_FRAMES_MAX))),
        pressure: Arc::new(Mutex::new(Vec::new())),
        settings: Arc::new(Mutex::new(store.settings().unwrap_or_default())),
        cleared: Arc::new(AtomicBool::new(false)),
        hiccups: Arc::new(Mutex::new(VecDeque::new())),
        captures_dir: data_dir.join("captures"),
        scans_dir: data_dir.join("scans"),
        dumps_dir: data_dir.join("dumps"),
        tweaks_dir: data_dir.join("tweaks"),
        tests_dir: data_dir.join("tests"),
        health_due: Arc::new(AtomicBool::new(false)),
        pipe_lost: Arc::new(AtomicBool::new(false)),
        exe: std::env::current_exe().map_err(failed)?,
    });

    if let Some(name) = set_aside.as_deref().and_then(Path::file_name) {
        lock(&shared.status).last_error = Some(format!(
            "the history file was damaged and set aside as {}",
            name.to_string_lossy()
        ));
    }
    report_setting_notes(&store, &shared.status);

    for dir in [
        &shared.captures_dir,
        &shared.scans_dir,
        &shared.dumps_dir,
        &shared.tweaks_dir,
        &shared.tests_dir,
    ] {
        if let Err(err) = fs::create_dir_all(dir) {
            lock(&shared.status).last_error = Some(err.to_string());
        }
    }

    release_orphaned_capture();

    let (probe_tx, probe_rx) = mpsc::channel();
    let (power_tx, power_rx) = mpsc::channel();
    let wake = Arc::new(Wake::new().map_err(failed)?);
    let (events_tx, events_rx) = mpsc::channel();
    // Without a readable bookmark the backfill runs again; stored events are not duplicated.
    let bookmark = store.bookmark_xml().ok().flatten();
    let subscription =
        EventSubscription::start(bookmark.as_deref(), EVENT_BACKFILL, events_tx.clone())
            .inspect_err(|err| lock(&shared.status).last_error = Some(err.to_string()))
            .ok();
    spawn_probe(shared.sensitive.probe(), shared.gate.clone(), probe_tx).map_err(failed)?;
    let sampler = Arc::new(
        run_sampler(
            store,
            Wiring {
                flush: options.flush,
                timing: Timing::default(),
                gate: shared.gate.clone(),
                probe_rx,
                power_rx,
                wake: wake.clone(),
                events_rx,
                subscription,
                resubscribe: Some(Box::new(move |bookmark| {
                    EventSubscription::start(bookmark, EVENT_BACKFILL, events_tx.clone())
                })),
                sensitive: shared.sensitive.clone(),
                status: shared.status.clone(),
                recent: shared.recent.clone(),
                now: shared.now.clone(),
                readings: shared.readings.clone(),
                programs: shared.programs.clone(),
                pressure: shared.pressure.clone(),
                hiccups: shared.hiccups.clone(),
                settings: shared.settings.clone(),
                cleared: shared.cleared.clone(),
                health_due: shared.health_due.clone(),
                ring_path: Some(data_dir.join("blackbox.ring")),
                source_set: windows_source_set(),
            },
        )
        .map_err(failed)?,
    );
    let stop = {
        let gate = shared.gate.clone();
        let wake = wake.clone();
        move || {
            gate.set(GateState::Stopping);
            wake.set();
            sampler.wait(STOP_TIMEOUT);
        }
    };

    {
        let shared = shared.clone();
        let pipe = pipe.clone();
        // Settings are written through a connection of the pipe's own.
        let mut second = SecondConnection::new(db.clone());
        second.store(&shared);
        let mut first = Some(server);
        thread::Builder::new()
            .name("pipe".to_string())
            .spawn(move || {
                keep_pipe_serving(&shared, PIPE_RETRY_WAIT, || {
                    let server = match first.take() {
                        Some(ready) => ready,
                        None => PipeServer::bind(&pipe).map_err(PipeFailure::Bind)?,
                    };
                    let ended = server.serve(|request| serve(request, &shared, &mut second));
                    Err(PipeFailure::Serve(ended.err().map_or_else(
                        || "the pipe closed".to_string(),
                        |err| err.to_string(),
                    )))
                })
            })
            .map_err(failed)?;
    }

    if let Err(err) = autostart::repair(&shared.exe) {
        lock(&shared.status).last_error = Some(err.to_string());
    }
    let lang = lang_for(lock(&shared.settings).language, system);
    let tray =
        Tray::new(&Catalog::load(lang), lang, &shared.exe, paths::ui_exe()).map_err(failed)?;
    let stop = Arc::new(stop);
    {
        let stop = stop.clone();
        let window = shutdown::watch_session_end(move || stop()).map_err(failed)?;
        power::watch(window, move |event| {
            power::announce(event, &power_tx, &wake, power::SUSPEND_WAIT);
        })
        .map_err(failed)?;
    }
    shutdown::pump(|| {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if tray.on_menu(&event.id, &shared) == Action::Exit {
                shutdown::quit();
            }
        }
        if shared.pipe_lost.load(Ordering::Relaxed) {
            shutdown::quit();
        }
        let lang = lang_for(lock(&shared.settings).language, system);
        if lang != tray.lang() {
            tray.relabel(&Catalog::load(lang), lang);
        }
        tray.refresh(&shared);
    });
    stop();
    Ok(())
}

/// What a held pipe name means once the per-user guard is ours: no watcher of this user runs in this
/// session, so the owner is either this program in another session or a stranger.
fn pipe_taken(owner: Option<PathBuf>, this: &Path) -> StartError {
    match owner {
        Some(owner) if same_program(&owner, this) => StartError::AlreadyRunning,
        other => StartError::PipeTaken(other),
    }
}

fn same_program(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

/// Says that another program holds the pipe name.
pub fn show_pipe_taken(catalog: &Catalog, owner: Option<&Path>) {
    let body = match owner {
        Some(owner) => catalog.format(
            "error.startup.pipe_taken",
            &[("owner", &owner.to_string_lossy())],
        ),
        None => catalog.get("error.startup.pipe_taken_unknown").to_string(),
    };
    message_box(catalog, &body);
}

/// Shows why the agent could not start.
pub fn show_error(catalog: &Catalog, reason: &str) {
    message_box(
        catalog,
        &catalog.format("error.startup.body", &[("error", reason)]),
    );
}

fn message_box(catalog: &Catalog, body: &str) {
    // SAFETY: a modal box with two strings that outlive the call.
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(body),
            &HSTRING::from(catalog.get("error.startup.title")),
            MB_OK | MB_ICONERROR,
        )
    };
}

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Tells the status what the stored settings had to replace by a default, when anything.
fn report_setting_notes(store: &Store, status: &Mutex<AgentStatus>) {
    let Ok(notes) = store.settings_notes() else {
        return;
    };
    if !notes.is_empty() {
        lock(status).last_error = Some(notes.join("; "));
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use vestigium_core::{EpisodeKind, HungWindow, Retention};
    use vestigium_ipc::{PROTOCOL_VERSION, TrayState};

    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    struct FakeSettings(Vec<(vestigium_core::Setting, vestigium_core::Value)>);

    impl SettingStore for FakeSettings {
        fn read(&mut self, setting: &vestigium_core::Setting) -> Option<vestigium_core::Value> {
            self.0
                .iter()
                .find(|(s, _)| s == setting)
                .map(|(_, v)| v.clone())
        }

        fn write(
            &mut self,
            setting: &vestigium_core::Setting,
            value: &vestigium_core::Value,
        ) -> Result<(), u32> {
            for (s, v) in &mut self.0 {
                if s == setting {
                    *v = value.clone();
                }
            }
            Ok(())
        }
    }

    #[test]
    fn user_tweak_records_before_and_after() {
        use vestigium_core::{SettingChange, TweakId, Value};
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let dvr = TweakId::GameDvr.settings();
        let mut settings = FakeSettings(dvr.iter().map(|s| (s.clone(), Value::Dword(1))).collect());
        let item = PlanItem {
            tweak: TweakId::GameDvr,
            subject: None,
            undo_of: None,
            changes: dvr
                .iter()
                .map(|s| SettingChange {
                    setting: s.clone(),
                    from: Value::Dword(1),
                    to: Value::Dword(0),
                })
                .collect(),
        };

        let Response::TweakApplied { id, change } =
            apply_user_tweak(&item, &mut store, &mut settings, 50)
        else {
            panic!("applied");
        };
        assert!(change.succeeded());
        assert!(settings.0.iter().all(|(_, v)| *v == Value::Dword(0)));
        assert_eq!(store.tweaks_in_place().unwrap()[0].id, id);

        let mut elevated = item.clone();
        elevated.tweak = TweakId::Mpo;
        elevated.changes = vec![SettingChange {
            setting: TweakId::Mpo.settings()[0].clone(),
            from: Value::Absent,
            to: Value::Dword(5),
        }];
        assert!(matches!(
            apply_user_tweak(&elevated, &mut store, &mut settings, 60),
            Response::Error {
                code: ErrorCode::InvalidSetting,
                ..
            }
        ));
    }

    #[test]
    fn options_default_to_five_minute_batches() {
        let options = Options::parse(args(&[]));
        assert_eq!(options.flush, Duration::from_secs(300));
        assert_eq!(options.data_dir, None);
        assert_eq!(options.sensitive_for, SENSITIVE_FOR);
    }

    #[test]
    fn options_read_both_flags() {
        let options = Options::parse(args(&["--flush-secs", "60", "--data-dir", r"D:\tmp\wmpc"]));
        assert_eq!(options.flush, Duration::from_secs(60));
        assert_eq!(options.data_dir, Some(PathBuf::from(r"D:\tmp\wmpc")));
    }

    #[test]
    fn options_read_the_sensitive_mode_length() {
        let options = Options::parse(args(&["--sensitive-secs", "45"]));
        assert_eq!(options.sensitive_for, Duration::from_secs(45));
    }

    #[test]
    fn options_ignore_a_bad_number() {
        let options = Options::parse(args(&["--flush-secs", "soon"]));
        assert_eq!(options.flush, DEFAULT_FLUSH);
    }

    pub(crate) fn shared() -> Shared {
        Shared {
            gate: RunGate::new(),
            status: Arc::new(Mutex::new(AgentStatus::default())),
            sensitive: SensitiveMode::new(ProbeConfig::new(DEFAULT_PROBE_PERIOD), SENSITIVE_FOR),
            recent: Arc::new(Mutex::new(RecentState::default())),
            now: Arc::new(Mutex::new(NowFrame::default())),
            readings: Arc::new(Mutex::new(Ring::with_capacity(READINGS_KEPT))),
            programs: Arc::new(Mutex::new(ProgramRing::new(RING_MS, PROGRAM_FRAMES_MAX))),
            pressure: Arc::new(Mutex::new(Vec::new())),
            settings: Arc::new(Mutex::new(Settings::default())),
            cleared: Arc::new(AtomicBool::new(false)),
            hiccups: Arc::new(Mutex::new(VecDeque::new())),
            captures_dir: PathBuf::from("captures"),
            scans_dir: PathBuf::from("scans"),
            dumps_dir: PathBuf::from("dumps"),
            tweaks_dir: PathBuf::from("tweaks"),
            tests_dir: PathBuf::from("tests"),
            health_due: Arc::new(AtomicBool::new(false)),
            pipe_lost: Arc::new(AtomicBool::new(false)),
            exe: PathBuf::from(r"C:\Program Files\Vestigium\Vestigium.exe"),
        }
    }

    #[test]
    fn a_pipe_held_by_this_program_is_a_running_watcher() {
        let this = Path::new(r"C:\Apps\Vestigium\Vestigium.exe");
        let same = Path::new(r"c:\apps\vestigium\vestigium.exe");

        assert!(matches!(
            pipe_taken(Some(same.to_path_buf()), this),
            StartError::AlreadyRunning
        ));
    }

    #[test]
    fn a_pipe_held_by_another_program_is_an_error() {
        let this = Path::new(r"C:\Apps\Vestigium\Vestigium.exe");
        let other = PathBuf::from(r"C:\Tools\other.exe");

        assert!(matches!(
            pipe_taken(Some(other.clone()), this),
            StartError::PipeTaken(Some(found)) if found == other
        ));
        assert!(matches!(
            pipe_taken(None, this),
            StartError::PipeTaken(None)
        ));
    }

    #[test]
    fn pipe_thread_serves_again_after_a_failed_run() {
        let shared = shared();
        let mut runs = 0;

        keep_pipe_serving(&shared, Duration::ZERO, || {
            runs += 1;
            match runs {
                1 => Err(PipeFailure::Serve("the pipe broke".to_string())),
                2 => panic!("a handler failed"),
                _ => Ok(()),
            }
        });

        assert_eq!(runs, 3);
        let last_error = lock(&shared.status).last_error.clone().unwrap();
        assert!(last_error.contains("a handler failed"), "{last_error}");
        assert!(!shared.pipe_lost.load(Ordering::Relaxed));
    }

    #[test]
    fn pipe_thread_gives_up_after_repeated_bind_failures() {
        let shared = shared();
        let mut runs = 0;

        keep_pipe_serving(&shared, Duration::ZERO, || {
            runs += 1;
            Err(PipeFailure::Bind(IpcError::AlreadyRunning))
        });

        assert_eq!(runs, PIPE_BIND_TRIES);
        assert!(shared.pipe_lost.load(Ordering::Relaxed));
        assert!(lock(&shared.status).last_error.is_some());
    }

    #[test]
    fn hello_with_the_same_protocol_is_answered() {
        let response = handle(
            Request::Hello {
                protocol: PROTOCOL_VERSION,
            },
            &shared(),
            None,
        );
        assert!(
            matches!(response, Response::Hello { protocol, .. } if protocol == PROTOCOL_VERSION)
        );
    }

    #[test]
    fn older_protocols_are_refused() {
        for protocol in [1, 2, 3, 4, 5] {
            let response = handle(Request::Hello { protocol }, &shared(), None);
            assert!(matches!(
                response,
                Response::Error {
                    code: ErrorCode::Protocol,
                    ..
                }
            ));
        }
    }

    #[test]
    fn status_reflects_the_agent() {
        let shared = shared();
        lock(&shared.status).started_at_ms = 42;
        shared.sensitive.set(true, 1_000);
        *lock(&shared.recent) = RecentState {
            occurrences: 1,
            severe: 1,
            episodes: 0,
            critical_findings: 0,
        };

        let Response::Status(status) = handle(Request::Status, &shared, None) else {
            panic!("not a status");
        };

        assert_eq!(status.started_at_ms, 42);
        assert_eq!(status.sensitive_until_ms, Some(1_000 + 30 * 60 * 1_000));
        assert_eq!(status.state, TrayState::Problem);
    }

    /// Runs two closures on their own threads and fails if either is still stuck after five seconds.
    pub(crate) fn both_finish(
        first: impl FnOnce() + Send + 'static,
        second: impl FnOnce() + Send + 'static,
    ) {
        let (done, finished) = mpsc::channel();
        let sender = done.clone();
        thread::spawn(move || {
            first();
            let _ = sender.send("first");
        });
        thread::spawn(move || {
            second();
            let _ = done.send("second");
        });
        for _ in 0..2 {
            finished
                .recv_timeout(Duration::from_secs(5))
                .expect("a thread is stuck: the two locks were taken in opposite orders");
        }
    }

    #[test]
    fn status_answer_never_deadlocks_with_a_status_then_recent_reader() {
        let shared = Arc::new(shared());
        let answering = Arc::clone(&shared);
        let reading = Arc::clone(&shared);

        both_finish(
            move || {
                for _ in 0..3_000 {
                    handle(Request::Status, &answering, None);
                }
            },
            move || {
                for _ in 0..300 {
                    let status = lock(&reading.status);
                    thread::sleep(Duration::from_millis(1));
                    let recent = lock(&reading.recent);
                    drop((status, recent));
                }
            },
        );
    }

    #[test]
    fn now_returns_the_latest_sample() {
        let shared = shared();
        let frame = NowFrame {
            hung: vec![HungWindow {
                pid: 7,
                name: "editor.exe".to_string(),
            }],
            pressure: vec![EpisodeKind::CpuSaturated],
            ..NowFrame::default()
        };
        *lock(&shared.now) = frame.clone();

        assert_eq!(
            handle(Request::Now, &shared, None),
            Response::Now(Box::new(frame))
        );
    }

    #[test]
    fn recent_returns_the_ring_in_order() {
        let shared = shared();
        let now = wall_ms();
        for ago_ms in [700_000, 590_000, 30_000, 1_000] {
            lock(&shared.readings).push(Reading {
                ts_ms: now - ago_ms,
                cpu_pct: 5.0,
                mem_used_pct: Some(30.0),
                disk_active_pct: 1.0,
                disk_latency_ms: 0.5,
                gpu_pct: None,
                probe_lag_ms: 1.0,
            });
        }

        let Response::Recent { readings, .. } = handle(Request::Recent, &shared, None) else {
            panic!("not the recent readings");
        };

        let times: Vec<i64> = readings.iter().map(|r| r.ts_ms).collect();
        assert_eq!(times, [now - 590_000, now - 30_000, now - 1_000]);
    }

    #[test]
    fn recent_brings_the_pressure_of_those_minutes() {
        let shared = shared();
        let now = wall_ms();
        let span = |start_ago_ms: i64, end_ago_ms: Option<i64>| PressureSpan {
            kind: EpisodeKind::CpuSaturated,
            start_ms: now - start_ago_ms,
            end_ms: end_ago_ms.map(|ago| now - ago),
        };
        *lock(&shared.pressure) = vec![
            span(900_000, Some(700_000)),
            span(650_000, Some(400_000)),
            span(90_000, None),
        ];

        let Response::Recent { pressure, .. } = handle(Request::Recent, &shared, None) else {
            panic!("not the recent readings");
        };

        assert_eq!(
            pressure,
            [span(650_000, Some(400_000)), span(90_000, None)],
            "the one that ended before the ten minutes is left out"
        );
    }

    #[test]
    fn set_settings_validates() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let shared = shared();
        let mut wrong = Settings::default();
        wrong.retention.samples_hours = 6;

        let refused = handle(Request::SetSettings(wrong), &shared, Some(&mut store));
        assert!(matches!(
            refused,
            Response::Error {
                code: ErrorCode::InvalidSetting,
                ..
            }
        ));
        assert_eq!(*lock(&shared.settings), Settings::default());

        let right = Settings {
            language: LanguagePref::PtBr,
            retention: Retention {
                records_days: 730,
                ..Retention::default()
            },
            ..Settings::default()
        };
        let accepted = handle(Request::SetSettings(right), &shared, Some(&mut store));
        assert_eq!(accepted, Response::Done);
        assert_eq!(*lock(&shared.settings), right);
        assert_eq!(store.settings().unwrap(), right);
    }

    #[test]
    fn a_stored_setting_replaced_by_its_default_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("vestigium.db");
        drop(Store::open(&db).unwrap());
        rusqlite::Connection::open(&db)
            .unwrap()
            .execute(
                "INSERT INTO setting (key, value) VALUES ('retention', ?1)",
                [r#"{"samples_hours":10,"minutes_days":60,"records_days":400}"#],
            )
            .unwrap();
        let store = Store::open(&db).unwrap();
        let shared = shared();

        report_setting_notes(&store, &shared.status);

        let last_error = lock(&shared.status).last_error.clone().unwrap();
        assert!(last_error.contains("samples_hours"), "{last_error}");
        assert_eq!(store.settings().unwrap().retention.minutes_days, 60);
    }

    #[test]
    fn a_history_that_opens_late_serves_the_requests_that_follow() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("data");
        fs::write(&blocker, b"a file where the data folder should be").unwrap();
        let shared = shared();
        let mut second = SecondConnection::new(blocker.join("vestigium.db"));
        let settings = Settings {
            language: LanguagePref::PtBr,
            ..Settings::default()
        };

        let closed = serve(Request::SetSettings(settings), &shared, &mut second);
        assert!(matches!(
            closed,
            Response::Error {
                code: ErrorCode::Failed,
                ..
            }
        ));
        assert!(lock(&shared.status).last_error.is_some());

        fs::remove_file(&blocker).unwrap();
        let opened = serve(Request::SetSettings(settings), &shared, &mut second);

        assert_eq!(opened, Response::Done);
        let stored = Store::open(&blocker.join("vestigium.db")).unwrap();
        assert_eq!(stored.settings().unwrap(), settings);
    }

    #[test]
    fn a_request_that_needs_no_history_does_not_open_it() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("vestigium.db");
        let shared = shared();
        let mut second = SecondConnection::new(db.clone());

        let answer = serve(Request::Now, &shared, &mut second);

        assert!(matches!(answer, Response::Now(_)));
        assert!(!db.exists());
    }

    #[test]
    fn clear_history_empties_the_store_and_what_the_screens_read() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let shared = shared();
        lock(&shared.readings).push(Reading {
            ts_ms: wall_ms(),
            cpu_pct: 5.0,
            mem_used_pct: Some(30.0),
            disk_active_pct: 1.0,
            disk_latency_ms: 0.5,
            gpu_pct: None,
            probe_lag_ms: 1.0,
        });
        *lock(&shared.recent) = RecentState {
            occurrences: 2,
            ..RecentState::default()
        };

        assert!(matches!(
            handle(Request::ClearHistory, &shared, None),
            Response::Error {
                code: ErrorCode::Failed,
                ..
            }
        ));
        assert_eq!(
            handle(Request::ClearHistory, &shared, Some(&mut store)),
            Response::Done
        );
        assert!(shared.cleared.load(Ordering::Acquire));
        assert_eq!(lock(&shared.readings).iter().count(), 0);
        assert_eq!(*lock(&shared.recent), RecentState::default());
    }

    #[test]
    fn a_failed_clear_keeps_the_live_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("vestigium.db");
        let mut store = Store::open(&db).unwrap();
        let shared = shared();
        lock(&shared.readings).push(Reading {
            ts_ms: wall_ms(),
            cpu_pct: 5.0,
            mem_used_pct: Some(30.0),
            disk_active_pct: 1.0,
            disk_latency_ms: 0.5,
            gpu_pct: None,
            probe_lag_ms: 1.0,
        });
        let refuser = rusqlite::Connection::open(&db).unwrap();
        refuser
            .execute_batch(
                "INSERT INTO gap (start_ms, end_ms, reason) VALUES (1, 2, 0);
                 CREATE TRIGGER refuse_clear BEFORE DELETE ON gap
                 BEGIN SELECT RAISE(ABORT, 'refused'); END",
            )
            .unwrap();

        let answer = handle(Request::ClearHistory, &shared, Some(&mut store));

        assert!(matches!(
            answer,
            Response::Error {
                code: ErrorCode::Failed,
                ..
            }
        ));
        assert!(!shared.cleared.load(Ordering::Acquire));
        assert_eq!(lock(&shared.readings).iter().count(), 1);
    }

    #[test]
    fn import_results_needs_the_store_and_answers_with_ids() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let mut shared = shared();
        shared.captures_dir = dir.path().join("captures");
        shared.scans_dir = dir.path().join("scans");
        shared.dumps_dir = dir.path().join("dumps");
        fs::create_dir(&shared.captures_dir).unwrap();
        fs::create_dir(&shared.scans_dir).unwrap();
        fs::create_dir(&shared.dumps_dir).unwrap();

        assert!(matches!(
            handle(Request::ImportResults, &shared, None),
            Response::Error {
                code: ErrorCode::Failed,
                ..
            }
        ));
        assert_eq!(
            handle(Request::ImportResults, &shared, Some(&mut store)),
            Response::Imported {
                captures: Vec::new(),
                scans: Vec::new(),
                dumps: Vec::new(),
                deleted: 0,
                tweaks: Vec::new(),
                tests: Vec::new(),
            }
        );
        assert!(!shared.health_due.load(Ordering::Acquire));

        let scan = vestigium_core::health::ScanResult {
            ts_ms: 1,
            boot: None,
        };
        let mut bytes = vec![vestigium_core::health::SCAN_VERSION];
        bytes.extend(postcard::to_allocvec(&scan).unwrap());
        fs::write(shared.scans_dir.join("7.scan"), bytes).unwrap();
        assert_eq!(
            handle(Request::ImportResults, &shared, Some(&mut store)),
            Response::Imported {
                captures: Vec::new(),
                scans: vec![1],
                dumps: Vec::new(),
                deleted: 0,
                tweaks: Vec::new(),
                tests: Vec::new(),
            }
        );
        assert!(
            shared.health_due.load(Ordering::Acquire),
            "a new scan asks for a pass"
        );
        assert_eq!(store.last_scan().unwrap(), Some(scan));
        assert!(!shared.scans_dir.join("7.scan").exists());
    }

    #[test]
    fn set_paused_moves_the_gate() {
        let shared = shared();

        assert_eq!(
            handle(
                Request::SetPaused {
                    paused: true,
                    until_ms: None
                },
                &shared,
                None
            ),
            Response::Done
        );
        assert_eq!(shared.gate.state(), GateState::Paused);
        assert!(matches!(
            handle(Request::Status, &shared, None),
            Response::Status(AgentStatus { paused: true, .. })
        ));

        handle(
            Request::SetPaused {
                paused: false,
                until_ms: None,
            },
            &shared,
            None,
        );
        assert_eq!(shared.gate.state(), GateState::Running);
    }

    #[test]
    fn program_recent_answers_from_the_ring() {
        let shared = shared();
        let build = vestigium_core::ProcessSample {
            pid: 7,
            name: "build.exe".to_string(),
            cpu_pct: 12.0,
            private_bytes: 64 << 20,
            read_bps: 0.0,
            write_bps: 0.0,
            gpu_pct: 0.0,
        };
        lock(&shared.programs).push(wall_ms(), &[build]);

        let answer = handle(
            Request::ProgramRecent {
                name: "build.exe".to_string(),
            },
            &shared,
            None,
        );

        assert!(matches!(answer, Response::ProgramRecent { points } if points.len() == 1));
    }

    #[test]
    fn a_timed_pause_reports_its_end_until_resumed() {
        let shared = shared();

        handle(
            Request::SetPaused {
                paused: true,
                until_ms: Some(1_700_000_000_000),
            },
            &shared,
            None,
        );
        assert!(matches!(
            handle(Request::Status, &shared, None),
            Response::Status(AgentStatus {
                paused_until_ms: Some(1_700_000_000_000),
                ..
            })
        ));

        handle(
            Request::SetPaused {
                paused: false,
                until_ms: Some(1_700_000_000_000),
            },
            &shared,
            None,
        );
        assert_eq!(lock(&shared.status).paused_until_ms, None);
    }

    #[test]
    fn language_follows_the_stored_choice() {
        assert_eq!(lang_for(LanguagePref::System, Lang::PtBr), Lang::PtBr);
        assert_eq!(lang_for(LanguagePref::En, Lang::PtBr), Lang::En);
        assert_eq!(lang_for(LanguagePref::PtBr, Lang::En), Lang::PtBr);
    }
}
