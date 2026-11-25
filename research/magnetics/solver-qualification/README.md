# Magnetic solver qualification (v0.1)

A zero-dependency Rust contract for validating provenance, model applicability, convergence records and comparability of external magnetic solver outputs.

This crate does not solve magnetic fields. It sits between solver execution and the existing exact-grid field-map comparator.

## Why this exists

Magnetic solvers answer different physical questions. A homogeneous, prescribed-polarization analytical cuboid is not a nonlinear B-H soft-iron yoke. A face-charge method for uniform isotropic susceptibility must not be presented as an anisotropic or nonlinear material solver. An FEM result is not automatically accurate just because it converged numerically.

Every run must declare:
- solver package, version, source revision and archived source/build digest;
- method / formulation / implementation family;
- physical model class and exact geometry/material/physics-input digests;
- coordinate-frame ID and exact ordered observer-grid digest;
- environment/lockfile digest, output-map digest and numeric precision;
- refinement history, solver error estimate, runtime, memory and unknown count when available.

A digest provides content identity only; it does not authenticate the producer or prove physical truth.

## Supported model/method strata

- Analytical closed form: uniformly prescribed magnetization.
- Independent surface quadrature: uniformly prescribed magnetization, with a refinement record.
- Surface/face charge: magnet plus linear isotropic soft material.
- Cell/volume method: linear isotropic, anisotropic or nonlinear-BH material classes.
- Finite element: all declared numerical model classes in this v0.1 contract.
- Physical metrology: a measured field map associated with the same declared specimen/model identity, but without a numerical convergence record.

This is a conservative admission table for the benchmark, not a claim that a solver package supports every listed case. The exact external API, solver build and model configuration must still be recorded.

## Convergence policy

Numerical methods require a convergence record. Refinement identifiers must increase, characteristic resolution must decrease, each refined level must carry a maximum map change, and the final map digest must match the run output digest.

A run can only be declared converged when it has at least the policy's required number of levels and its final solver-reported map change and absolute error estimate meet the policy thresholds. This checks declared evidence and arithmetic; it does not independently validate the error estimator.

Closed-form analytical references and physical measurements do not require numerical convergence records. Physical measurement uncertainty remains separate from solver error.

## Matched comparison

Two runs are directly comparable only when all of these match exactly:
- physical model class;
- geometry digest;
- material/constitutive-law digest;
- remaining physics-input digest;
- coordinate-frame identity;
- exact ordered observer-grid digest.

Different solver settings, precision, hardware and environments are recorded separately. The pair's lineage label is derived only from declared software and formulation identities. It is not proof of statistical independence or absence of shared code.

## Run

    cargo fmt --manifest-path research/magnetics/solver-qualification/Cargo.toml -- --check
    cargo clippy --manifest-path research/magnetics/solver-qualification/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path research/magnetics/solver-qualification/Cargo.toml

## Explicit non-goals

No magnetostatic field solver, constitutive-law implementation, mesh generator, FEM or BEM backend, physical instrument calibration, material-property estimation, probability/yield inference or component qualification is included. The external Magpylib material-response package is experimental; it is an optional pinned research comparator, not a production dependency.
