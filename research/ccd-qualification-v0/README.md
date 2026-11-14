# CCD Qualification v0

This research lane qualifies Symtropy's analytical translational CCD queries.
It deliberately separates query correctness, integrated-world activation,
numerical verification and performance.

## Implemented query contract

- `sphere_sphere_checked` and `sphere_halfspace_checked` return
  `Result<Option<CcdHit>, CcdInputError>`: invalid input is distinct from a
  valid no-hit result.
- Legacy `Option` APIs remain as compatibility wrappers and map invalid input
  to `None`. New research/production callers that need observability should
  use the checked forms.
- TOI is measured in seconds from step start, in `[0, dt]`.
- Sphere-sphere TOI uses relative closest-approach geometry rather than the
  cancellation-prone direct quadratic discriminant.
- Sphere-halfspace normal and offset are normalized together, preserving the
  same plane under positive normal scaling.
- World activation now attempts a sphere sweep if either the absolute speed
  guard is exceeded or swept displacement exceeds 0.5 sphere radii. This
  dimensionless threshold is an activation heuristic, not a proof of no
  tunnelling when a sweep is skipped.

## Frozen scenario

The machine-readable fixture file is
[`manifests/ccd-analytic-v0.json`](../physics-corpus-v0/manifests/ccd-analytic-v0.json).
It declares SI-compatible units, head-on and grazing sphere hits, a near-tangent
miss, a million-metre high-speed case, a sub-threshold small-sphere/plane
world test, and positive plane-normal scale invariance.

The integrated world path remains limited to translating sphere colliders
against sphere and half-space colliders. Rotational CCD, boxes, capsules and
arbitrary convex sweep remain unsupported.

## Criterion measurements

The `ccd_bench` target separates:
- checked vs compatibility query cost;
- head-on, large-separation and grazing sphere-sphere cases;
- unit and scaled half-space normals;
- small-radius sub-threshold activation, low-displacement activation and the
  absolute-speed guard.

No performance result is included yet. Published comparisons must use the same
geometry, timestep, radii, velocity distributions and hit/miss proportions.
Report query latency, false-positive/negative counts, TOI error, contact
residual and world-step time together; query speed alone is not a quality win.

## Required qualification

1. Analytic TOI agreement with frozen exact/closed-form inputs.
2. Positive plane-normal scaling invariance.
3. Near-grazing hit and miss separation.
4. Negative controls for non-finite values, negative radii, invalid timestep,
   zero normal, separating motion, zero relative motion and impact after dt.
5. Integrated small sphere below the legacy speed threshold does not tunnel.
6. Multiple-timestep sweeps report TOI and post-step penetration.
7. 20+ repetitions for published benchmark statistics and confidence intervals.

## External comparison

- MuJoCo 3.17.0 batch determinism is a batch/replay comparator, not a CCD
  accuracy result. Its documentation distinguishes saved integration state
  from derived simulation state and notes that contact trajectories can
  diverge after small numerical perturbations.
- Rapier 0.35.3 is the stable tagged Rust-engine lane found for this research.
  Dimforge's Q3 2026 parallel architecture report is a separate development
  lane and must be pinned independently.
- Project Chrono 10.0.0 is relevant only for contact/material scenario families
  with matching model assumptions.
- ASME V&V 20 supplies a useful discipline: compare specified solution
  variables at specified validation points and include numerical and
  experimental-data uncertainty.

No universal CCD-SOTA statement follows from these fixtures. A defensible claim
requires equivalent geometry/material/timestep, identical initial conditions,
independent reference measurements, and reported false-positive/negative rates.
