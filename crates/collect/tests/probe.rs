use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::{
    HighResTimer, ProbeConfig, monotonic_ms, spawn_probe, unbiased_ms, wall_ms,
};
use vestigium_core::{GateState, ProbeTick, RunGate};

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
fn unbiased_clock_advances() {
    let before = unbiased_ms();
    thread::sleep(Duration::from_millis(50));
    let after = unbiased_ms();

    let advanced = after - before;
    assert!(
        (40.0..2_000.0).contains(&advanced),
        "advanced {advanced} ms"
    );
}

#[test]
fn probe_sends_every_tick() {
    let gate = RunGate::new();
    let (tx, rx) = mpsc::sync_channel(4096);
    let started = wall_ms();
    let probe = spawn_probe(
        ProbeConfig::new(Duration::from_millis(100)),
        gate.clone(),
        tx,
    )
    .unwrap();

    thread::sleep(Duration::from_millis(1_000));
    gate.set(GateState::Stopping);
    probe.join().unwrap();

    let ticks: Vec<ProbeTick> = rx.try_iter().collect();
    assert!(ticks.len() >= 6, "only {} ticks", ticks.len());
    assert!(ticks.iter().all(|t| t.lag_ms >= 0.0));
    assert!(ticks.windows(2).all(|w| w[0].wall_ms <= w[1].wall_ms));
    assert!(
        ticks
            .iter()
            .all(|t| t.wall_ms >= started && t.wall_ms <= wall_ms())
    );
}

#[test]
fn paused_probe_sends_nothing() {
    let gate = RunGate::new();
    let (tx, rx) = mpsc::sync_channel(4096);
    let _probe = spawn_probe(
        ProbeConfig::new(Duration::from_millis(20)),
        gate.clone(),
        tx,
    )
    .unwrap();

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
    let (tx, _rx) = mpsc::sync_channel(4096);
    let probe = spawn_probe(
        ProbeConfig::new(Duration::from_millis(50)),
        gate.clone(),
        tx,
    )
    .unwrap();
    thread::sleep(Duration::from_millis(100));

    let asked = Instant::now();
    gate.set(GateState::Stopping);
    probe.join().unwrap();

    assert!(asked.elapsed() < Duration::from_secs(1));
}

#[test]
fn probe_follows_period_change() {
    let gate = RunGate::new();
    let (tx, rx) = mpsc::sync_channel(4096);
    let config = ProbeConfig::new(Duration::from_millis(100));
    let probe = spawn_probe(config.clone(), gate.clone(), tx).unwrap();

    thread::sleep(Duration::from_millis(500));
    let slow = rx.try_iter().count();
    config.set(Duration::from_millis(16));
    thread::sleep(Duration::from_millis(1_000));
    let fast = rx.try_iter().count();
    gate.set(GateState::Stopping);
    probe.join().unwrap();

    assert_eq!(config.period(), Duration::from_millis(16));
    assert!(slow <= 8, "{slow} ticks in 500 ms at 100 ms");
    assert!(fast >= 25, "only {fast} ticks in 1 s at 16 ms");
}
