# Full-Tensor Rigid-Body Reference v0.1

This is an isolated, dependency-free Rust research oracle for the 3D full-inertia rotational-dynamics work in [Symtropy issue #6679](https://github.com/Luminous-Dynamics/symtropy/issues/6679), which is a focused child of the older [world-space inertia issue #3770](https://github.com/Luminous-Dynamics/symtropy/issues/3770).

It is deliberately **not** the production Symtropy integrator and does not change the engine's current behaviour.

## Mathematical contract

The reference stores a symmetric positive-definite body-frame inertia tensor and body-frame angular velocity:

\[
I_B\dot{\omega}_B + \omega_B \times (I_B\omega_B)=\tau_B
\]

The orientation quaternion maps body coordinates into world coordinates and uses scalar-first internal storage (w, x, y, z). Its kinematic equation is:

\[
\dot q_{WB}=\tfrac12 q_{WB}\otimes(0,\omega_B)
\]

The world angular momentum and rotational energy are:

\[
L_W=R(q_{WB})I_B\omega_B,\qquad
T=\tfrac12\omega_B^T I_B\omega_B.
\]

A configured tensor is admitted only if it is finite, symmetric, positive definite, and its principal moments satisfy the rigid-body triangle inequality. Invalid tensors are rejected, never projected or silently clipped.

## Reference numerical method

The v0.1 integrator uses classical fourth-order Runge–Kutta on the coupled body-angular-velocity and quaternion equations, followed by quaternion normalization. It is an independent high-order comparator, **not** a claimed Lie-group, symplectic, or energy-preserving production method. The next numerical comparison should include the legacy scalar-average method, explicit RK, midpoint, symmetric splitting, and a geometric rigid-body integrator under identical initial states and compute budgets.

The reference has no contact solver, friction, collision detection, articulated joints, sensor model, or physical calibration. Torque is held constant in body coordinates during a single step.

## Current executable controls

The crate includes tests for:

- spherical, symmetric, and asymmetric physical inertia tensors;
- rejection of non-finite, non-symmetric, non-positive-definite, and triangle-inequality-violating tensors;
- closed-form constant-rate spherical-top rotation;
- torque-free asymmetric-top energy and world-angular-momentum conservation;
- the intermediate-axis instability;
- the body-frame angular-momentum impulse identity under constant body torque;
- covariance of Euler's equation under a proper body-basis rotation;
- fourth-order observed timestep convergence against a refined RK4 reference;
- invalid timestep, torque, and zero-quaternion negative controls.

## Run locally

```bash
cargo fmt --manifest-path research/full-tensor-rigid-body-reference/v0.1/Cargo.toml --all -- --check
cargo test --manifest-path research/full-tensor-rigid-body-reference/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/full-tensor-rigid-body-reference/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

## Evidence and claim boundary

A passing unit test suite would qualify this isolated mathematical reference only. It would not prove that the native engine applies this tensor, that contacts conserve angular momentum, that sensor estimates are calibrated, or that any simulator matches GAUGE or a physical robot.

The production adapter must continue to report the **effective applied** inertia and integrator mode, separate from configured parameters and independently identified physical inertia. Integration into the native rigid-body state, collision impulses, compound-body mass properties, kinetic-energy reporting, and cross-engine/physical validation remain separate gates.
