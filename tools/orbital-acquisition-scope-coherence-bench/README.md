# Orbital acquisition scope coherence benchmark v0.1

A Rust-first synthetic adapter that checks whether separately qualified motion, geometric-support, calibration and timing receipts describe the **same acquisition**.

## Contract

The acquisition declares one canonical scope: acquisition ID, camera and sensor mode, lens/intrinsics revision, robot/mount, target/asset revision, camera and asset frames, transform convention, feature/support identity, motion-model revision, calibration scope, clock-map revision, canonical time basis, exposure profile and projector/pattern profile.

Each owner-specific evidence receipt carries the subset of those exact identifiers it claims to qualify, its source receipt digest reference, a source-state disposition and a half-open validity interval in the declared canonical time basis.

- Known identity, configuration, revision, frame, feature or interval mismatch returns OUT_OF_SCOPE.
- Missing scope fields, stale/revoked/contradicted/unknown upstream disposition, missing evidence owner or unresolved applicability returns INDETERMINATE.
- Malformed intervals, duplicate/contradictory bindings, unsupported fields or malformed digest shape return INVALID_INPUT.
- Only a complete exact match across the required owner bindings returns SCOPE_COHERENT.

The interval adapter requires the acquisition interval to be contained strictly within each owner validity interval [start, end). It never compares raw timestamps from different clocks; all binding intervals must already use the same canonical time basis supplied by the clock owner.

SHA-256 strings are syntax-checked references only. The receipt hard-codes source_bytes_authenticated=false and authority_granted=false. Digest equality is not authentication and scope coherence is not measurement truth.

## Reproduce

```sh
cargo fmt --manifest-path tools/orbital-acquisition-scope-coherence-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-acquisition-scope-coherence-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-acquisition-scope-coherence-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --locked --manifest-path tools/orbital-acquisition-scope-coherence-bench/Cargo.toml --bin orbital-acquisition-scope-coherence-bench -- docs/research/orbital-truss/fixtures/acquisition-scope-coherence-v0_1.json <40-hex-source-commit>
python3 tools/orbital-acquisition-scope-coherence-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/acquisition-scope-coherence-v0_1.json <40-hex-source-commit>
```

The independent Python challenger uses only the standard library. It is test-only.

## Ownership and limits

- #7339 owns calibration calculation and time/state validity.
- #5631 owns clock-map uncertainty and temporal ordering.
- #7043 owns feature identity/conditioning.
- #6846 owns datum registration and metrological uncertainty.
- #7384/#7399/#7422 own acquisition motion/timing bounds.
- This crate owns only cross-receipt identity and already-admitted canonical interval coherence.

No issuer signatures, underlying bytes, physical capture, measurement accuracy, POD/NDE, safety or flight readiness are authenticated or qualified here.
