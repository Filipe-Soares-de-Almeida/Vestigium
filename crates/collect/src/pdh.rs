use std::collections::HashMap;

use vestigium_core::DiskLoad;
use windows::Win32::System::Performance::{
    PDH_CALC_NEGATIVE_DENOMINATOR, PDH_CALC_NEGATIVE_TIMEBASE, PDH_CALC_NEGATIVE_VALUE,
    PDH_CSTATUS_INVALID_DATA, PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_INVALID_DATA,
    PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData,
    PdhGetFormattedCounterArrayW, PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::core::{HSTRING, PCWSTR};

use crate::{CollectError, GpuReading, gpu_usage};

const PERF_LIMIT_PATH: &str = r"\Processor Information(_Total)\% Performance Limit";
const PROC_PERF_PATH: &str = r"\Processor Information(_Total)\% Processor Performance";
const HARD_FAULTS_PATH: &str = r"\Memory\Pages Input/sec";
const GPU_PATH: &str = r"\GPU Engine(*)\Utilization Percentage";
const DISK_PATHS: [&str; 4] = [
    r"\PhysicalDisk(*)\% Idle Time",
    r"\PhysicalDisk(*)\Avg. Disk sec/Transfer",
    r"\PhysicalDisk(*)\Current Disk Queue Length",
    r"\PhysicalDisk(*)\Disk Bytes/sec",
];
const TOTAL: &str = "_Total";
/// Statuses that say a counter has no value this second, not that the query is broken: no data,
/// or a rate whose two collections do not give a positive span.
const NO_VALUE: [u32; 5] = [
    PDH_INVALID_DATA,
    PDH_CSTATUS_INVALID_DATA,
    PDH_CALC_NEGATIVE_DENOMINATOR,
    PDH_CALC_NEGATIVE_TIMEBASE,
    PDH_CALC_NEGATIVE_VALUE,
];
/// Times the size of a wildcard counter is asked before its read gives up.
const ARRAY_TRIES: usize = 3;

/// The PDH counters of one second, system-wide; per-core CPU comes from `CpuSampler`.
#[derive(Clone, Debug, PartialEq)]
pub struct PdhReading {
    /// Each counter is `None` when it has no value this second.
    pub perf_limit_pct: Option<f32>,
    pub proc_perf_pct: Option<f32>,
    pub hard_faults_per_s: Option<f32>,
    /// Empty when the disk counters could not be read at all.
    pub disks: Vec<DiskLoad>,
    /// `None` when the machine has no GPU Engine counters or they could not be read.
    pub gpu: Option<GpuReading>,
}

/// One PDH query holding every counter of the 1 Hz sample, added by English path.
#[derive(Debug)]
pub struct PdhSampler {
    query: Query,
    perf_limit: PDH_HCOUNTER,
    proc_perf: PDH_HCOUNTER,
    hard_faults: PDH_HCOUNTER,
    disk: [PDH_HCOUNTER; 4],
    /// Absent on virtual machines and old drivers, which does not stop the other counters.
    gpu: Option<PDH_HCOUNTER>,
    buf: Vec<u64>,
    primed: bool,
}

impl PdhSampler {
    pub fn open() -> Result<PdhSampler, CollectError> {
        let mut handle = PDH_HQUERY::default();
        // SAFETY: opens a live-data query into `handle`, closed by `Query`'s drop.
        check("PdhOpenQueryW", unsafe {
            PdhOpenQueryW(PCWSTR::null(), 0, &mut handle)
        })?;
        let query = Query(handle);
        let perf_limit = query.add(PERF_LIMIT_PATH)?;
        let proc_perf = query.add(PROC_PERF_PATH)?;
        let hard_faults = query.add(HARD_FAULTS_PATH)?;
        let mut disk = [PDH_HCOUNTER::default(); 4];
        for (slot, path) in disk.iter_mut().zip(DISK_PATHS) {
            *slot = query.add(path)?;
        }
        let gpu = query.add(GPU_PATH).ok();
        Ok(PdhSampler {
            query,
            perf_limit,
            proc_perf,
            hard_faults,
            disk,
            gpu,
            buf: Vec::new(),
            primed: false,
        })
    }

    /// Collects now; `None` on the first collection after `open` or `reset`, which has no rates.
    /// A counter that cannot be read costs only its own value.
    pub fn sample(&mut self) -> Result<Option<PdhReading>, CollectError> {
        // SAFETY: the query handle is valid while `self` lives.
        check("PdhCollectQueryData", unsafe {
            PdhCollectQueryData(self.query.0)
        })?;
        if !self.primed {
            self.primed = true;
            return Ok(None);
        }

        let disk = self.disk;
        let [idle, transfer, queue, bytes] = disk.map(|c| self.read_array(c).unwrap_or_default());
        let gpu = match self.gpu {
            Some(counter) => self
                .read_array(counter)
                .ok()
                .map(|values| gpu_usage(&values)),
            None => None,
        };
        Ok(Some(PdhReading {
            perf_limit_pct: self.single(self.perf_limit),
            proc_perf_pct: self.single(self.proc_perf),
            hard_faults_per_s: self.single(self.hard_faults),
            disks: disk_loads(&idle, &transfer, &queue, &bytes),
            gpu,
        }))
    }

    /// Makes the next `sample` a priming collection again, as after a pause.
    pub fn reset(&mut self) {
        self.primed = false;
    }

    /// Valid values of a wildcard counter by instance name; empty when it has no value.
    fn read_array(&mut self, counter: PDH_HCOUNTER) -> Result<HashMap<String, f64>, CollectError> {
        let mut status = 0;
        for _ in 0..ARRAY_TRIES {
            // The instance count can change between two calls, so the size is asked every time.
            let mut bytes = 0u32;
            let mut count = 0u32;
            // SAFETY: with no target the call only reports the size it needs in `bytes`.
            status = unsafe {
                PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None)
            };
            if NO_VALUE.contains(&status) || (status == 0 && bytes == 0) {
                return Ok(HashMap::new());
            }
            if status != PDH_MORE_DATA && status != 0 {
                continue;
            }
            if self.buf.len() * 8 < bytes as usize {
                self.buf.resize((bytes as usize).div_ceil(8), 0);
            }
            let mut bytes = (self.buf.len() * 8) as u32;
            let target = self.buf.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            // SAFETY: `target` points at `bytes` writable bytes aligned to 8.
            status = unsafe {
                PdhGetFormattedCounterArrayW(
                    counter,
                    PDH_FMT_DOUBLE,
                    &mut bytes,
                    &mut count,
                    Some(target),
                )
            };
            if NO_VALUE.contains(&status) {
                return Ok(HashMap::new());
            }
            if status != 0 {
                continue;
            }
            // SAFETY: on success the buffer starts with `count` items whose names point inside it.
            let items = unsafe {
                std::slice::from_raw_parts(
                    self.buf.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                    count as usize,
                )
            };
            return Ok(items
                .iter()
                .filter(|item| is_valid(&item.FmtValue))
                // SAFETY: the name is a NUL-terminated string inside the buffer; the value is a double.
                .map(|item| unsafe {
                    (
                        String::from_utf16_lossy(item.szName.as_wide()),
                        item.FmtValue.Anonymous.doubleValue,
                    )
                })
                .filter(|(_, value)| value.is_finite())
                .collect());
        }
        Err(CollectError::Pdh {
            call: "PdhGetFormattedCounterArrayW",
            status,
        })
    }

    /// One counter's value; `None` when it has none or cannot be read.
    fn single(&self, counter: PDH_HCOUNTER) -> Option<f32> {
        self.read_single(counter).ok().flatten().map(|v| v as f32)
    }

    fn read_single(&self, counter: PDH_HCOUNTER) -> Result<Option<f64>, CollectError> {
        let mut value = PDH_FMT_COUNTERVALUE::default();
        // SAFETY: writes one formatted value of a counter of this query into `value`.
        let status =
            unsafe { PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, None, &mut value) };
        // SAFETY: PDH_FMT_DOUBLE fills the double member of the union.
        let raw = unsafe { value.Anonymous.doubleValue };
        counter_value(status, value.CStatus, raw)
    }
}

