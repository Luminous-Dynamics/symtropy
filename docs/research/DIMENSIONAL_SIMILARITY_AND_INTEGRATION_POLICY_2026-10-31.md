# Dimensional similarity and explicit integration policy

**Status:** experimental numerical-integrity improvement.
**Date:** 2026-10-31.
**Implementation:** `research/dimensional-integration-policy-2026-10-31`.

## Problem

Two hard-coded integration rules depended on arbitrary numeric units:

- linear speed was silently capped at 1,000;
- angular-rate norm was silently capped at 100;
- rotation integration was skipped when angular-rate squared was <= (10^{-24}).

These rules alter the physical trajectory. A body moving at 1,500 m/s is silently changed, while the same physical motion expressed in mm/ms may be below the cap. Likewise, a small angular rate integrated over a proportionally long timestep can produce a finite rotation, but the absolute angular-rate threshold previously skipped it.

A scientific integrator must not silently impose arbitrary unitful limits.

## Implemented policy

- `try_integrate` and the legacy `integrate` wrapper now use an **unbounded** default policy.
- `IntegrationLimits::unbounded()` expresses that default explicitly.
- `IntegrationLimits::with_speed_caps(max_linear_speed, max_angular_speed)` creates application-defined limits in the caller's unit system. Each cap is optional and must be finite and positive.
- `try_integrate_with_limits` applies those caps only when the caller explicitly provides them.
- `PhysicsWorld::set_integration_limits` makes the same policy available to complete world steps; new worlds default to `IntegrationLimits::unbounded()`.
- The old 1,000 linear-speed and 100 angular-rate clamps are removed from the default path.
- The absolute (10^{-24}) angular-rate-squared gate is replaced by a test for a non-zero representable angular generator.

These limits are suitable for real-time envelopes, safety interlocks, or visualization clients when an application has declared units and a legitimate bound. They are not physical laws and should not be used to silently stabilize a scientific simulation.

## Dimensional similarity contract

For positive scaling factors (alpha) (length), (eta) (mass) and (gamma) (time), the same dimensionless rigid-body problem transforms as:

\[
\begin{aligned}
x' &= \alpha x, & m' &= \beta m, & t' &= \gamma t,\\
v' &= \frac{\alpha}{\gamma}v, &
a' &= \frac{\alpha}{\gamma^2}a, &
F' &= \frac{\beta\alpha}{\gamma^2}F,\\
S' &= \beta\alpha^2 S, &
\tau' &= \frac{\beta\alpha^2}{\gamma^2}\tau, &
\omega' &= \frac{1}{\gamma}\omega,\\
E' &= \frac{\beta\alpha^2}{\gamma^2}E.
\end{aligned}
\]

The normalized trajectory, orientation and normalized kinetic energy should be invariant within floating-point error. This is stronger than multiplying a solver matrix and right-hand side by the same scalar: it transforms every dimensional input according to its physical units.

## Regression coverage

A deterministic free-flight fixture uses an asymmetric point-mass inertia distribution, non-zero angular velocity in multiple planes, gravity, force and torque. It compares normalized position, linear velocity, angular velocity, orientation and kinetic energy under coherent changes of length, mass and time.

Scale cases include:
- micro-scale length/mass/time;
- macro-scale length/mass/time;
- length and mass scaled in opposite directions;
- an extreme coherent length/time scaling that drives angular-rate magnitude below the former absolute threshold while preserving a finite angular displacement.

Additional tests require:
- velocities above the former hard caps remain unchanged under the unbounded policy;
- explicit body-level and world-level limits clamp only when configured;
- zero, negative, NaN or infinite bounds are rejected.

The target for these free-flight similarity fixtures is normalized state/energy error <= (10^{-8}). This is a regression target, not a published benchmark result.

## World-level configuration

A world owns one `IntegrationLimits` policy that is copied for each body-integration phase. Use `PhysicsWorld::set_integration_limits` to configure real-time caps. The policy is configuration, not simulation state, and does not change during a transactional step. A future evidence receipt should include the chosen limits alongside timestep, material parameters and source/config hashes.

## What this does not prove

This campaign covers the rigid-body free-flight integration path. It does not yet establish dimensional similarity for:
- collision detection margins and GJK/EPA tolerances;
- broadphase expansion and CCD time-of-impact thresholds;
- contact compliance, Baumgarte bias and friction regularization;
- joint limits and stabilization parameters;
- sleeping/waking thresholds;
- mesh tolerances, fluid/deformable constitutive parameters or GPU backends.

Those are likely places where implicit unit assumptions remain. They need their own scale families and explicit unit contracts. Do not describe the whole engine as unit-invariant based on this one fixture.

## Industry comparison

- MuJoCo 3.17's `discrete` integrator couples constraints and implicit updates in (widehat M=M+hD+h^2K), so stiffness/damping and contact response are handled in one effective metric. That is a more mature discrete-step architecture than Symtropy currently has. See [MuJoCo computation](https://mujoco.readthedocs.io/en/3.17.0/computation.html).
- MuJoCo 3.17's `mjbatch` API documents bit-identical results to a serial loop across thread counts for its batch model. Symtropy has not established an equivalent guarantee. See [MuJoCo 3.17 changelog](https://mujoco.readthedocs.io/en/3.17.0/changelog.html).
- Rapier's 2026 work adds intra-island parallelism, persistent graph coloring and parallel broad phase. Symtropy should benchmark one large connected island, not only many independent bodies. Dimforge's Q3 report says its cross-engine graphs use early-August builds, so those results are not a current universal ranking. See [Dimforge Q3 2026 report](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/).
- Physmark splits large rigid-body scenes, many-environment robotics, and MPM into separate workload categories. Symtropy should follow that separation and pin all engine builds/hardware before comparing. See [Physmark](https://github.com/dimforge/physmark).

## Strict failure and receipt semantics

The world rejects invalid timestep/gravity as `WorldConfiguration` failures before any phase. The strict integration path returns typed `IntegrationError` variants for invalid parameters, non-finite body/force/torque input, non-finite candidate state, and wrapped rotational-inertia errors. The candidate body state is computed in locals and committed only if finite. The world wraps this in `PhysicsStepCause::Integration` and rolls back the entire physics-owned step on failure.

Each attempted world step receives a monotonic `step_id`. The identifier is consumed even when a step is rejected, appears in `PhysicsStepError`, and is also attached to an accepted `InvariantStepReceipt`. It allows an external command/input log to correlate accepted and rejected attempts; it is not a replay hash or a substitute for source/configuration provenance.

## Next work

1. Add dimensional-similarity fixtures for sphere/plane contact, multi-point box contact, friction, CCD and joint constraints.
2. Replace unexplained tolerances with named, dimension-aware configuration values and document their units.
3. Add per-scenario scale sweeps spanning at least (10^{-6}) to (10^6) in characteristic length and mass, with coherent time scaling.
4. Provide separate `Scientific` and `Realtime` integration profiles at the world level; realtime caps must be explicit and included in receipts.
5. Add work/energy accounting so a cap or constraint can report the energy removed or added, rather than silently changing it.

The key architectural principle: *a change of units must not change the physical answer; an application policy may constrain a trajectory, but it must declare that intervention and account for its effect.*
