# Generalized-coordinate covariance reference v0.1

This is a standalone, dependency-free Rust research reference for one constrained 3D dumbbell step expressed in two different generalized-coordinate bases.

It is intentionally separate from Symtropy's production solver and from the existing SO(3) momentum-map benchmark. It answers a different question:

> Does the same discrete mechanics problem produce the same physical step when the generalized coordinates are changed by an arbitrary admissible dense, non-orthogonal basis map?

## Why this matters

Rotation equivariance and coordinate-representation covariance are not the same property.

- **Physical symmetry:** rotate the physical configuration and its physical forces; the equations should commute with that group action.
- **Coordinate covariance:** re-describe the same physical configuration using a new coordinate basis. The numerical implementation must transform its metric, force covectors and constraint maps consistently.

A system can pass generic SO(3) metamorphic tests while still mishandling the mass matrix or dual quantities after a non-orthogonal coordinate change.

For q = S y, the same mechanical problem requires:

```text
q       = S y
v_q     = S v_y
M_y     = S^T M_q S
Q_y     = S^T Q_q
p_y     = S^T p_q
g_y(y)  = g_q(S y)
J_y     = J_q S
A_y     = S^-1 A_q S
```

The canonical one-form remains unchanged:

```text
p_q^T dq = p_y^T dy
```

The transformed physical symmetry generator is generally not Euclidean-skew. Instead:

```text
A_y^T M_y + M_y A_y = 0
```

For a finite action R:

```text
R_y = S^-1 R_q S
R_y^T M_y R_y = M_y
```

## Executable slice

The reference solves the midpoint constrained equations directly in a six-dimensional generalized-coordinate representation.

The state contains two three-dimensional point masses joined by one massless rigid link. The mass metric is a full 6×6 matrix, and the constraint Jacobian is a 3×6 map.

The midpoint equations are:

```text
M (v+ - v) = h (Q + lambda J^T r_bar)
q+          = q + h v_bar
v_bar       = (v+ + v)/2
r_bar       = J (q+ + q)/2
r_bar^T J v_bar = 0
```

The multiplier is solved through the three-dimensional Delassus reduction. The dense-coordinate branch uses the transformed mass metric and transformed force/constraint maps directly. It does **not** transform its inputs back to Cartesian coordinates and call the canonical step routine.

The metric is admitted once. A small declared antisymmetric serialization defect is recorded, the metric is exactly symmetrized for the effective solve, and all solves and energy accounting use that same effective metric. Larger asymmetry is rejected. This avoids a split contract where Cholesky validates one matrix but the solver uses another.

## Frozen dense basis changes

The two basis families are products of integer unit-triangular matrices:

```text
S = L U, det(S) = 1
```

They are dense, non-orthogonal and exactly unimodular. Their floating-point condition numbers are:

| Basis family | det(S) | cond(S) | cond(S^T M S) |
|---:|---:|---:|---:|
| 0 | 1 | 387.327 | 206,372 |
| 2 | 1 | 210.941 | 60,244 |

These are useful conditioning stresses, not extreme or arbitrary-coordinate coverage.

## Independent numerical reference

A separate NumPy implementation recomputed the transformed equations without calling the Rust implementation.

Frozen case:
- masses: 1.3 kg and 2.1 kg;
- initial link vector: (0.8, 0.8, 0.4) m, length 1.2 m;
- off-axis angular velocity: (0.2, -0.3, 0.5) rad/s;
- nonzero center-of-mass velocity;
- one-sided constant force on the first mass: (0.18, -0.07, 0.11) N;
- timestep: 0.02 s.

The force deliberately excites the constraint reaction. The canonical reference obtains a multiplier of approximately -0.2410915652007, with a raw midpoint constraint residual of -3.75e-18 (normalized by the declared fixture scale: 5.21e-20).

| Basis family | Lifted position difference | Lifted velocity difference | Multiplier difference |
|---:|---:|---:|---:|
| 0 | 5.02e-15 | 1.34e-14 | 1.62e-13 |
| 2 | 2.68e-15 | 4.05e-15 | 5.04e-14 |

