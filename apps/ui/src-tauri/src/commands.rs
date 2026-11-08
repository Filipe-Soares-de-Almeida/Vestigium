use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use vestigium_core::{Episode, Settings};
use vestigium_ipc::{Request, Response};

use vestigium_store::CaptureSummary;

use crate::agent::{AgentLink, Link, SettingsDto, UiError, refused};
use crate::capture::{
    CAPTURE_MINUTES, CaptureFinished, CaptureRun, DeleteFinished, DumpScanFinished, ScanFinished,
    capture_file, delete_dumps_args, dump_scan_args, dumps_file, helper_args, launch_elevated,
    scan_args, scan_file, sibling_helper,
};
use crate::history::{
    CaptureDto, CrashControlDto, DumpsDto, EpisodeDetailDto, HealthDto, History, MarksDto,
    OccurrenceDto, Recent, TimelineDto,
};
use crate::hwtests::{
    DiskDetailDto, RUNS_SHOWN_MS, RunningTest, TestFinished, TestProgress, TestsDto, disk_list,
    disk_scan_args, letter_of, memory_diagnostic_exe, memory_now, read_progress, run_benchmark,
    run_memory_test, scan_under_way, tests_file, volume_check_args, volume_list, write_versioned,
};
use crate::live::{LIVE_PERIOD, LiveMessage, run_live};
use crate::tweaks::{TweakPick, TweaksApplied, TweaksDto};
use vestigium_core::health::Subject;
use vestigium_core::hwtest::{BENCH_FILE_BYTES, QUICK_MINUTES, TestKind, TestRun};

