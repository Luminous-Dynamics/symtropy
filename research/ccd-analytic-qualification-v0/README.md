# Analytic CCD Qualification V0

This tranche qualifies the swept-sphere query primitives in
`symtropy-physics::ccd`. It is deliberately a query-level analytic verification
lane; it does not yet establish full step-loop collision response correctness.

## Geometry contract

- `CcdHit::toi` is measured in seconds and lies within the closed `[0, dt]`
  interval. It is not a normalized fraction.
- `sphere_sphere` rejects non-finite position/velocity values, non-finite or
  negative radii, and non-finite or negative timestep by returning no hit.
- `sphere_halfspace` accepts a plane equation `n · x = offset`, normalizes
  both sides of the plane equation, and returns a unit contact normal.
- Positive scaling of both `n` and `offset` does not alter the represented
  plane; the TOI/contact point are invariant to such scaling.
- Zero/non-finite normals, non-finite offsets, invalid radius/timestep and
  non-finite input state do not manufacture NaN-valued hits.

The existing `Option<CcdHit>` API is retained for compatibility. New
`sphere_sphere_checked` and `sphere_halfspace_checked` entry points return
`Result<Option<CcdHit>, CcdInputError>`: valid no-hit queries return `Ok(None)`,
while malformed inputs return a typed error. The legacy APIs still map both
cases to `None`.

## Numerical method

Sphere-sphere TOI solves

```text
|Δp + Δv t|² = (r_a + r_b)²
a t² + b t + c = 0
```

The implementation parameterizes the step by normalized interval
coordinate `u = t/dt` and uses relative displacement over the complete
interval. This removes the previous dimensionful `|Δv|² < 1e-20` cutoff,
which incorrectly discarded very slow motion over long timesteps. The
quadratic uses the q-form roots to reduce cancellation when the roots have
very different magnitudes, fused multiply-add for the discriminant, and
rejects non-finite intermediate results. A scale-resistant vector norm avoids
unnecessary overflow/underflow in normalization and contact-normal recovery.

Sphere-halfspace TOI solves the normalized plane-distance equation:

```text
n̂ = n / |n|
d = offset / |n|
s(t) = n̂ · (p + v t) - d
t_hit = (s(0) - radius) / (-n̂ · v)
```

## Analytic acceptance cases

- Head-on sphere-sphere collision with known TOI.
- Exact grazing/tangent sphere-sphere contact (zero discriminant).
- TOI exactly at `t = dt` included; the same event just beyond `dt` excluded.
- Sphere-halfspace closed-form TOI and world-space contact point.
- Positive scaling invariance of the plane normal and offset together,
  including nonzero offsets and scale factors 1e-20, 1e-6, 1, 1e6 and 1e20.
- A relative speed of 1e-10 over a 1e10-second step still reports the exact
  endpoint collision; no arbitrary speed-squared cutoff is applied.
- Returned plane/contact normals are unit length.
- A zero-dimensional vector-space query returns a typed error/None and never
  indexes a nonexistent normal component.
- Contact-point surface residuals meet analytic tolerances.
- Galilean invariance under a common velocity boost.
- TOI and contact point obey time reparameterization.
- Checked APIs distinguish invalid inputs from valid no-hit queries.
- Already-penetrating and moving-away cases remain distinct.
- Negative controls for zero normal, non-finite offset, negative radius,
  non-finite/negative timestep and invalid sphere parameters.

## Claim boundary

Passing these tests verifies the analytic query primitive for the tested
geometry. It does not prove:
- CCD is invoked at the correct place in every PhysicsWorld integration path;
- a hit is integrated into contact generation/solver response correctly;
- angular/rotational CCD for arbitrary convex shapes;
- robust multi-contact behavior, friction or post-impact energy behavior;
- collision accuracy against physical experiments.

Those belong in end-to-end scenario families in PHYS-VALID-06 #460 and the
engine-level CCD torture corpus in PHYS-CCD-07 #463.

## External comparisons

Use the same equations, initial states, radii, plane equations, timestep,
collision event policy and precision for cross-engine comparisons. Compare
continuous TOI error, contact-point distance error, normal-angle error,
missed/false-positive rate and latency distributions—not only a Boolean
collision result. Record the exact engine tag/commit and build flags.

PhysMark is a useful external measurement methodology: it pins upstream engine
commits, runs identical scenes/timesteps/mass properties, warms and synchronizes
rollouts, checks physical sanity and reports performance/memory curves. It is a
comparator protocol, not itself a mathematical oracle.
