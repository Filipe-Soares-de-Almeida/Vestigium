//! The kernel session against the real ETW; needs an elevated test run (`-- --ignored`).

use std::thread;
use std::time::Duration;

use vestigium_collect::{
    EventSink, KernelRecord, KernelSession, RawEvent, kernel_record, spawn_consumer,
};
use windows::Win32::System::Diagnostics::Etw::{
    EVENT_TRACE_FLAG_DPC, EVENT_TRACE_FLAG_IMAGE_LOAD, EVENT_TRACE_FLAG_INTERRUPT,
};
use windows::core::GUID;

const NAME: &str = "Vestigium Test Session";
const TEST_GUID: GUID = GUID::from_u128(0x2b6d6a3e_93c1_4f0b_8c7e_5a1f0d9e4c21);

#[derive(Default)]
struct Count {
    routines: u64,
    kernel_images: u64,
}

impl EventSink for Count {
    fn event(&mut self, event: &RawEvent<'_>) {
        match kernel_record(event) {
            Some(KernelRecord::Routine { .. }) => self.routines += 1,
            Some(KernelRecord::Image { pid: 0, .. }) => self.kernel_images += 1,
            _ => {}
        }
    }
}

fn start() -> KernelSession {
    KernelSession::start(
        NAME,
        TEST_GUID,
        EVENT_TRACE_FLAG_DPC | EVENT_TRACE_FLAG_INTERRUPT | EVENT_TRACE_FLAG_IMAGE_LOAD,
        None,
    )
    .expect("an elevated run starts the session")
}

#[test]
#[ignore]
fn session_delivers_routines_and_kernel_images() {
    let session = start();
    let consumer = spawn_consumer(NAME, Count::default()).unwrap();
    thread::sleep(Duration::from_secs(3));
    let stats = session.stop().unwrap();
    let read = consumer.join().unwrap();
    assert_eq!(read.status, Ok(()));
    assert!(read.events > 0);
    let count = read.sink;
    assert!(count.routines > 0, "no DPC or ISR in 3 s");
    assert!(
        count.kernel_images > 50,
        "{} kernel images",
        count.kernel_images
    );
    assert_eq!(stats.events_lost, 0);
}

/// Run by hand: `cargo test -p vestigium-collect --test etw -- --ignored consumer_of_a_missing_session_fails`.
#[test]
#[ignore]
fn consumer_of_a_missing_session_fails() {
    let read = spawn_consumer("Vestigium Missing Session", Count::default())
        .unwrap()
        .join()
        .unwrap();
    assert!(read.status.is_err(), "{:?}", read.status);
    assert_eq!(read.events, 0);
}

#[test]
#[ignore]
fn stops_an_orphan_session_first() {
    let orphan = start();
    std::mem::forget(orphan);
    let again = start();
    drop(again);
}

#[test]
#[ignore]
fn session_stops_on_drop() {
    drop(start());
    let fresh = start();
    fresh.stop().unwrap();
}
