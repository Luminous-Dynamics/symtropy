# Symtropy Fragment Morphology Bench

Standalone, read-only validator/evaluator for particle morphology measurements stratified by the exact size bins of an existing fragmentation-to-transport trial.

This crate implements the measurement contract tracked by [issue #3891](https://github.com/Luminous-Dynamics/symtropy/issues/3891). It **consumes** an existing parent case from #3864/#3876; it does not replace the parent PSD, mass balance, transport-capacity, or accepted-output authorities.

## Run

    cargo test --manifest-path crates/domains/symtropy-fragment-morphology-bench/Cargo.toml

## v0.1 design

- Morphology records bind to an exact parent case, interval, trial, machine configuration, and material cohort.
- Each size stratum references a parent PSD bin by index and must exactly match its lower and upper boundaries. The parent bin remains the mass/PSD authority.
- The sample count is partitioned into valid, occluded, ambiguous, and rejected particles. These populations are never silently re-labelled as valid.
- Descriptors are distributions (p05/p25/p50/p75/p95), not a single average shape.
- Each descriptor names its dimensional basis: 2D projection or 3D reconstruction. A 2D circularity is not 3D sphericity.
- Every stratum and descriptor carries source, calibration, method and definition provenance.
- Model-estimated shape cannot be promoted to independent measurement.
- Unsampled parent bins remain explicitly uncovered; the report does not interpolate or impute their morphology.
- Single-view uncontrolled orientation is retained as a warning, not hidden behind an apparently complete distribution.

## Descriptor vocabulary

v0.1 defines:
- `FERET_ASPECT_RATIO_2D` (2D projection, dimensionless, >= 1)
- `CIRCULARITY_2D`, `SOLIDITY_2D` (2D projection, (0, 1])
- `FOURIER_ANGULARITY_2D`, `FOURIER_ROUGHNESS_2D` (2D projection, non-negative method-defined indices)
- `SPHERICITY_3D`, `ASPECT_RATIO_3D`, `CONVEXITY_3D` (3D reconstruction)

Descriptor definitions must be cited by a versioned external reference. The schema does not silently define one universal implementation for every descriptor; inter-method comparison requires the cited definition to match.

## Important boundaries

- Morphology sample counts are particle counts, not mass fractions. Do not multiply them by PSD mass without a declared sampling/weighting design.
- A bin with no morphology sample remains unknown. No average-shape fill is allowed.
- Friction, rolling resistance, wall friction, strength, abrasivity, moisture and density remain independent physical properties. They cannot be inferred from a shape descriptor.
- A 2D projection cannot support a 3D sphericity claim.
- Quasi-3D reconstruction and DEM particle geometry remain model artifacts with reconstruction provenance; they are not independent measurements of the original fragment.
- A validated morphology measurement contract does not predict pneumatic transport. Its value must be demonstrated in held-out transport response experiments.

The bundled parent and morphology cases are synthetic. Authored tests are not execution evidence until run against the exact commit.
