# Orbital projective acquisition benchmark v0.1

A Rust-first synthetic evaluator for conservative image-space displacement bounds caused by bounded **relative** camera-to-target translation and rotation over exposure and rolling-shutter row times.

## Model

For a point with camera-frame support (Z \ge Z_{min}), (|X|\le X_{max}), (|Y|\le Y_{max}), and radius at most (R_{max}), bound relative point speed by:

`V = V_translation + Omega * R_max`

with the angular contribution rounded upward from micro-radians/s times micrometres. For elapsed time `dt`, outward-round the point displacement `DeltaP = ceil(V * dt)`. The perspective displacement bound is:

`DeltaU <= fx * DeltaP * (Zmin + Xmax) / Zmin^2`

and likewise for vertical displacement using `fy` and `Ymax`. The implementation uses checked integer arithmetic and ceilings every division. It calculates exposure integration and first-to-last row midpoint span separately, then sums their pixel bounds for a conservative frame-wide bound.

## Reproduce

```sh
cargo fmt --manifest-path tools/orbital-projective-acquisition-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-projective-acquisition-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-projective-acquisition-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --locked --manifest-path tools/orbital-projective-acquisition-bench/Cargo.toml --bin orbital-projective-acquisition-bench -- docs/research/orbital-truss/fixtures/projective-acquisition-v0_1.json <40-hex-source-commit>
python3 tools/orbital-projective-acquisition-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/projective-acquisition-v0_1.json <40-hex-source-commit>
```

The frozen corpus contains 11 synthetic cases, including invalid depth and arithmetic-overflow controls. The Python oracle uses exact `fractions.Fraction` arithmetic and standard library only. It is test-only.

## Scope and limits

This checks supplied upper bounds; it does not estimate or validate relative motion, solve camera calibration, synchronize clocks, interpolate robot joints, reconstruct trajectories, deblur images, qualify feature measurability, infer POD or certify a measurement. The minimum depth and coordinate/radius envelope must hold throughout the acquisition interval. Input digest fields are references only, not authenticated issuer evidence.

The model assumes a pinhole projection bound and an upper bound on relative point speed for the entire time span. It intentionally uses a conservative component-wise displacement envelope. Acceleration/vibration, lens distortion, non-pinhole optics, changing intrinsics, flexure, occlusion and feature-local shape changes are not modeled. Integrate only after the relevant upstream calibration, timing and motion-model receipts are qualified.