/// What every command works with: the database, the pipe and the live stream.
pub struct AppState {
    history: Arc<Mutex<History>>,
    link: Arc<Link>,
    /// Dropping it ends the live stream.
    live_stop: Mutex<Option<Sender<()>>>,
    /// The deep capture this window started, until its helper exits.
    capture_run: Arc<Mutex<Option<CaptureRun>>>,
    /// The hardware test this window started, until it ends.
    test: Arc<Mutex<Option<RunningTest>>>,
    /// The thread of a test that runs in this window, which closing the window waits for.
    test_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Default for AppState {
    fn default() -> AppState {
        AppState {
            history: Arc::new(Mutex::new(History::new(History::default_path()))),
            link: Arc::new(Link::new(Link::sibling_agent())),
            live_stop: Mutex::new(None),
            capture_run: Arc::new(Mutex::new(None)),
            test: Arc::new(Mutex::new(None)),
            test_thread: Arc::new(Mutex::new(None)),
        }
    }
}

/// How far back the watcher keeps its readings in memory.
const RECENT_MS: i64 = 10 * 60_000;
/// How far back the SMART page draws an attribute.
const SMART_SHOWN_MS: i64 = 30 * 24 * 60 * 60_000;

/// The watcher's last minutes, for a range that reaches into them; empty when it does not answer.
fn recent(link: &Link, to_ms: i64, now_ms: i64) -> Recent {
    if to_ms < now_ms - RECENT_MS {
        return Recent::default();
    }
    match link.call(&Request::Recent) {
        Ok(Response::Recent { readings, pressure }) => Recent {
            readings,
            pressure: pressure
                .iter()
                .map(|span| (span.start_ms, span.end_ms.unwrap_or(now_ms)))
                .collect(),
        },
        _ => Recent::default(),
    }
}

#[tauri::command]
pub async fn timeline(
    state: State<'_, AppState>,
    from_ms: i64,
    to_ms: i64,
    max_points: u32,
) -> Result<TimelineDto, UiError> {
    let history = state.history.clone();
    let link = state.link.clone();
    blocking(move || {
        let recent = recent(&link, to_ms, now_ms());
        Ok(lock(&history).timeline(from_ms, to_ms, max_points, &recent)?)
    })
    .await
}

#[tauri::command]
pub async fn marks(
    state: State<'_, AppState>,
    from_ms: i64,
    to_ms: i64,
) -> Result<MarksDto, UiError> {
    let history = state.history.clone();
    blocking(move || Ok(lock(&history).marks(from_ms, to_ms)?)).await
}

#[tauri::command]
pub async fn occurrence(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Option<OccurrenceDto>, UiError> {
    let history = state.history.clone();
    blocking(move || Ok(lock(&history).occurrence(id)?)).await
}

#[tauri::command]
pub async fn episode_detail(
    state: State<'_, AppState>,
    episode: Episode,
) -> Result<EpisodeDetailDto, UiError> {
    let history = state.history.clone();
    let link = state.link.clone();
    blocking(move || {
        let now = now_ms();
        let recent = recent(&link, episode.end_ms + RECENT_MS, now);
        Ok(lock(&history).episode(&episode, now, &recent)?)
    })
    .await
}

/// Streams the last ten minutes, then one second a second; replaces any running stream.
#[tauri::command]
pub fn live_start(
    state: State<'_, AppState>,
    on_frame: Channel<LiveMessage>,
) -> Result<(), UiError> {
    let (stop_tx, stop_rx) = mpsc::channel();
    *lock(&state.live_stop) = Some(stop_tx);
    let link = state.link.clone();
    std::thread::Builder::new()
        .name("live".to_string())
        .spawn(move || {
            run_live(
                |request| link.call(request),
                |message| on_frame.send(message).is_ok(),
                &stop_rx,
                LIVE_PERIOD,
            )
        })
        .map(drop)
        .map_err(|err| UiError::Failed(err.to_string()))
}

#[tauri::command]
pub fn live_stop(state: State<'_, AppState>) {
    lock(&state.live_stop).take();
}

#[tauri::command]
pub async fn agent_status(state: State<'_, AppState>) -> Result<AgentLink, UiError> {
    let link = state.link.clone();
    blocking(move || Ok(link.status())).await
}

#[tauri::command]
pub async fn start_agent(state: State<'_, AppState>) -> Result<(), UiError> {
    let link = state.link.clone();
    blocking(move || link.start_agent()).await
}

/// Pauses or resumes watching; `until_ms` ends the pause by itself at that time.
#[tauri::command]
pub async fn set_paused(
    state: State<'_, AppState>,
    paused: bool,
    until_ms: Option<i64>,
) -> Result<(), UiError> {
    send(&state, Request::SetPaused { paused, until_ms }).await
}

#[tauri::command]
pub async fn set_sensitive(state: State<'_, AppState>, on: bool) -> Result<(), UiError> {
    send(&state, Request::SetSensitive { on }).await
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<(), UiError> {
    send(&state, Request::ClearHistory).await
}

#[tauri::command]
pub async fn set_autostart(state: State<'_, AppState>, on: bool) -> Result<(), UiError> {
    send(&state, Request::SetAutostart { on }).await
}

#[tauri::command]
pub async fn set_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), UiError> {
    send(&state, Request::SetSettings(settings)).await
}

/// The watcher's settings, or the stored ones, read-only, while it is stopped.
#[tauri::command]
pub async fn settings(state: State<'_, AppState>) -> Result<SettingsDto, UiError> {
    let link = state.link.clone();
    let history = state.history.clone();
    blocking(move || match link.call(&Request::Settings) {
        Ok(Response::Settings {
            settings,
            autostart,
        }) => Ok(SettingsDto {
            settings,
            autostart: Some(autostart),
            editable: true,
        }),
        Ok(other) => Err(refused(other)),
        Err(UiError::AgentStopped) => Ok(SettingsDto {
            settings: lock(&history).settings()?.unwrap_or_default(),
            autostart: None,
            editable: false,
        }),
        Err(err) => Err(err),
    })
    .await
}

/// Starts a deep capture of `minutes` in the elevated helper; the window hears `capture-finished`.
#[tauri::command]
pub async fn start_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    minutes: u64,
) -> Result<CaptureRun, UiError> {
    if !CAPTURE_MINUTES.contains(&minutes) {
        return Err(UiError::Failed(format!("{minutes} minutes is not offered")));
    }
    let link = state.link.clone();
    let slot = state.capture_run.clone();
    blocking(move || {
        if lock(&slot).is_some() {
            return Err(UiError::Failed("a deep capture is already running".into()));
        }
        let db = History::default_path()
            .ok_or_else(|| UiError::Failed("the LOCALAPPDATA folder is not set".into()))?;
        let helper = sibling_helper()
            .ok_or_else(|| UiError::Failed("the helper is not next to the window".into()))?;
        let started_ms = now_ms();
        let out = capture_file(&db, started_ms)
            .ok_or_else(|| UiError::Failed("no data folder".into()))?;
        if let Some(folder) = out.parent() {
            std::fs::create_dir_all(folder).map_err(|err| UiError::Failed(err.to_string()))?;
        }
        match link.command(&Request::SetSensitive { on: true }) {
            Ok(()) | Err(UiError::AgentStopped) => {}
            Err(err) => return Err(err),
        }
        let process = launch_elevated(&helper, &helper_args(minutes, &out))?;
        let run = CaptureRun {
            started_ms,
            ends_ms: started_ms + minutes as i64 * 60_000,
        };
        *lock(&slot) = Some(run);
        std::thread::spawn(move || {
            let exit_code = process.wait();
            let ids = match link.call(&Request::ImportResults) {
                Ok(Response::Imported { captures, .. }) => captures,
                _ => Vec::new(),
            };
            *lock(&slot) = None;
            let _ = app.emit("capture-finished", CaptureFinished { ids, exit_code });
        });
        Ok(run)
    })
    .await
}

/// The deep capture this window started, while its helper runs.
#[tauri::command]
pub fn capture_run(state: State<'_, AppState>) -> Option<CaptureRun> {
    *lock(&state.capture_run)
}

/// Asks the watcher to store every finished capture it has not stored yet; returns their ids.
#[tauri::command]
pub async fn import_captures(state: State<'_, AppState>) -> Result<Vec<i64>, UiError> {
    let link = state.link.clone();
    blocking(move || match link.call(&Request::ImportResults)? {
        Response::Imported { captures, .. } => Ok(captures),
        other => Err(refused(other)),
    })
    .await
}

/// The health page: open findings, the ones that closed lately and the last full scan.
#[tauri::command]
pub async fn health(state: State<'_, AppState>) -> Result<HealthDto, UiError> {
    let history = state.history.clone();
    blocking(move || Ok(lock(&history).health(now_ms())?)).await
}

/// Runs a full scan in the elevated helper and waits for it; Windows asks for permission first.
#[tauri::command]
pub async fn start_scan(state: State<'_, AppState>) -> Result<ScanFinished, UiError> {
    let link = state.link.clone();
    blocking(move || {
        let db = History::default_path()
            .ok_or_else(|| UiError::Failed("the LOCALAPPDATA folder is not set".into()))?;
        let helper = sibling_helper()
            .ok_or_else(|| UiError::Failed("the helper is not next to the window".into()))?;
        let out =
            scan_file(&db, now_ms()).ok_or_else(|| UiError::Failed("no data folder".into()))?;
        if let Some(folder) = out.parent() {
            std::fs::create_dir_all(folder).map_err(|err| UiError::Failed(err.to_string()))?;
        }
        let exit_code = launch_elevated(&helper, &scan_args(&out))?.wait();
        let scans = match link.call(&Request::ImportResults) {
            Ok(Response::Imported { scans, .. }) => scans,
            _ => Vec::new(),
        };
        Ok(ScanFinished { scans, exit_code })
    })
    .await
}

/// The dumps Windows kept, what each says and the space they take.
#[tauri::command]
pub async fn dumps(state: State<'_, AppState>) -> Result<DumpsDto, UiError> {
    let history = state.history.clone();
    blocking(move || {
        let mut dto = lock(&history).dumps(now_ms())?;
        dto.crash = vestigium_collect::crash_control().map(|c| CrashControlDto {
            dump_type: c.dump_type,
            dump_file: c.dump_file.to_string_lossy().into_owned(),
            minidump_dir: c.minidump_dir.to_string_lossy().into_owned(),
            overwrite: c.overwrite,
        });
        Ok(dto)
    })
    .await
}

/// Reads every dump in the elevated helper and waits for it; Windows asks for permission first.
#[tauri::command]
pub async fn start_dump_scan(state: State<'_, AppState>) -> Result<DumpScanFinished, UiError> {
    let link = state.link.clone();
    blocking(move || {
        let (helper, out) = helper_and_file("dumps")?;
        let exit_code = launch_elevated(&helper, &dump_scan_args(&out))?.wait();
        let dumps = match link.call(&Request::ImportResults) {
            Ok(Response::Imported { dumps, .. }) => dumps,
            _ => Vec::new(),
        };
        Ok(DumpScanFinished { dumps, exit_code })
    })
    .await
}

/// Deletes the dumps with these ids in the elevated helper, which checks each path again.
#[tauri::command]
pub async fn delete_dumps(
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<DeleteFinished, UiError> {
    let (history, link) = (state.history.clone(), state.link.clone());
    blocking(move || {
        let paths = lock(&history).dump_paths(&ids)?;
        if paths.is_empty() {
            return Ok(DeleteFinished {
                deleted: 0,
                exit_code: None,
            });
        }
        let (helper, list) = helper_and_file("delete")?;
        std::fs::write(&list, paths.join("\r\n"))
            .map_err(|err| UiError::Failed(err.to_string()))?;
        let exit_code = launch_elevated(&helper, &delete_dumps_args(&list))?.wait();
        let deleted = match link.call(&Request::ImportResults) {
            Ok(Response::Imported { deleted, .. }) => deleted,
            _ => 0,
        };
        Ok(DeleteFinished { deleted, exit_code })
    })
    .await
}

/// The helper beside the window and a new `<now>.<extension>` in the dumps folder.
fn helper_and_file(extension: &str) -> Result<(std::path::PathBuf, std::path::PathBuf), UiError> {
    let db = History::default_path()
        .ok_or_else(|| UiError::Failed("the LOCALAPPDATA folder is not set".into()))?;
    let helper = sibling_helper()
        .ok_or_else(|| UiError::Failed("the helper is not next to the window".into()))?;
    let file = dumps_file(&db, now_ms(), extension)
        .ok_or_else(|| UiError::Failed("no data folder".into()))?;
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|err| UiError::Failed(err.to_string()))?;
    }
    Ok((helper, file))
}

