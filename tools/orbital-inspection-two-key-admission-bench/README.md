# Orbital inspection two-key admission benchmark v0.1

A small Rust reference adapter for non-compensatory inspection-action admission.

## Core rule

An INSPECTION action is eligible only when all three upstream dispositions pass for the exact same action scope:

1. a typed physical-safety claim bundle with both finite-horizon keep-out and safe-continuation references, plus a separate external proof-verification disposition;
2. complete post-action event-portfolio disposition;
3. measurement-acquisition admissibility and exact scope coherence.

A failure or unknown on one side cannot be compensated by high information value, low fuel cost, or another positive score. Recovery actions (HOLD / SAFE-RETREAT / ABORT-INSPECTION) may be admitted under the physical-safety binding alone, but they carry no measurement or portfolio claim.

A structurally complete claim bundle is not itself a verified proof. The physical-safety binding records the #7575 claim disposition and the individual finite-horizon and safe-continuation receipt references, then separately requires the external proof-verifier state to be PASS. RISK_ONLY, finite-only, missing-component, unknown or stale verification cannot admit either an inspection or recovery action. The adapter does not implement collision probability, trajectory propagation, a control barrier function, a reachable-tube proof, calibration, time synchronization or measurement uncertainty. It consumes supplied upstream dispositions and checks their action/vehicle/trajectory/horizon/acquisition identities.

## Reproduce

```sh
cargo fmt --manifest-path tools/orbital-inspection-two-key-admission-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-inspection-two-key-admission-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-inspection-two-key-admission-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --locked --manifest-path tools/orbital-inspection-two-key-admission-bench/Cargo.toml --bin orbital-inspection-two-key-admission-bench -- docs/research/orbital-truss/fixtures/two-key-admission-v0_1.json <40-hex-source-commit>
python3 tools/orbital-inspection-two-key-admission-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/two-key-admission-v0_1.json <40-hex-source-commit>
```

The independent Python challenger uses the standard library only.

## Benchmark boundary

The 30 fixtures test independent hard-gate composition, missing/stale/failed receipts, scope mismatches, incomplete post-action event inventory, recovery semantics, invalid digest shape, and the fact that utility/cost never changes admissibility.

SHA-256 fields are references only: source bytes and issuers are not authenticated. The receipt hard-codes physical_validation=NOT_ASSESSED and authority_granted=false. No operational spacecraft command, collision-avoidance guarantee, mission readiness or measurement acceptance is claimed.
