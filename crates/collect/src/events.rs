use std::ffi::c_void;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use vestigium_core::{Channel, EventMeaning, WinEvent, classify, query_xml};
use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EVT_RENDER_FLAGS, EVT_SUBSCRIBE_NOTIFY_ACTION, EVT_SYSTEM_PROPERTY_ID, EVT_VARIANT,
    EvtClose, EvtCreateBookmark, EvtCreateRenderContext, EvtRender, EvtRenderBookmark,
    EvtRenderContextSystem, EvtRenderEventValues, EvtRenderEventXml, EvtSubscribe,
    EvtSubscribeActionDeliver, EvtSubscribeStartAfterBookmark, EvtSubscribeStartAtOldestRecord,
    EvtSystemChannel, EvtSystemEventID, EvtSystemEventRecordId, EvtSystemLevel,
    EvtSystemProviderName, EvtSystemTimeCreated, EvtUpdateBookmark, EvtVarTypeString,
};
use windows::core::{HSTRING, PCWSTR};

use crate::CollectError;

/// Largest event data kept, in bytes.
const MAX_DATA: usize = 4096;
/// Milliseconds from the FILETIME epoch (1601) to the Unix epoch.
pub(crate) const FILETIME_TO_UNIX_MS: i64 = 11_644_473_600_000;

/// What an event means when it is worth keeping; `classify` for the curated events.
pub type Classifier = fn(Channel, &str, u16) -> Option<EventMeaning>;

/// One kept event, with the bookmark that resumes right after it.
#[derive(Clone, Debug, PartialEq)]
pub struct Delivered {
    pub event: WinEvent,
    pub bookmark_xml: String,
}

/// What the callback needs; owned by the subscription and freed after it closes.
struct Context {
    tx: Sender<Delivered>,
    meaning_of: Classifier,
    render: EVT_HANDLE,
    bookmark: Mutex<EVT_HANDLE>,
}

impl Drop for Context {
    fn drop(&mut self) {
        let bookmark = *self.bookmark.lock().unwrap_or_else(PoisonError::into_inner);
        // SAFETY: both handles were created for this context and are closed exactly once.
        unsafe {
            let _ = EvtClose(self.render);
            let _ = EvtClose(bookmark);
        }
    }
}

/// A push subscription to Windows events; each kept one arrives on the channel given to `start`.
#[derive(Debug)]
pub struct EventSubscription {
    subscription: EVT_HANDLE,
    context: *mut Context,
}

// SAFETY: the handles belong to the Event Log service; only the callback and drop touch the context.
unsafe impl Send for EventSubscription {}

impl EventSubscription {
    /// Curated events after `bookmark_xml`, or from the last `backfill` when there is none yet.
    pub fn start(
        bookmark_xml: Option<&str>,
        backfill: Duration,
        tx: Sender<Delivered>,
    ) -> Result<EventSubscription, CollectError> {
        let query = query_xml(bookmark_xml.is_none().then_some(backfill));
        EventSubscription::start_with_query(&query, bookmark_xml, classify, tx)
    }

    /// Events matching a structured `query`, kept when `meaning_of` names them.
    pub fn start_with_query(
        query: &str,
        bookmark_xml: Option<&str>,
        meaning_of: Classifier,
        tx: Sender<Delivered>,
    ) -> Result<EventSubscription, CollectError> {
        // SAFETY: a context for the system properties, closed with `Context`.
        let render = unsafe { EvtCreateRenderContext(None, EvtRenderContextSystem.0)? };
        // SAFETY: parses the bookmark, or creates an empty one; closed with `Context`.
        let bookmark = unsafe {
            match bookmark_xml {
                Some(xml) => EvtCreateBookmark(&HSTRING::from(xml)),
                None => EvtCreateBookmark(PCWSTR::null()),
            }
        };
        let bookmark = match bookmark {
            Ok(bookmark) => bookmark,
            Err(err) => {
                // SAFETY: closes the render context created above, once.
                let _ = unsafe { EvtClose(render) };
                return Err(err.into());
            }
        };
        let context = Box::into_raw(Box::new(Context {
            tx,
            meaning_of,
            render,
            bookmark: Mutex::new(bookmark),
        }));
        let flags = if bookmark_xml.is_some() {
            EvtSubscribeStartAfterBookmark
        } else {
            EvtSubscribeStartAtOldestRecord
        };
        // SAFETY: the context stays alive until drop has closed the subscription.
        let subscribed = unsafe {
            EvtSubscribe(
                None,
                None,
                PCWSTR::null(),
                &HSTRING::from(query),
                bookmark_xml.map(|_| bookmark),
                Some(context.cast_const().cast::<c_void>()),
                Some(on_event),
                flags.0,
            )
        };
        match subscribed {
            Ok(subscription) => Ok(EventSubscription {
                subscription,
                context,
            }),
            Err(err) => {
                // SAFETY: no subscription holds the context, which came from Box::into_raw above.
                drop(unsafe { Box::from_raw(context) });
                Err(err.into())
            }
        }
    }
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        // SAFETY: closing the subscription waits for a callback in progress, so nothing uses the context after.
        let _ = unsafe { EvtClose(self.subscription) };
        // SAFETY: the context came from Box::into_raw in `start_with_query` and is freed once.
        drop(unsafe { Box::from_raw(self.context) });
    }
}

unsafe extern "system" fn on_event(
    action: EVT_SUBSCRIBE_NOTIFY_ACTION,
    context: *const c_void,
    event: EVT_HANDLE,
) -> u32 {
    if action != EvtSubscribeActionDeliver {
        return 0;
    }
    // SAFETY: the subscription hands back the context it was created with, alive until it closes.
    let context = unsafe { &*context.cast::<Context>() };
    if let Some(delivered) = deliver(context, event) {
        let _ = context.tx.send(delivered);
    }
    0
}

