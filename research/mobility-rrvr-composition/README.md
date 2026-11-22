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

## Run

```sh
cargo fmt --manifest-path research/mobility-rrvr-composition/Cargo.toml -- --check
cargo test --manifest-path research/mobility-rrvr-composition/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-rrvr-composition/Cargo.toml --all-targets --locked -- -D warnings
```

## Non-claims

`ReadyForOwnerNativeRevalidation` is only a consistency assessment. The real native boundary must still revalidate M1/M2/current mission and configuration, authenticate and atomically consume the one-shot recovery permit, and enforce all final queue/hardware gates. This crate has no native sink or action-capability type.

The wrapper does not authenticate any receipts or evidence, prove command-path inventory completeness, prove true sensor/failure-domain independence, establish physical NoEffect, or qualify a robot, driver, firmware, simulator, HIL setup or safety case.
