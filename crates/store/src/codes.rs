//! The integers the history stores for enums, and the enums they read back as.

use vestigium_core::{Channel, EventMeaning, GapReason, OccurrenceKind, Severity};

use crate::StoreError;

/// Writes the code of each variant and its reverse from one table; the code match has no wildcard,
/// so a variant missing from the table stops the build.
macro_rules! stored_codes {
    ($ty:ident, $what:literal, $to:ident, $from:ident, { $($variant:ident = $code:literal),+ $(,)? }) => {
        pub(crate) fn $to(value: $ty) -> i64 {
            match value {
                $($ty::$variant => $code),+
            }
        }

        pub(crate) fn $from(code: i64) -> Result<$ty, StoreError> {
            match code {
                $($code => Ok($ty::$variant),)+
                _ => Err(StoreError::UnknownCode { what: $what, code }),
            }
        }
    };
}

stored_codes!(OccurrenceKind, "occurrence kind", kind_code, kind_from_code, {
    Freeze = 1,
    Stutter = 2,
    AppHang = 3,
    Reported = 4,
    PostMortem = 5,
});

stored_codes!(Severity, "severity", severity_code, severity_from_code, {
    Light = 1,
    Medium = 2,
    Severe = 3,
});

stored_codes!(Channel, "channel", channel_code, channel_from_code, {
    System = 1,
    Application = 2,
});

stored_codes!(GapReason, "gap reason", gap_code, gap_from_code, {
    Paused = 1,
    Suspended = 2,
    Stopped = 3,
    Crashed = 4,
    ClockChanged = 5,
});

stored_codes!(EventMeaning, "event meaning", meaning_code, meaning_from_code, {
    StorageReset = 1,
    DiskRetry = 2,
    DiskBadBlock = 3,
    PagingError = 4,
    DisplayReset = 5,
    HardwareError = 6,
    PowerLoss = 7,
    UnexpectedShutdown = 8,
    BugCheck = 9,
    CpuFirmwareLimit = 10,
    LowVirtualMemory = 11,
    AppHang = 12,
    AppCrash = 13,
    ServiceCrash = 14,
    NoCrashDump = 15,
    MemoryDiagnostic = 16,
});

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks the stored number of each variant against the list written out here, and that every
    /// number reads back as its variant.
    macro_rules! codes_are {
        ($test:ident, $ty:ident, $to:ident, $from:ident, $what:literal, [$($code:literal),+ $(,)?]) => {
            #[test]
            fn $test() {
                let literal: &[i64] = &[$($code),+];
                assert_eq!($ty::ALL.len(), literal.len(), "a variant has no literal code here");
                for (value, &code) in $ty::ALL.into_iter().zip(literal) {
                    assert_eq!($to(value), code, "{value:?}");
                    assert_eq!($from(code).unwrap(), value, "code {code}");
                }
                assert!(matches!(
                    $from(0),
                    Err(StoreError::UnknownCode { what: $what, code: 0 })
                ));
                assert!(matches!(
                    $from(literal.len() as i64 + 1),
                    Err(StoreError::UnknownCode { what: $what, .. })
                ));
            }
        };
    }

    codes_are!(
        occurrence_kind_codes_are_fixed,
        OccurrenceKind,
        kind_code,
        kind_from_code,
        "occurrence kind",
        [1, 2, 3, 4, 5]
    );
    codes_are!(
        severity_codes_are_fixed,
        Severity,
        severity_code,
        severity_from_code,
        "severity",
        [1, 2, 3]
    );
    codes_are!(
        channel_codes_are_fixed,
        Channel,
        channel_code,
        channel_from_code,
        "channel",
        [1, 2]
    );
    codes_are!(
        gap_reason_codes_are_fixed,
        GapReason,
        gap_code,
        gap_from_code,
        "gap reason",
        [1, 2, 3, 4, 5]
    );
    codes_are!(
        event_meaning_codes_are_fixed,
        EventMeaning,
        meaning_code,
        meaning_from_code,
        "event meaning",
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    );
}
