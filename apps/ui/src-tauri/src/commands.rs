use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use vestigium_core::Settings;
use vestigium_ipc::{Request, Response};

use vestigium_store::CaptureSummary;

use crate::agent::{AgentLink, Link, SettingsDto, UiError, refused};
use crate::capture::{
    CAPTURE_MINUTES, CaptureFinished, CaptureRun, DeleteFinished, DumpScanFinished, ScanFinished,
    capture_file, delete_dumps_args, dump_scan_args, dumps_file, helper_args, launch_elevated,
    scan_args, scan_file, sibling_helper,
};
use crate::history::{
    CaptureDto, CrashControlDto, DumpsDto, HealthDto, History, MarksDto, OccurrenceDto, TimelineDto,
};
use crate::live::{LIVE_PERIOD, LiveMessage, run_live};

/// What every command works with: the database, the pipe and the live stream.
pub struct AppState {
    history: Arc<Mutex<History>>,
    link: Arc<Link>,
    /// Dropping it ends the live stream.
    live_stop: Mutex<Option<Sender<()>>>,
    /// The deep capture this window started, until its helper exits.
    capture_run: Arc<Mutex<Option<CaptureRun>>>,
}

impl Default for AppState {
    fn default() -> AppState {
        AppState {
            history: Arc::new(Mutex::new(History::new(History::default_path()))),
            link: Arc::new(Link::new(Link::sibling_agent())),
            live_stop: Mutex::new(None),
            capture_run: Arc::new(Mutex::new(None)),
        }
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
    blocking(move || Ok(lock(&history).timeline(from_ms, to_ms, max_points)?)).await
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

#[tauri::command]
pub async fn set_paused(state: State<'_, AppState>, paused: bool) -> Result<(), UiError> {
    send(&state, Request::SetPaused { paused }).await
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
