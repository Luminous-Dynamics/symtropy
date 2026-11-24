# Orbital Acquisition Integrity Benchmark v0.1

**Benchmark:** ORBITAL-ACQUISITION-INTEGRITY-001  
**Evidence class:** synthetic  
**Claim ceiling:** constant relative image-plane motion, exposure and readout bounds only

This standalone Rust evaluator checks a bounded image-acquisition condition using a declared constant relative image-plane velocity bound, local GSD, exposure duration, shutter mode, rolling-readout duration, timestamp uncertainty, trigger jitter and (when applicable) a structured-light projector pattern window.

The model is intentionally limited to translation represented in image-plane micrometres per second. It does not model 6-DoF pose trajectories, perspective changes, depth variation, angular motion, acceleration, vibration, rolling-shutter correction, image deblurring or camera response. The supplied velocity and GSD are evidence-bearing inputs; a pass does not independently validate them.

## Arithmetic and semantics

- Motion is represented as an L1 upper bound on the two image-plane velocity components. This conservatively upper-bounds Euclidean displacement.
- Pixel bounds are emitted in micro-pixels. The integer formula is ceil((|vx|+|vy|) * duration_ns / (1000 * GSD_um_per_pixel)).
- Exposure smear, rolling-shutter row-time displacement and timestamp/trigger association error are reported separately, then added conservatively for a combined budget.
- Global shutter requires zero readout duration and uncertainty. Rolling shutter requires height >= 2 and a non-zero full-frame readout.
- The timestamp must refer to the centre-row exposure midpoint for numeric temporal admission. First-row-start and host-arrival references remain INDETERMINATE in v0.1.
- The maximum rolling-shutter row offset is ceil((readout duration + readout uncertainty) / 2), relative to the centre-row reference.
- Structured-light pattern windows must contain the complete uncertainty-expanded acquisition interval. A disjoint window is PROJECTOR_PATTERN_MISMATCH; a partial overlap is INDETERMINATE.
- A valid calibration and clock-map receipt must be supplied; the adapter consumes #7339 and #5631 state references but does not authenticate their external bytes.
- No camera pose, geometric visibility, photometric usability, feature-measurability, metrological accuracy, defect detectability, POD or conformity is inferred.

## Run locally

```sh
cargo fmt --manifest-path tools/orbital-acquisition-integrity-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-acquisition-integrity-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-acquisition-integrity-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --locked --manifest-path tools/orbital-acquisition-integrity-bench/Cargo.toml --bin orbital-acquisition-integrity-bench -- docs/research/orbital-truss/fixtures/acquisition-integrity-v0_1.json <40-hex-source-commit>
python3 tools/orbital-acquisition-integrity-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/acquisition-integrity-v0_1.json <40-hex-source-commit>
```

See [#7350](https://github.com/Luminous-Dynamics/symtropy/issues/7350) and the [contract protocol](../../docs/research/orbital-truss/acquisition-integrity-v0_1.md).
