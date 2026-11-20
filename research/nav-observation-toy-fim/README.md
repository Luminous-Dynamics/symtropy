# Navigation observation toy FIM oracle

**Status:** isolated research prototype source; not compiled or executed in the current work session.  
**Parent:** Symtropy NAV-OBSERVATION-001 #6438.  
**Claim ceiling:** tiny synthetic 2D range-geometry enumeration only.

## Purpose

This crate creates a known-answer, fully enumerable problem for testing whether navigation observation-selection code recognises geometry and observability rather than counting sensors or maximizing an unqualified confidence number.

It is intentionally outside the production workspace. It does not estimate a live position, issue movement commands, query hardware, or claim a deployable planner.

## Mathematical model

Assume a fixed 2D linearization point and independent scalar range observations. A candidate observation has a line-of-sight unit vector

    u_i = [cos(theta_i), sin(theta_i)]^T

and independent Gaussian range noise with standard deviation sigma_i. Its local Fisher-information contribution is

    J_i = (1 / sigma_i^2) u_i u_i^T.

For a selected subset S:

    J(S) = sum_(i in S) J_i.

The resulting 2x2 symmetric matrix is analysed using its eigenvalues, numerical rank, determinant, D-optimal log-determinant, A-optimal trace of the inverse, and condition number. The minimum eigenvalue is also reported as the E-optimal worst-observed direction diagnostic. A rank-one matrix has no finite 2D log-determinant or condition number and is reported as unobservable in one direction. The code does not add a fictitious prior to hide singular geometry.

## Small exact instance

The example contains six candidate range observations with different bearings, noise and abstract integer costs. Under budget 2:

- An exhaustive subset oracle considers every feasible subset (at most 20 candidates by contract).
- A deterministic myopic greedy baseline adds the feasible action with the best immediate information improvement.
- The objective is lexicographic: maximize observable rank; then maximize log(det(J)) for full-rank geometry (or trace(J) if rank-deficient); then minimize abstract cost; then use canonical action IDs as a deterministic tie-break.
- A deliberately costly north-facing observation cannot be selected with the two-unit budget, even though it could make an east-facing observation orthogonal.

The thirteen included tests assert that orthogonal observations produce isotropic full-rank information, collinear and near-collinear observations remain rank deficient, the exhaustive oracle finds the budget-feasible orthogonal diagonal pair, the greedy baseline is strictly suboptimal on the frozen instance, common rotation and uniform noise scaling obey the expected information invariants, the result is invariant to input ordering, and invalid/overflowing input is rejected.

## Scope and limitations

This is a local range-geometry Fisher-information toy, not a full UWB model. It assumes a fixed linearization point, independent Gaussian scalar range noise, known observation geometry, no clock bias, no NLOS, no multipath, no shared calibration, no anchor survey error, no motion, no covariance prior, and no nonlinear relinearization. It does not model acquisition probability or physical reachability. Those are separate extensions and must not be silently inferred.

Fisher information is not itself a calibrated posterior covariance, an integrity bound, or proof that a navigation solution is safe. The larger benchmark must still use matched baselines, independent truth, action/observation replay, common-mode controls, profile-specific calibration, and resource/feasibility reporting.

## Run

    cargo test --manifest-path research/nav-observation-toy-fim/Cargo.toml
    cargo run --manifest-path research/nav-observation-toy-fim/Cargo.toml

This package has no third-party dependencies and declares its own Cargo workspace boundary. The commands are provided but have not been executed here; a Rust toolchain and exact-head CI are still required.


## Why D, A and E optimality must be compared separately

The CLI includes a second four-candidate counterexample with a two-observation budget. D-optimality prefers a pair with a larger determinant but a weakly constrained direction. A- and E-optimality select a more balanced pair. For the frozen numeric values, an independent arithmetic check gives:

- D-optimal pair: det(J) ≈ 500, minimum eigenvalue ≈ 4.751, A-optimal trace(J^-1) ≈ 0.220, condition number ≈ 22.15.
- A-/E-optimal pair: det(J) = 90, minimum eigenvalue = 9, A-optimal trace(J^-1) ≈ 0.211, condition number ≈ 1.11.

This is not a universal ordering of criteria. It demonstrates that the selected geometry depends on the objective and that a benchmark should report the full information spectrum and downstream decision quality rather than calling one criterion universally best. The values are a host-side arithmetic check, not an executed Rust result.