#[tauri::command]
pub async fn captures(
    state: State<'_, AppState>,
    from_ms: i64,
    to_ms: i64,
) -> Result<Vec<CaptureSummary>, UiError> {
    let history = state.history.clone();
    blocking(move || Ok(lock(&history).captures(from_ms, to_ms)?)).await
}

#[tauri::command]
pub async fn capture(state: State<'_, AppState>, id: i64) -> Result<Option<CaptureDto>, UiError> {
    let history = state.history.clone();
    blocking(move || Ok(lock(&history).capture(id)?)).await
}

/// The Tests page: memory free now, Windows' memory diagnostics, disks, volumes and earlier runs.
#[tauri::command]
pub async fn tests(state: State<'_, AppState>) -> Result<TestsDto, UiError> {
    let (history, running) = (state.history.clone(), state.test.clone());
    blocking(move || {
        let now = now_ms();
        let (runs, machine) = {
            let mut history = lock(&history);
            (
                history.test_runs(now - RUNS_SHOWN_MS, now)?,
                history.machine()?,
            )
        };
        let volumes = volume_list();
        Ok(TestsDto {
            memory: memory_now(),
            diagnostics: crate::hwtests::diagnostics(),
            disks: disk_list(machine.as_ref(), &volumes),
            volumes,
            runs,
            running: lock(&running).clone(),
            scan: tests_dir().and_then(|dir| scan_under_way(&dir, now)),
            bench_file_bytes: BENCH_FILE_BYTES,
        })
    })
    .await
}

