# Orbital Photometric Usability Benchmark v0.1

A standalone Rust benchmark for synthetic normalized sensor response under caller-declared directional illumination, material BRDF and sensor-mode dynamic range.

It separates geometric disposition (an input), Lambertian / Blinn–Phong / Cook–Torrance GGX response, under-exposure, usable signal, saturation, alternate exposure modes and missing-parameter abstention.

The eight frozen controls cover diffuse usability, specular saturation, shadow/zero illumination, dark under-exposure, roughness-dependent partial usability, alternate exposure modes, unknown material parameters and geometry rejection despite photometric usability.

Run:
    cargo test --locked --manifest-path tools/orbital-photometric-usability-bench/Cargo.toml
    cargo run --quiet --locked --manifest-path tools/orbital-photometric-usability-bench/Cargo.toml --bin orbital-photometric-usability-bench -- tools/orbital-photometric-usability-bench/docs/photometric-usability-v0_1.json <40-hex-source-commit>
    python3 tools/orbital-photometric-usability-bench/tests/photometric_reference_oracle.py tools/orbital-photometric-usability-bench/docs/photometric-usability-v0_1.json <40-hex-source-commit>

The model uses one declared directional light and a caller-supplied illumination factor. That factor is not ray-traced shadowing. All values are synthetic model inputs, not measured material or calibrated camera parameters.

Claim ceiling: synthetic normalized BRDF response and declared signal thresholds only. No calibrated camera radiometry, real reflectance, geometric feature measurability, dimensional uncertainty, defect detection, POD, NDE or structural/flight qualification is inferred.


## v0.2 — RGB camera/projector photometric visibility matrix

The v0.2 contract is an additional matrix gate, not a replacement for the scalar v0.1 BRDF model. For each sample it consumes four upstream outcomes: camera FOV, projector FOV, camera depth-of-field and camera line-of-sight. This module does not calculate these geometric preconditions; it preserves PASS / FAIL / INDETERMINATE exactly as supplied.

The RGB rule is applied independently to caller-supplied normalized linear RGB:
- every channel must be at or below the declared saturation ceiling;
- at least one channel must be at or above the declared darkness floor;
- all channels below the darkness floor are UNDEREXPOSED;
- any channel above saturation is SATURATED;
- a known geometry failure remains GEOMETRY_REJECTED even when RGB is usable or unknown;
- missing RGB response or an indeterminate geometry precondition remains INDETERMINATE.

The frozen ten-case corpus covers one-channel saturation, all-channel darkness, camera FOV failure, projector FOV failure, depth-of-field failure, unknown camera line-of-sight, unknown RGB response, mixed sample partial usability, and inclusive threshold boundaries. It has its own Rust CLI and independent standard-library Python oracle.

The matrix uses normalized linear RGB in v0.2 as an explicitly declared input convention. It is not a byte-for-byte reproduction of Staderini's rendered RGB pipeline, and it does not infer material parameters or calculate camera/projector geometry. A later calibrated renderer/measurement adapter must establish color space, gamma/transfer function, channel response, exposure and ray-tested illumination before physical interpretation.

Run:

    cargo run --quiet --locked --manifest-path tools/orbital-photometric-usability-bench/Cargo.toml --bin orbital-photometric-rgb-bench -- tools/orbital-photometric-usability-bench/docs/photometric-rgb-v0_2.json <40-hex-source-commit>
    python3 tools/orbital-photometric-usability-bench/tests/rgb_reference_oracle_v0_2.py tools/orbital-photometric-usability-bench/docs/photometric-rgb-v0_2.json <40-hex-source-commit>

This is a synthetic acquisition-usability contract only. The geometry flags are consumed upstream results, not new geometry evidence, and caller-supplied RGB values are not calibrated camera measurements.
