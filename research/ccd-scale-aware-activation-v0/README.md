# Scale-Aware CCD Activation V0

This tranche replaces the physics world's sole absolute-speed CCD activation
gate with a deterministic dimensionless swept-displacement criterion.

## Motivation

A fixed threshold such as 5 m/s is not scale invariant. A small sphere can
cross its own diameter during a timestep at a speed far below that threshold.
For sphere pairs, checking only the moving body's absolute speed also misses a
slow body struck by a faster target.

The activation decision now uses:

- Sphere against a stationary halfspace:
  `s = |v_i| * dt / r_i`
- Sphere against another dynamic sphere:
  `s = |v_i - v_j| * dt / min(r_i, r_j)`

A query is selected when `s >= CCD_SWEEP_FRACTION`, initially 0.5. The helper
rejects non-finite/negative displacement and non-finite/non-positive feature
sizes. It is a scheduling heuristic that reduces missed sweeps; it is not an
accuracy guarantee or an analytic error bound.

## Existing CCD query contract

The actual time-of-impact remains computed by the analytic functions in
`ccd.rs`. This activation policy does not alter TOI equations, impact response,
contact solver parameters, restitution, friction or the integration method.
The legacy `CCD_SPEED_THRESHOLD` constant remains available for downstream
compatibility but is no longer the only world-level activation gate.

## Tests

- Dimensionless activation helper: below/at threshold, scale-equivalent
  ratios, zero/negative/non-finite negative controls.
- Integrated world test: a 4 m/s dynamic sphere crosses a stationary 0.1 m
  sphere over a 1 s step. It is below the old 5 m/s threshold but has a large
  relative swept-distance ratio; the world pass must prevent the target from
  ending on the far side.
- Integrated small-sphere test: a 0.8 m/s sphere with 0.025 m radius over a
  1 s step must be swept despite low absolute speed.

Tests verify the current protected fixtures only. They do not validate every
collider pair or every motion regime.

## Scope and limitations

Currently supported activation/query combinations:

- Dynamic sphere vs stationary halfspace.
- Dynamic sphere vs dynamic sphere, using relative linear velocity.

Not currently covered by this activation pass:

- Rotational sweep / angular displacement.
- Box, capsule, convex mesh or deformable colliders.
- Moving kinematic barriers; the kinematic target trajectory is not integrated
  by this pass.
- Multiple impacts and dense contact-island coupling.
- CCD impact response accuracy, frictional impact or physical validation.

The selected dynamic sphere is clamped to its own position at the earliest TOI.
The query and clamp remain separate from a full time-of-impact contact solver.
Extreme or unsupported cases must not be described as generally protected.

## Benchmark and validation requirements

Before increasing the activation aggressiveness or changing the fraction:

- Sweep radii over at least 3 orders of magnitude.
- Sweep `dt` and velocity independently while holding the physical trajectory
  fixed, then vary them to expose missed/false activations.
- Include a slow body vs fast dynamic target, fast small sphere vs thin plane,
  glancing sphere pair and exact TOI-at-endpoint cases.
- Report activation rate, query count, false-positive activations, missed
  tunnelling events, penetration after the step and wall-clock overhead.
- Require zero missed tunnelling events on the declared supported fixtures;
  preserve numerical-health and contact/solver acceptance separately.
- Benchmark against Rapier and MuJoCo with matched geometry/timestep/contact
  semantics and exact engine commits. Do not use a fixed speed threshold as
  the reference truth.