/// SMART or NVMe detail of one disk, read now.
#[tauri::command]
pub async fn disk_detail(number: u32, name: String) -> Result<DiskDetailDto, UiError> {
    blocking(move || Ok(crate::hwtests::disk_detail(number, &name, now_ms()))).await
}

/// The last 30 days of one SMART attribute of a disk, one reading a day.
#[tauri::command]
pub async fn smart_history(
    state: State<'_, AppState>,
    disk: String,
    attr: u8,
) -> Result<Vec<crate::history::SmartPointDto>, UiError> {
    let history = state.history.clone();
    blocking(move || Ok(lock(&history).smart_history(&disk, attr, now_ms() - SMART_SHOWN_MS)?))
        .await
}

/// Opens Windows' own Memory Diagnostic, which asks for permission and offers to restart.
#[tauri::command]
pub async fn open_memory_diagnostic() -> Result<(), UiError> {
    blocking(|| launch_elevated(&memory_diagnostic_exe(), "").map(drop)).await
}

/// Tests part of the free memory for `minutes`, in this window; it hears `test-progress` and
/// `test-finished`.
#[tauri::command]
pub async fn start_memory_test(
    app: AppHandle,
    state: State<'_, AppState>,
    minutes: u32,
) -> Result<RunningTest, UiError> {
    if !QUICK_MINUTES.contains(&minutes) {
        return Err(UiError::Failed(format!("{minutes} minutes is not offered")));
    }
    if memory_now().quick_bytes.is_none() {
        return Err(UiError::Failed(
            "too little free memory for a quick test".into(),
        ));
    }
    start_in_window(
        app,
        &state,
        TestKind::MemoryQuick,
        Subject::Pc,
        Some(i64::from(minutes) * 60_000),
        move |stop, progress| run_memory_test(minutes, stop, progress),
    )
}

