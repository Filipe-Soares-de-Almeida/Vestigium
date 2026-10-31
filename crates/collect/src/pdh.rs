use std::collections::HashMap;

use vestigium_core::{CoreLoad, DiskLoad};
use windows::Win32::System::Performance::{
    PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::core::{HSTRING, PCWSTR};

use crate::CollectError;

const CPU_PATHS: [&str; 5] = [
    r"\Processor Information(*)\% Processor Time",
    r"\Processor Information(*)\% DPC Time",
    r"\Processor Information(*)\% Interrupt Time",
    r"\Processor Information(*)\% Performance Limit",
    r"\Processor Information(*)\% Processor Performance",
];
const HARD_FAULTS_PATH: &str = r"\Memory\Pages Input/sec";
const DISK_PATHS: [&str; 4] = [
    r"\PhysicalDisk(*)\% Idle Time",
    r"\PhysicalDisk(*)\Avg. Disk sec/Transfer",
    r"\PhysicalDisk(*)\Current Disk Queue Length",
    r"\PhysicalDisk(*)\Disk Bytes/sec",
];
const TOTAL: &str = "_Total";

/// The PDH counters of one second, system-wide.
#[derive(Clone, Debug, PartialEq)]
pub struct PdhReading {
    pub cpu_pct: f32,
    pub dpc_pct: f32,
    pub isr_pct: f32,
    pub perf_limit_pct: f32,
    pub proc_perf_pct: f32,
    pub hard_faults_per_s: f32,
    pub cores: Vec<CoreLoad>,
    pub disks: Vec<DiskLoad>,
}

/// An instance of the "Processor Information" counter set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CpuInstance {
    Total,
    GroupTotal,
    Core { group: u16, index: u16 },
}

/// Parses "_Total", "0,_Total" and "0,3".
pub fn parse_cpu_instance(name: &str) -> Option<CpuInstance> {
    if name == TOTAL {
        return Some(CpuInstance::Total);
    }
    let (group, index) = name.split_once(',')?;
    let group = group.parse().ok()?;
    if index == TOTAL {
        return Some(CpuInstance::GroupTotal);
    }
    Some(CpuInstance::Core {
        group,
        index: index.parse().ok()?,
    })
}

/// One PDH query holding every counter of the 1 Hz sample, added by English path.
#[derive(Debug)]
pub struct PdhSampler {
    query: Query,
    cpu: [PDH_HCOUNTER; 5],
    hard_faults: PDH_HCOUNTER,
    disk: [PDH_HCOUNTER; 4],
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
        let mut cpu = [PDH_HCOUNTER::default(); 5];
        for (slot, path) in cpu.iter_mut().zip(CPU_PATHS) {
            *slot = query.add(path)?;
        }
        let hard_faults = query.add(HARD_FAULTS_PATH)?;
        let mut disk = [PDH_HCOUNTER::default(); 4];
        for (slot, path) in disk.iter_mut().zip(DISK_PATHS) {
            *slot = query.add(path)?;
        }
        Ok(PdhSampler {
            query,
            cpu,
            hard_faults,
            disk,
            buf: Vec::new(),
            primed: false,
        })
    }

    /// Collects now; `None` on the first collection after `open` or `reset`, which has no rates.
    pub fn sample(&mut self) -> Result<Option<PdhReading>, CollectError> {
        // SAFETY: the query handle is valid while `self` lives.
        check("PdhCollectQueryData", unsafe {
            PdhCollectQueryData(self.query.0)
        })?;
        if !self.primed {
            self.primed = true;
            return Ok(None);
        }

        let cpu = self.cpu;
        let [time, dpc, isr, limit, perf] = cpu.map(|c| self.read_array(c));
        let (time, dpc, isr, limit, perf) = (time?, dpc?, isr?, limit?, perf?);
        let total =
            |values: &HashMap<String, f64>| values.get(TOTAL).copied().unwrap_or(0.0) as f32;

        let mut cores: Vec<(CpuInstance, &str)> = time
            .keys()
            .filter_map(|name| Some((parse_cpu_instance(name)?, name.as_str())))
            .filter(|(instance, _)| matches!(instance, CpuInstance::Core { .. }))
            .collect();
        cores.sort();
        let value = |values: &HashMap<String, f64>, name: &str| {
            values.get(name).copied().unwrap_or(0.0) as f32
        };
        let cores = cores
            .into_iter()
            .map(|(_, name)| CoreLoad {
                cpu_pct: value(&time, name),
                dpc_pct: value(&dpc, name),
                isr_pct: value(&isr, name),
            })
            .collect();

        let disk = self.disk;
        let [idle, transfer, queue, bytes] = disk.map(|c| self.read_array(c));
        let (idle, transfer, queue, bytes) = (idle?, transfer?, queue?, bytes?);
        let mut names: Vec<&String> = idle.keys().filter(|name| *name != TOTAL).collect();
        names.sort();
        let disks = names
            .into_iter()
            .map(|name| DiskLoad {
                name: name.clone(),
                active_pct: (100.0 - value(&idle, name)).clamp(0.0, 100.0),
                latency_ms: value(&transfer, name) * 1000.0,
                queue_len: value(&queue, name),
                bytes_per_s: value(&bytes, name),
            })
            .collect();

        Ok(Some(PdhReading {
            cpu_pct: total(&time),
            dpc_pct: total(&dpc),
            isr_pct: total(&isr),
            perf_limit_pct: total(&limit),
            proc_perf_pct: total(&perf),
            hard_faults_per_s: self.read_single(self.hard_faults)? as f32,
            cores,
            disks,
        }))
    }

    /// Makes the next `sample` a priming collection again, as after a pause.
    pub fn reset(&mut self) {
        self.primed = false;
    }

    /// Valid values of a wildcard counter by instance name.
    fn read_array(&mut self, counter: PDH_HCOUNTER) -> Result<HashMap<String, f64>, CollectError> {
        for _ in 0..3 {
            let mut bytes = (self.buf.len() * 8) as u32;
            let mut count = 0u32;
            let target = (!self.buf.is_empty())
                .then(|| self.buf.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>());
            // SAFETY: `target`, when given, points at `bytes` writable bytes aligned to 8.
            let status = unsafe {
                PdhGetFormattedCounterArrayW(
                    counter,
                    PDH_FMT_DOUBLE,
                    &mut bytes,
                    &mut count,
                    target,
                )
            };
            if status == PDH_MORE_DATA || (status == 0 && target.is_none() && bytes > 0) {
                self.buf.resize((bytes as usize).div_ceil(8), 0);
                continue;
            }
            check("PdhGetFormattedCounterArrayW", status)?;
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
                .collect());
        }
        Err(CollectError::Pdh {
            call: "PdhGetFormattedCounterArrayW",
            status: PDH_MORE_DATA,
        })
    }

    fn read_single(&self, counter: PDH_HCOUNTER) -> Result<f64, CollectError> {
        let mut value = PDH_FMT_COUNTERVALUE::default();
        // SAFETY: writes one formatted value of a counter of this query into `value`.
        check("PdhGetFormattedCounterValue", unsafe {
            PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, None, &mut value)
        })?;
        if !is_valid(&value) {
            return Ok(0.0);
        }
        // SAFETY: PDH_FMT_DOUBLE fills the double member of the union.
        Ok(unsafe { value.Anonymous.doubleValue })
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
