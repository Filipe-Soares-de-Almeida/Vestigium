use argos_core::{GateState, RunGate};
use std::thread;
use std::time::{Duration, Instant};

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