/// Measures one volume with a temporary file, in this window.
#[tauri::command]
pub async fn start_benchmark(
    app: AppHandle,
    state: State<'_, AppState>,
    volume: String,
) -> Result<RunningTest, UiError> {
    let found = volume_list()
        .into_iter()
        .find(|v| v.root.eq_ignore_ascii_case(&volume))
        .ok_or_else(|| UiError::Failed(format!("{volume} is not a fixed volume")))?;
    start_in_window(
        app,
        &state,
        TestKind::DiskBenchmark,
        Subject::Volume(found.root.clone()),
        None,
        move |stop, progress| Ok(run_benchmark(&found, stop, progress)),
    )
}

fn start_in_window(
    app: AppHandle,
    state: &State<'_, AppState>,
    kind: TestKind,
    subject: Subject,
    length_ms: Option<i64>,
    work: impl FnOnce(&AtomicBool, &mut dyn FnMut(TestProgress)) -> Result<TestRun, UiError>
    + Send
    + 'static,
) -> Result<RunningTest, UiError> {
    let (slot, link) = (state.test.clone(), state.link.clone());
    let running = claim(&slot, kind, subject, length_ms, None)?;
    let stop = running.stop.clone();
    let worker_slot = slot.clone();
    let spawned = std::thread::Builder::new()
        .name("test".to_string())
        .spawn(move || {
            let result = work(&stop, &mut |p| {
                let _ = app.emit("test-progress", p);
            });
            let finished = match result {
                Ok(run) => {
                    let written = History::default_path()
                        .and_then(|db| tests_file(&db, run.started_ms, "test"))
                        .map(|path| write_versioned(&path, &run));
                    let id = match written {
                        Some(Ok(())) => imported_test(&link),
                        _ => None,
                    };
                    TestFinished {
                        id,
                        run: Some(run),
                        exit_code: None,
                    }
                }
                Err(_) => TestFinished {
                    id: None,
                    run: None,
                    exit_code: None,
                },
            };
            *lock(&worker_slot) = None;
            let _ = app.emit("test-finished", finished);
        });
    match spawned {
        Ok(thread) => {
            *lock(&state.test_thread) = Some(thread);
            Ok(running)
        }
        Err(err) => {
            *lock(&slot) = None;
            Err(UiError::Failed(err.to_string()))
        }
    }
}

