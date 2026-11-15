# Physics Discrete Step Map 001

This standalone Rust crate is an independent scalar oracle for the end-of-step
mass-spring-damper map used to qualify a transition-level physics contract.

## Model

For mass (m>0), damping (c\ge0), stiffness (k\ge0), fixed step (h>0),
state ((x,v)), and held input force (u):

```text
m (v_next - v) / h = -k x_next - c v_next + u
x_next = x + h v_next

den = m + h c + h^2 k
v_next = (m v - h k x + h u) / den
x_next = x + h v_next
```

The result's step acceleration is `(v_next - v)/h`. It is not the continuous
vector-field acceleration evaluated at the beginning of the step.

## Evidence included

- Analytic state Jacobian for `[x_next, v_next]) with respect to `[x, v]`.
- Analytic sensitivities with respect to `[mass, damping, stiffness, force, h]`.
- Central finite-difference step ladder for all seven input coordinates.
- Exact continuous-time reference for unforced underdamped, critically damped,
  overdamped, and zero-stiffness systems.
- Deterministic 10,000-case stability sweep over positive scalar parameters.
- Explicit and semi-implicit Euler negative controls at an oversized timestep.
- Separate tests for large-step stability versus trajectory accuracy.
- First-order convergence under timestep refinement.
- Coherent SI base-unit rescaling.
- Typed rejection of invalid inputs and non-finite arithmetic.

## Run

From the repository root:

```bash
cargo +1.96.0 fmt --manifest-path research/physics-discrete-map-001/Cargo.toml -- --check
cargo +1.96.0 test --manifest-path research/physics-discrete-map-001/Cargo.toml --all-targets
cargo +1.96.0 clippy --manifest-path research/physics-discrete-map-001/Cargo.toml --all-targets -- -D warnings
```

## Claim boundary

This is a scalar reference kernel, not the production Symtropy integrator, not a
general constrained solver, and not an implementation of MuJoCo. It qualifies
a smooth linear discrete transition only. Contact branches, articulated
systems, spatial inertia, general effective-metric solves, solver cost,
GPU throughput, and hardware validation remain separate work.

Stability is not accuracy: implicit stepping may remain bounded at large (h)
while strongly damping or distorting the physical trajectory. The tests make
that distinction explicit.
