use std::collections::HashMap;

/// GPU use at one moment: the busiest engine of the machine and, per process, its busiest engine.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GpuReading {
    pub system_pct: f32,
    pub per_pid: HashMap<u32, f32>,
}

/// Folds `GPU Engine` instances, named like "pid_1234_luid_0x0_0x1_phys_0_eng_3_engtype_3D".
pub fn gpu_usage(values: &HashMap<String, f64>) -> GpuReading {
    let mut engines: HashMap<&str, f64> = HashMap::new();
    let mut by_pid: HashMap<(u32, &str), f64> = HashMap::new();
    for (name, &value) in values {
        if let Some((pid, engine)) = parse(name) {
            *engines.entry(engine).or_default() += value;
            *by_pid.entry((pid, engine)).or_default() += value;
        }
    }
    let mut per_pid: HashMap<u32, f32> = HashMap::new();
    for ((pid, _), value) in by_pid {
        let busiest = per_pid.entry(pid).or_default();
        *busiest = busiest.max(percent(value));
    }
    GpuReading {
        system_pct: engines.into_values().map(percent).fold(0.0, f32::max),
        per_pid,
    }
}

/// The process id and the engine (adapter, physical GPU, engine number) of an instance name.
fn parse(name: &str) -> Option<(u32, &str)> {
    let rest = name.strip_prefix("pid_")?;
    let (pid, rest) = rest.split_once("_luid_")?;
    let (engine, _kind) = rest.split_once("_engtype_")?;
    Some((pid.parse().ok()?, engine))
}

fn percent(value: f64) -> f32 {
    value.clamp(0.0, 100.0) as f32
}
