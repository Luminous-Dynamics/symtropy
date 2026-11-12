# Frame-equivariant scan observability v0.1

Tracking: [SHIPYARD-SCAN-REGISTRATION-ASSURANCE-001 #4770](https://github.com/Luminous-Dynamics/symtropy/issues/4770)

## Why low ICP residual is not enough

A small point-to-plane residual only says that the matched surface constraints are locally satisfied. It does not prove that all six rigid-body twist directions are observable. A plane can align with nearly zero residual while leaving two in-plane translations and rotation about its normal unconstrained. A cylinder can leave axial translation and axial roll weak. Feeding either registration into residual-volume or bead-offset calculation as a fully known six-DOF transform can produce confident but wrong geometry.

Zhang et al. (2026 preprint, arXiv:2608.15532) show that ordinary Hessian eigenvalue thresholds and six per-axis weak-direction flags are not invariant to body-frame changes. The information matrix transforms by congruence, preserving nullity and the physical weak subspace but not the conventional eigenvalues or per-axis footprint. The paper proposes a point-displacement metric and a generalized eigenproblem whose dimensionless spectrum is frame-equivariant. This v0.1 crate reproduces that core mathematical contract in synthetic tests; it does not claim a new theorem or welding-specific physical validation.

## Mathematical contract

Twist order: `[omega_x, omega_y, omega_z, v_x, v_y, v_z]`.

For source point p and point-to-plane unit target normal n:

```
J(p) = [-[p]x | I]
H = Σ wᵢ Jᵢᵀ nᵢ nᵢᵀ Jᵢ
M = Σ wᵢ Jᵢᵀ Jᵢ
H v = λ M v
```

H is the point-to-plane information matrix. M is the point-displacement metric: it measures total point displacement induced by a twist. It is **not a registration covariance** and its generalized eigenvalues are not a complete metrology uncertainty budget.

The generalized eigenvalues lie in [0,1] in the exact point-to-plane construction. Small eigenvalues identify a weak subspace. The subspace is the result; six coordinate-axis booleans are not a physically invariant summary.

The implementation diagonally equilibrates M, checks positive definiteness with Cholesky, whitens the generalized eigenproblem, and uses a symmetric Jacobi eigensolver. Non-finite, malformed or rank-deficient inputs fail closed. It does not silently regularize a singular metric into a fully observed result.

## V0 implementation

- Dependency-free Rust 2024 nested Cargo workspace and local lockfile.
- 6×6 H and M construction from synthetic point/normal/weight correspondences.
- Dimensionless generalized eigenvalues and explicit weak-subspace basis.
- Separate reported residual metadata; residual does not decide observability.
- SE(3) adjoint/congruence helpers to test pure body-origin translation, pure body rotation, combined frame metamorphisms, and rebuilding H/M from physically transformed points and normals.
- 21 unit/mutation tests: three-face corner, plane, cylinder, collinear rank failure, high residual/well-conditioned, zero residual/degenerate, SE(3) frame transform with mapped weak-subspace projector, unit rescaling, scene scaling, global weight scaling, invalid normal, zero weight, NaN point, invalid threshold, and metric-not-covariance claim ceiling.
- Dedicated workflow: Rust 1.96 rustfmt, tests, Clippy with warnings denied, and demonstration executable.

## What v0.1 proves—and does not prove

It proves only the mathematical behavior of this implementation over synthetic correspondence sets. The tests do not validate an actual optical scanner, point-correspondence estimator, robot TCP, fixture datum, BAM raw scan, calibration procedure, or physical uncertainty coverage.

The generalized observability spectrum does not replace:
- sensor noise/repeatability characterization;
- intrinsic/extrinsic calibration uncertainty;
- robot/TCP and tool-change calibration;
- fixture/joint datum uncertainty;
- correspondence/outlier/occlusion assessment;
- transform parameter covariance;
- systematic bias and drift;
- propagation into surface points and downstream residual fill geometry.

External priors such as robot pose, fiducials or fixture datums must be separately typed and sourced. A prior may make an otherwise unobservable direction estimable, but must not be laundered as scan-derived information.

## Next benchmark ladder

1. Expand exact synthetic families: intersecting planes, corridor, asymmetric cylinder, clusters, outliers, normal corruption and correspondence duplication.
2. Compare naïve H eigenvalue/per-axis thresholds to generalized H/M eigenspectrum under body-frame origin shifts and rotations. Measure false degeneracy classification changes and weak-subspace projector error.
3. Validate unit/scene-scale metamorphisms and numerical conditioning over realistic coordinate ranges.
4. Use calibrated artefacts to measure transform repeatability, covariance coverage and systematic bias, separately from observability.
5. On rights-cleared BAM scans, assess whether the weak subspace contains directions material to residual groove area/volume and pass offset. Keep five-WID limitations explicit.
6. Integrate the receipt into #4649; if a decision-relevant direction is weak, request a better scan or a separately qualified external prior rather than optimizing on unsupported geometry.

## Standards / comparator boundary

- ISO 10360-13:2021 is a relevant optical 3D coordinate measurement system acceptance/reverification reference, but applicability to robot-mounted line profiling and in-weld surfaces must be assessed.
- ISO 10360-8:2013 has a narrower Cartesian CMM + optical distance sensor scope.
- ISO 23247-7 remains under development; it may inform digital-twin commissioning metadata, not a current published conformance claim.
- NIST's registration error propagation work distinguishes parameter uncertainty and transformed point error; systematic bias needs separate treatment.
- The 2026 arXiv preprint is a research comparator only, not a metrology standard.

## Claim ceiling

`SYNTHETIC_REGISTRATION_OBSERVABILITY_ONLY`. No scanner accuracy, physical weld geometry, WPS, NDT, released-state distortion, metrology conformity or production authority is established.


## Task-specific decision observability (added 2026-11-11)

A global six-DOF test is conservative, but it can also reject a measurement that is sufficient for one particular geometric decision. V0 now adds a decision-gradient assessment.

For a scalar downstream decision f(x) with local twist-coordinate gradient g = ∂f/∂x, and M-orthonormal weak eigenvectors v_j, calculate:

```
weak_energy = Σ_j (gᵀ v_j)²
total_energy = gᵀ M⁻¹ g
normalized_weak_sensitivity = sqrt(weak_energy / total_energy)
```

The ratio is dimensionless and frame-invariant when the geometry, metric and gradient covector are transformed consistently. If x_old = B x_new, the gradient must transform as g_new = Bᵀ g_old. The result is compared with an explicit maximum accepted weak-sensitivity fraction for that specific decision.

Known answers:
- A single plane has three global weak directions. A decision whose gradient asks for in-plane translation has normalized weak sensitivity 1.0 and is blocked.
- A scalar normal-offset decision has normalized weak sensitivity 0.0 and can pass the task-specific weak-subspace check, despite the scan remaining globally three-DOF-degenerate.
- A zero gradient is rejected; it must not be called "fully observable."
- The score remains 1.0 after an SE(3) coordinate change when the gradient is transformed as a covector.

This is a local first-order sensitivity check, not a nonlinear bound on area/volume estimation and not covariance propagation. The downstream geometry model must supply a defensible gradient, units, decision threshold and applicability range. Nonlinear geometry, active constraints, discontinuities and calibration bias need additional bounds. A task-specific PASS only says that this supplied local gradient has sufficiently little component in the detected weak subspace.

The implementation has 26 Rust unit/mutation tests. An independent SciPy replay found the planar in-plane/normal decision fractions at 1.0/0.0 and preserved the in-plane score under frame transformation (1.0 → 1.0). This is a numerical second-implementation check, not a Rust test run.


**Decision-sensitivity is not decision accuracy.** A small weak-subspace projection only says that the supplied first-order gradient is mostly outside the detected weak subspace. It does not bound error from observable-but-noisy directions, sensor calibration, systematic bias, nonlinear curvature, wrong correspondences, surface condition or model discrepancy. A downstream geometry decision still requires a separate propagated uncertainty budget, calibrated measurement evidence, declared output tolerance and applicability envelope. It is unsafe to use a task-specific PASS as a weld acceptance or metrology certificate.
