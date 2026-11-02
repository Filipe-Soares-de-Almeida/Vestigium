use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tauri::State;
use tauri::ipc::Channel;
use vestigium_core::Settings;
use vestigium_ipc::{Request, Response};

use crate::agent::{AgentLink, Link, SettingsDto, UiError, refused};
use crate::history::{History, MarksDto, OccurrenceDto, TimelineDto};
use crate::live::{LIVE_PERIOD, LiveMessage, run_live};

/// What every command works with: the database, the pipe and the live stream.
pub struct AppState {
    history: Arc<Mutex<History>>,
    link: Arc<Link>,
    /// Dropping it ends the live stream.
    live_stop: Mutex<Option<Sender<()>>>,
}

impl Default for AppState {
    fn default() -> AppState {
        AppState {
            history: Arc::new(Mutex::new(History::new(History::default_path()))),
            link: Arc::new(Link::new(Link::sibling_agent())),
            live_stop: Mutex::new(None),
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
