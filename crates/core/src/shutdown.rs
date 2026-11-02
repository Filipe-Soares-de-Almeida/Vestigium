use serde::{Deserialize, Serialize};

/// The SYSTEM_POWER_STATE of a shutdown, as Kernel-Power 41 writes it in SleepInProgress.
pub const SHUTTING_DOWN: u8 = 6;

/// What Kernel-Power 41 recorded about the last time the PC went down without a clean shutdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerLossDetail {
    /// Stop code of the blue screen; 0 when Windows recorded none.
    pub bugcheck_code: u32,
    pub bugcheck_params: [u64; 4],
    /// SYSTEM_POWER_STATE being entered: 0 none, 2 to 5 sleep or hibernation, 6 shutdown.
    pub power_state: u8,
    /// The power button was held, as people do when the PC stopped responding.
    pub power_button: bool,
}

/// The details in a Kernel-Power 41's `<EventData>`; `None` when it has no stop code.
pub fn power_loss_detail(data_xml: &str) -> Option<PowerLossDetail> {
    let bugcheck_code = data_named(data_xml, "BugcheckCode")?.parse().ok()?;
    let mut bugcheck_params = [0u64; 4];
    for (i, param) in bugcheck_params.iter_mut().enumerate() {
        *param = data_named(data_xml, &format!("BugcheckParameter{}", i + 1))
            .and_then(parse_hex)
            .unwrap_or(0);
    }
    let power_state = data_named(data_xml, "SleepInProgress")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let pressed = data_named(data_xml, "PowerButtonTimestamp")
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|stamp| stamp != 0);
    let long_press = data_named(data_xml, "LongPowerButtonPressDetected")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    Some(PowerLossDetail {
        bugcheck_code,
        bugcheck_params,
        power_state,
        power_button: pressed || long_press,
    })
}

/// When the PC went down, from an EventLog 6008: the UTC time in its binary data, Unix epoch ms.
pub fn unexpected_shutdown_ms(data_xml: &str) -> Option<i64> {
    let bytes = binary(data_xml)?;
    // Two SYSTEMTIMEs, local then UTC: year, month, weekday, day, hour, minute, second, ms.
    let utc: Vec<i64> = bytes
        .get(16..32)?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i64::from(u16::from_le_bytes(*pair)))
        .collect();
    let (year, month, day) = (utc[0], utc[1], utc[3]);
    let (hour, minute, second, ms) = (utc[4], utc[5], utc[6], utc[7]);
    let valid = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second < 60
        && ms < 1000;
    valid.then(|| {
        let days = days_from_civil(year, month, day);
        ((days * 24 + hour) * 60 + minute) * 60_000 + second * 1000 + ms
    })
}

/// The text of `<Data Name='name'>`, written with either quote style.
fn data_named<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    ['\'', '"'].into_iter().find_map(|quote| {
        let open = format!("<Data Name={quote}{name}{quote}>");
        let start = xml.find(&open)? + open.len();
        let len = xml[start..].find("</Data>")?;
        Some(xml[start..start + len].trim())
    })
}

fn parse_hex(text: &str) -> Option<u64> {
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    u64::from_str_radix(digits, 16).ok()
}

/// The bytes of the `<Binary>` element, which Windows writes in hex.
fn binary(xml: &str) -> Option<Vec<u8>> {
    let start = xml.find("<Binary>")? + "<Binary>".len();
    let hex = &xml[start..start + xml[start..].find("</Binary>")?];
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Days from 1970-01-01 to a date of the proleptic Gregorian calendar.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}
