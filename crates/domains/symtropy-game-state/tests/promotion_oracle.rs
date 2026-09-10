// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Authority-bearing v0.11 semantic-oracle pilot for `symtropy-game-state`.
//!
//! The hosted pilot does not trust the candidate branch's copy of this file.
//! It executes the exact copy from the pull request's target-base commit against
//! the candidate's `src/lib.rs` inside the target-base workspace harness.

use symtropy_game_state::{EventChain, SimulationClock, StableId, StateError};

#[test]
fn stable_id_vector_is_exact_and_reproducible() {
    let first = StableId::derive("resident", 41, 7);
    let second = StableId::derive("resident", 41, 7);

    assert_eq!(first, second);
    assert_eq!(first.as_str(), "resident:50131ad2375108880cec72473bfc9aaf");
}

#[test]
fn fixed_clock_vector_is_exact() {
    let mut clock = SimulationClock::from_hz(20).expect("20 Hz divides one second exactly");
    assert_eq!(clock.step_nanoseconds(), 50_000_000);

    clock.advance_by(40).expect("40 deterministic steps remain representable");
    assert_eq!(clock.tick(), 40);
    assert_eq!(
        clock.elapsed_nanoseconds().expect("elapsed simulation time remains representable"),
        2_000_000_000,
    );
}

#[test]
fn causal_chain_detects_payload_tampering() {
    let mut chain = EventChain::new("firstlight", 99);
    chain
        .append(1, "observation", None, None, Vec::new(), 5_u32)
        .expect("append first event");
    chain
        .append(2, "repair", None, None, Vec::new(), 8_u32)
        .expect("append second event");
    chain.verify().expect("untampered chain verifies");

    let mut events = chain.events().to_vec();
    events[0].payload = 6;
    let tampered = EventChain::from_events("firstlight", 99, events);

    assert!(matches!(
        tampered.verify(),
        Err(StateError::EventHashMismatch { .. })
    ));
}
