# Exact inter-pass geometry metrology oracle v0.1

Tracking: [SHIPYARD-GEOMETRY-METROLOGY-ORACLE-001 #4716](https://github.com/Luminous-Dynamics/symtropy/issues/4716)

## What is implemented

A dependency-free Rust 2024 library in an isolated nested Cargo workspace compares synthetic 1D cross-sectional profiles. It checks workpiece/joint/layer/pass identity, source digests, common coordinate frame and exact x-grid, units, timestamp freshness, registration receipt/residual, explicit missingness, scan coverage, unsupported gaps and integrated uncertainty before calculating area change.

It never interpolates across missing points. It keeps three geometry outcomes separate:
- signed net area change;
- positive added-surface area;
- negative/removed-surface area.

Each is represented as **twice the area in µm²** (`area2_um2`) so trapezoid integration avoids half-unit rounding. Zero-crossing segments are split at their linear zero crossing; added and removed area are rounded upward at the final integer quantum, while signed net area remains the exact signed trapezoid sum.

The conservative integrated uncertainty is the trapezoid sum of reference-point bound + observed-point bound + registration residual bound. The output returns the signed net-area interval [net−uncertainty, net+uncertainty].

## Frozen synthetic examples

- Positive-only triangular profile: signed/added area2 = 80.
- Negative-only triangular profile: signed/removed area2 = −80 / 80.
- Mixed crossing profile [0, −2, +2] over x=[0,10,20]: signed net area2 = −20, added area2 = 10, removed area2 = 30.
- Flat unchanged surface: all areas = 0.
- Combined point and registration bounds of 1 + 2 + 1 µm over 20 µm ROI: uncertainty area2 = 160.

These are synthetic integer known answers, not actual BAM geometry or a claimed physical scan resolution.

## Rights and evidence boundary

V0.1 admits only SyntheticContractOnly. The BAM Zenodo raw dataset remains RIGHTS_PENDING in #4643. The CC BY 4.0 paper licence is not a licence for the separate raw archive. No raw scan or measurement value is imported here.

A future measured adapter must provide real source-byte digests, a reproducible registration receipt, calibrated axis units, scan uncertainty and rights-admitted source identity before this library can be used for physical geometry reporting.

## CI and claim ceiling

The focused workflow runs pinned Rust 1.96 formatting, unit tests and Clippy with warnings denied. The oracle is independent of the root Cargo workspace.

A PASS demonstrates deterministic synthetic profile integration and quality-gate behavior only. It does not establish scan calibration, physical weld volume, residual stress, fusion, penetration, NDT, WPS qualification or fully released distortion.
