# CCD Qualification Corpus v0

This corpus qualifies the current analytical sphere/sphere and sphere/half-space
sweep paths. It is designed to make time units, hit/miss semantics and numerical
error explicit before comparing throughput.

## Contract

- `CcdHit::toi` is elapsed physical time in seconds in `[0, dt_seconds]`.
- A normalized time fraction is a separate derived quantity:
  `toi_fraction = toi_seconds / dt_seconds`.
- A valid miss is `Ok(None)`; invalid input is `Err(CcdQueryError)`.
- Sphere radii are strictly positive and finite.
- Time steps are finite and non-negative.
- The sphere/half-space plane normal is finite and unit length; the plane
  equation is `n · x = offset`.
- The old Option-returning APIs remain compatibility wrappers, failing closed
  as `None` on invalid query input. New validation harnesses should use the
  checked APIs so an invalid input cannot be counted as a geometric miss.

## Frozen analytic fixtures

1. Sphere/half-space: center `[0,10,0]`, velocity `[0,-1000,0]`, radius
   `0.5 m`, plane `y=0`, timestep `0.02 s`. Expected TOI is `0.0095 s`,
   normalized fraction `0.475`.
2. Sphere/sphere: centers at `0 m` and `5 m`, radii `0.5 m`, opposing
   velocities `+10 m/s` and `-10 m/s`, timestep `0.25 s`. Expected TOI
   is `0.2 s`, normalized fraction `0.8`.
3. Grazing contact: moving unit-radius-sum path tangent at `t=5 s`; offset
   the second sphere by `1e-8 m` to form a near-miss negative control.
4. Step boundary: contact exactly at `t=dt=1 s` must be a hit; reduce the
   horizon by `1e-9 s` and it must be a miss.
5. Galilean/translation invariance: add a common displacement and common
   velocity to both spheres; relative TOI must remain unchanged.
6. Fixed 1,000-case head-on sweep: for each case independently compute
   `t_ref = (separation - radius_a - radius_b) / relative_closing_speed`.
   Compare checked sphere/sphere TOI to `t_ref` at the declared tolerance.

## End-to-end integration fixture

A sphere of radius `0.05 m` begins at `y=5 m` with `v_y=-500 m/s`.
A discrete `16 ms` step would move it `8 m`, entirely through a plane at
`y=0`. The integrated PhysicsWorld path must keep the center at or above
`radius - 1e-6 m` and produce finite, numerically healthy body state.

This fixture checks the current supported sphere/half-space case only. It does
not qualify rotating shapes, boxes/capsules, curved surfaces, moving planes,
multi-impact substepping, swept broadphase, or post-impact impulse accuracy.

## Comparison metrics for engine lanes

For each frozen trajectory and solver/backend:
- hit/miss classification accuracy;
- absolute and relative TOI error in seconds;
- normalized TOI error as a separately labelled derived metric;
- false-positive and false-negative rate;
- tunnelling rate after the complete world step;
- maximum post-resolution penetration depth;
- post-impact momentum and kinetic-energy change under an explicitly declared
  restitution/friction model;
- runtime per query and per full step;
- performance distributions and supported-scope energy per completed case.

Do not compare one engine's normalized TOI fraction with another engine's
seconds. Do not treat TOI agreement as proof of correct contact impulses or
full physical validation.

## External baselines

Pin exact releases and configure equivalent geometry, timestep, collision
filters, restitution, friction, solver iterations and CCD settings:

- MuJoCo 3.17.0 for supported continuous collision and batch stepping.
- Rapier 0.35.3 stable release; track the newer intra-island parallelism lane
  separately at an exact development commit.
- Project Chrono 10.0.0 for broader multibody/FSI scenarios only where the model
  assumptions align; it is not the primary rigid-sphere CCD comparator.

The first improvement target is zero missed impacts and zero false positives
on the frozen analytic corpus, with TOI error bounded by the numerical tolerance.
Only after this correctness gate passes should query throughput or energy be
compared. No current SOTA superiority result is claimed.
