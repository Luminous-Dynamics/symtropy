# Contact-Rich Physics Benchmarks v0

This benchmark fills a coverage gap in the original one-step physics benches: widely spaced bodies primarily exercise integration and broadphase. These scenarios produce sustained contacts and exercise narrowphase, manifold generation, warm-starting, island construction and the TGS contact solver.

## Scenarios

| Scenario | Bodies | Contact topology | Purpose |
|---|---:|---|---|
| no_contact_64_spheres | 64 | none | Integration/broadphase control |
| resting_sphere_stack_16 | 16 | plane + vertical sphere chain | Resting contact, warm-start and long contact graph |
| multi_point_box_stack_3 | 3 | ground plus multi-point box contacts | Manifold generation and multi-point solver |
| known_unqualified_ten_box_stack | 10 | ground plus nine box-to-box patches | Solver convergence stress at 8 vs 30 iterations; measurement-only |
| dense_sphere_bed_8x8 | 64 | ground + dense lateral neighbors | Broadphase candidate density and contact throughput |

The first four workloads use 120 simulation steps at 120 Hz and eight solver iterations. The ten-box stress lanes use 1,800 steps at 60 Hz (30 simulated seconds), once at 8 and once at 30 solver iterations to match the existing ignored tall-stack regression. Contact scenes use declared initial overlaps; the no-contact scene is gravity-free and has no plane. The body positions, masses, geometry and solver settings are frozen in `manifests/contact-rich-workloads-v0.json`.

## Measurement contract

- Criterion reports body-steps/second with one consistent work unit across all four scenarios.
- Setup/build is outside the timed iteration; the timed section runs the full 120-step world campaign.
- An untimed calibration campaign captures actual average contact manifolds/step, contact points/step, collision-event count, maximum penetration and maximum linear speed; those values are encoded in the Criterion benchmark identity. A scene that unexpectedly loses contacts cannot silently look like a throughput win.
- Contact scenarios must produce at least one contact point over the campaign; the no-contact control must produce zero contact points.
- The ten-box stress lanes are **measurement-only / known-unqualified**: record final maximum vertical center-position error against the nominal settled stack and peak speed, but do not mark the scenario physically passed solely because all values remain finite.
- Every step checks the numerical-health invariant; NaN/Inf and invalid rotation state fail the campaign.
- Repeat each scenario for at least 20 independent campaigns when publishing comparisons. Report median, p05/p95, dispersion and 95% confidence intervals; include failed or panicked campaigns in the failure denominator.
- Run identical scene geometry and semantics in every engine. Report both matched solver quality and the achieved quality; don't compare a soft/low-iteration run against a hard/high-iteration run as if equivalent.

Suggested invocation:

```sh
cargo bench -p symtropy-physics --bench contact_rich_physics_bench
```

## Why the paradigm matters

The performance objective is a qualification frontier, not one headline FPS value:

- throughput and p50/p95 step latency;
- contact points/manifolds processed per step;
- maximum and percentile penetration;
- post-settle positional drift and contact churn;
- linear/angular kinetic-energy and momentum drift, interpreted with correct external-boundary assumptions;
- solver convergence vs iteration count and timestep;
- CPU time, allocations, peak memory and cache behaviour;
- energy per completed world-step or contact-point-step, with sensor scope and raw receipt;
- determinism class and failure rate.

### Contact recycling is a new architecture opportunity

Symtropy currently has a contact cache used for warm-start impulse reuse (`ContactCache`); that is not the same as reusing geometric contact manifolds. The narrowphase presently regenerates contact manifolds each world step. Box2D 3.2's contact recycling is a useful reference: it keeps prior contact data and updates separation for small relative motion, recomputing the contact when motion exceeds a threshold. This benchmark first establishes workload and contact-count baselines; a future contact-recycling implementation should be a separate feature branch with explicit invalidation for rotation, feature changes, topology changes and discontinuous motion.

Do not add manifold recycling before the baseline exists. It can easily make the engine faster by keeping stale contacts, which would be a correctness regression. Require differential output/penetration checks and contact churn diagnostics before accepting it.

## Industry comparisons

- **Box2D 3.2 (18 Oct 2026)**: Box2D's own release article reports roughly 3x over 3.1; its breakdown attributes about 40% to contact recycling, 20% to solver refinement, 15% to AVX2 dispatch, 10% to solver optimization and 10% to broadphase work. These are vendor-reported 2D Box2D-vs-Box2D measurements, not a direct Symtropy comparison. Its contact-recycling mechanism is still a useful design reference.
- **Rapier 0.36.1 (4 Oct 2026)**: latest tagged stable Rust release found. Pin solver parameters/features. Dimforge's Q3 2026 report describes a separate, commit-pinned development architecture with intra-island parallelism, persistent graph coloring and parallel broadphase; don't attribute those results to the 0.36.1 tag without verifying its source.
- **MuJoCo 3.17.0 (22 Oct 2026)**: useful for deterministic multi-simulation batch throughput. Its batch API is not automatically a matched contact-solver workload; align contact model, solver, timestep and observation contract first.
- **Project Chrono 10.0.0**: reference for multibody/vehicle/FSI/deformable cases when equations and model assumptions match.

Official references:
- https://box2d.org/posts/2026/10/box2d-3.2/
- https://box2d.org/documentation/
- https://github.com/dimforge/rapier/blob/master/CHANGELOG.md
- https://www.dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/
- https://mujoco.readthedocs.io/en/3.17.0/changelog.html
- https://projectchrono.org/news/

## Initial improvement targets (not results)

- 0 non-finite bodies and 0 missing contact-work records.
- Contact-scene benchmark IDs must always expose measured contact-point/manifold counts.
- Candidate resting-scene target: median maximum penetration below 0.5% of characteristic body diameter after settling, p95 below 1%, and no progressive drift. These are programme targets and require scenario-specific qualification; do not claim them from this benchmark until measured and validated.
- Tall-stack convergence target: report final stack-height residual, maximum speed and penetration at 8 vs 30 iterations; a higher iteration budget is not an improvement unless residual/error falls enough to justify its added cost. The ten-box case remains unqualified until it meets a declared settled-height and drift envelope.
- A contact-recycling candidate must reduce median contact-pipeline CPU time by at least 10% on two contact families while keeping penetration, restitution, friction, stability, event ordering and replay outputs within predeclared limits.
- Publish performance improvements only after at least 20 repeats on a pinned device/compiler/build, with p95 latency and energy included.

## Claim ceiling

This PR creates a repeatable contact-rich measurement lane. It has no measured performance result yet, makes no cross-engine speed claim, and does not prove experimental physical accuracy.