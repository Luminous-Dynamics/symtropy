# Discrete effective-metric contact step — research seed

This standalone Rust crate is a small reference implementation for a discrete constrained step in which inertia, damping, position stiffness and unilateral contact are coupled through one effective metric.

## Mathematical contract

For a linearized backward-Euler force model:

- M (v_next - v) = h (f - D v_next - K (q_error + h v_next)) + J^T p
- M_hat = M + h D + h^2 K
- M_hat v_next = M v + h f - h K q_error + J^T p

For each frictionless unilateral contact:

- J v_next >= -gap / h
- p >= 0
- p dot (J v_next + gap/h) = 0

The contact Delassus matrix is J M_hat^-1 J^T. It deliberately uses the same M_hat as the unconstrained step.

## Why this exists

A simulator's step map is the object optimized by a discrete-time controller. Comparing one engine's continuous acceleration with another engine's post-contact discrete transition can conceal different physical and numerical semantics. This seed makes the discrete step explicit, small enough to inspect, and independently testable.

## Scope

- Dense deterministic reference, at most 12 generalized coordinates and 10 contacts.
- Frictionless, zero-restitution unilateral contacts.
- Linearized contact Jacobian and position error over one step.
- Exhaustive small active-set enumeration.
- No CCD, friction, impact restitution, changing contact features, nonlinear geometry, articulated-tree assembly, or production solver integration.
- No performance or superiority claim.

## Initial tests

- Analytic scalar backward-Euler oscillator.
- Single-contact impulse and complementarity.
- Coupled 2-DOF stiffness/contact metric.
- Deterministic two-contact active-set resolution.
- Invalid timestep, non-SPD mass, and asymmetric-matrix negative controls.

## Benchmark extension

The next matched comparison should add the same frozen matrix problem to Symtropy's native step, MuJoCo's discrete integrator, and an independently assembled KKT/LCP reference. Compare velocity and impulse, metric/KKT residual, complementarity, next-step gap, energy/work balance, transition derivative, cost, and memory. Only compare a lane after semantic admission confirms contact law, constraint target, integration and damping equivalence.
