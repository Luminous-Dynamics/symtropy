# Contact Jacobian / Effective-Mass Reference v0.1

**Tracking issue:** [PHYS-ROTATIONAL-RESPONSE-PORTS-001 #6443](https://github.com/Luminous-Dynamics/symtropy/issues/6443)  
**Related references:** [compound mass properties #6418](https://github.com/Luminous-Dynamics/symtropy/pull/6418), [principal-plane inertia #6416](https://github.com/Luminous-Dynamics/symtropy/pull/6416)  
**Source baseline:** `main` at `b93beb5f68f6fa0bb0c5d4f58d3afb79a19cb68e`

**Status:** M1 independent 3D analytic oracle with synthetic tests. It does not alter production code and does not claim physical validation or simulator superiority.

## Paradigm shift: benchmark the generalized contact Jacobian

A contact impulse acts at a point displaced from a body's center of mass. The scalar inverse effective mass in a direction n is:

`k = n^T J M^{-1} J^T n`.

For a 3D rigid body with lever arm r, mass m, and world inertia tensor I, the point-impulse response is independently reconstructed as:

`delta_v = j/m; delta_omega = I^-1 (r x j); delta_v_contact = delta_v + delta_omega x r`.

For unit direction n, the single-body inverse effective mass is `k = n dot delta_v_contact` when `j=n`. Contributions from two dynamic bodies add; static/kinematic endpoints contribute zero.

The local mass second-moment tensor `C = integral(r r^T dm)` gives `I = trace(C) identity - C`; orientation maps it into world coordinates by `R I R^T`.

This reference uses explicit 3D vector/matrix arithmetic and the standard cross product. It does not call production Bivector, inertia, contact or solver functions.

## Known-answer fixtures

- **Isotropic sphere:** 2 kg, radius 0.5 m; inverse effective mass is 1.75 kg^-1.
- **Anisotropic cuboid:** 2 kg, dimensions 2 x 1 x 0.5 m; selected face contact gives 0.8 kg^-1. If its principal moments are passed to a scalar mean-of-reciprocal-axes approximation, that shortcut gives 1.117647 kg^-1. This comparison is conditional because the current production vector's frame semantics still need the M0/M2 API audit.
- **Rotated cuboid:** rotating inertia, contact point and normal together preserves effective mass.
- **Endpoint composition:** a fixed endpoint contributes zero; dynamic/dynamic contact adds both endpoint responses.
- **Work closure:** midpoint contact work equals pair kinetic-energy change from rest for equal-and-opposite impulses.

## Tests

Sphere closed-form, directional anisotropic response, frame covariance, fixed/dynamic endpoint accounting, midpoint work-energy closure, and negative controls for non-unit contact direction, non-PSD second moment and improper rotation.

## Reproduction

```sh
cargo fmt --manifest-path benchmarks/contact-jacobian-reference/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/contact-jacobian-reference/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/contact-jacobian-reference/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

## Promotion

M1 is this independent oracle. M2 should use the same qualified operator in production normal/friction contact denominators. M3 extends to multi-point manifolds, tangent/friction cones, off-center impacts and constraints. M4 compares pinned MuJoCo/Bullet/PhysX versions under identical geometry, mass properties, contact law, solver iterations, timestep, materials and tolerances. M5 uses instrumented physical articles and held-out tests.

An isolated mathematical oracle does not establish production-engine correctness, physical contact accuracy or safety. No compile/test/Clippy/CI pass is claimed until toolchain-backed execution completes.
