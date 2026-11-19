# Port Nexus Multi-Domain Event Trigger Monitor

A standalone, std-only Rust library and demonstration binary for deciding when an existing port, energy or compute optimizer should be asked to re-plan.

## Run locally

    cargo test --manifest-path tools/port-nexus-event-trigger-monitor/Cargo.toml
    cargo run --manifest-path tools/port-nexus-event-trigger-monitor/Cargo.toml

This package is isolated in a nested Cargo workspace and does not change the Symtropy root workspace membership or dependency graph.

## Signal domains

- vessel/logistics and berth;
- shore power and terminal equipment;
- PCC/feeder grid margins;
- renewables;
- compute queue and deadline slack;
- cooling and seawater thermal margin;
- thermal-store state and reserve.

## Trigger behavior

- Every signal carries a unit, frozen baseline, absolute and relative soft thresholds, maximum age, evidence reference/quality and optional hard bound.
- Critical missing, unknown or stale evidence returns DATA_INCOMPLETE and requires evidence refresh; it never silently returns NO_TRIGGER.
- Future-available observations and decision-time leakage are rejected.
- Hard-boundary violations and stale plans bypass normal cooldown and soft hysteresis.
- Persistent identical hard events do not emit a duplicate request on every sample; a changed hard-reason signature emits a new request.
- Soft events use cooldown and a lower reset threshold to prevent replanning chatter.
- Baseline revision changes reset the old latch/cooldown state.
- Definitions, measurements, reason codes and receipts are canonicalized for deterministic output.
- Repeated snapshot sequence numbers are accepted only as exact same-ID replays; conflicting content at the same sequence is invalid.

## Ownership

This is a read-only trigger monitor. It does not schedule vessels, cranes, compute, energy storage or cooling; it does not mutate a plan or reservation; and it never issues a dispatch or actuator command. It composes with Symtropy #4582, #6211, #2683, #1916, #6209, Symthaea #11572/#11473 and Mycelix #5974.

The current implementation has 21 authored unit tests covering stable conditions, logistics/compute/cooling deviations, hard-boundary bypass, persistent hard events, missing/stale/future evidence, cooldown, hysteresis, baseline changes, ordering and snapshot replay identity. They are authored but not executed in this environment because Cargo/rustc are unavailable.

## Benchmark

Compare fixed 15-minute and fixed hourly schedules, one-sided energy triggers, bidirectional logistics-plus-energy triggers, and the multi-domain trigger. Report re-plan calls together with missed events, stale-plan age, service completion, grid/thermal margin events, objective regret and plan churn.

## Claim ceiling

A result qualifies only deterministic trigger logic over caller-supplied, time-qualified signals. It is not an optimizer, forecast, grid protection function, thermal safety controller, operational authority or evidence that re-planning caused a physical improvement.