#[derive(Debug)]
struct Query(PDH_HQUERY);

impl Query {
    fn add(&self, path: &str) -> Result<PDH_HCOUNTER, CollectError> {
        let mut counter = PDH_HCOUNTER::default();
        // SAFETY: the path outlives the call; the counter belongs to this query.
        check("PdhAddEnglishCounterW", unsafe {
            PdhAddEnglishCounterW(self.0, &HSTRING::from(path), 0, &mut counter)
        })?;
        Ok(counter)
    }
}

impl Drop for Query {
    fn drop(&mut self) {
        // SAFETY: closes the query and its counters exactly once.
        let _ = unsafe { PdhCloseQuery(self.0) };
    }
}

/// What a formatted counter says: its value, `None` when it has none this second, or an error
/// for any other status. PDH documents both an error status and a success with a bad `CStatus`.
fn counter_value(status: u32, cstatus: u32, raw: f64) -> Result<Option<f64>, CollectError> {
    if NO_VALUE.contains(&status) {
        return Ok(None);
    }
    check("PdhGetFormattedCounterValue", status)?;
    let valid = cstatus == PDH_CSTATUS_VALID_DATA || cstatus == PDH_CSTATUS_NEW_DATA;
    Ok(valid.then_some(raw).filter(|v| v.is_finite()))
}

