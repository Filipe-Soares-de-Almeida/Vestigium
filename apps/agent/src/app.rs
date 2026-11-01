use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread;
use std::time::Duration;

use tray_icon::menu::MenuEvent;
use vestigium_collect::{EventSubscription, ProbeConfig, spawn_probe, wall_ms};
use vestigium_core::{
    DEFAULT_PROBE_PERIOD, GateState, RecentState, RunGate, SENSITIVE_FOR, Timing,
};
use vestigium_i18n::Catalog;
use vestigium_ipc::{
    AgentStatus, IpcError, PROTOCOL_VERSION, PipeServer, Request, Response, current_user_sid,
    pipe_name,
};
use vestigium_store::Store;
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
use windows::core::HSTRING;

use crate::sampler::{Wiring, run_sampler};
use crate::sensitive::SensitiveMode;
use crate::tray::{Action, Tray};
use crate::{paths, power, shutdown};

const DEFAULT_FLUSH: Duration = Duration::from_secs(60);
/// How long Exit and logoff wait for the last batch to be written.
const STOP_TIMEOUT: Duration = Duration::from_secs(2);
/// How far back the first run reads the curated Windows events.
const EVENT_BACKFILL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

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
    Failed(String),
}

fn failed(err: impl std::fmt::Display) -> StartError {
    StartError::Failed(err.to_string())
}

/// Answers one request from the interface.
pub fn handle(request: Request, status: &AgentStatus) -> Response {
    match request {
        Request::Hello { protocol } if protocol == PROTOCOL_VERSION => Response::Hello {
            protocol: PROTOCOL_VERSION,
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        Request::Hello { protocol } => Response::Error {
            message: format!(
                "protocol {protocol} is not supported; this agent speaks {PROTOCOL_VERSION}"
            ),
        },
        Request::Status => Response::Status(status.clone()),
    }
}

/// Starts the agent and runs it until Exit or the end of the session.
pub fn run(catalog: &Catalog, options: Options) -> Result<(), StartError> {
    let server = match PipeServer::bind(&pipe_name(&current_user_sid().map_err(failed)?)) {
        Err(IpcError::AlreadyRunning) => return Err(StartError::AlreadyRunning),
        bound => bound.map_err(failed)?,
    };
    let data_dir = match options.data_dir {
        Some(dir) => dir,
        None => paths::data_dir().ok_or_else(|| failed("the LOCALAPPDATA folder is not set"))?,
    };
    let store = Store::open(&data_dir.join("vestigium.db")).map_err(failed)?;
    let gate = RunGate::new();
    let status = Arc::new(Mutex::new(AgentStatus {
        started_at_ms: wall_ms(),
        ..AgentStatus::default()
    }));

    let (probe_tx, probe_rx) = mpsc::channel();
    let (power_tx, power_rx) = mpsc::channel();
    let (events_tx, events_rx) = mpsc::channel();
    // Without a readable bookmark the backfill runs again; stored events are not duplicated.
    let bookmark = store.bookmark_xml().ok().flatten();
    let subscription = EventSubscription::start(bookmark.as_deref(), EVENT_BACKFILL, events_tx)
        .inspect_err(|err| lock(&status).last_error = Some(err.to_string()))
        .ok();
    let probe = ProbeConfig::new(DEFAULT_PROBE_PERIOD);
    let sensitive = SensitiveMode::new(probe.clone(), options.sensitive_for);
    let recent = Arc::new(Mutex::new(RecentState::default()));
    spawn_probe(probe, gate.clone(), probe_tx).map_err(failed)?;
    let sampler = Arc::new(
        run_sampler(
            store,
            Wiring {
                flush: options.flush,
                timing: Timing::default(),
                gate: gate.clone(),
                probe_rx,
                power_rx,
                events_rx,
                subscription,
                sensitive: sensitive.clone(),
                status: status.clone(),
                recent: recent.clone(),
            },
        )
        .map_err(failed)?,
    );
    let stop = {
        let gate = gate.clone();
        move || {
            gate.set(GateState::Stopping);
            sampler.wait(STOP_TIMEOUT);
        }
    };

    {
        let status = status.clone();
        thread::Builder::new()
            .name("pipe".to_string())
            .spawn(move || server.serve(|request| handle(request, &lock(&status))))
            .map_err(failed)?;
    }

    let exe = std::env::current_exe().map_err(failed)?;
    let tray = Tray::new(catalog, &exe, paths::ui_exe()).map_err(failed)?;
    let stop = Arc::new(stop);
    {
        let stop = stop.clone();
        let window = shutdown::watch_session_end(move || stop()).map_err(failed)?;
        power::watch(window, move |event| {
            let _ = power_tx.send(event);
        })
        .map_err(failed)?;
    }
    shutdown::pump(|| {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if tray.on_menu(&event.id, &gate, &status, &sensitive, &lock(&recent)) == Action::Exit {
                shutdown::quit();
            }
        }
        tray.refresh(&gate, &sensitive, &lock(&recent));
    });
    stop();
    Ok(())
}

/// Shows why the agent could not start.
pub fn show_error(catalog: &Catalog, reason: &str) {
    let body = catalog.format("error.startup.body", &[("error", reason)]);
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

#[cfg(test)]
mod tests {
    use vestigium_ipc::PROTOCOL_VERSION;

    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_default_to_one_minute_batches() {
        let options = Options::parse(args(&[]));
        assert_eq!(options.flush, Duration::from_secs(60));
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

    #[test]
    fn hello_with_the_same_protocol_is_answered() {
        let response = handle(
            Request::Hello {
                protocol: PROTOCOL_VERSION,
            },
            &AgentStatus::default(),
        );
        assert!(
            matches!(response, Response::Hello { protocol, .. } if protocol == PROTOCOL_VERSION)
        );
    }

    #[test]
    fn hello_with_another_protocol_is_an_error() {
        let response = handle(
            Request::Hello {
                protocol: PROTOCOL_VERSION + 1,
            },
            &AgentStatus::default(),
        );
        assert!(matches!(response, Response::Error { .. }));
    }

    #[test]
    fn status_reflects_the_agent() {
        let status = AgentStatus {
            paused: true,
            started_at_ms: 42,
            last_sample_ms: Some(43),
            last_error: None,
        };
        assert_eq!(
            handle(Request::Status, &status),
            Response::Status(status.clone())
        );
    }
}
