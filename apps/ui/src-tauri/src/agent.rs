use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, PoisonError, TryLockError};
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
/// `ERROR_BROKEN_PIPE`, `ERROR_NO_DATA` and `ERROR_PIPE_NOT_CONNECTED`: the other end is gone.
const GONE: [i32; 3] = [109, 232, 233];

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
    #[error("the permission prompt was declined")]
    Declined,
    #[error("a setting now holds a value that Vestigium did not write")]
    ChangedSince,
}

impl From<IpcError> for UiError {
    fn from(err: IpcError) -> UiError {
        match err {
            IpcError::Io(io) if io.raw_os_error() == Some(NO_PIPE) || is_gone(&io) => {
                UiError::AgentStopped
            }
            IpcError::ServerMismatch { .. } => UiError::Mismatch,
            other => UiError::Ipc(other),
        }
    }
}

/// Whether the other end of the pipe has closed.
fn is_gone(err: &std::io::Error) -> bool {
    err.raw_os_error().is_some_and(|code| GONE.contains(&code))
        || err.kind() == std::io::ErrorKind::UnexpectedEof
}

/// Whether a connection error means the pipe was closed from the other end, as opposed to slow.
fn dead(err: &IpcError) -> bool {
    matches!(err, IpcError::Io(io) if is_gone(io))
}

