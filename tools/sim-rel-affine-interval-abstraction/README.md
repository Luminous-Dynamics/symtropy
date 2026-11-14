# SIM-REL-026 exact affine interval abstraction

This crate is the first bounded numerical abstraction slice for
[SIM-REL-026 / issue #3117](https://github.com/Luminous-Dynamics/symtropy/issues/3117).

## What is implemented

A one-dimensional affine transition with interval-valued coefficients and
controls:

`x' = a x + b u`

All interval endpoints are exact signed rationals. Interval addition and
multiplication therefore produce a conservative enclosure without floating-point
rounding. The result is intersected with a validated finite partition of a
declared state domain.

The abstraction reports:
- the exact rational enclosure of all possible next states;
- every successor cell intersecting the reachable interval outside the declared
  hazard region;
- an explicit hazard outcome whenever the reachable interval intersects the
  hazard region.

Partition cells are closed and adjacent, so a boundary point may map to both
neighboring cells. This is intentional over-approximation, not an ambiguity
silently resolved by a midpoint. A state cell entirely inside the hazard region
is not emitted as a safe successor.

The set of outcomes defines a **support simplex**: any probability distribution
supported on these possible outcomes is admitted. The crate does not invent a
probability law for uncertain parameters or assign arbitrary midpoint
probabilities.

## Falsification controls

Tests cover:
- exact affine interval propagation across multiple cells;
- negative values and sign-changing interval multiplication;
- hazard-boundary intersection;
- a transition whose entire reachable set is hazardous;
- partition gaps and out-of-domain reachable states;
- rational overflow and singleton exactness.

## Deliberate limits

This is a one-step, scalar, deterministic-affine interval abstraction. It is not
an ODE integrator, a validated Taylor solver, a joint physical/sensor/apparatus
model, or a multi-step robust planner. Reapplying an interval coefficient at
each step can over-approximate a parameter that is physically persistent; this
slice does not claim to preserve that cross-time correlation. A later adapter
must retain persistent complete model identity instead of silently
rectangularizing it.

The support simplex is sound for every transition contained in the computed
reachable interval, but it may be conservative. It is not yet a probability
polytope with empirically calibrated weights and does not establish physical
model adequacy or safety.

## Qualification

The dedicated workflow pins Rust 1.96.0, applies formatter-only changes before
checking, then runs exact rational tests, strict Clippy, exact-head identity and
clean-tree verification. A passing exact-head run is required before this
reference is called qualified.
