# AeroWeft Intent Conformance Oracle — Synthetic Slice 0.1

This dependency-free Rust crate is the finite reference oracle for issue #5284.

Golden fixture:
fixtures/aeroweift-conformance/tiny-intent-conformance-v0.json

## Semantics

- Compares observed 4D occupancy windows against an immutable authorization revision.
- Applies only an explicit profile-owned spatial/time deviation expansion.
- Uses integer coordinates and half-open time windows.
- Classifies observations as definitely inside, definitely outside, or ambiguous.
- Separately accumulates definite, possible, unassessable and unobserved duration.
- Calculates sample age from observed-at time, not receipt time.
- Rejects stale, PNT-unqualified, surveillance-unqualified and frame/datum-mismatched observations.
- Detects superseded, expired or withdrawn authorization and intersecting dynamic restrictions.
- Produces CONTINGENT after the configured contact timeout; no arbitrary global timeout is embedded.
- Preserves authorization, policy, source observation, timestamp and position lineage in the result.

## Deliberate limitations

This does not create operational flight intent, determine legal authorization, issue an alert to an operator, calculate an avoidance manoeuvre, estimate missing aircraft motion, provide separation assurance or control an aircraft.

A 95%/5% EASA U-space deviation guidance profile can be represented by an explicit policy value, but the oracle does not hard-code this value and does not generalize it to other regimes.

## Exact-head commands

cargo fmt --manifest-path tools/aeroweift-intent-conformance-oracle/Cargo.toml -- --check

cargo clippy --locked --manifest-path tools/aeroweift-intent-conformance-oracle/Cargo.toml --all-targets -- -D warnings

cargo test --locked --manifest-path tools/aeroweift-intent-conformance-oracle/Cargo.toml

No local Rust pass is claimed until the exact-head workflow completes.