/// The disks named by the idle counter, each with the counters that have a value for it.
fn disk_loads(
    idle: &HashMap<String, f64>,
    transfer: &HashMap<String, f64>,
    queue: &HashMap<String, f64>,
    bytes: &HashMap<String, f64>,
) -> Vec<DiskLoad> {
    let mut names: Vec<&String> = idle.keys().filter(|name| *name != TOTAL).collect();
    names.sort();
    names
        .into_iter()
        .map(|name| DiskLoad {
            name: name.clone(),
            active_pct: (100.0 - idle[name]).clamp(0.0, 100.0) as f32,
            latency_ms: transfer.get(name).map(|v| (v * 1000.0) as f32),
            queue_len: queue.get(name).map(|v| *v as f32),
            bytes_per_s: bytes.get(name).map(|v| *v as f32),
        })
        .collect()
}

fn is_valid(value: &PDH_FMT_COUNTERVALUE) -> bool {
    value.CStatus == PDH_CSTATUS_VALID_DATA || value.CStatus == PDH_CSTATUS_NEW_DATA
}

fn check(call: &'static str, status: u32) -> Result<(), CollectError> {
    if status == 0 {
        Ok(())
    } else {
        Err(CollectError::Pdh { call, status })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    #[test]
    fn counter_value_without_valid_data_is_absent() {
        assert_eq!(
            counter_value(0, PDH_CSTATUS_VALID_DATA, 4.5).unwrap(),
            Some(4.5)
        );
        assert_eq!(
            counter_value(0, PDH_CSTATUS_NEW_DATA, 4.5).unwrap(),
            Some(4.5)
        );
        assert_eq!(
            counter_value(0, PDH_CSTATUS_INVALID_DATA, 0.0).unwrap(),
            None
        );
        assert_eq!(counter_value(PDH_INVALID_DATA, 0, 0.0).unwrap(), None);
        assert_eq!(
            counter_value(0, PDH_CSTATUS_VALID_DATA, f64::NAN).unwrap(),
            None
        );
    }

    #[test]
    fn a_rate_with_no_positive_span_is_absent() {
        for status in [
            PDH_CALC_NEGATIVE_DENOMINATOR,
            PDH_CALC_NEGATIVE_TIMEBASE,
            PDH_CALC_NEGATIVE_VALUE,
        ] {
            assert_eq!(counter_value(status, 0, 0.0).unwrap(), None, "{status:#x}");
        }
    }

    #[test]
    fn any_other_status_is_an_error() {
        let err = counter_value(0xC000_0BBD, 0, 0.0).unwrap_err();

        assert!(matches!(
            err,
            CollectError::Pdh {
                status: 0xC000_0BBD,
                ..
            }
        ));
    }

    #[test]
    fn a_disk_without_its_latency_has_no_latency() {
        let idle = values(&[("0 C:", 90.0), ("1 D:", 40.0), (TOTAL, 65.0)]);
        let transfer = values(&[("0 C:", 0.002)]);
        let queue = values(&[("0 C:", 0.5), ("1 D:", 3.0)]);
        let bytes = values(&[("0 C:", 1e6), ("1 D:", 2e6)]);

        let disks = disk_loads(&idle, &transfer, &queue, &bytes);

        assert_eq!(disks.len(), 2);
        assert_eq!(disks[0].name, "0 C:");
        assert_eq!(disks[0].latency_ms, Some(2.0));
        assert_eq!(disks[1].name, "1 D:");
        assert_eq!(disks[1].active_pct, 60.0);
        assert_eq!(disks[1].latency_ms, None);
        assert_eq!(disks[1].queue_len, Some(3.0));
        assert_eq!(disks[1].bytes_per_s, Some(2e6));
    }

    #[test]
    fn disks_need_the_idle_counter_to_be_named() {
        let none = HashMap::new();
        let transfer = values(&[("0 C:", 0.002)]);

        assert!(disk_loads(&none, &transfer, &transfer, &transfer).is_empty());
    }
}
