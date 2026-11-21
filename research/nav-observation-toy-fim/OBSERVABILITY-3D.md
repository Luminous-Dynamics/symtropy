# Full 3D observability oracle

The existing FIM prototype began as a 2D local tangent-plane model. This companion adds a 3D scalar-range geometry oracle to prevent a horizontal projection from hiding an unconstrained vertical axis.

For a unit line-of-sight vector u_i ∈ R³ and independent scalar range variance σ_i²:

J = Σ_i (u_i u_iᵀ / σ_i²)

The symmetric 3×3 information matrix is diagonalized using Jacobi rotations. The full spectrum is reported in ascending order; rank is classified against an explicit relative tolerance. D-, A- and E-optimal subset criteria are available for exhaustive search over at most sixteen candidates.

## Why this changes the benchmark

A horizontal ENU projection is useful when the platform and motion are genuinely planar. It is not a full 3D observability certificate. The 2026 GSFN cross-scenario paper explicitly uses an xy-plane projection of the uncertainty ellipsoid for its DoS availability assessment because its UGV motion is predominantly horizontal; it also notes that strongly non-planar scenarios may degrade this approximation.

The 3D oracle keeps all axes in the objective. A set of east/north/diagonal observations has rank two and zero vertical information; adding a vertical observation restores rank three. A near-planar set with a very weak vertical component remains rank deficient at the declared tolerance.

This is a local linearized geometry benchmark, not a 3D estimator, six-degree-of-freedom pose solver, nonlinear SLAM system, integrity monitor or physical navigation qualification.

## Reproducible commands

Run the standalone crate's 3D demonstration with Cargo using binary name observability_3d_demo. It prints planar and full-3D ranks, eigenvalues and selected candidate IDs.

## Metrics to extend

- minimum 3D information eigenvalue and rank;
- condition number and axis-wise uncertainty directions;
- rotation invariance under frame rotation;
- vertical/horizontal availability transitions;
- 3D-to-2D mode transition and return-to-3D recovery;
- information gain per movement/energy cost;
- later extension to 6D pose and 15-state inertial error-state observability, only where the state/measurement model is explicitly derived.

The 3D FIM output is not itself a protection level or calibrated covariance. Source independence and common-mode errors still need the correlated covariance model and held-out audit.


## Shared-error 3D extension

The companion `correlated_3d.rs` uses an explicit measurement covariance:

`R = diag(private_sigma²) + Σ_g tau_g² 1_g 1_gᵀ`

It computes `J = Hᵀ R⁻¹ H` using Cholesky solves and the full 3D line-of-sight vectors. It does not turn group labels into an arbitrary correlation coefficient; one declared latent scalar error is shared by the members of that group, and inconsistent sigmas fail closed. Its six tests cover duplicated common-mode precision, off-diagonal cross-axis information, planar rank loss, permutation invariance, malformed shared-error declarations and budgeted vertical-coverage selection.

Run the correlated example with the isolated crate's `correlated_3d_demo` binary. This is still a narrow local linearized range model: it does not cover multiple latent factors per source, anisotropic vector errors, time-varying correlation, or general pairwise 3D covariance scenarios.


## Benchmark protocol: projected versus full 3D observability

For each matched episode, report both the 2D horizontal projection and the full 3D FIM. Do not score a system as fully observable when it has only two horizontal constraints and no vertical information.

Required profile variants:
- planar UGV with genuinely constrained vertical motion (report the motion-model assumption explicitly);
- UAV ascent/descent and banking, where vertical/attitude geometry changes;
- subterranean ramps, shafts and stairwells;
- surface-to-underwater or depth-mode transitions;
- lander/local-surface frame transitions where body-fixed and local-up directions matter.

For every time window, report rank, all eigenvalues/eigenvectors, minimum eigenvalue, condition number, axis-wise uncertainty orientation, availability state and the time spent in rank-deficient or poorly conditioned states. Track transition and recovery latency, not just average trajectory error.

The 3D model currently describes only a 3D position state with scalar range observations. It is not a six-degree-of-freedom pose or 15-state inertial error-state model. Those require separately derived Jacobians and coupling terms; they must not be approximated by relabelling this 3x3 matrix.

The correlated 3D extension includes one shared scalar additive error per declared group. It still does not model multiple independent latent factors, anisotropic measurement errors, time-varying correlations or arbitrary joint covariance constraints. OCI and full pairwise-correlation uncertainty remain separate research work.
