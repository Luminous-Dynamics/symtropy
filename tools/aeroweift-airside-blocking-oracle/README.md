# AeroWeft Airside Blocking Oracle — Synthetic Slice 0.1

This dependency-free Rust crate is a tiny finite-resource reference oracle for:
- issue #5261, AEROWEFT-AIRSIDE-BLOCKING-001;
- specification PR #5263;
- golden fixture fixtures/aeroweift-airside-blocking/tiny-blocking-cascade-v0.json.

## What it checks

- Five finite capacity-one resources: TLOF, stand, taxi link, charger and terminal holding.
- Half-open interval occupancy and resource-capacity peaks.
- Exact acquisition/release correspondence for each resource reservation.
- Event-ID uniqueness and predecessor causality.
- Explicit blocker identity for downstream-acquire-blocked events.
- A's five-tick and C's six-tick blocking-after-service intervals.
- C's four-tick terminal holding and synthetic -4 energy delta.
- Fail-closed unknown resources, invalid intervals, duplicate capacities, missing blocker witnesses and causal time reversal.

The fixture has a valid, hand-enumerated resource trace. It is a reference answer for one fictional scenario only.

## Deliberate limitations

This is not a generic discrete-event simulation platform, flight scheduler, aircraft model, energy/charging solver, passenger demand model, vertiport design tool or operational UTM/ATC system. The synthetic energy units are not physical battery quantities. The oracle verifies resource/event semantics, not the validity of the aircraft's underlying physical model.

The event trace is an input receipt. Future integration should consume exact schedule/resource events from #5186 and qualified aircraft energy/reserve evidence from Symthaea #6249 rather than inventing those values.

## Exact-head commands

cargo fmt --manifest-path tools/aeroweift-airside-blocking-oracle/Cargo.toml -- --check

cargo clippy --locked --manifest-path tools/aeroweift-airside-blocking-oracle/Cargo.toml --all-targets -- -D warnings

cargo test --locked --manifest-path tools/aeroweift-airside-blocking-oracle/Cargo.toml

Source and tests are authored; no test pass is claimed until the exact-head workflow completes.
