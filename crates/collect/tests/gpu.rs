use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use vestigium_collect::{PdhSampler, gpu_usage, physical_drive};

fn values(list: &[(&str, f64)]) -> HashMap<String, f64> {
    list.iter()
        .map(|(name, value)| (name.to_string(), *value))
        .collect()
}

#[test]
fn gpu_usage_sums_pids_per_engine_and_takes_the_busiest() {
    let reading = gpu_usage(&values(&[
        (
            "pid_100_luid_0x00000000_0x0000D1A2_phys_0_eng_0_engtype_3D",
            30.0,
        ),
        (
            "pid_200_luid_0x00000000_0x0000D1A2_phys_0_eng_0_engtype_3D",
            25.0,
        ),
        (
            "pid_200_luid_0x00000000_0x0000D1A2_phys_0_eng_2_engtype_Copy",
            10.0,
        ),
        (
            "pid_300_luid_0x00000000_0x0000D1A2_phys_0_eng_3_engtype_VideoDecode",
            0.0,
        ),
    ]));

    assert_eq!(reading.system_pct, 55.0);
    assert_eq!(reading.per_pid.get(&100), Some(&30.0));
    assert_eq!(reading.per_pid.get(&200), Some(&25.0));
    assert_eq!(reading.per_pid.get(&300), Some(&0.0));
}

#[test]
fn gpu_instance_without_pid_is_ignored() {
    let reading = gpu_usage(&values(&[
        ("luid_0x00000000_0x0000D1A2_phys_0_eng_0_engtype_3D", 80.0),
        (
            "pid_abc_luid_0x00000000_0x0000D1A2_phys_0_eng_0_engtype_3D",
            70.0,
        ),
        (
            "pid_100_luid_0x00000000_0x0000D1A2_phys_0_eng_0_engtype_3D",
            5.0,
        ),
    ]));

    assert_eq!(reading.system_pct, 5.0);
    assert_eq!(reading.per_pid.len(), 1);
}

#[test]
fn gpu_usage_clamps_to_100() {
    let reading = gpu_usage(&values(&[
        ("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 70.0),
        ("pid_2_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 60.0),
    ]));

    assert_eq!(reading.system_pct, 100.0);
    assert_eq!(gpu_usage(&HashMap::new()).system_pct, 0.0);
}

#[test]
fn physical_drive_parses_instance_names() {
    assert_eq!(physical_drive("0 C:"), Some(0));
    assert_eq!(physical_drive("1 D: E:"), Some(1));
    assert_eq!(physical_drive("12"), Some(12));
    assert_eq!(physical_drive("_Total"), None);
    assert_eq!(physical_drive(""), None);
}

#[test]
fn pdh_sampler_opens_without_gpu_counters() {
    let mut sampler = PdhSampler::open().expect("opening never depends on the GPU object");
    assert_eq!(sampler.sample().unwrap(), None);
    thread::sleep(Duration::from_millis(1_100));

    let reading = sampler
        .sample()
        .unwrap()
        .expect("second collection has rates");

    if let Some(gpu) = reading.gpu {
        assert!((0.0..=100.0).contains(&gpu.system_pct));
        assert!(gpu.per_pid.values().all(|pct| (0.0..=100.0).contains(pct)));
    }
}
