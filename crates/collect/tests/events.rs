//! These tests write events to the real Application log, so they are ignored by default and run
//! on CI with `--ignored`.

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vestigium_collect::{Delivered, EventSubscription};
use vestigium_core::{Channel, EventMeaning};
use windows::Win32::System::EventLog::{
    DeregisterEventSource, EVENTLOG_INFORMATION_TYPE, RegisterEventSourceW, ReportEventW,
};
use windows::core::{HSTRING, PCWSTR, w};

const SOURCE: &str = "VestigiumTest";
const PATIENCE: Duration = Duration::from_secs(15);

/// An event id no earlier run used, so old test events never match.
fn fresh_id() -> u16 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    20_000 + (secs % 40_000) as u16
}

fn query(id: u16) -> String {
    format!(
        "<QueryList><Query Id=\"0\"><Select Path=\"Application\">\
         *[System[(Provider[@Name='{SOURCE}']) and (EventID={id})]]</Select></Query></QueryList>"
    )
}

fn test_meaning(channel: Channel, provider: &str, _id: u16) -> Option<EventMeaning> {
    (channel == Channel::Application && provider == SOURCE).then_some(EventMeaning::AppCrash)
}

fn report(id: u16, text: &str) {
    // SAFETY: registers a source on this machine and reports one event; the handle is released below.
    unsafe {
        let source = RegisterEventSourceW(PCWSTR::null(), &HSTRING::from(SOURCE)).unwrap();
        let text = HSTRING::from(text);
        ReportEventW(
            source,
            EVENTLOG_INFORMATION_TYPE,
            0,
            u32::from(id),
            None,
            0,
            Some(&[PCWSTR(text.as_ptr()), w!("vestigium test event")]),
            None,
        )
        .unwrap();
        DeregisterEventSource(source).unwrap();
    }
}

fn receive(rx: &Receiver<Delivered>, count: usize) -> Vec<Delivered> {
    let deadline = Instant::now() + PATIENCE;
    let mut got = Vec::new();
    while got.len() < count {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(delivered) => got.push(delivered),
            Err(_) => break,
        }
    }
    got
}

#[test]
#[ignore]
fn subscription_delivers_a_reported_event() {
    let id = fresh_id();
    let (tx, rx) = mpsc::channel();
    let _subscription = EventSubscription::start_with_query(&query(id), None, test_meaning, tx)
        .expect("subscribing needs no elevation");

    report(id, "first");
    let got = receive(&rx, 1);

    assert_eq!(got.len(), 1, "no event delivered");
    let event = &got[0].event;
    assert_eq!(event.channel, Channel::Application);
    assert_eq!(event.provider, SOURCE);
    assert_eq!(event.id, id);
    assert_eq!(event.meaning, EventMeaning::AppCrash);
    assert!(event.data_xml.contains("first"), "{}", event.data_xml);
    assert!(
        got[0]
            .bookmark_xml
            .as_deref()
            .is_some_and(|xml| xml.contains("BookmarkList"))
    );
}

#[test]
#[ignore]
fn bookmark_resumes_after_the_last_event() {
    let id = fresh_id().wrapping_add(1);
    report(id, "older");
    report(id, "newer");
    let (tx, rx) = mpsc::channel();
    let subscription =
        EventSubscription::start_with_query(&query(id), None, test_meaning, tx).unwrap();
    let both = receive(&rx, 2);
    drop(subscription);
    assert_eq!(
        both.len(),
        2,
        "the two reported events were not both delivered"
    );

    let (tx, rx) = mpsc::channel();
    let _resumed = EventSubscription::start_with_query(
        &query(id),
        both[0].bookmark_xml.as_deref(),
        test_meaning,
        tx,
    )
    .unwrap();
    let after = receive(&rx, 2);

    assert_eq!(after.len(), 1, "resuming replays only what came after");
    assert_eq!(after[0].event.record_id, both[1].event.record_id);
}

#[test]
#[ignore]
fn a_refused_bookmark_starts_the_backfill_instead() {
    let (tx, _rx) = mpsc::channel();

    let subscription =
        EventSubscription::start(Some("not a bookmark"), Duration::from_secs(60), tx);

    assert!(subscription.is_ok(), "{:?}", subscription.err());
}