/// Whether a request can be sent again after its answer was lost: it changes nothing, or sets a
/// value that sending twice leaves as it is.
fn may_replay(request: &Request) -> bool {
    match request {
        Request::Hello { .. }
        | Request::Status
        | Request::Now
        | Request::Recent
        | Request::ProgramRecent { .. }
        | Request::SetPaused { .. }
        | Request::SetSensitive { .. }
        | Request::Settings
        | Request::SetAutostart { .. } => true,
        Request::SetSettings(_)
        | Request::ClearHistory
        | Request::ImportResults
        | Request::ApplyUserTweak { .. } => false,
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
            UiError::Declined => "declined",
            UiError::ChangedSince => "changed_since",
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
    /// The pipe to connect to; the user's own unless a test names one.
    pipe: Option<String>,
    client: Mutex<Option<PipeClient>>,
}

impl Link {
    pub fn new(agent_exe: Option<PathBuf>) -> Link {
        Link {
            agent_exe,
            pipe: None,
            client: Mutex::new(None),
        }
    }

    /// A link to the pipe called `pipe`, for tests that run a watcher of their own.
    #[cfg(test)]
    pub(crate) fn on_pipe(agent_exe: Option<PathBuf>, pipe: &str) -> Link {
        Link {
            pipe: Some(pipe.to_string()),
            ..Link::new(agent_exe)
        }
    }

    /// `Vestigium.exe` next to this executable, the only program trusted to own the pipe.
    pub fn sibling_agent() -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        Some(exe.with_file_name("Vestigium.exe"))
    }

    /// Sends one request, connecting first if needed. A kept connection that turns out to be dead
    /// is replaced once; a request that may already have run is not sent again after its answer
    /// was lost. A failed call drops the connection.
    pub fn call(&self, request: &Request) -> Result<Response, UiError> {
        self.exchange(&mut lock(&self.client), request)
    }

    /// Like `call`, but gives up with `None` at once when another call holds the connection.
    pub fn try_call(&self, request: &Request) -> Option<Result<Response, UiError>> {
        let mut client = match self.client.try_lock() {
            Ok(client) => client,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return None,
        };
        Some(self.exchange(&mut client, request))
    }

    fn exchange(
        &self,
        client: &mut Option<PipeClient>,
        request: &Request,
    ) -> Result<Response, UiError> {
        let mut fresh = client.is_none();
        loop {
            let mut open = match client.take() {
                Some(open) => open,
                None => {
                    fresh = true;
                    self.connect()?
                }
            };
            if let Err(err) = open.send(request) {
                if !fresh && dead(&err) {
                    fresh = true;
                    continue;
                }
                return Err(err.into());
            }
            match open.receive(request.answer_deadline()) {
                Ok(response) => {
                    *client = Some(open);
                    return Ok(response);
                }
                Err(err) if !fresh && dead(&err) && may_replay(request) => fresh = true,
                Err(err) => return Err(err.into()),
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
        let pipe = match &self.pipe {
            Some(pipe) => pipe.clone(),
            None => pipe_name(&current_user_sid()?),
        };
        let mut client = PipeClient::connect(&pipe, exe)?;
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

/// A watcher that answers the Hello and then never replies, for tests of the window's side.
#[cfg(test)]
pub(crate) mod mute {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::mpsc::{self, Sender};

    use vestigium_ipc::{PROTOCOL_VERSION, PipeServer};

    use super::*;

    /// A link to a mute watcher; dropping the sender lets the watcher's handler go.
    pub(crate) fn link() -> (Link, Sender<()>) {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let name = pipe_name(&format!("mute-test-{}-{n}", std::process::id()));
        let (release, hold) = mpsc::channel::<()>();
        let server = PipeServer::bind(&name).unwrap();
        std::thread::spawn(move || {
            server.serve(move |request| match request {
                Request::Hello { .. } => Response::Hello {
                    protocol: PROTOCOL_VERSION,
                    agent_version: "test".into(),
                },
                _ => {
                    let _ = hold.recv();
                    Response::Done
                }
            })
        });
        let exe = std::env::current_exe().unwrap();
        (Link::on_pipe(Some(exe), &name), release)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use std::sync::mpsc;

    use vestigium_ipc::{ErrorCode, PipeServer};

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

    fn unique_pipe() -> String {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        pipe_name(&format!("link-test-{}-{n}", std::process::id()))
    }

    fn link_to(pipe: &str) -> Link {
        Link::on_pipe(Some(std::env::current_exe().unwrap()), pipe)
    }

    fn answer(request: Request) -> Response {
        match request {
            Request::Hello { .. } => Response::Hello {
                protocol: PROTOCOL_VERSION,
                agent_version: "test".into(),
            },
            Request::Status => Response::Status(AgentStatus::default()),
            _ => Response::Done,
        }
    }

    /// A watcher whose first pipe ends when a `doomed` request arrives and that opens a second one
    /// at once; `seen` counts the doomed requests either pipe received, and the channel says when
    /// the second pipe is ready.
    fn watcher_that_restarts(
        name: &str,
        doomed: fn(&Request) -> bool,
        seen: Arc<AtomicUsize>,
    ) -> mpsc::Receiver<()> {
        let (ready, restarted) = mpsc::channel();
        let name = name.to_string();
        let first = PipeServer::bind(&name).unwrap();
        std::thread::spawn(move || {
            let counted = seen.clone();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                first.serve(|request| {
                    if doomed(&request) {
                        counted.fetch_add(1, Ordering::SeqCst);
                        panic!("the first watcher ends");
                    }
                    answer(request)
                })
            }));
            drop(first);
            let second = PipeServer::bind(&name).unwrap();
            ready.send(()).unwrap();
            let _ = second.serve(|request| {
                if doomed(&request) {
                    seen.fetch_add(1, Ordering::SeqCst);
                }
                answer(request)
            });
        });
        restarted
    }

    #[test]
    fn a_connection_the_agent_dropped_is_retried_once() {
        let name = unique_pipe();
        let restarted = watcher_that_restarts(
            &name,
            |request| matches!(request, Request::SetSensitive { .. }),
            Arc::new(AtomicUsize::new(0)),
        );
        let link = link_to(&name);
        assert!(matches!(link.status(), AgentLink::Running(_)));
        let mut dead = lock(&link.client).take().unwrap();
        dead.call(&Request::SetSensitive { on: true }).unwrap_err();
        restarted.recv().unwrap();
        *lock(&link.client) = Some(dead);

        assert!(matches!(link.status(), AgentLink::Running(_)));
    }

    #[test]
    fn a_request_that_may_have_run_is_not_replayed_after_a_read_failure() {
        let name = unique_pipe();
        let seen = Arc::new(AtomicUsize::new(0));
        let restarted = watcher_that_restarts(
            &name,
            |request| matches!(request, Request::ClearHistory),
            seen.clone(),
        );
        let link = link_to(&name);
        assert!(matches!(link.status(), AgentLink::Running(_)));

        let result = link.call(&Request::ClearHistory);

        assert!(result.is_err());
        restarted.recv().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(seen.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn only_requests_that_change_nothing_twice_are_replayed() {
        assert!(may_replay(&Request::Status));
        assert!(may_replay(&Request::Recent));
        assert!(may_replay(&Request::SetPaused {
            paused: true,
            until_ms: None
        }));
        assert!(!may_replay(&Request::ClearHistory));
        assert!(!may_replay(&Request::ImportResults));
        assert!(!may_replay(&Request::SetSettings(Settings::default())));
    }

    #[test]
    fn a_mute_agent_times_out_instead_of_blocking() {
        let (link, release) = mute::link();
        let started = Instant::now();

        let result = link.call(&Request::Status);

        assert!(matches!(
            &result,
            Err(UiError::Ipc(IpcError::Io(err))) if err.kind() == std::io::ErrorKind::TimedOut
        ));
        assert!(started.elapsed() < Duration::from_secs(7));
        drop(release);
    }

    #[test]
    fn a_slow_request_that_changes_history_is_answered_not_dropped() {
        let name = unique_pipe();
        let server = PipeServer::bind(&name).unwrap();
        std::thread::spawn(move || {
            server.serve(|request| match request {
                Request::ImportResults => {
                    std::thread::sleep(Duration::from_secs(4));
                    Response::Done
                }
                other => answer(other),
            })
        });
        let link = link_to(&name);

        let result = link.call(&Request::ImportResults);

        assert!(matches!(result, Ok(Response::Done)), "got {result:?}");
        assert!(matches!(link.status(), AgentLink::Running(_)));
        assert!(
            lock(&link.client).is_some(),
            "the connection is kept after a slow answer"
        );
    }

    #[test]
    fn a_busy_link_is_not_waited_for() {
        let (link, release) = mute::link();
        let link = Arc::new(link);
        let slow = link.clone();
        let busy = std::thread::spawn(move || slow.call(&Request::Status));
        std::thread::sleep(Duration::from_millis(500));
        let started = Instant::now();

        let result = link.try_call(&Request::Status);

        assert!(result.is_none());
        assert!(started.elapsed() < Duration::from_millis(200));
        assert!(busy.join().unwrap().is_err());
        drop(release);
    }

    #[test]
    fn a_broken_connection_means_the_watcher_stopped() {
        for code in GONE {
            let gone = std::io::Error::from_raw_os_error(code);
            assert!(matches!(
                UiError::from(IpcError::Io(gone)),
                UiError::AgentStopped
            ));
        }
        let slow = std::io::Error::from(std::io::ErrorKind::TimedOut);
        assert!(matches!(UiError::from(IpcError::Io(slow)), UiError::Ipc(_)));
    }

    #[test]
    fn an_older_watcher_is_another_version() {
        let refusal = Response::Error {
            code: ErrorCode::Protocol,
            message: format!(
                "protocol {PROTOCOL_VERSION} is not supported; this agent speaks {}",
                PROTOCOL_VERSION - 1
            ),
        };
        assert!(matches!(same_protocol(refusal), Err(UiError::Mismatch)));
        let older = Response::Hello {
            protocol: PROTOCOL_VERSION - 1,
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
