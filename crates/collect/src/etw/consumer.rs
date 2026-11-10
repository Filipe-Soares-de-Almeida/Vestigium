use std::thread::{self, JoinHandle};

use windows::Win32::Foundation::ERROR_SUCCESS;
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

/// What a consumer thread read: the sink, how many events reached it, and whether the session
/// opened and ended without an error.
#[derive(Debug)]
pub struct Consumed<S> {
    pub sink: S,
    pub events: u64,
    pub status: Result<(), String>,
}

/// The sink and the count of events its callback has handed it.
struct Counting<S> {
    sink: S,
    events: u64,
}

/// Consumes the real-time session `name` until it stops, then hands the sink back with what it saw.
pub fn spawn_consumer<S: EventSink + 'static>(
    name: &str,
    sink: S,
) -> Result<JoinHandle<Consumed<S>>, CollectError> {
    let name = name.to_string();
    Ok(thread::Builder::new()
        .name("etw consumer".into())
        .spawn(move || consume(&name, sink))?)
}

fn consume<S: EventSink>(name: &str, sink: S) -> Consumed<S> {
    let mut sink = Box::new(Counting { sink, events: 0 });
    let mut logger: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut logfile = EVENT_TRACE_LOGFILEW {
        LoggerName: PWSTR(logger.as_mut_ptr()),
        Context: (&mut *sink as *mut Counting<S>).cast(),
        ..Default::default()
    };
    logfile.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME
        | PROCESS_TRACE_MODE_EVENT_RECORD
        | PROCESS_TRACE_MODE_RAW_TIMESTAMP;
    logfile.Anonymous2.EventRecordCallback = Some(on_event::<S>);
    // SAFETY: logfile, the logger name and the sink outlive the trace, which ends before this returns.
    let trace = unsafe { OpenTraceW(&mut logfile) };
    let status = if trace.Value == u64::MAX {
        Err(format!(
            "the trace session could not be opened: {}",
            windows::core::Error::from_thread()
        ))
    } else {
        // SAFETY: trace is the handle OpenTraceW just returned; this blocks until the session stops.
        let ended = unsafe { ProcessTrace(&[trace], None, None) };
        // SAFETY: closes the handle opened above.
        let _ = unsafe { CloseTrace(trace) };
        if ended == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!(
                "the trace reader stopped with an error: {}",
                windows::core::Error::from(ended.to_hresult())
            ))
        }
    };
    let Counting { sink, events } = *sink;
    Consumed {
        sink,
        events,
        status,
    }
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
    // SAFETY: UserContext is the counted sink this consumer's thread owns, set in `consume`.
    let counting = unsafe { &mut *record.UserContext.cast::<Counting<S>>() };
    counting.events += 1;
    counting.sink.event(&event);
}
