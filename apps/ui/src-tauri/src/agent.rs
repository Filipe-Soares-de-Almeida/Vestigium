use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::{Serialize, Serializer};
use vestigium_core::Settings;
use vestigium_ipc::{
    AgentStatus, IpcError, PROTOCOL_VERSION, PipeClient, Request, Response, current_user_sid,
    pipe_name,
};
use vestigium_store::StoreError;

/// How long the window waits for a watcher it started to answer.
const START_TIMEOUT: Duration = Duration::from_secs(3);
const START_POLL: Duration = Duration::from_millis(100);
/// `ERROR_FILE_NOT_FOUND`: nobody has created the pipe.
const NO_PIPE: i32 = 2;

/// Why a command from the window failed.
#[derive(Debug, thiserror::Error)]
pub enum UiError {
    #[error("the watcher is not running")]
    AgentStopped,
    #[error("the pipe belongs to another program, or to a watcher of another version")]
    Mismatch,
    #[error("the watcher refused: {0}")]
    Refused(String),
    #[error("the watcher did not answer within {} s", START_TIMEOUT.as_secs())]
    StartTimeout,
    #[error("history: {0}")]
    Store(#[from] StoreError),
    #[error("pipe: {0}")]
    Ipc(IpcError),
    #[error("{0}")]
    Failed(String),
}

impl From<IpcError> for UiError {
    fn from(err: IpcError) -> UiError {
        match err {
            IpcError::Io(io) if io.raw_os_error() == Some(NO_PIPE) => UiError::AgentStopped,
            IpcError::ServerMismatch { .. } => UiError::Mismatch,
            other => UiError::Ipc(other),
        }
    }
}

/// Sent to the window as `{ kind, message }`; the window words it from `kind`.
impl Serialize for UiError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            kind: &'a str,
            message: String,
        }
        let kind = match self {
            UiError::AgentStopped => "agent_stopped",
            UiError::Mismatch => "mismatch",
            UiError::Refused(_) => "refused",
            UiError::StartTimeout => "start_timeout",
            UiError::Store(_) => "store",
            UiError::Ipc(_) => "ipc",
            UiError::Failed(_) => "failed",
        };
        Wire {
            kind,
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

/// Whether the watcher answers, and what it is doing.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "link", rename_all = "snake_case")]
pub enum AgentLink {
    Running(AgentStatus),
    Stopped,
    /// Something else owns the pipe, or the watcher speaks another protocol.
    Mismatch,
}

/// Settings as the window shows them; read from the database while the watcher is stopped.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SettingsDto {
    pub settings: Settings,
    /// `None` while the watcher is stopped: it reads the registry, not the window.
    pub autostart: Option<bool>,
    /// Only a running watcher writes settings.
    pub editable: bool,
}

/// The one pipe connection to the watcher, shared by every command and the live stream.
pub struct Link {
    agent_exe: Option<PathBuf>,
    client: Mutex<Option<PipeClient>>,
}

impl Link {
    pub fn new(agent_exe: Option<PathBuf>) -> Link {
        Link {
            agent_exe,
            client: Mutex::new(None),
        }
    }

    /// `Vestigium.exe` next to this executable, the only program trusted to own the pipe.
    pub fn sibling_agent() -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        Some(exe.with_file_name("Vestigium.exe"))
    }

    /// Sends one request, connecting first if needed; a failed call drops the connection.
    pub fn call(&self, request: &Request) -> Result<Response, UiError> {
        let mut client = lock(&self.client);
        let connected = match client.take() {
            Some(open) => open,
            None => self.connect()?,
        };
        let open = client.insert(connected);
        match open.call(request) {
            Ok(response) => Ok(response),
            Err(err) => {
                *client = None;
                Err(err.into())
            }
        }
    }

    /// Sends a command whose only answer is `Done`.
    pub fn command(&self, request: &Request) -> Result<(), UiError> {
        match self.call(request)? {
            Response::Done => Ok(()),
            other => Err(refused(other)),
        }
    }

    pub fn status(&self) -> AgentLink {
        match self.call(&Request::Status) {
            Ok(Response::Status(status)) => AgentLink::Running(status),
            Err(UiError::AgentStopped) => AgentLink::Stopped,
            _ => AgentLink::Mismatch,
        }
    }

    /// Starts the sibling watcher and waits until it answers.
    pub fn start_agent(&self) -> Result<(), UiError> {
        let exe = self
            .agent_exe
            .as_ref()
            .filter(|exe| exe.is_file())
            .ok_or_else(|| UiError::Failed("Vestigium.exe is not next to the window".into()))?;
        Command::new(exe)
            .spawn()
            .map_err(|err| UiError::Failed(err.to_string()))?;
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            match self.call(&Request::Status) {
                Ok(_) => return Ok(()),
                Err(UiError::AgentStopped) if Instant::now() < deadline => {
                    std::thread::sleep(START_POLL)
                }
                Err(UiError::AgentStopped) => return Err(UiError::StartTimeout),
                Err(err) => return Err(err),
            }
        }
    }

    /// Opens the pipe and checks both ends speak the same protocol.
    fn connect(&self) -> Result<PipeClient, UiError> {
        let exe = self.agent_exe.as_deref().ok_or(UiError::AgentStopped)?;
        let sid = current_user_sid()?;
        let mut client = PipeClient::connect(&pipe_name(&sid), exe)?;
        same_protocol(client.call(&Request::Hello {
            protocol: PROTOCOL_VERSION,
        })?)?;
        Ok(client)
    }
}

