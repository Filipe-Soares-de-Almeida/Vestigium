use std::mem::offset_of;

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Diagnostics::Etw::{
    CONTROLTRACE_HANDLE, ControlTraceW, EVENT_TRACE_CONTROL, EVENT_TRACE_CONTROL_QUERY,
    EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_FLAG, EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE,
    EVENT_TRACE_SYSTEM_LOGGER_MODE, StartTraceW, WNODE_FLAG_TRACED_GUID,
};
use windows::core::{GUID, PCWSTR};

use crate::CollectError;

/// The deep capture's kernel session; one name, so a leftover one is found and stopped.
pub const DEEP_CAPTURE_SESSION: &str = "Vestigium Deep Capture";
/// The named mutex that exists while a helper runs a deep capture.
pub const DEEP_CAPTURE_LOCK: &str = r"Global\Vestigium.DeepCapture";
pub const DEEP_CAPTURE_GUID: GUID = GUID::from_u128(0x5e0c7a1d_4b2f_4c8e_9a63_d1f27b4e08c5);

/// 64 KB buffers, all allocated when the session starts.
const BUFFER_KB: u32 = 64;
const MIN_BUFFERS: u32 = 64;
const MAX_BUFFERS: u32 = 128;

#[repr(C)]
struct Properties {
    props: EVENT_TRACE_PROPERTIES,
    name: [u16; 64],
    file: [u16; 260],
}

/// What a session lost so far; a capture with losses is not complete.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionStats {
    pub events_lost: u32,
    pub realtime_buffers_lost: u32,
}

/// A real-time system logger session under its own name and GUID; stopped on drop.
#[derive(Debug)]
pub struct KernelSession {
    handle: CONTROLTRACE_HANDLE,
    name: String,
    stopped: bool,
}

impl KernelSession {
    /// Starts `name` with the kernel event groups in `flags`, stopping a leftover session of that name first.
    pub fn start(
        name: &str,
        guid: GUID,
        flags: EVENT_TRACE_FLAG,
    ) -> Result<KernelSession, CollectError> {
        let wide = wide(name);
        let mut handle = CONTROLTRACE_HANDLE::default();
        let mut props = properties(guid, flags);
        // SAFETY: props is a zeroed, sized EVENT_TRACE_PROPERTIES followed by its name and file buffers.
        let mut status =
            unsafe { StartTraceW(&mut handle, PCWSTR(wide.as_ptr()), &mut props.props) };
        if status == ERROR_ALREADY_EXISTS {
            let _ = control(
                CONTROLTRACE_HANDLE::default(),
                name,
                EVENT_TRACE_CONTROL_STOP,
            );
            props = properties(guid, flags);
            // SAFETY: as above, with fresh properties.
            status = unsafe { StartTraceW(&mut handle, PCWSTR(wide.as_ptr()), &mut props.props) };
        }
        win32(status)?;
        Ok(KernelSession {
            handle,
            name: name.to_string(),
            stopped: false,
        })
    }

    /// Losses so far, while the session runs.
    pub fn query(&self) -> Result<SessionStats, CollectError> {
        control(self.handle, &self.name, EVENT_TRACE_CONTROL_QUERY)
    }

    /// Stops the session, which ends its consumer, and returns the final losses.
    pub fn stop(mut self) -> Result<SessionStats, CollectError> {
        self.stopped = true;
        control(self.handle, &self.name, EVENT_TRACE_CONTROL_STOP)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for KernelSession {
    fn drop(&mut self) {
        if !self.stopped {
            let _ = control(self.handle, &self.name, EVENT_TRACE_CONTROL_STOP);
        }
    }
}

/// Whether a session called `name` exists, asked without elevation.
pub fn session_exists(name: &str) -> bool {
    control(
        CONTROLTRACE_HANDLE::default(),
        name,
        EVENT_TRACE_CONTROL_QUERY,
    )
    .is_ok()
}

/// Stops the session called `name`; `false` when there is none or the caller may not stop it.
pub fn stop_session(name: &str) -> bool {
    control(
        CONTROLTRACE_HANDLE::default(),
        name,
        EVENT_TRACE_CONTROL_STOP,
    )
    .is_ok()
}

fn properties(guid: GUID, flags: EVENT_TRACE_FLAG) -> Box<Properties> {
    // SAFETY: Properties is plain data; all-zero is its documented starting state.
    let mut p: Box<Properties> = Box::new(unsafe { std::mem::zeroed() });
    p.props.Wnode.BufferSize = size_of::<Properties>() as u32;
    p.props.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
    p.props.Wnode.Guid = guid;
    // QPC timestamps, the clock of the DPC and ISR entry times.
    p.props.Wnode.ClientContext = 1;
    p.props.BufferSize = BUFFER_KB;
    p.props.MinimumBuffers = MIN_BUFFERS;
    p.props.MaximumBuffers = MAX_BUFFERS;
    p.props.FlushTimer = 1;
    p.props.LogFileMode = EVENT_TRACE_SYSTEM_LOGGER_MODE | EVENT_TRACE_REAL_TIME_MODE;
    p.props.EnableFlags = flags;
    p.props.LoggerNameOffset = offset_of!(Properties, name) as u32;
    p
}

fn control(
    handle: CONTROLTRACE_HANDLE,
    name: &str,
    code: EVENT_TRACE_CONTROL,
) -> Result<SessionStats, CollectError> {
    let wide = wide(name);
    let mut props = properties(GUID::zeroed(), EVENT_TRACE_FLAG(0));
    // SAFETY: props is sized for the name and file buffers that follow it.
    let status = unsafe { ControlTraceW(handle, PCWSTR(wide.as_ptr()), &mut props.props, code) };
    win32(status)?;
    Ok(SessionStats {
        events_lost: props.props.EventsLost,
        realtime_buffers_lost: props.props.RealTimeBuffersLost,
    })
}

fn win32(status: WIN32_ERROR) -> Result<(), CollectError> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(windows::core::Error::from(status.to_hresult()).into())
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
