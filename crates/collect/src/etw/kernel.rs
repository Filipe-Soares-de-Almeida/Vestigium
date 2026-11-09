use vestigium_core::RoutineKind;
use windows::core::GUID;

use super::RawEvent;

/// Classic kernel event class of DPC, ISR and sampled-profile events.
pub const PERF_INFO: GUID = GUID::from_u128(0xce1dbfb4_137e_4da6_87b0_3f59aa102cbc);
/// Classic kernel event class of image load and unload events.
pub const IMAGE: GUID = GUID::from_u128(0x2cb15d1d_5fc1_11d2_abe1_00a0c911f518);

/// One DPC/ISR call or one loaded image, from the kernel session.
#[derive(Clone, Debug, PartialEq)]
pub enum KernelRecord {
    /// A routine that ran from `initial` to the event's own timestamp, both raw QPC.
    Routine {
        kind: RoutineKind,
        routine: u64,
        initial: i64,
    },
    /// An image in memory; kernel modules have pid 0.
    Image {
        base: u64,
        size: u64,
        pid: u32,
        path: String,
    },
}

/// The DPC, ISR or image record in a kernel event; other events, and short ones, give `None`.
pub fn kernel_record(event: &RawEvent) -> Option<KernelRecord> {
    let data = event.data;
    if event.provider == PERF_INFO {
        let kind = match event.opcode {
            66 | 68 | 69 => RoutineKind::Dpc,
            // Windows 10 logs the ISR as opcode 50; the documentation still says 67.
            50 | 67 => RoutineKind::Isr,
            _ => return None,
        };
        return Some(KernelRecord::Routine {
            kind,
            initial: i64::from_le_bytes(data.get(0..8)?.try_into().ok()?),
            routine: u64::from_le_bytes(data.get(8..16)?.try_into().ok()?),
        });
    }
    if event.provider == IMAGE && matches!(event.opcode, 3 | 10) && event.version >= 2 {
        let units: Vec<u16> = data
            .get(56..)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .take_while(|u| *u != 0)
            .collect();
        return Some(KernelRecord::Image {
            base: u64::from_le_bytes(data.get(0..8)?.try_into().ok()?),
            size: u64::from_le_bytes(data.get(8..16)?.try_into().ok()?),
            pid: u32::from_le_bytes(data.get(16..20)?.try_into().ok()?),
            path: String::from_utf16_lossy(&units),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(provider: GUID, opcode: u8, version: u8, data: &[u8]) -> RawEvent<'_> {
        RawEvent {
            provider,
            opcode,
            version,
            cpu: 0,
            timestamp: 0,
            data,
        }
    }

    fn routine_bytes(initial: i64, routine: u64, tail: &[u8]) -> Vec<u8> {
        let mut bytes = initial.to_le_bytes().to_vec();
        bytes.extend(routine.to_le_bytes());
        bytes.extend(tail);
        bytes
    }

    #[test]
    fn parses_dpc_payload() {
        let bytes = routine_bytes(1_000, 0xfffff800_12345678, &[]);
        for opcode in [66, 68, 69] {
            assert_eq!(
                kernel_record(&event(PERF_INFO, opcode, 2, &bytes)),
                Some(KernelRecord::Routine {
                    kind: RoutineKind::Dpc,
                    routine: 0xfffff800_12345678,
                    initial: 1_000
                })
            );
        }
    }

    #[test]
    fn parses_isr_under_both_opcodes() {
        let bytes = routine_bytes(7, 0xfffff800_00000010, &[1, 0x41, 0, 0]);
        for opcode in [50, 67] {
            assert!(matches!(
                kernel_record(&event(PERF_INFO, opcode, 2, &bytes)),
                Some(KernelRecord::Routine {
                    kind: RoutineKind::Isr,
                    routine: 0xfffff800_00000010,
                    initial: 7
                })
            ));
        }
    }

    #[test]
    fn parses_image_load_name() {
        let mut bytes = vec![0u8; 56];
        bytes[0..8].copy_from_slice(&0xfffff801_00000000u64.to_le_bytes());
        bytes[8..16].copy_from_slice(&0x20000u64.to_le_bytes());
        bytes[16..20].copy_from_slice(&0u32.to_le_bytes());
        for unit in r"\SystemRoot\System32\drivers\example.sys".encode_utf16() {
            bytes.extend(unit.to_le_bytes());
        }
        bytes.extend([0, 0]);
        for opcode in [3, 10] {
            assert_eq!(
                kernel_record(&event(IMAGE, opcode, 3, &bytes)),
                Some(KernelRecord::Image {
                    base: 0xfffff801_00000000,
                    size: 0x20000,
                    pid: 0,
                    path: r"\SystemRoot\System32\drivers\example.sys".into()
                })
            );
        }
    }

    #[test]
    fn ignores_other_events_and_short_payloads() {
        let bytes = routine_bytes(1, 2, &[]);
        assert_eq!(kernel_record(&event(PERF_INFO, 46, 2, &bytes)), None);
        assert_eq!(kernel_record(&event(IMAGE, 2, 3, &bytes)), None);
        assert_eq!(kernel_record(&event(PERF_INFO, 68, 2, &bytes[..12])), None);
        assert_eq!(kernel_record(&event(GUID::zeroed(), 68, 2, &bytes)), None);
    }
}
