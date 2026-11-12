# Scan-to-BIM Decision Capture Adapter

A standalone Rust adapter from building-specific capture evidence into the existing finite-state spatial decision engine.

## What it owns

This package does not implement a second VOI engine, next-best-view planner, Pareto evaluator, survey registry, component identity service or adaptive policy solver.

It qualifies the capture-side prerequisites for each action:
- exact survey/source revision;
- sensor profile identity and digest;
- pose revision and evidence digest;
- pose qualification and reachability;
- coordinate frame and frame revision;
- geometric visibility receipt;
- measurement-evidence receipt and task-specific measurement qualification;
- exact model and requirement revision;
- building decision profile.

It then delegates finite state partitions, robust feasibility, Pareto recomputation and action ordering to spatial-resilience-qualifier (#1892/#1922). Any decision-engine recommendation lacking qualified capture evidence makes the adapter fail closed.

## Run

```sh
cargo fmt --manifest-path tools/scan-bim-decision-capture-adapter/Cargo.toml -- --check
cargo test --manifest-path tools/scan-bim-decision-capture-adapter/Cargo.toml
cargo clippy --manifest-path tools/scan-bim-decision-capture-adapter/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/scan-bim-decision-capture-adapter/Cargo.toml
```

## Frozen corpus

Eight synthetic decision worlds are delegated unchanged to the existing #1892 qualifier. Eleven synthetic capture actions bind to sensor, pose, frame, visibility and task evidence. Ten are eligible under the fixture; one stale source-revision action is explicitly unqualified and remains excluded by the delegated decision engine.

The embedded test suite includes negative controls for sensor-profile ID and digest mismatches, as well as:
- externally occluded capture recommended by the generic decision engine;
- stale coordinate-frame revision;
- unreachable/unqualified pose;
- missing action-to-capture binding;
- unknown input fields.


## Determinism and expected-profile binding

Each building case declares the expected sensor-profile ID and SHA-256 in its asset binding. Every capture action must match both values; a syntactically valid but different profile ID or digest is unqualified. This is a synthetic profile-contract check, not proof that the external profile bytes are authentic or that the instrument is calibrated.

Case results are sorted by case ID and capture-gate results by action ID before the decision receipt is hashed. The exact input SHA-256 remains byte-order-sensitive. Regression tests reverse the case array and a case's capture-binding array to verify that equivalent input permutations preserve the semantic core receipt while changing the input digest.

## Claim ceiling

All building IDs, sensor profiles, poses, digests, states, requirements and cost/time bases in the frozen corpus are synthetic. The adapter validates consistency and gate behaviour only. It does not authenticate producer artifacts, prove a pose is physically reachable, calibrate a sensor, establish visibility in a real building, validate measurement uncertainty or establish real acquisition cost. Those claims require independent upstream evidence from #1607/#4292/#4589 and real-world validation.
