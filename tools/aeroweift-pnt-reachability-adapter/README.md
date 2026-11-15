# AeroWeft PNT Reachability Adapter — Synthetic Slice 0.1

This dependency-free adapter crate converts an upstream, already-qualified PNT/clock claim into a conservative initial position set for the existing `aeroweift-reachability-core` crate.

It is tracked by:
- Symtropy issue #5270 — AEROWEFT-PNT-REACHABILITY-001.
- Research specification: docs/research/aviation/AEROWEFT_PNT_REACHABILITY_001.md.
- Golden fixture: fixtures/aeroweift-pnt-reachability/tiny-pnt-time-envelope-v0.json.

## What it checks and computes

- NORMAL uses the primary deterministic hard-bound claim.
- DEGRADED, INTEGRITY_UNCERTAIN and UNAVAILABLE may use only a separately qualified fallback claim.
- RECOVERING and INDETERMINATE do not produce an admissible hard-bound initial set.
- Statistical prediction/confidence regions are not accepted as deterministic hard bounds.
- Clock sample and evaluation time are both represented as intervals, normalized by Mycelix #435.
- Full sample/evaluation time uncertainty is converted outward into an elapsed-time interval.
- Position is expanded by the supplied signed velocity interval over that entire elapsed-time interval.
- Source claim, clock correlation, profile, frame/datum and Mycelix common-mode snapshot requirements are preserved as explicit inputs.
- Unknown, shared, contradictory, stale, future-time or unqualified evidence fails closed with a typed reason.

## Deliberate limitations

The adapter does not:
- detect GNSS interference or classify the cause of a PNT anomaly;
- convert clock domains or time scales;
- compute common-mode dependencies or decide sensor independence;
- authenticate source signatures or verify source payloads;
- derive velocity bounds from aircraft physics;
- integrate aircraft dynamics or schedule trajectories;
- authorize a flight or clear an airspace conflict.

It only checks that the upstream claims say they are qualified under a declared profile, and carries the claim identifiers into the output. Digest syntax and upstream status fields are not proof of source authenticity or physical truth.

The existing reachability core remains the integrator. This crate does not create a second reachable-set engine.

## Exact-head commands

```sh
cargo fmt --manifest-path tools/aeroweift-pnt-reachability-adapter/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/aeroweift-pnt-reachability-adapter/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/aeroweift-pnt-reachability-adapter/Cargo.toml
```

The authoring environment does not have Cargo/rustc, so no local test pass is claimed. The dedicated workflow is the authority for exact-head formatting, Clippy and locked tests.
