use std::sync::OnceLock;

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
}
