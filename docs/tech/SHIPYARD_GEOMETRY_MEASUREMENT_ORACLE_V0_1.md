# Shipyard geometry measurement oracle v0.1

Tracking: [SHIPYARD-GEOMETRY-MEASUREMENT-ORACLE-001 #4895](https://github.com/Luminous-Dynamics/symtropy/issues/4895)

## Scope

This crate implements deterministic qualification and comparison for synthetic 2D cross-sectional profiles in integer micrometres. It is an independent measurement gate between a scan and inter-pass planning. It does not perform 3D point-cloud registration or infer weld quality.

## Measurement identity

Every observed profile is bound to:
- scan ID and 64-hex SHA-256 of the source bytes;
- 64-hex registration-receipt digest;
- registration receipt that repeats the scan digest, receipt digest and target frame;
- declared coordinate frame and units;
- strictly increasing x stations;
- per-point uncertainty.

A registration receipt for another scan or a different target frame is rejected. Missing uncertainty is not zero. Gaps are evaluated by spatial interval, not by the number of points elsewhere in the profile.

## Geometry calculations

The reference and observed profiles must share identical x stations in v0.1. For each station, the signed vertical error is observed z minus reference z.

Absolute cross-sectional difference area is integrated exactly for a piecewise-linear difference profile. For two stations separated by Δx:
- same-sign or zero-crossing endpoint values: trapezoidal absolute area;
- opposite-sign values: split at the linear zero crossing and sum the two triangle areas.

Each segment area is rounded to the nearest integer µm² using integer arithmetic; ties round upward. The uncertainty-expanded upper-area metric integrates |observed-reference| + reference uncertainty + observed uncertainty at each station.

Coverage is calculated from the declared x-domain and only counts spans no wider than a separate coverage-gap threshold. The maximum gap is evaluated against its own, usually larger, hard limit and includes domain-end gaps and internal gaps. This keeps coverage and worst-gap checks independent: a profile may have no catastrophic gap while still having too little usable coverage.

## Frozen known answers

The 16-case corpus has 3 positive numeric controls and 13 negative controls:
- exact match: 0 µm max/mean deviation, 0 µm² observed area, 6,000 µm² uncertainty-expanded upper area;
- uniform +1,000 µm offset: max/mean deviation 1,000 µm, area 2,000,000 µm², expanded upper area 2,006,000 µm²;
- sign-crossing profile: max deviation 100 µm, mean 67 µm, area 100,000 µm², expanded upper area 156,000 µm².

Negative controls cover long gaps, wrong frame, scan digest mismatch, transform-receipt mismatch, excessive registration residual, high or missing uncertainty, duplicate/non-monotonic stations, mismatched station grids and invalid domain, coverage below threshold without a gap-limit violation, and points outside the declared domain.

## Relationship to the rest of the programme

- #4649 consumes the qualification result only when its scan and transform digests still match the planner observation.
- #4643 may eventually supply lawful BAM scan profiles after the raw archive licence is resolved and the real scanner/robot frame is qualified.
- #4614 remains responsible for physical-model VVUQ; geometric measurement does not validate thermal/residual-stress outputs.
- #4633 remains responsible for fixture/restraint/release and final free-state distortion.

## CI

The standalone Rust 2024 crate runs pinned Rust 1.96 formatting, unit/mutation tests, Clippy with warnings denied and the frozen 14-case corpus replay. It has an isolated nested Cargo workspace and lockfile; no root Cargo workspace changes.

## Claim ceiling

PASS means only that the supplied synthetic 2D profiles satisfy identity, frame, registration, coverage, uncertainty and deterministic geometry-comparison rules. It does not establish scanner calibration, 3D registration accuracy, physical bead geometry, NDT acceptance, weld quality, thermal/residual stress or released-state distortion.
