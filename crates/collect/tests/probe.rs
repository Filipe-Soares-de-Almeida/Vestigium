use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use argos_collect::{HighResTimer, monotonic_ms, spawn_probe};
use argos_core::{GateState, RunGate};

#[test]
fn timer_waits_at_least_the_period() {
    let timer = HighResTimer::new().unwrap();
    let started = Instant::now();

    for _ in 0..10 {
        timer.wait(Duration::from_millis(20)).unwrap();
    }

    let elapsed = started.elapsed();
    assert!(elapsed >= Duration::from_millis(195), "took {elapsed:?}");
    assert!(elapsed < Duration::from_millis(2_000), "took {elapsed:?}");
}

#[test]
fn monotonic_ms_advances() {
    let before = monotonic_ms();
    thread::sleep(Duration::from_millis(30));
    let after = monotonic_ms();

    assert!(after - before >= 25.0, "advanced {} ms", after - before);
}

#[test]
fn probe_reports_ticks() {
    let gate = RunGate::new();
    let (tx, rx) = mpsc::channel();
    let probe = spawn_probe(Duration::from_millis(50), gate.clone(), tx).unwrap();

    thread::sleep(Duration::from_millis(1_000));
    gate.set(GateState::Stopping);
    probe.join().unwrap();

    let lags: Vec<f32> = rx.try_iter().collect();
    assert!(lags.len() >= 10, "only {} ticks", lags.len());
    assert!(lags.iter().all(|&lag| lag >= 0.0));
}

#[test]
fn paused_probe_sends_nothing() {
    let gate = RunGate::new();
    let (tx, rx) = mpsc::channel();
    let _probe = spawn_probe(Duration::from_millis(20), gate.clone(), tx).unwrap();

    gate.set(GateState::Paused);
    thread::sleep(Duration::from_millis(200));
    let _in_flight = rx.try_iter().count();
    thread::sleep(Duration::from_millis(500));

    assert_eq!(rx.try_iter().count(), 0);
    gate.set(GateState::Stopping);
}

#[test]
fn stopping_ends_the_probe_thread() {
    let gate = RunGate::new();
    let (tx, _rx) = mpsc::channel();
    let probe = spawn_probe(Duration::from_millis(50), gate.clone(), tx).unwrap();
    thread::sleep(Duration::from_millis(100));

    let asked = Instant::now();
    gate.set(GateState::Stopping);
    probe.join().unwrap();

    assert!(asked.elapsed() < Duration::from_secs(1));
}
