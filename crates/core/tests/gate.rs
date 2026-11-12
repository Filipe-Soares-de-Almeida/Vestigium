use std::thread;
use std::time::{Duration, Instant};
use vestigium_core::{GateState, RunGate};

#[test]
fn running_gate_does_not_block() {
    let gate = RunGate::new();
    assert_eq!(gate.wait_while_paused(), GateState::Running);
}

#[test]
fn paused_gate_blocks_until_resumed() {
    let gate = RunGate::new();
    gate.set(GateState::Paused);
    let waiter = {
        let gate = gate.clone();
        thread::spawn(move || {
            let started = Instant::now();
            (gate.wait_while_paused(), started.elapsed())
        })
    };

    thread::sleep(Duration::from_millis(150));
    gate.set(GateState::Running);
    let (state, waited) = waiter.join().unwrap();

    assert_eq!(state, GateState::Running);
    assert!(waited >= Duration::from_millis(100), "waited {waited:?}");
}

#[test]
fn stopping_releases_paused_waiters() {
    let gate = RunGate::new();
    gate.set(GateState::Paused);
    let waiter = {
        let gate = gate.clone();
        thread::spawn(move || gate.wait_while_paused())
    };

    thread::sleep(Duration::from_millis(50));
    gate.set(GateState::Stopping);

    assert_eq!(waiter.join().unwrap(), GateState::Stopping);
    assert_eq!(gate.state(), GateState::Stopping);
}

#[test]
fn a_timed_wait_gives_up_while_still_paused() {
    let gate = RunGate::new();
    gate.set(GateState::Paused);
    let started = Instant::now();

    let state = gate.wait_while_paused_for(Duration::from_millis(100));

    assert_eq!(state, GateState::Paused);
    assert!(started.elapsed() >= Duration::from_millis(90));
}

#[test]
fn a_timed_wait_returns_when_resumed() {
    let gate = RunGate::new();
    gate.set(GateState::Paused);
    let waiter = {
        let gate = gate.clone();
        thread::spawn(move || gate.wait_while_paused_for(Duration::from_secs(10)))
    };

    thread::sleep(Duration::from_millis(50));
    gate.set(GateState::Running);

    assert_eq!(waiter.join().unwrap(), GateState::Running);
}

#[test]
fn a_running_wait_lasts_the_whole_timeout() {
    let gate = RunGate::new();
    let started = Instant::now();

    let state = gate.wait_while_running_for(Duration::from_millis(100));

    assert_eq!(state, GateState::Running);
    assert!(started.elapsed() >= Duration::from_millis(90));
}

#[test]
fn a_running_wait_ends_when_the_gate_stops() {
    let gate = RunGate::new();
    let waiter = {
        let gate = gate.clone();
        thread::spawn(move || {
            let started = Instant::now();
            (
                gate.wait_while_running_for(Duration::from_secs(10)),
                started.elapsed(),
            )
        })
    };

    thread::sleep(Duration::from_millis(50));
    gate.set(GateState::Stopping);
    let (state, waited) = waiter.join().unwrap();

    assert_eq!(state, GateState::Stopping);
    assert!(waited < Duration::from_secs(5), "waited {waited:?}");
}