/// Accepts only a Hello in this window's protocol; any other answer is another version.
fn same_protocol(answer: Response) -> Result<(), UiError> {
    match answer {
        Response::Hello { protocol, .. } if protocol == PROTOCOL_VERSION => Ok(()),
        _ => Err(UiError::Mismatch),
    }
}

/// An answer other than the expected one, as an error.
pub fn refused(response: Response) -> UiError {
    match response {
        Response::Error { message, .. } => UiError::Refused(message),
        other => UiError::Failed(format!("unexpected answer: {other:?}")),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use vestigium_ipc::ErrorCode;

    use super::*;

    #[test]
    fn missing_pipe_means_the_watcher_is_stopped() {
        let missing = std::io::Error::from_raw_os_error(NO_PIPE);
        assert!(matches!(
            UiError::from(IpcError::Io(missing)),
            UiError::AgentStopped
        ));
        let found = PathBuf::from(r"C:\Other\Impostor.exe");
        assert!(matches!(
            UiError::from(IpcError::ServerMismatch { found }),
            UiError::Mismatch
        ));
    }

    #[test]
    fn an_older_watcher_is_another_version() {
        let refusal = Response::Error {
            code: ErrorCode::Protocol,
            message: format!("protocol {PROTOCOL_VERSION} is not supported; this agent speaks 2"),
        };
        assert!(matches!(same_protocol(refusal), Err(UiError::Mismatch)));
        let older = Response::Hello {
            protocol: 2,
            agent_version: "0.0.0".into(),
        };
        assert!(matches!(same_protocol(older), Err(UiError::Mismatch)));
        let same = Response::Hello {
            protocol: PROTOCOL_VERSION,
            agent_version: "0.0.0".into(),
        };
        assert!(same_protocol(same).is_ok());
    }

    #[test]
    fn errors_reach_the_window_with_a_kind() {
        let json = serde_json::to_value(UiError::Refused("records kept 30 days".into())).unwrap();
        assert_eq!(json["kind"], "refused");
        assert_eq!(json["message"], "the watcher refused: records kept 30 days");
    }

    #[test]
    fn an_agent_error_is_a_refusal() {
        let error = refused(Response::Error {
            code: ErrorCode::InvalidSetting,
            message: "samples kept 6 h".into(),
        });
        assert!(matches!(error, UiError::Refused(m) if m == "samples kept 6 h"));
    }

    /// Needs the debug watcher running: `cargo run -p vestigium-agent`.
    #[test]
    #[ignore]
    fn talks_to_a_running_watcher() {
        let built = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(r"..\..\..\target\debug\Vestigium.exe")
            .canonicalize()
            .unwrap();
        let exe = PathBuf::from(built.to_string_lossy().trim_start_matches(r"\\?\"));
        let link = Link::new(Some(exe));
        assert!(matches!(link.status(), AgentLink::Running(_)));

        let (_keep, stop) = std::sync::mpsc::channel();
        let mut sent = Vec::new();
        crate::live::run_live(
            |request| link.call(request),
            |message| {
                sent.push(message);
                sent.len() < 3
            },
            &stop,
            Duration::from_millis(1_000),
        );

        let crate::live::LiveMessage::Recent { points } = &sent[0] else {
            panic!("the stream did not open with the recent minutes");
        };
        assert!(!points.is_empty());
        assert!(
            points
                .windows(2)
                .all(|w| w[0].reading.ts_ms < w[1].reading.ts_ms)
        );
        assert!(matches!(
            sent[2],
            crate::live::LiveMessage::Now { point: Some(_), .. }
        ));
        assert!(matches!(
            link.call(&Request::Settings),
            Ok(Response::Settings { .. })
        ));
    }

    #[test]
    fn starting_needs_the_watcher_next_to_the_window() {
        let link = Link::new(Some(PathBuf::from(r"C:\Nowhere\Vestigium.exe")));
        assert!(matches!(link.start_agent(), Err(UiError::Failed(_))));
    }
}
