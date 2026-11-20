# Power restoration transition audit v0.1

**Owner:** Symtropy #6430  
**Action contract:** #6348 / PR #6350  
**Case contract:** #6346 / PR #6347  
**Claim gate:** #6419 / PR #6427  
**Programme:** #6320 / #5241

## Purpose

A transition receipt's signature and before/after hashes do not establish that the successor state follows from the action proposal. This crate independently replays a narrow, explicitly specified subset of discrete state changes and compares the recomputed state with the supplied successor snapshot.

## Supported action semantics

- set_switch_state: explicit idempotent open/closed state;
- connect_mobile_source: only to a permitted power node that matches the source's current road-node/power-node interface;
- disconnect_mobile_source: only from the node at which that source is currently recorded as connected;
- observation, query, simulation, escalation and wait actions: non-mutating with respect to the tracked live state.

The state snapshot binds exact switch inventory, crew road positions, mobile-source road positions and mobile-source connection state to the frozen case digest and decision epoch.

## Explicitly unsupported

- crew movement and travel-time progression;
- mobile-source movement and route/arrival timing;
- repair progress, work-unit accounting, crew calendars and partial execution;
- electrical power flow, radiality, voltage/thermal/short-circuit/protection qualification;
- service delivery, commissioning and human/operator authority.

An applied action outside the supported subset produces INCONCLUSIVE; it is never guessed or treated as a valid state delta. A failed action also produces INCONCLUSIVE because the current state model cannot exclude partial physical effects. Rejected/not-attempted actions are expected to leave tracked state unchanged.

## What is checked

- case and action proposal are independently validated against the frozen case;
- transition envelope binds case, proposal, epoch and previous/successor state digests;
- before/after snapshots exactly cover the case's switches, crews and mobile sources;
- every proposal action has exactly one execution outcome, with no duplicate or extra action IDs;
- supported applied actions are replayed deterministically;
- any unrequested mutation or missing action-derived mutation is rejected;
- mobile connection must match the declared permitted node and mobile-source road/power interface.

## Run

    cargo fmt --manifest-path tools/power-restoration-transition-audit/Cargo.toml --all -- --check
    cargo clippy --manifest-path tools/power-restoration-transition-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/power-restoration-transition-audit/Cargo.toml --all-targets
    cargo run --manifest-path tools/power-restoration-transition-audit/Cargo.toml --bin power-restoration-transition-audit -- case.json proposal.json previous-state.json successor-state.json transition-envelope.json

CLI exit codes: 0 = supported state delta recomputed; 3 = INCONCLUSIVE for unsupported semantics; 1 = malformed or contradictory evidence; 2 = report serialization failure.

## Integration target

The end-to-end claim verifier #6427 should invoke this semantic audit on each transition artifact before accepting the TransitionEngine attestation as a semantically verified state change. The attestation authenticates who asserted the result; this crate checks a narrow set of state semantics independently.

## Claim boundary

Deterministic discrete state-delta verification for the supported action subset only. No electrical feasibility, physical safety, operation authorization, service delivery or field qualification is established.
