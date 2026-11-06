# CCD qualification lane v0

This lane qualifies analytic time-of-impact queries separately from discrete
contact generation, contact resolution, and whole-world physical validation.

## Contract

New code should use checked CCD queries:

- `sphere_sphere_checked`
- `sphere_halfspace_checked`

They return three semantic outcomes:

- `Ok(Some(hit))`: a finite hit with TOI in the closed interval `[0, dt]`
  and a finite unit normal.
- `Ok(None)`: valid inputs and a valid computation establish no hit in the
  declared interval.
- `Err(CcdError)`: invalid data, degenerate geometry, or arithmetic that
  cannot establish a physical result.

Legacy `sphere_sphere` and `sphere_halfspace` entry points remain available
and map checked errors to `None` for compatibility. New code should not use
that legacy ambiguity when invalid inputs must be distinguished from misses.

## Numerical method

The sphere-sphere checked path computes time of closest approach and the
perpendicular separation. It obtains the contact half-chord from
`r_combined^2 - d_perpendicular^2`, avoiding the direct subtraction of the
large, nearly equal terms in the quadratic discriminant. A small negative
chord-square within a frozen roundoff tolerance is treated as a grazing
contact; larger negative values are valid misses.

The sphere-halfspace checked path normalizes both the normal and plane offset.
The physical plane is therefore invariant under non-zero scaling of its plane
equation. Degenerate normals and non-finite offsets are typed errors.

## Oracle and negative controls

See `manifests/ccd-analytic-oracles-v0.json` for the frozen scenario catalog.
The minimum corpus includes head-on, tangent, grazing miss, initial overlap,
zero relative speed, high dynamic range, plane scaling, initial penetration,
moving-away, exact end-of-step collision, after-end-of-step miss, invalid
scalars/vectors, degenerate normal, and arithmetic-overflow controls.

The large-scale fixture has a 10^12 m initial separation, 10^12 m/s relative
speed, and a 1 m combined radius. Its expected TOI is 1 - 10^-12 seconds. This
is specifically intended to expose loss of the small contact chord when a
quadratic discriminant subtracts terms around 10^48.

## Comparison discipline

Compare only equivalent swept-query semantics. For MuJoCo 3.17.0 and Rapier
0.35.3, first document what each engine's public CCD/speculative-contact API
actually computes. If one engine uses speculative contacts or a different
substepping method rather than a matching TOI query, record the case as
non-equivalent rather than calling it a failed result.

Report TOI error, normal error, false positives/negatives, invalid-input
classification, scale invariance, body-order symmetry and p50/p95 runtime.
A passing analytic query test does not validate the contact solver, the
whole-body trajectory, or physical fidelity.

## Current implementation status

The checked query functions, compatibility wrappers, closest-approach method,
plane normalization, and focused negative-control tests are on
`research/ccd-checked-stable-queries-v0`. The corpus manifest is the target
contract. Full cargo tests, formatting, Clippy and platform CI must still pass
before merge; no solver-level or industry-superiority claim is made.
