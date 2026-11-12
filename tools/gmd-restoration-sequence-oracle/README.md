# GMD restoration-sequence feasibility oracle

A dependency-free Rust evaluator for a **supplied** top-down, bottom-up or hybrid black-start/restoration schedule. It checks task ordering, shared exclusive crews/tools, active/reactive/apparent-power envelopes and finite net AC energy/reserve against a single declared service-bus boundary.

## What it proves

For a frozen input schedule and a qualified static envelope, the crate can report:
- exact intervals where active kW, reactive kVAr magnitude, apparent kVA or energy reserve is exceeded;
- exact overlap windows for shared exclusive resources;
- unmet prerequisite ordering;
- integrated net AC energy over the declared horizon;
- minimum remaining energy and reserve margin;
- first **planned** completion/pickup milestone by service;
- separate authority and evidence readiness.

A schedule may be arithmetically feasible while awaiting approval. The result always includes `execution_authorized: false`.

## Time and load semantics

- All offsets are elapsed seconds from scenario t0.
- Baseline load is continuous over the horizon.
- A step's transient active/reactive increment applies on its execution interval [start, finish).
- At finish, that transient increment is replaced by the step's steady incremental load, which remains through the horizon.
- Reactive demand is represented as a non-negative magnitude and added conservatively. No cancellation between inductive/capacitive loads is assumed without a separately qualified phase/sign model.
- Active energy integrates net active kW at the same service-bus boundary: kW × seconds / 3600 = kWh.
- The capability envelope's available energy is a finite, owner-qualified total net-AC energy budget over the horizon; the reserve floor must remain unused. It is not an SOC/dispatch/generator-dynamics model.

## Capability boundary

The source envelope must be obtained from qualified specialist owners. This crate does not derive:
- P/Q capability from inverter or generator controls;
- apparent-power capability or current-limits;
- transient inrush, cold-load pickup, frequency/voltage stability or inertia;
- protection selectivity, relay behaviour, synchronization, islanding or black-start physics;
- fuel conversion, auxiliary energy, storage efficiency, generator start and ramp dynamics.

Represent those effects in qualified upstream models or in the supplied, owner-approved envelope and schedule. Do not interpret this arithmetic evaluator as an electrical simulation.

## Composition

- Symtropy POWER-SYSTEM-RESILIENCE-001 #3418 owns GFM, islanding, protection, transient and hardware qualification.
- Symtropy CIVDYN-CASCADE-001 #3894 owns service dependency propagation and degraded service trajectories.
- Symtropy #3401/#3411/#3417 own fuel and matched net-service energy accounting.
- Sol-Atlas #716 owns service-floor continuity and recovery metrics.
- Sol-Atlas #721 owns operator action-window checks.
- Mycelix #6018 owns provenance, authority and offline coordination.

This crate does not duplicate any of those solvers. It exports planned service pickup milestones; actual service delivery must be independently observed/qualified and scored downstream.

## Files

- Input schema: `docs/research/gmd-restoration-sequence/GMD_RESTORATION_SEQUENCE_INPUT_V0.1.schema.json`.
- Synthetic input: `docs/research/gmd-restoration-sequence/GMD_RESTORATION_SEQUENCE_SYNTHETIC_CASE_V0.1.json`.
- Expected result: `docs/research/gmd-restoration-sequence/GMD_RESTORATION_SEQUENCE_SYNTHETIC_RESULT_V0.1.json`.

The CI Python preflight is test-only and recomputes the synthetic interval arithmetic. It is not production code or a full JSON Schema validator.

## Run

    cargo fmt --manifest-path tools/gmd-restoration-sequence-oracle/Cargo.toml -- --check
    cargo test --manifest-path tools/gmd-restoration-sequence-oracle/Cargo.toml --locked --offline
    cargo clippy --manifest-path tools/gmd-restoration-sequence-oracle/Cargo.toml --locked --offline --all-targets -- -D warnings

## Claim ceiling

P0 proves deterministic schedule arithmetic against supplied static envelopes and explicit resource/authority declarations. It does not prove grid stability, safe switching, hardware black-start, operator authorization, actual service energization or regional resilience. Real qualification requires the applicable network/transformer models, protection review, HIL/PHIL and owner-authorized field exercises.
