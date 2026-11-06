# Symtropy Residual Contact Bench

Standalone, read-only validator/evaluator for spatially registered excavation-floor residue maps and machine support footprints.

Implements the geometric measurement contract tracked by [issue #3888](https://github.com/Luminous-Dynamics/symtropy/issues/3888). It is complementary to #3864/#3876: those own generated/recovered/residual mass and fragmentation-to-transport accounting; this crate evaluates where measured residue lies relative to declared support footprints.

## Run

    cargo test --manifest-path crates/domains/symtropy-residual-contact-bench/Cargo.toml

## v0.1 contract

- A map declares one coordinate frame, registration evidence, spatial resolution, bounds, observation time and registration uncertainty.
- Every grid cell is represented exactly once. Duplicate, out-of-bounds or missing cells are rejected; missing coverage is never assumed to be clear floor.
- Each cell carries a non-negative residual-thickness interval and independent measurement, calibration and source references.
- Each support footprint is bound to the exact map frame and an external machine/site acceptance-profile reference. No universal thickness threshold is supplied.
- Per-support cells are classified CLEAR, OBSTRUCTED or AMBIGUOUS by comparing the measurement interval with that support profile's declared threshold.
- If map registration uncertainty exceeds the support profile's declared limit, the entire support footprint is UNKNOWN.
- Residual volume is reported as an interval in mm^3 from measured thickness intervals and map cell area. It is not converted to mass.
- Overlapping support footprints do not duplicate map-wide residual volume because the map sum is evaluated once.

## Non-claims

This crate does not infer mass without independently measured density, nor does it infer contact force, friction, traction, floor competence, stability, safety, machine limits or control actions. A geometric OBSTRUCTED result means only that the measured thickness interval is above a declared profile threshold. It is not a stability or safety verdict.

The bundled fixture is fabricated conformance data. Authored tests are not execution evidence until run on the exact commit.
