use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use vestigium_collect::Wake;

use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::System::Power::RegisterSuspendResumeNotification;
use windows::Win32::UI::WindowsAndMessaging::{
    DEVICE_NOTIFY_WINDOW_HANDLE, PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMECRITICAL,
    PBT_APMRESUMESUSPEND, PBT_APMSUSPEND,
};

/// The whole system going to sleep or coming back, modern standby included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerEvent {
    Suspend,
    Resume,
}

/// What the sampler is told; a suspend carries the channel it answers on once everything is written.
pub enum PowerNotice {
    Suspend(Sender<()>),
    Resume,
}

/// How long a suspend notice waits for the sampler before it lets the system go to sleep.
pub const SUSPEND_WAIT: Duration = Duration::from_millis(1500);

/// Passes `event` to the sampler. A suspend wakes it at once and waits up to `limit` for its
/// answer, so what must reach the disk is written before the system sleeps.
pub fn announce(event: PowerEvent, notices: &Sender<PowerNotice>, wake: &Wake, limit: Duration) {
    match event {
        PowerEvent::Suspend => {
            let (done, answered) = mpsc::channel();
            if notices.send(PowerNotice::Suspend(done)).is_err() {
                return;
            }
            wake.set();
            let _ = answered.recv_timeout(limit);
        }
        PowerEvent::Resume => {
            let _ = notices.send(PowerNotice::Resume);
        }
    }
}

type Handler = Box<dyn Fn(PowerEvent) + Send + Sync>;

static ON_POWER: OnceLock<Handler> = OnceLock::new();

/// Maps a WM_POWERBROADCAST code; a resume started by the user sends two resume codes.
pub fn power_event(wparam: usize) -> Option<PowerEvent> {
    match u32::try_from(wparam).ok()? {
        PBT_APMSUSPEND => Some(PowerEvent::Suspend),
        PBT_APMRESUMECRITICAL | PBT_APMRESUMESUSPEND | PBT_APMRESUMEAUTOMATIC => {
            Some(PowerEvent::Resume)
        }
        _ => None,
    }
}

/// Sends suspend and resume to `on_event`; modern standby tells only the windows that subscribe.
pub fn watch(
    hwnd: HWND,
    on_event: impl Fn(PowerEvent) + Send + Sync + 'static,
) -> windows::core::Result<()> {
    let _ = ON_POWER.set(Box::new(on_event));
    // SAFETY: subscribes this process's own window; the subscription lasts as long as the process.
    unsafe { RegisterSuspendResumeNotification(HANDLE(hwnd.0), DEVICE_NOTIFY_WINDOW_HANDLE)? };
    Ok(())
}

/// Called by the window procedure for WM_POWERBROADCAST.
pub fn on_broadcast(wparam: usize) {
    if let (Some(event), Some(handler)) = (power_event(wparam), ON_POWER.get()) {
        handler(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_variants_map_to_one_event() {
        for wparam in [0x6, 0x7, 0x12] {
            assert_eq!(power_event(wparam), Some(PowerEvent::Resume));
        }
        assert_eq!(power_event(0x4), Some(PowerEvent::Suspend));
        assert_eq!(power_event(0xA), None);
    }

    #[test]
    fn a_suspend_with_no_answer_returns_after_the_limit() {
        let (notices, _unread) = mpsc::channel();
        let wake = Wake::new().unwrap();
        let began = std::time::Instant::now();
        announce(
            PowerEvent::Suspend,
            &notices,
            &wake,
            Duration::from_millis(150),
        );
        let took = began.elapsed();
        assert!(took >= Duration::from_millis(140), "{took:?}");
        assert!(took < Duration::from_secs(2), "{took:?}");
    }

    #[test]
    fn a_suspend_that_is_answered_returns_at_once() {
        let (notices, received) = mpsc::channel();
        let wake = Wake::new().unwrap();
        let sampler = std::thread::spawn(move || {
            if let Ok(PowerNotice::Suspend(done)) = received.recv() {
                let _ = done.send(());
            }
        });
        let began = std::time::Instant::now();
        announce(
            PowerEvent::Suspend,
            &notices,
            &wake,
            Duration::from_secs(30),
        );
        sampler.join().unwrap();
        assert!(began.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_resume_does_not_wait() {
        let (notices, received) = mpsc::channel();
        let wake = Wake::new().unwrap();
        let began = std::time::Instant::now();
        announce(PowerEvent::Resume, &notices, &wake, Duration::from_secs(30));
        assert!(began.elapsed() < Duration::from_secs(1));
        assert!(matches!(received.try_recv(), Ok(PowerNotice::Resume)));
    }
}