Additional independent negative controls:
- applying the inverse-basis force law instead of the covector law changes the lifted position by approximately 1.58e-3;
- using the inverse-congruence mass matrix changes the lifted position by approximately 3.48e-2 for basis family 2;
- using J S^-1 rather than J S changes the lifted position by approximately 1.08e-3;
- using the wrong generator conjugation changes the metric-skew defect from approximately 2.9e-15 to approximately 0.707.

For a generic SO(3) action, the independently transformed finite action preserved the transformed mass metric to approximately 2.1e-15 (family 2) and 3.8e-14 (family 0).

These are **independent equation-level NumPy results**, not output from the committed Rust tests, Rust compiler, Cargo, or CI.

## Authored Rust regressions

The current source includes nine synthetic Rust test functions covering:

1. dense unimodular coordinate covariance with a nonzero multiplier;
2. lifted position, velocity, linear momentum and angular-momentum agreement;
3. metric-noise canonicalization and rejection outside tolerance;
4. incorrect mass-metric congruence;
5. force covector transformation and work invariance;
6. physical SO(3) metric isometry after dense re-basing;
7. discrete-step commutation with a physical rotation in the dense basis;
8. incorrect constraint-Jacobian re-basing;
9. incorrect generator conjugation;
10. singular coordinate-change rejection.

The Rust tests are authored but have not yet been compiled or executed in the local editing environment.

## Reproduction

From the repository root:

```bash
cargo fmt --manifest-path research/generalized-coordinate-covariance-001/Cargo.toml --all -- --check
cargo test --locked --manifest-path research/generalized-coordinate-covariance-001/Cargo.toml
cargo clippy --locked --manifest-path research/generalized-coordinate-covariance-001/Cargo.toml --all-targets -- -D warnings
```

The dedicated workflow pins Rust 1.96.0, checks formatting, runs tests and runs strict Clippy on the exact triggering revision. A queued workflow is not a pass.

## What this does not establish

This is a bounded one-link, two-point-mass, one-step reference. It does not qualify arbitrary generalized-coordinate charts, singular chart transitions, general multibody systems, friction/contact, nonlinear constitutive models, adaptive integration, physical calibration, or production runtime performance.

It is not a new pHDAE or RATTLE method and does not claim superiority over mature simulation systems. The next integration step is to compose this fixture with the existing 3D pHDAE/RATTLE and coupled-constraint work, while adding a separately admitted dense-coordinate reference in the shared physics corpus.

## External comparison snapshot — 2026-11-11

Use matched model semantics, not API similarity:

- **MuJoCo 4.0.0** (released 2026-11-06) is the current MuJoCo baseline. Its documented discrete dynamics, constraint solve and effective-metric treatment make it an important reference. MuJoCo 3.17.0's batch API is a distinct throughput/determinism comparator, not a solver-accuracy result.
- **Drake 1.58.0** is the current pinned MultibodyPlant/spatial-algebra candidate in the Symtropy physics programme. Its frame-labelled spatial types and SAP ecosystem are relevant to multi-body constraints, but a dense arbitrary coordinate transformation still needs a specific mapped experiment.
- **Pinocchio 4.1.0** is an important Lie-group and articulated-body baseline, with constrained-dynamics APIs and configuration/tangent-space distinctions. Treat it as a robotics dynamics library; do not imply its API alone is an equivalent standalone time-stepping experiment.
- **Project Chrono 10.0.0** is a multiphysics and multibody baseline with implicit integrator/Jacobian developments. Match the exact constraint and solver settings.
- **Ceres Solver's Manifold API** is a relevant local-parameterization and tangent-Jacobian comparator, not a constrained-mechanics engine.

No external system has been run on this frozen fixture yet. The benchmark must first map its model, constraints, force ports, integration rule, units, precision and solver tolerances. Do not compare unmatched integrators and call the result coordinate covariance.

Current upstream references:
- MuJoCo releases: https://github.com/google-deepmind/mujoco/releases
- Drake 1.58.0 notes: https://drake.mit.edu/release_notes/v1.58.0.html
- Pinocchio 4.1.0 releases: https://github.com/stack-of-tasks/pinocchio/releases
- Project Chrono 10.0.0 announcement: https://projectchrono.org/news/
- Ceres manifold API: https://ceres-solver.readthedocs.io/latest/nnls_modeling.html
