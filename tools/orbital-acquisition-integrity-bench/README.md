# Orbital Acquisition Integrity Benchmark v0.1

**Benchmark:** ORBITAL-ACQUISITION-INTEGRITY-001  
**Evidence class:** synthetic  
**Claim ceiling:** constant relative image-plane motion, exposure and readout bounds only

This standalone Rust evaluator checks a bounded image-acquisition condition using a declared constant relative translational-velocity bound in a local two-axis support-plane basis, a conservative local object-space GSD, exposure duration, shutter mode, rolling-readout duration, timestamp uncertainty, trigger jitter and (when applicable) a structured-light projector pattern window.

The model is intentionally limited to constant translation over a declared support-plane basis. It does not model 6-DoF pose trajectories, perspective changes across depth, angular motion, acceleration, vibration, non-rigid flex, rolling-shutter correction, image deblurring or camera response. The supplied velocity and GSD are evidence-bearing inputs; a pass does not independently validate them.

## Arithmetic and semantics

- Motion is represented as an L1 upper bound on two relative-translation components in the local support-plane basis. Dividing by conservative local object-space GSD converts the bound to pixels; the L1 norm upper-bounds Euclidean displacement.
- Pixel bounds are emitted in micro-pixels. The integer formula is ceil((|vx|+|vy|) * duration_ns / (1000 * GSD_um_per_pixel)).
- Exposure smear, rolling-shutter row-time displacement and timestamp/trigger association error are reported separately, then added conservatively for a combined budget.
- Global shutter requires zero readout duration and uncertainty. Rolling shutter requires height >= 2 and a non-zero full-frame readout.
- The timestamp must refer to the centre-row exposure midpoint for numeric temporal admission. First-row-start and host-arrival references remain INDETERMINATE in v0.1.
- The maximum rolling-shutter row offset is ceil((full first-to-last-row readout duration + readout uncertainty) / 2), relative to the centre-row reference. Readout direction is retained in the receipt, while the symmetric maximum bound is direction-invariant.
- Structured-light pattern windows must contain the complete uncertainty-expanded acquisition interval in the qualified common timebase. A disjoint window is PROJECTOR_PATTERN_MISMATCH; a partial overlap is INDETERMINATE. If the clock-map receipt or timestamp semantics are not qualified, pattern association is not assessed.
- A valid calibration and clock-map receipt must be supplied; the adapter consumes #7339 and #5631 state references but does not authenticate their external bytes.
- The v0.1 model uses one composite acquisition budget, with separate raw bounds retained for exposure, readout and timing. It does not optimize robot speed, illumination or exposure.
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
