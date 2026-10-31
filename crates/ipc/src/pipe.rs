use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_NO_DATA, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, HANDLE,
};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeServerProcessId,
    PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT, WaitNamedPipeW,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::{HSTRING, PWSTR};

use crate::security::{SecurityDescriptor, current_user_sid, user_only_sddl};
use crate::{IpcError, Request, Response, read_frame, write_frame};

const BUFFER_BYTES: u32 = 64 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Full pipe path for `suffix`; the agent uses the user's SID.
pub fn pipe_name(suffix: &str) -> String {
    format!(r"\\.\pipe\vestigium-{suffix}")
}

/// The agent's end: one instance, local clients of the same user only, one client at a time.
#[derive(Debug)]
pub struct PipeServer {
    pipe: File,
}

impl PipeServer {
    /// Creates the pipe; fails with `AlreadyRunning` when it already exists.
    pub fn bind(name: &str) -> Result<PipeServer, IpcError> {
        let descriptor = SecurityDescriptor::from_sddl(&user_only_sddl(&current_user_sid()?))?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.as_ptr(),
            bInheritHandle: false.into(),
        };
        // SAFETY: the name and the security attributes outlive the call.
        let handle = unsafe {
            CreateNamedPipeW(
                &HSTRING::from(name),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                BUFFER_BYTES,
                BUFFER_BYTES,
                0,
                Some(&attributes),
            )
        };
        if handle.is_invalid() {
            let err = io::Error::last_os_error();
            // Busy: the one allowed instance exists. Access denied: someone else created the first.
            let taken = [ERROR_PIPE_BUSY.0, ERROR_ACCESS_DENIED.0].map(|code| Some(code as i32));
            if taken.contains(&err.raw_os_error()) {
                return Err(IpcError::AlreadyRunning);
            }
            return Err(err.into());
        }
        // SAFETY: `handle` is a valid pipe handle that the File now owns.
        let pipe = unsafe { File::from_raw_handle(handle.0) };
        Ok(PipeServer { pipe })
    }

    /// Answers clients forever; returns only when the pipe itself fails.
    pub fn serve(&self, mut handler: impl FnMut(Request) -> Response) -> Result<(), IpcError> {
        let handle = HANDLE(self.pipe.as_raw_handle());
        loop {
            // SAFETY: `handle` stays valid while `self.pipe` lives; synchronous connect.
            match unsafe { ConnectNamedPipe(handle, None) } {
                Ok(()) => self.answer(&mut handler),
                Err(err) if err.code() == ERROR_PIPE_CONNECTED.to_hresult() => {
                    self.answer(&mut handler)
                }
                // A client that connected and left before this call; drop it and listen again.
                Err(err) if err.code() == ERROR_NO_DATA.to_hresult() => {}
                Err(err) => return Err(err.into()),
            }
            // SAFETY: drops the current client so the next one can connect.
            unsafe { DisconnectNamedPipe(handle)? };
        }
    }

    /// Answers one client until it disconnects or sends something malformed.
    fn answer(&self, handler: &mut impl FnMut(Request) -> Response) {
        let mut pipe = &self.pipe;
        loop {
            let response = match read_frame::<_, Request>(&mut pipe) {
                Ok(request) => handler(request),
                Err(IpcError::Io(_)) => return,
                Err(err) => {
                    let _ = write_frame(
                        &mut pipe,
                        &Response::Error {
                            message: err.to_string(),
                        },
                    );
                    return;
                }
            };
            if write_frame(&mut pipe, &response).is_err() {
                return;
            }
        }
    }
}

/// The interface's end, connected only after checking which program owns the pipe.
#[derive(Debug)]
pub struct PipeClient {
    pipe: File,
}

impl PipeClient {
    pub fn connect(name: &str, expected_server: &Path) -> Result<PipeClient, IpcError> {
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        let pipe = loop {
            let opened = OpenOptions::new()
                .read(true)
                .write(true)
                .security_qos_flags(SECURITY_IDENTIFICATION.0)
                .open(name);
            match opened {
                Ok(pipe) => break pipe,
                Err(err)
                    if err.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32)
                        && Instant::now() < deadline =>
                {
                    // SAFETY: the name outlives the call; the result only shortens the wait.
                    let _ = unsafe { WaitNamedPipeW(&HSTRING::from(name), 250) };
                }
                Err(err) => return Err(err.into()),
            }
        };
        let found = server_image(&pipe)?;
        if !same_path(&found, expected_server) {
            return Err(IpcError::ServerMismatch { found });
        }
        Ok(PipeClient { pipe })
    }

    pub fn call(&mut self, request: &Request) -> Result<Response, IpcError> {
        write_frame(&mut &self.pipe, request)?;
        read_frame(&mut &self.pipe)
    }
}

/// Full path of the program that created the pipe.
fn server_image(pipe: &File) -> Result<PathBuf, IpcError> {
    let mut pid = 0u32;
    // SAFETY: `pipe` is an open client end of a named pipe.
    unsafe { GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pid)? };
    // SAFETY: plain query access to a process by id; the handle is closed below.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)? };
    let mut buf = vec![0u16; 32 * 1024];
    let mut len = buf.len() as u32;
    // SAFETY: `buf` holds `len` UTF-16 units; the call writes at most that many.
    let queried = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    // SAFETY: `process` was opened above and is closed exactly once.
    unsafe { CloseHandle(process)? };
    queried?;
    Ok(PathBuf::from(String::from_utf16_lossy(
        &buf[..len as usize],
    )))
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}
