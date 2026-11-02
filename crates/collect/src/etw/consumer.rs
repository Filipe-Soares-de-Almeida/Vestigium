use std::thread::{self, JoinHandle};

use windows::Win32::System::Diagnostics::Etw::{
    CloseTrace, EVENT_RECORD, EVENT_TRACE_LOGFILEW, OpenTraceW, PROCESS_TRACE_MODE_EVENT_RECORD,
    PROCESS_TRACE_MODE_RAW_TIMESTAMP, PROCESS_TRACE_MODE_REAL_TIME, ProcessTrace,
};
use windows::core::PWSTR;

use super::RawEvent;
use crate::CollectError;

/// Receives every event of a session, on the consumer's own thread.
pub trait EventSink: Send {
    fn event(&mut self, event: &RawEvent<'_>);
}

/// Consumes the real-time session `name` until it stops, then hands the sink back.
pub fn spawn_consumer<S: EventSink + 'static>(
    name: &str,
    sink: S,
) -> Result<JoinHandle<S>, CollectError> {
    let name = name.to_string();
    Ok(thread::Builder::new()
        .name("etw consumer".into())
        .spawn(move || consume(&name, sink))?)
}

fn consume<S: EventSink>(name: &str, sink: S) -> S {
    let mut sink = Box::new(sink);
    let mut logger: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut logfile = EVENT_TRACE_LOGFILEW {
        LoggerName: PWSTR(logger.as_mut_ptr()),
        Context: (&mut *sink as *mut S).cast(),
        ..Default::default()
    };
    logfile.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME
        | PROCESS_TRACE_MODE_EVENT_RECORD
        | PROCESS_TRACE_MODE_RAW_TIMESTAMP;
    logfile.Anonymous2.EventRecordCallback = Some(on_event::<S>);
    // SAFETY: logfile, the logger name and the sink outlive the trace, which ends before this returns.
    let trace = unsafe { OpenTraceW(&mut logfile) };
    if trace.Value != u64::MAX {
        // SAFETY: trace is the handle OpenTraceW just returned; this blocks until the session stops.
        let _ = unsafe { ProcessTrace(&[trace], None, None) };
        // SAFETY: closes the handle opened above.
        let _ = unsafe { CloseTrace(trace) };
    }
    *sink
}

unsafe extern "system" fn on_event<S: EventSink>(record: *mut EVENT_RECORD) {
    // SAFETY: ETW passes a valid record for the duration of the callback.
    let record = unsafe { &*record };
    let data: &[u8] = if record.UserData.is_null() {
        &[]
    } else {
        // SAFETY: UserData points to UserDataLength bytes owned by ETW during the callback.
        unsafe {
            std::slice::from_raw_parts(
                record.UserData as *const u8,
                usize::from(record.UserDataLength),
            )
        }
    };
    // SAFETY: the processor number is the documented member of the buffer context union.
    let cpu = unsafe { record.BufferContext.Anonymous.Anonymous.ProcessorNumber };
    let header = &record.EventHeader;
    let event = RawEvent {
        provider: header.ProviderId,
        opcode: header.EventDescriptor.Opcode,
        version: header.EventDescriptor.Version,
        cpu: u16::from(cpu),
        timestamp: header.TimeStamp,
        data,
    };
    // SAFETY: UserContext is the sink this consumer's thread owns, set in `consume`.
    let sink = unsafe { &mut *record.UserContext.cast::<S>() };
    sink.event(&event);
}
