# Orbital inspection safety-claim benchmark v0.1

This Rust-first synthetic reference validator checks whether a spacecraft inspection action has the two distinct deterministic safety components required by its declared policy:

1. a complete finite-horizon keep-out exclusion claim; and
2. a safe-continuation claim bound to the action terminal epoch, expressed as either a passive invariant set or a separately qualified abort-reachable set.

Both claims must bind the exact same action, vehicle/configuration, protected object, keep-out geometry, reference frame, dynamics, initial state, uncertainty set, time basis and declared operational clearance.

## Critical distinction

A finite-horizon trajectory that avoids the keep-out region does not prove a safe terminal state. A passive-safe terminal state does not excuse a keep-out violation during the action. A Pc/Nc/confidence-interval result is a probabilistic risk assessment and receives RISK_ONLY; it is never promoted to a deterministic shield.

The finite-horizon claim must provide a complete temporal enclosure and a lower bound on minimum clearance at least as large as the action's declared required clearance. The continuation claim must bind to the action end and include a separately referenced invariant/abort-set proof.

The validator checks declared structure, exact scope, temporal coverage, claim type and declared numerical margin. It does **not** reproduce or authenticate the external dynamics proof, propagator, invariant set or source bytes.

The frozen corpus contains 26 synthetic cases, including exact half-open interval boundary controls. The Rust tests and independent Python oracle are required to agree on the full receipt.

## Reproduce

```sh
cargo fmt --manifest-path tools/orbital-inspection-safety-claim-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-inspection-safety-claim-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-inspection-safety-claim-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --locked --manifest-path tools/orbital-inspection-safety-claim-bench/Cargo.toml --bin orbital-inspection-safety-claim-bench -- docs/research/orbital-truss/fixtures/inspection-safety-claim-v0_1.json <40-hex-source-commit>
python3 tools/orbital-inspection-safety-claim-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/inspection-safety-claim-v0_1.json <40-hex-source-commit>
```

The independent Python oracle uses the standard library only.

## Claim boundary

Receipt digests are syntax-checked references, not authenticated source bytes or issuers. SAFETY_CLAIMS_COMPLETE means only that the declared synthetic component receipts match and satisfy the adapter's declared contract. It does not establish actual spacecraft collision avoidance, proof correctness, numerical-model validity, mission authority or flight readiness.
