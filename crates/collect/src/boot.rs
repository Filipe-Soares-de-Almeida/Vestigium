use vestigium_core::health::BootReport;
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EVT_SYSTEM_PROPERTY_ID, EVT_VARIANT, EvtClose, EvtCreateRenderContext, EvtNext,
    EvtQuery, EvtQueryChannelPath, EvtQueryForwardDirection, EvtRenderContextSystem,
    EvtRenderEventValues, EvtRenderEventXml, EvtSystemEventID, EvtSystemTimeCreated,
};
use windows::core::HSTRING;

use crate::CollectError;
use crate::events::{FILETIME_TO_UNIX_MS, event_data, render, render_text};

/// Windows' boot performance log, which only an elevated process may read.
pub const BOOT_LOG: &str = "Microsoft-Windows-Diagnostics-Performance/Operational";
const BATCH: usize = 32;
const DAY_MS: u64 = 24 * 60 * 60_000;

/// An event log handle, closed when dropped.
pub(crate) struct Handle(pub(crate) EVT_HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from the event log API and is closed once, here.
        let _ = unsafe { EvtClose(self.0) };
    }
}

/// The boots Windows measured and the steps that slowed them over the last `days`; it needs
/// elevation, and reads only those ids and that span instead of the whole log.
pub fn boot_report(days: u32) -> Result<BootReport, CollectError> {
    let query = format!(
        "*[System[(EventID>=100 and EventID<=110) and TimeCreated[timediff(@SystemTime) <= {}]]]",
        u64::from(days) * DAY_MS
    );
    // SAFETY: a forward query over one channel; closed by `Handle`.
    let results = Handle(unsafe {
        EvtQuery(
            None,
            &HSTRING::from(BOOT_LOG),
            &HSTRING::from(query),
            EvtQueryChannelPath.0 | EvtQueryForwardDirection.0,
        )?
    });
    // SAFETY: a context for the system properties; closed by `Handle`.
    let context = Handle(unsafe { EvtCreateRenderContext(None, EvtRenderContextSystem.0)? });
    let mut report = BootReport::default();
    read_events(
        // SAFETY: `batch` receives up to BATCH event handles, each closed by `Handle` below.
        |batch| unsafe {
            let mut returned = 0u32;
            EvtNext(results.0, batch, u32::MAX, 0, &mut returned)?;
            Ok(returned)
        },
        |raw| {
            let event = Handle(EVT_HANDLE(raw));
            let read = id_and_time(context.0, event.0)
                .and_then(|at| Some((at, render_text(event.0, EvtRenderEventXml)?)));
            match read {
                Some(((id, ts_ms), xml)) => {
                    report.add(id, ts_ms, &event_data(&xml));
                }
                None => report.skipped += 1,
            }
        },
    )?;
    Ok(report)
}

/// Hands every event `next` delivers to `each`, until the list ends; any other error from `next`
/// is returned, so a failed read is not taken for a log with nothing in it.
fn read_events(
    mut next: impl FnMut(&mut [isize]) -> Result<u32, windows::core::Error>,
    mut each: impl FnMut(isize),
) -> Result<(), CollectError> {
    loop {
        let mut batch = [0isize; BATCH];
        match next(&mut batch) {
            Ok(returned) => {
                for raw in &batch[..(returned as usize).min(BATCH)] {
                    each(*raw);
                }
            }
            Err(err) if err.code() == ERROR_NO_MORE_ITEMS.to_hresult() => return Ok(()),
            Err(err) => return Err(err.into()),
        }
    }
}

/// The event's id and when it was logged, in Unix epoch milliseconds.
fn id_and_time(context: EVT_HANDLE, event: EVT_HANDLE) -> Option<(u16, i64)> {
    let (values, count) = render(Some(context), event, EvtRenderEventValues)?;
    // SAFETY: rendering system values fills the buffer with `count` variants, aligned by the u64 buffer.
    let variants = unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<EVT_VARIANT>(), count as usize)
    };
    let get = |id: EVT_SYSTEM_PROPERTY_ID| variants.get(usize::try_from(id.0).ok()?);
    // SAFETY: the system context renders the event id as a u16 and the time as a FILETIME.
    let (id, created) = unsafe {
        (
            get(EvtSystemEventID)?.Anonymous.UInt16Val,
            get(EvtSystemTimeCreated)?.Anonymous.FileTimeVal,
        )
    };
    Some((
        id,
        i64::try_from(created / 10_000).ok()? - FILETIME_TO_UNIX_MS,
    ))
}

#[cfg(test)]
mod tests {
    use windows::Win32::Foundation::ERROR_INVALID_HANDLE;

    use super::*;

    fn fails(code: windows::Win32::Foundation::WIN32_ERROR) -> windows::core::Error {
        windows::core::Error::from(code.to_hresult())
    }

    #[test]
    fn a_read_error_in_the_middle_is_an_error_not_an_empty_report() {
        let mut calls = 0;
        let mut seen = Vec::new();
        let result = read_events(
            |batch| {
                calls += 1;
                if calls == 1 {
                    batch[0] = 7;
                    Ok(1)
                } else {
                    Err(fails(ERROR_INVALID_HANDLE))
                }
            },
            |raw| seen.push(raw),
        );
        assert!(result.is_err());
        assert_eq!(seen, [7]);
    }

    #[test]
    fn the_end_of_the_list_is_not_an_error() {
        let mut calls = 0;
        let mut seen = Vec::new();
        let result = read_events(
            |batch| {
                calls += 1;
                if calls == 1 {
                    batch[..2].copy_from_slice(&[3, 4]);
                    Ok(2)
                } else {
                    Err(fails(ERROR_NO_MORE_ITEMS))
                }
            },
            |raw| seen.push(raw),
        );
        assert!(result.is_ok());
        assert_eq!(seen, [3, 4]);
    }

    #[test]
    fn a_log_with_no_events_is_an_empty_list() {
        assert!(read_events(|_| Err(fails(ERROR_NO_MORE_ITEMS)), |_| {}).is_ok());
    }
}
