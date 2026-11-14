# Analytic Physics Verification v0

This tranche adds independent mathematical checks for the core rigid-body integrator. It is deliberately distinct from replay determinism and from experimental validation.

## Why this exists

A solver can reproduce the same wrong answer bit-for-bit. Determinism is a reproducibility property; it is not a correctness proof. The integrator needs known-solution tests that characterize both the expected answer and the discretization error of its declared numerical method.

The first fixtures are deliberately simple and analytically closed-form:

1. Free inertial motion with no force or damping.
2. Constant acceleration using the current semi-implicit Euler update.
3. Uniform-gravity mechanical-energy drift using the exact signed discretization error of semi-implicit Euler.

These tests are placed inside the symtropy-physics library test module because the repository's existing CI invokes cargo test -p symtropy-physics --lib.

## Exact analytical contracts

### Free inertial motion

For zero acceleration and zero damping: x(T) = x0 + v0*T; v(T) = v0.

Acceptance: position norm error below 1e-10 m after 3 seconds at 240 Hz; velocity norm error below 1e-12 m/s. These are floating-point regression thresholds for this exact constant-velocity path, not general physical tolerances.

### Constant acceleration

The current integrator updates velocity before position: v[n+1] = v[n] + a*h; x[n+1] = x[n] + v[n+1]*h.

For total time T = N*h, v_num(T) = v0 + a*T and x_num(T) = x0 + v0*T + 0.5*a*T*T + 0.5*a*T*h.

Thus final velocity is exact (for constant acceleration), while global position error is the known positive value 0.5*a*T*h. Halving the timestep must halve this error. The test checks three resolutions (30, 60 and 120 Hz), rather than merely accepting an arbitrary endpoint tolerance.

### Uniform gravity: signed mechanical-energy drift

For a body released from rest at height y0, with no damping and constant acceleration -g, semi-implicit Euler gives y_num(T) = y0 - 0.5*g*T*T - 0.5*g*T*h; v_num(T) = -g*T; E_num(T) - E(0) = -0.5*m*g*g*T*h.

The energy drift is negative and first-order in the timestep. The test checks the signed analytical value at 30, 60 and 120 Hz. This is more informative than a generic drift-under-1% threshold because it verifies that the error has the expected sign, scale and convergence order.

## What this proves and does not prove

The fixtures establish exact behavior for the tested free-flight and constant-acceleration paths, plus the expected method error for uniform gravity. They do not validate contact resolution, friction, CCD, angular inertia, constraint stiffness, deformable dynamics, fluid coupling or correspondence to experiments.

The 4D rotation tests already in integrator.rs test a one-step exponential-map reference and long-run SO(4) orthogonality/determinant invariants. Those remain separate checks.

## Next analytical families

- Harmonic oscillator: period and amplitude error vs timestep for a defined force law, with symplectic-energy envelope.
- Kepler two-body orbit: angular momentum, orbital energy and phase error over multiple periods.
- Asymmetric rigid-body rotation: Euler equations with the actual body inertia tensor; this must wait for the current scalar-mean-inertia limitation to be resolved or explicitly scope tests to isotropic bodies.
- Two-body elastic impact: analytical impulse, momentum conservation and restitution error for frictionless spheres.
- Distance constraint: residual-vs-iteration curve and stiffness/substep sweep.
- CCD: analytical time-of-impact residual and grazing/tangent cases.
- Contact stacks: penetration and drift envelopes rather than a single stable/unstable boolean.

## Industry comparison and benchmark framing

- MuJoCo 3.17.0 provides a native deterministic batch API; its changelog notes bit-identical batch results to a plain sequential loop across thread counts. Compare throughput only after matching each model's dynamics and observation contract.
- Rapier 0.36.1 (4 Oct 2026) is the latest tagged stable Rust reference found. Rapier 0.36.0 added soft bodies and an experimental FEM solver; 0.36.1 added reduced-coordinate multibody APIs. Dimforge's Q3 report separately describes intra-island parallelism, persistent graph coloring and parallel broadphase on its optimized development revision. Pin the exact crate tag or development commit, feature flags and solver parameters for every comparison.
- Box2D 3.2 (18 Oct 2026) reports about 3x performance over 3.1 from a bundle of contact recycling, solver refinement, AVX2 dispatch and broadphase changes. Its 2D workload is not directly comparable to Symtropy 3D; the lesson is to isolate each optimization and measure scene-family effects.
- ASME V&V 20 (2009, reaffirmed 2021) frames validation accuracy at specified points using both numerical-solution and experimental-data uncertainty. Preserve that distinction when physical test data is introduced.

References:
- https://mujoco.readthedocs.io/en/3.17.0/changelog.html
- https://github.com/dimforge/rapier/blob/master/CHANGELOG.md
- https://box2d.org/posts/2026/10/box2d-3.2/
- https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer

## Current implementation status

This slice adds three exact-source unit tests in integrator.rs: analytic free-flight trajectory; exact constant-acceleration velocity plus the method's closed-form first-order position error and 2x convergence under timestep halving; and signed gravity energy drift vs the exact semi-implicit Euler error.

No benchmark speed result or external-engine superiority claim is made.