/// Holds the one test slot for a new run; another test running refuses it.
fn claim(
    slot: &Mutex<Option<RunningTest>>,
    kind: TestKind,
    subject: Subject,
    length_ms: Option<i64>,
    stop_file: Option<PathBuf>,
) -> Result<RunningTest, UiError> {
    let mut held = lock(slot);
    if held.is_some() {
        return Err(UiError::Failed("a test is already running".into()));
    }
    let started_ms = now_ms();
    let running = RunningTest {
        kind,
        subject,
        started_ms,
        ends_ms: length_ms.map(|l| started_ms + l),
        stop: Arc::new(AtomicBool::new(false)),
        stop_file,
    };
    *held = Some(running.clone());
    Ok(running)
}

/// The id of the newest test run the watcher just stored, while it runs.
fn imported_test(link: &Link) -> Option<i64> {
    match link.call(&Request::ImportResults) {
        Ok(Response::Imported { tests, .. }) => tests.last().copied(),
        _ => None,
    }
}

/// The tests folder beside the watcher's database.
fn tests_dir() -> Option<PathBuf> {
    let db = History::default_path()?;
    tests_file(&db, 0, "test")?.parent().map(PathBuf::from)
}

/// Reads physical drive `disk` from `from_mib` in the elevated helper; the window hears progress
/// every few seconds and `test-finished` when the helper exits.
#[tauri::command]
pub async fn start_surface_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    disk: u32,
    from_mib: u64,
) -> Result<RunningTest, UiError> {
    let (slot, link) = (state.test.clone(), state.link.clone());
    blocking(move || {
        let (helper, out) = helper_and_test_file("surface")?;
        let running = claim(
            &slot,
            TestKind::DiskSurface,
            Subject::Disk(disk.to_string()),
            None,
            Some(out.with_extension("stop")),
        )?;
        let process = match launch_elevated(&helper, &disk_scan_args(disk, from_mib, &out)) {
            Ok(process) => process,
            Err(err) => {
                *lock(&slot) = None;
                return Err(err);
            }
        };
        std::thread::spawn(move || {
            let progress = out.with_extension("progress");
            let exit_code = loop {
                if let Some(code) = process.exited_within(2_000) {
                    break code;
                }
                if let Some(p) = read_progress(&progress) {
                    let _ = app.emit("test-progress", TestProgress::Surface(p));
                }
            };
            let id = imported_test(&link);
            *lock(&slot) = None;
            let _ = app.emit(
                "test-finished",
                TestFinished {
                    id,
                    run: None,
                    exit_code,
                },
            );
        });
        Ok(running)
    })
    .await
}

/// Checks the file system of one volume in the elevated helper; the window hears `test-finished`.
#[tauri::command]
pub async fn start_volume_check(
    app: AppHandle,
    state: State<'_, AppState>,
    volume: String,
) -> Result<RunningTest, UiError> {
    let letter = letter_of(&volume)
        .ok_or_else(|| UiError::Failed(format!("{volume} is not a drive letter")))?;
    let (slot, link) = (state.test.clone(), state.link.clone());
    blocking(move || {
        let (helper, out) = helper_and_test_file("chkdsk")?;
        let running = claim(
            &slot,
            TestKind::VolumeCheck,
            Subject::Volume(format!("{letter}:\\")),
            None,
            None,
        )?;
        let process = match launch_elevated(&helper, &volume_check_args(letter, &out)) {
            Ok(process) => process,
            Err(err) => {
                *lock(&slot) = None;
                return Err(err);
            }
        };
        std::thread::spawn(move || {
            let exit_code = process.wait();
            let id = imported_test(&link);
            *lock(&slot) = None;
            let _ = app.emit(
                "test-finished",
                TestFinished {
                    id,
                    run: None,
                    exit_code,
                },
            );
        });
        Ok(running)
    })
    .await
}

