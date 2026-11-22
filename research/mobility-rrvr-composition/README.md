# Mobility RRVR Composition Contract v0.1

Dependency-free wrapper crate that composes three already-owned contracts:

- `mobility-restart-reconciliation`: exact old-attempt physical outcome, persistence and fresh-attempt assessment;
- `mobility-downstream-fence`: graph-wide, per-command-holding-layer quiescence evidence;
- `mobility-evidence-verdict`: Confirmed / Refuted / InsufficientEvidence for the exact old-attempt NoEffect claim.

## Cross-owner join

Before returning `ReadyForOwnerNativeRevalidation`, this wrapper requires:
- restart and downstream packets to bind the same logical-effect ID, old attempt ID, old/new boot IDs and old/new controller epochs;
- restart contract to return `FreshAttemptCandidate`;
- every node in the supplied downstream graph to have admissible evidence;
- the evidence evaluator to Confirm the exact required old-attempt NoEffect claim and supplied scope commitment.

The three component results remain visible in the returned assessment; no compensating score or merged Boolean hides a failed plane.

## Claim-specific observer assurance

The separate `observer_gate` module composes [mobility-observer-assurance](../mobility-observer-assurance/README.md) without changing the existing `assess_pre_native` API.

For a fresh-attempt candidate, it requires two distinct, claim-bound observer packets:
- observer quorum for the exact old-attempt NoEffect claim; and
- observer quorum for downstream command-graph quiescence.

Each packet must match the exact claim ID and claim-scope commitment emitted by the base composition assessment. Each quorum is evaluated against its own transitive dependency graph, channel calibration/clock/freshness scope and declared common-cause scenarios. A qualified NoEffect observer quorum cannot compensate for a failed downstream graph quorum, or vice versa.

Already-observed Applied outcomes remain no-retry outcomes, including Applied with IndeterminateCommit persistence. Observer quorum results do not authorize or reverse those outcomes.

## Native owner handoff

See [OBSERVER_GATE_NATIVE_HANDOFF_V0.1.md](OBSERVER_GATE_NATIVE_HANDOFF_V0.1.md) for the exact pre-native decision map and race/fault requirements for integration with #6749.

This composition layer does not replace the native application owner. Its `ReadyForOwnerNativeRevalidation` result is not a dispatch capability; the existing owner still authenticates/consumes the permit and revalidates M1/M2, live binding, controller epoch and queue lease at the native callback.

## Run

```sh
cargo fmt --manifest-path research/mobility-rrvr-composition/Cargo.toml -- --check
cargo test --manifest-path research/mobility-rrvr-composition/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-rrvr-composition/Cargo.toml --all-targets --locked -- -D warnings
```

## Non-claims

`ReadyForOwnerNativeRevalidation` is only a consistency assessment. The real native boundary must still revalidate M1/M2/current mission and configuration, authenticate and atomically consume the one-shot recovery permit, and enforce all final queue/hardware gates. This crate has no native sink or action-capability type.

The wrapper does not authenticate any receipts or evidence, prove command-path inventory completeness, prove true sensor/failure-domain independence, establish physical NoEffect, or qualify a robot, driver, firmware, simulator, HIL setup or safety case.
