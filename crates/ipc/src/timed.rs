use std::io::{self, Read, Write};
use std::time::Duration;

use windows::Win32::Foundation::{
    CloseHandle, ERROR_IO_PENDING, ERROR_OPERATION_ABORTED, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, ResetEvent, WaitForSingleObject};
use windows::core::PCWSTR;

/// Read and write access to an overlapped pipe handle where every operation has a deadline.
pub(crate) struct TimedPipe {
    handle: HANDLE,
    event: HANDLE,
    read_timeout: Duration,
    write_timeout: Duration,
    next_read: Option<Duration>,
}

impl TimedPipe {
    /// Wraps a handle opened with `FILE_FLAG_OVERLAPPED`; the handle stays owned by the caller.
    pub(crate) fn new(
        handle: HANDLE,
        read_timeout: Duration,
        write_timeout: Duration,
    ) -> io::Result<TimedPipe> {
        // SAFETY: an unnamed manual-reset event with default security.
        let event = unsafe { CreateEventW(None, true, false, PCWSTR::null()).map_err(os_error)? };
        Ok(TimedPipe {
            handle,
            event,
            read_timeout,
            write_timeout,
            next_read: None,
        })
    }

    /// Gives the next read, and only that one, its own deadline.
    pub(crate) fn first_read_within(&mut self, timeout: Duration) {
        self.next_read = Some(timeout);
    }

    /// Runs one overlapped operation to its end or cancels it when `timeout` passes.
    fn run(
        &self,
        timeout: Option<Duration>,
        start: impl FnOnce(*mut OVERLAPPED) -> windows::core::Result<()>,
    ) -> io::Result<usize> {
        let mut overlapped = OVERLAPPED {
            hEvent: self.event,
            ..OVERLAPPED::default()
        };
        // SAFETY: the event is valid and no operation uses it yet.
        unsafe { ResetEvent(self.event).map_err(os_error)? };
        match start(&mut overlapped) {
            Ok(()) => {}
            Err(err) if err.code() == ERROR_IO_PENDING.to_hresult() => {
                let wait = timeout.map_or(INFINITE, |t| {
                    t.as_millis().min(u128::from(INFINITE - 1)) as u32
                });
                // SAFETY: the event is valid for the lifetime of `self`.
                let waited = unsafe { WaitForSingleObject(self.event, wait) };
                if waited != WAIT_OBJECT_0 {
                    debug_assert_eq!(waited, WAIT_TIMEOUT);
                    // SAFETY: cancels the one operation this call started on the handle.
                    let _ = unsafe { CancelIoEx(self.handle, Some(&raw const overlapped)) };
                }
            }
            Err(err) => return Err(os_error(err)),
        }
        let mut done = 0u32;
        // SAFETY: waits for the operation to finish or finish cancelling, so the buffer and
        // `overlapped` stay alive until the system is done with them.
        let result = unsafe { GetOverlappedResult(self.handle, &overlapped, &mut done, true) };
        match result {
            Ok(()) => Ok(done as usize),
            Err(err) if err.code() == ERROR_OPERATION_ABORTED.to_hresult() => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "the pipe did not answer in time",
            )),
            Err(err) => Err(os_error(err)),
        }
    }

    /// Waits for a client to connect to a server pipe handle, with no deadline.
    pub(crate) fn connect(handle: HANDLE) -> io::Result<()> {
        let pipe = TimedPipe::new(handle, Duration::ZERO, Duration::ZERO)?;
        pipe.run(None, |overlapped| {
            // SAFETY: `overlapped` outlives the operation; `run` waits for it.
            unsafe { windows::Win32::System::Pipes::ConnectNamedPipe(handle, Some(overlapped)) }
        })
        .map(|_| ())
    }
}

impl Read for TimedPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let timeout = self.next_read.take().unwrap_or(self.read_timeout);
        let handle = self.handle;
        self.run(Some(timeout), |overlapped| {
            // SAFETY: `buf` and `overlapped` stay alive until `run` has seen the operation end.
            unsafe { ReadFile(handle, Some(buf), None, Some(overlapped)) }
        })
    }
}

impl Write for TimedPipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let handle = self.handle;
        self.run(Some(self.write_timeout), |overlapped| {
            // SAFETY: `buf` and `overlapped` stay alive until `run` has seen the operation end.
            unsafe { WriteFile(handle, Some(buf), None, Some(overlapped)) }
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for TimedPipe {
    fn drop(&mut self) {
        // SAFETY: the event was created in `new` and is closed exactly once.
        let _ = unsafe { CloseHandle(self.event) };
    }
}

/// The Win32 error behind a `windows` error, so callers can compare `raw_os_error` with codes.
fn os_error(err: windows::core::Error) -> io::Error {
    let code = err.code().0;
    match code as u32 & 0xFFFF_0000 {
        0x8007_0000 => io::Error::from_raw_os_error(code & 0xFFFF),
        _ => io::Error::from_raw_os_error(code),
    }
}
