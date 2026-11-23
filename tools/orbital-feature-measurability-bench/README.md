# Orbital Feature Measurability Benchmark v0.1

**Benchmark:** ORBITAL-FMQ-ANALYTIC-001  
**Evidence class:** synthetic  
**Claim ceiling:** synthetic 2D circle geometric identifiability only

This standalone Rust evaluator computes a normalized geometric information matrix for the center-x, center-y and radius parameters of a 2D circular feature, using unique observed contour support angles. It reports nominal and robust rank/conditioning separately. Repeated observations at the same contour angle do not increase unique geometric support or rank.

The matrix is dimensionless and is not a calibrated statistical Fisher-information model or measurement covariance. No sensor noise model, calibration, GUM uncertainty, feature-fitting error, POD or physical qualification is implemented.

Run:

    cargo fmt --manifest-path tools/orbital-feature-measurability-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/orbital-feature-measurability-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/orbital-feature-measurability-bench/Cargo.toml --all-targets -- -D warnings
    cargo run --quiet --locked --manifest-path tools/orbital-feature-measurability-bench/Cargo.toml --bin feature-measurability-bench -- docs/research/orbital-truss/fixtures/feature-measurement-quality-v0_1.json <40-hex-source-commit>
    python3 tools/orbital-feature-measurability-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/feature-measurement-quality-v0_1.json <40-hex-source-commit>

See [#7043](https://github.com/Luminous-Dynamics/symtropy/issues/7043) and [the research protocol](../../docs/research/orbital-truss/feature-measurement-quality-v0_1.md).