/// Reads one event, keeps it when it is curated, and moves the bookmark past it.
fn deliver(context: &Context, event: EVT_HANDLE) -> Option<Delivered> {
    let (values, count) = render(Some(context.render), event, EvtRenderEventValues)?;
    // SAFETY: rendering system values fills the buffer with `count` variants, aligned by the u64 buffer.
    let variants = unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<EVT_VARIANT>(), count as usize)
    };
    let get = |id: EVT_SYSTEM_PROPERTY_ID| variants.get(usize::try_from(id.0).ok()?);
    let channel = Channel::from_path(&text_of(get(EvtSystemChannel)?)?)?;
    let provider = text_of(get(EvtSystemProviderName)?)?;
    // SAFETY: the system context renders the event id as a u16, the level as a byte and the rest as u64.
    let (id, level, created, record_id) = unsafe {
        (
            get(EvtSystemEventID)?.Anonymous.UInt16Val,
            get(EvtSystemLevel)?.Anonymous.ByteVal,
            get(EvtSystemTimeCreated)?.Anonymous.FileTimeVal,
            get(EvtSystemEventRecordId)?.Anonymous.UInt64Val,
        )
    };
    let meaning = (context.meaning_of)(channel, &provider, id)?;
    let data_xml = render_text(event, EvtRenderEventXml)
        .map(|xml| event_data(&xml))
        .unwrap_or_default();

    let bookmark = *context
        .bookmark
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    // SAFETY: the bookmark belongs to this subscription and `event` is the event being delivered.
    unsafe { EvtUpdateBookmark(bookmark, event) }.ok()?;
    let bookmark_xml = render_text(bookmark, EvtRenderBookmark)?;

    Some(Delivered {
        event: WinEvent {
            ts_ms: i64::try_from(created / 10_000).ok()? - FILETIME_TO_UNIX_MS,
            channel,
            provider,
            id,
            record_id,
            level,
            meaning,
            data_xml,
        },
        bookmark_xml,
    })
}

/// Renders `fragment` into a u64 buffer, so variants inside it are aligned; returns it and the value count.
pub(crate) fn render(
    context: Option<EVT_HANDLE>,
    fragment: EVT_HANDLE,
    flags: EVT_RENDER_FLAGS,
) -> Option<(Vec<u64>, u32)> {
    let (mut used, mut count) = (0u32, 0u32);
    // SAFETY: a size query with no buffer.
    let sized = unsafe { EvtRender(context, fragment, flags.0, 0, None, &mut used, &mut count) };
    match sized {
        Err(err) if err.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => {}
        _ => return None,
    }
    let mut buf = vec![0u64; (used as usize).div_ceil(8)];
    let bytes = u32::try_from(buf.len() * 8).ok()?;
    // SAFETY: `buf` holds `bytes` writable bytes.
    unsafe {
        EvtRender(
            context,
            fragment,
            flags.0,
            bytes,
            Some(buf.as_mut_ptr().cast::<c_void>()),
            &mut used,
            &mut count,
        )
    }
    .ok()?;
    Some((buf, count))
}

/// Renders `fragment` as text, such as an event's XML or a bookmark.
pub(crate) fn render_text(fragment: EVT_HANDLE, flags: EVT_RENDER_FLAGS) -> Option<String> {
    let (buf, _) = render(None, fragment, flags)?;
    let units: Vec<u16> = buf
        .iter()
        .flat_map(|word| {
            let bytes = word.to_le_bytes();
            [0, 2, 4, 6].map(|at| u16::from_le_bytes([bytes[at], bytes[at + 1]]))
        })
        .take_while(|&unit| unit != 0)
        .collect();
    Some(String::from_utf16_lossy(&units))
}

fn text_of(variant: &EVT_VARIANT) -> Option<String> {
    if variant.Type != EvtVarTypeString.0 as u32 {
        return None;
    }
    // SAFETY: a string variant holds a NUL-terminated string inside the rendered buffer.
    unsafe {
        let text = variant.Anonymous.StringVal;
        (!text.is_null()).then(|| text.to_string().ok()).flatten()
    }
}

/// The `<EventData>` or `<UserData>` element of an event's XML, cut to `MAX_DATA` bytes.
pub(crate) fn event_data(xml: &str) -> String {
    let element = ["EventData", "UserData"].into_iter().find_map(|name| {
        let start = xml.find(&format!("<{name}"))?;
        let close = format!("</{name}>");
        let end = xml[start..].find(&close)? + start + close.len();
        Some(&xml[start..end])
    });
    let mut data = element.unwrap_or_default().to_string();
    if data.len() > MAX_DATA {
        let mut cut = MAX_DATA;
        while !data.is_char_boundary(cut) {
            cut -= 1;
        }
        data.truncate(cut);
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_data_keeps_only_the_data_element() {
        let xml = "<Event><System><EventID>129</EventID></System>\
                   <EventData><Data>\\Device\\RaidPort0</Data></EventData></Event>";
        assert_eq!(
            event_data(xml),
            "<EventData><Data>\\Device\\RaidPort0</Data></EventData>"
        );
        assert_eq!(
            event_data("<Event><UserData><Info/></UserData></Event>"),
            "<UserData><Info/></UserData>"
        );
        assert_eq!(event_data("<Event><System/></Event>"), "");
    }

    #[test]
    fn event_data_is_cut_at_four_kib() {
        let xml = format!("<EventData>{}</EventData>", "é".repeat(5_000));
        let data = event_data(&xml);
        assert!(data.len() <= MAX_DATA);
        assert!(data.starts_with("<EventData>"));
    }
}
