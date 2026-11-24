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
