//! Compositor frame counters once a second: `dwm_frames --seconds <s>`.

use std::thread;
use std::time::Duration;

use windows::Win32::Graphics::Dwm::{DWM_TIMING_INFO, DwmGetCompositionTimingInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seconds: u64 = std::env::args()
        .skip_while(|a| a != "--seconds")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let mut prev = read()?;
    let (refresh, compose) = ({ prev.rateRefresh }, { prev.rateCompose });
    println!(
        "refresh {}/{} Hz, compose {}/{}",
        { refresh.uiNumerator },
        { refresh.uiDenominator },
        { compose.uiNumerator },
        { compose.uiDenominator }
    );
    println!(
        "| s | refreshes | frames composed | displayed | late | dropped | missed | pending | complete |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for s in 1..=seconds {
        thread::sleep(Duration::from_secs(1));
        let now = read()?;
        println!(
            "| {s} | {} | {} | {} | {} | {} | {} | {} | {} |",
            { now.cRefresh }.wrapping_sub(prev.cRefresh),
            { now.cFrame }.wrapping_sub(prev.cFrame),
            { now.cFramesDisplayed },
            { now.cFramesLate },
            { now.cFramesDropped },
            { now.cFramesMissed },
            { now.cFramesPending },
            { now.cFramesComplete }
        );
        prev = now;
    }
    Ok(())
}

fn read() -> windows::core::Result<DWM_TIMING_INFO> {
    let mut info = DWM_TIMING_INFO {
        cbSize: size_of::<DWM_TIMING_INFO>() as u32,
        ..Default::default()
    };
    // SAFETY: fills a local struct whose cbSize is set; hwnd must be NULL since Windows 8.1.
    unsafe { DwmGetCompositionTimingInfo(Default::default(), &mut info)? };
    Ok(info)
}
