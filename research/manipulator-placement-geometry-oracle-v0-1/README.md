# Manipulator Placement Geometry Oracle v0.1

Parent: Symtropy #5762  
Application benchmark: #5732  
Portfolio: #5700 / #5702  
Backend/model comparison: #5727  
Status: standalone analytic Rust reference; runner qualification pending.

## Purpose

Evaluate four-part assembly placements against a frozen fixture specification using:
- 3-D translation error;
- shortest quaternion orientation error;
- seat-depth error;
- conservative error-plus-uncertainty bounds;
- exact workpiece, fixture, frame, tool/mount and calibration revision identity.

The output is only geometric. WithinTolerance is not functional acceptance, production acceptance, proof of strength, safe operation or actuation authority.

## Conservative predicates

For each dimension, the upper bound is the absolute measured error plus the declared measurement uncertainty. A part is WithinTolerance only when all three upper bounds are no greater than their respective declared tolerances.

Measured and target quaternions must be finite and unit-normalized within 1e-6. The shortest angular error uses the absolute quaternion dot product, so q and -q represent the same physical orientation.

## Integrity behavior

- Missing required part observation: Unknown / IncompleteAssembly.
- Wrong or unknown component ID, duplicate observation ID or multiple observations per component: Inconsistent.
- Stale workpiece/fixture/tool/frame/calibration revision: InvalidEvidence.
- Non-finite, zero-norm/non-unit quaternion, negative uncertainty or missing evidence reference: InvalidEvidence.
- Any validly measured pose outside a conservative tolerance: GeometricMismatch.

The evaluator makes no implicit frame transformation and ignores no extra component.

## Run

From this directory:

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo run

No third-party dependencies are required.

## Tests

The synthetic suite covers nominal four-part assembly, tolerance boundaries, uncertainty crossing, quaternion sign equivalence, rotation and seat-depth mismatch, missing/wrong/duplicate parts, stale revisions, invalid numeric fields, order invariance and the robot-self-report negative control.

These fixtures test this geometry contract only. They do not qualify the real Franka workcell or its metrology.

## Ownership and claim ceiling

- #5732 owns the full manipulator assembly task, comparator arms and independent functional acceptance.
- #3605 owns accepted functional units, rejected work and WIP.
- #1607 owns metrology/frame/calibration validity.
- #5488 owns configuration-change applicability.
- #3522/#3531 own physical evidence admissibility.

This crate does not authenticate issuers, independently verify the evidence source, assess material/structural integrity, determine grasp retention, command the robot, authorize actuation, accept production, or declare safety/conformity.
