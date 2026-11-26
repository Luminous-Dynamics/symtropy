# Orbital inspection recourse benchmark v0.1

This crate evaluates a **finite synthetic contingent inspection policy**. It is a small exact policy-tree oracle for deciding which admissible observation action to take next, based only on an already observed outcome history.

## Contract

- The initial candidate-world set and each world’s engineering decision class are evaluator inputs; the policy only sees the public support induced by the outcomes it actually receives.
- An action is eligible only when all required gates are PASS and every candidate world has a known outcome-support mapping.
- FAIL, UNKNOWN, incomplete outcome mapping and unknown outcome support never become positive evidence.
- An outcome branch retains every world for which that outcome is possible. Overlapping outcome support is preserved; it is not replaced with a single most-likely world.
- A terminal leaf is allowed only when all surviving worlds imply the same declared decision class. It does not require identifying a unique physical world.
- Each action may be used at most once along a policy path. Time, energy and action-count budgets are enforced.
- The exact finite search minimizes worst-case duration, then worst-case energy, then worst-case action count. It exposes no weighted safety score.
- Exceeding the state cap returns SEARCH_LIMIT_NOT_PROVEN. A fully enumerated problem without a resolving admissible policy returns ABSTAIN.
- Receipts preserve model, gate and world/outcome mapping digest references. Digests are syntactically checked, not authenticated.

## Reproduce

```sh
cargo fmt --manifest-path tools/orbital-inspection-recourse-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-inspection-recourse-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-inspection-recourse-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --locked --manifest-path tools/orbital-inspection-recourse-bench/Cargo.toml --bin orbital-inspection-recourse-bench -- docs/research/orbital-truss/fixtures/inspection-recourse-v0_1.json <40-hex-source-commit>
python3 tools/orbital-inspection-recourse-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/inspection-recourse-v0_1.json <40-hex-source-commit>
```

The independent Python implementation is test-only. Production code is Rust.

## Non-goals and claim ceiling

This evaluator does not estimate probabilities, likelihoods, or e-processes; qualify the physical world-set; generate camera poses; plan trajectories; compute photometry, geometric visibility, feature conditioning or metrology uncertainty; authenticate evidence; command a robot; measure physical POD; or authorize acceptance of flight hardware. Compose with #6887 for sequential statistical evidence and with the existing acquisition, calibration, geometry, metrology and mission-resource owners. A PASS means only that the declared finite synthetic policy contract and its frozen controls replay as specified.
