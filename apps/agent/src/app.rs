use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tray_icon::menu::MenuEvent;
use watchmypc_collect::spawn_probe;
use watchmypc_core::{GateState, RunGate};
use watchmypc_i18n::Catalog;
use watchmypc_ipc::{
    AgentStatus, IpcError, PROTOCOL_VERSION, PipeServer, Request, Response, current_user_sid,
    pipe_name,
};
use watchmypc_store::Store;
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
use windows::core::HSTRING;

use crate::sampler::run_sampler;
use crate::tray::{Action, Tray};
use crate::{paths, shutdown};

const DEFAULT_FLUSH: Duration = Duration::from_secs(5);
const PROBE_PERIOD: Duration = Duration::from_millis(100);
/// How long Exit and logoff wait for the last batch to be written.
const STOP_TIMEOUT: Duration = Duration::from_secs(2);

/// Command-line options, for tests and measurements.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub data_dir: Option<PathBuf>,
    pub flush: Duration,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Options {
        let mut options = Options {
            data_dir: None,
            flush: DEFAULT_FLUSH,
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
        None => paths::data_dir().map_err(failed)?,
    };
    let store = Store::open(&data_dir.join("watchmypc.db")).map_err(failed)?;
    let gate = RunGate::new();
    let status = Arc::new(Mutex::new(AgentStatus {
        started_at_ms: now_ms(),
        ..AgentStatus::default()
    }));

    let (probe_tx, probe_rx) = mpsc::channel();
    spawn_probe(PROBE_PERIOD, gate.clone(), probe_tx).map_err(failed)?;
    let sampler = Arc::new(
        run_sampler(store, options.flush, gate.clone(), probe_rx, status.clone())
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
        shutdown::watch_session_end(move || stop()).map_err(failed)?;
    }
    shutdown::pump(|| {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if tray.on_menu(&event.id, &gate, &status) == Action::Exit {
                shutdown::quit();
            }
        }
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

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use watchmypc_ipc::PROTOCOL_VERSION;

    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_default_to_five_second_batches() {
        let options = Options::parse(args(&[]));
        assert_eq!(options.flush, Duration::from_secs(5));
        assert_eq!(options.data_dir, None);
    }

    #[test]
    fn options_read_both_flags() {
        let options = Options::parse(args(&["--flush-secs", "60", "--data-dir", r"D:\tmp\wmpc"]));
        assert_eq!(options.flush, Duration::from_secs(60));
        assert_eq!(options.data_dir, Some(PathBuf::from(r"D:\tmp\wmpc")));
    }

    #[test]
    fn options_ignore_a_bad_number() {
        let options = Options::parse(args(&["--flush-secs", "soon"]));
        assert_eq!(options.flush, Duration::from_secs(5));
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
