use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

use vestigium_ipc::{
    AgentStatus, IpcError, PROTOCOL_VERSION, PipeClient, PipeServer, Request, Response,
    current_user_sid, pipe_name,
};

fn unique_name() -> String {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    pipe_name(&format!("test-{}-{n}", std::process::id()))
}

fn answer(request: Request) -> Response {
    match request {
        Request::Hello { .. } => Response::Hello {
            protocol: PROTOCOL_VERSION,
            agent_version: "test".to_string(),
        },
        Request::Status => Response::Status(AgentStatus {
            paused: true,
            ..AgentStatus::default()
        }),
    }
}

fn start_server(name: &str) {
    let server = PipeServer::bind(name).unwrap();
    thread::spawn(move || server.serve(answer));
}

#[test]
fn pipe_name_is_under_the_pipe_namespace() {
    assert_eq!(pipe_name("S-1-5-21-1"), r"\\.\pipe\vestigium-S-1-5-21-1");
}

#[test]
fn current_user_sid_is_a_user_sid() {
    assert!(current_user_sid().unwrap().starts_with("S-1-5-"));
}

#[test]
fn second_bind_is_already_running() {
    let name = unique_name();
    let _first = PipeServer::bind(&name).unwrap();

    let second = PipeServer::bind(&name);

    assert!(
        matches!(second, Err(IpcError::AlreadyRunning)),
        "got {second:?}"
    );
}

#[test]
fn hello_and_status_over_the_pipe() {
    let name = unique_name();
    start_server(&name);
    let exe = std::env::current_exe().unwrap();

    let mut client = PipeClient::connect(&name, &exe).unwrap();

    assert_eq!(
        client
            .call(&Request::Hello {
                protocol: PROTOCOL_VERSION
            })
            .unwrap(),
        Response::Hello {
            protocol: PROTOCOL_VERSION,
            agent_version: "test".to_string()
        }
    );
    assert!(matches!(client.call(&Request::Status).unwrap(), Response::Status(s) if s.paused));
}

#[test]
fn client_rejects_unexpected_server() {
    let name = unique_name();
    start_server(&name);

    let result = PipeClient::connect(&name, Path::new(r"C:\Program Files\Other\other.exe"));

    assert!(matches!(result, Err(IpcError::ServerMismatch { .. })));
}

#[test]
fn server_survives_garbage_client() {
    let name = unique_name();
    start_server(&name);
    {
        let mut raw = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&name)
            .unwrap();
        raw.write_all(&[0xFF; 7]).unwrap();
    }

    let exe = std::env::current_exe().unwrap();
    let mut client = PipeClient::connect(&name, &exe).unwrap();

    assert!(matches!(
        client.call(&Request::Status).unwrap(),
        Response::Status(_)
    ));
}

#[test]
fn server_serves_clients_one_after_another() {
    let name = unique_name();
    start_server(&name);
    let exe = std::env::current_exe().unwrap();

    for _ in 0..3 {
        let mut client = PipeClient::connect(&name, &exe).unwrap();
        assert!(matches!(
            client.call(&Request::Status).unwrap(),
            Response::Status(_)
        ));
    }
}