/// The helper beside the window and a new `<now>.<extension>` in the tests folder.
fn helper_and_test_file(extension: &str) -> Result<(PathBuf, PathBuf), UiError> {
    let db = History::default_path()
        .ok_or_else(|| UiError::Failed("the LOCALAPPDATA folder is not set".into()))?;
    let helper = sibling_helper()
        .ok_or_else(|| UiError::Failed("the helper is not next to the window".into()))?;
    let file = tests_file(&db, now_ms(), extension)
        .ok_or_else(|| UiError::Failed("no data folder".into()))?;
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|err| UiError::Failed(err.to_string()))?;
    }
    Ok((helper, file))
}

/// Stops the test this window started, or the disk scan whose files `scan` names.
#[tauri::command]
pub fn stop_test(state: State<'_, AppState>, scan: Option<String>) -> Result<(), UiError> {
    if let Some(running) = lock(&state.test).as_ref() {
        crate::hwtests::stop(running);
        return Ok(());
    }
    let stem = scan.filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
    match (stem, tests_dir()) {
        (Some(stem), Some(dir)) => std::fs::write(dir.join(format!("{stem}.stop")), b"")
            .map_err(|err| UiError::Failed(err.to_string())),
        _ => Err(UiError::Failed("no test is running".into())),
    }
}

/// The test this window started, while it runs.
#[tauri::command]
pub fn test_running(state: State<'_, AppState>) -> Option<RunningTest> {
    lock(&state.test).clone()
}

/// Closing the window ends its own test and waits a little for the result to be written; a disk
/// scan in the helper goes on.
pub fn finish_tests(state: &AppState) {
    if let Some(running) = lock(&state.test).as_ref()
        && running.stop_file.is_none()
    {
        running
            .stop
            .store(true, std::sync::atomic::Ordering::Release);
    }
    let Some(thread) = lock(&state.test_thread).take() else {
        return;
    };
    let waited = Instant::now();
    while !thread.is_finished() && waited.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

async fn send(state: &State<'_, AppState>, request: Request) -> Result<(), UiError> {
    let link = state.link.clone();
    blocking(move || link.command(&request)).await
}

/// Runs work that waits on the disk or the pipe away from the async runtime.
/// Every tweak with its state and signals, the changes made so far and Windows' last restore point.
#[tauri::command]
pub async fn tweaks(state: State<'_, AppState>) -> Result<TweaksDto, UiError> {
    let history = state.history.clone();
    blocking(move || crate::tweaks::read(&mut lock(&history), now_ms())).await
}

/// Applies the tweaks picked; Windows asks for permission once for those that need it.
#[tauri::command]
pub async fn apply_tweaks(
    state: State<'_, AppState>,
    picks: Vec<TweakPick>,
    without_restore_point: bool,
) -> Result<TweaksApplied, UiError> {
    let (history, link) = (state.history.clone(), state.link.clone());
    blocking(move || {
        crate::tweaks::apply(
            &mut lock(&history),
            &link,
            &picks,
            without_restore_point,
            now_ms(),
        )
    })
    .await
}

/// Puts back what one change wrote.
#[tauri::command]
pub async fn undo_tweak(
    state: State<'_, AppState>,
    id: i64,
    without_restore_point: bool,
) -> Result<TweaksApplied, UiError> {
    let (history, link) = (state.history.clone(), state.link.clone());
    blocking(move || {
        crate::tweaks::undo_change(
            &mut lock(&history),
            &link,
            id,
            without_restore_point,
            now_ms(),
        )
    })
    .await
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, UiError> + Send + 'static,
) -> Result<T, UiError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|err| UiError::Failed(err.to_string()))?
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
