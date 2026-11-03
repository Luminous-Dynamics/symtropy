# Symtropy Engine Qualification and Industry Comparison Guide

**Status:** Qualification guide; no new benchmark result claimed.  
**Date:** 2 November 2026  
**Relationship:** Implements the evaluation direction in [PHYS-VALID-06 / #460](https://github.com/Luminous-Dynamics/symtropy/issues/460), using the existing focused work in [Rapier differential runner / #443](https://github.com/Luminous-Dynamics/symtropy/issues/443), [tall-stack solver hardening / #444](https://github.com/Luminous-Dynamics/symtropy/issues/444), and [parallel/SIMD qualification / #453](https://github.com/Luminous-Dynamics/symtropy/issues/453).

## 1. The comparison thesis

Symtropy should not claim that it is globally faster or more physically accurate than every established physics engine. The defensible thesis is narrower:

1. Match established engines on common 2D / 3D workloads for correctness, stability, and performance.
2. Prove N-dimensional behavior against analytic references where conventional engines do not share the same dimensional scope.
3. Show that generic application-defined state coupling is useful on a held-out task, not only in a demonstration.
4. Keep numerical correctness and safety ahead of throughput. A faster divergent simulation is not an improvement.

The likely differentiation is N-dimensional geometry plus generic state-coupling. It is not a blanket claim of 3D rigid-body engine superiority.

## 2. Current benchmark surface and its limits

The existing Criterion source is [`crates/core/symtropy-physics/benches/physics_bench.rs`](../crates/core/symtropy-physics/benches/physics_bench.rs).

It already measures useful low-level operations:

- GJK queries for spheres, convex hulls, capsules, HyperBoxes, and a 4D shape.
- One EPA penetration query.
- Raycasting against a 100-body scene.
- Physics stepping at several body counts in 2D and 3D.

However, the current 10 / 100 / 500 body step scenes space bodies at 2.5–3.0 units while using spheres of radius 0.5. They are deliberately sparse, non-contact scenes. These measurements are useful for integration / per-body overhead, but they do **not** measure a dense broadphase, narrowphase, manifold, contact-solver, or stack-stability workload.

The isolated GJK/EPA timings are microbenchmarks of those query routines, not end-to-end engine throughput. Keep the distinction in reports.

## 3. Comparison layers

### Layer A — analytical correctness first

Use analytic or tightly bounded reference cases wherever available:

| Case | Reference observable | Primary checks |
|---|---|---|
| Constant-acceleration free fall | Closed-form position and velocity | Position / velocity error over time |
| Projectile motion without drag | Closed-form trajectory | Trajectory error, energy consistency |
| 1D / 2D elastic collision | Momentum and energy relations | Post-collision momentum / energy residual |
| Restitution collision | Declared coefficient and momentum balance | Relative rebound speed, penetration |
| Frictionless incline | Acceleration from gravity component | Acceleration error |
| Simple pendulum (small-angle and nonlinear reference) | Analytic approximation / high-accuracy ODE reference | Period, energy drift |
| N-dimensional free motion | Closed-form vector trajectory | Coordinate-wise error and dimension scaling |

Reference equations, coordinate conventions, units, tolerances, integrator, and timestep must be frozen in the case definition. Do not select tolerances after looking at a candidate engine's output.

### Layer B — common 2D / 3D engine comparisons

Compare only on overlapping features and match the following wherever the APIs permit:

- Geometry, mass, inertia, initial positions / orientations, velocities.
- Gravity, timestep, substeps, solver iterations, and collision margins.
- Friction / restitution coefficients and contact filtering.
- CCD settings, sleeping policy, and warm-start policy.
- Hardware, compiler / runtime, build mode, and thread count.

Competitor-specific configuration is allowed only when documented and applied under a declared tuning budget. Publish both the native/default profile and the tuned profile; do not silently tune one engine and leave another at defaults.

### Layer C — Symtropy-specific N-dimensional tests

External 3D engines cannot act as direct comparators for dimensions they do not implement. Use analytic references, invariants, property-based tests, and metamorphic relations:

- Translation invariance where the equations require it.
- Rotation / orthogonal-transform invariance where the implementation claims it.
- Dimensional consistency of support maps and collision predicates.
- Symmetry of pairwise intersection queries.
- Permutation invariance for body insertion where semantics permit.
- Finite outputs and bounded residuals over supported dimensions.
- Scaling behavior as dimension and convex-hull vertex count increase.

A 4D timing number without a correctness reference is not a 4D physics qualification.

### Layer D — state-coupling experiments

Evaluate state-coupling separately from the baseline rigid-body engine:

1. Baseline physics with no callback.
2. Callback present but configured as a no-op.
3. Callback applying a fixed, declared metric.
4. Metric changing under a reproducible, held-out scenario.
5. Adversarial / invalid metric inputs and out-of-range values.
6. Ablation of each coupling channel.

Measure the effect on forces / impulses, energy accounting, solver residuals, stability, and application outcomes. Prove that disabling the callback recovers the baseline behavior within the declared tolerance.

If a metric is Phi or an IIT-inspired score, describe it as an experimental computational indicator unless independently validated for the specific control task. Do not equate it to a physically established field, safety signal, or proof of consciousness.

## 4. Industry reference matrix

| Engine / reference | Strong comparison area | Important caveat |
|---|---|---|
| [Rapier](https://rapier.rs/docs/) | Rust-native 2D / 3D rigid-body, collision, joints, queries, snapshots, soft-body support | Use the same scenarios and supported primitives. The Symtropy Rapier bridge / #443 must actually step the reference pipeline before it is used as a differential runner. |
| [MuJoCo](https://mujoco.readthedocs.io/en/stable/computation.html) | Robotics, articulated systems, contact-rich manipulation, model-based control | Different contact and constraint formulations can produce different valid outcomes; compare observables and tolerances, not bitwise states by default. |
| [Jolt Physics](https://github.com/jrouwe/JoltPhysics) | 3D rigid-body game / simulation workloads, stacks and constraints | Match body types, collision geometry, and solver profiles. |
| [Box2D](https://box2d.org/documentation/) | Mature 2D rigid-body and contact workloads | Useful 2D comparator; not a 3D or N-D reference. |
| [NVIDIA PhysX 5](https://nvidia-omniverse.github.io/PhysX/physx/5.7.0/docs/GPURigidBodies.html) | CPU / GPU rigid-body simulation, articulations, large-scale workloads | Pin exact PhysX version, GPU backend, and all solver / scene settings. |
| [Newton](https://github.com/newton-physics/newton) | GPU-accelerated robotics simulation built on NVIDIA Warp, with integrations across robotics and simulation ecosystems | Track its release maturity separately; do not compare beta / evolving APIs without version pinning. |

No external engine is automatically “ground truth.” Agreement is useful evidence; disagreement identifies a case for analysis. For physical truth, use analytic solutions or validated experimental data with uncertainty where available.

### Version-pinned comparator snapshot (3 November 2026)

The names in the matrix above are not sufficient for reproducibility. Pin exact
versions and feature profiles in every run. The current reference snapshot is:

| Reference | Current documented baseline | Consequence for Symtropy qualification |
|---|---|---|
| Rapier | `rapier3d 0.36.1` (6 October 2026 registry release) | Keep the existing `0.18` runner as a historical lane. Qualify `0.36.1` in a separate backend profile; its API and math-type changes are not a transparent version bump. |
| MuJoCo | `3.17.0` (22 October 2026) | Add a separate batch-rollout experiment. Its batch API is documented to match plain serial stepping bit-for-bit across thread counts for the same model/configuration; compare this determinism property and throughput separately from contact-model equivalence. |
| Newton | Pin an exact Git commit/release, Warp version, OpenUSD schema/runtime and solver configuration for each run | Treat it as a GPU-accelerated robotics simulation comparator, not an unnamed moving target. Record GPU/driver and whether the backend is MuJoCo Warp. |

Rapier 0.36.1 is an active, pre-1.0 API line. Its current docs expose
SIMD-batched contact processing as always enabled, with `parallel` and
`enhanced-determinism` as explicit features. Its published package manifest
declares Rust 1.86 MSRV. These are not details to normalize away: record the
feature flags, target and exact version as part of the backend identity. The
repo's Rapier 0.18 bridge therefore cannot be described as “current Rapier.”

MuJoCo's 3.17 batch API changes the performance comparison question. Report
both:
- **single-rollout latency:** time to advance one fixed scenario;
- **batch throughput:** completed independent rollouts per second at fixed
  model, horizon, output sampling and thread budget.

For both, record per-rollout final-state digests and the full state/observation
policy. Do not compare aggregate throughput when one engine emits fewer
observations or computes a different horizon. Do not equate cross-engine
trajectory equality with physical truth.

The repository already tracks the safe Rapier migration in
[PHYS-RAPIER-14 / #495](https://github.com/Luminous-Dynamics/symtropy/issues/495):
preserve `0.18` as a historical baseline, finish the executable D0 lane,
converge on one common trace schema, then qualify `0.36.1` separately. Do not
make a third overlapping D0 bridge or silently retarget #488/#1643. The current
SO(4) production/oracle campaign remains separate from this 2D/3D differential
reference programme.

## 5. Initial workload matrix

| Workload | Scale | Correctness / stability outputs | Performance outputs |
|---|---:|---|---|
| Sparse free motion | 10 / 100 / 500 / 1k / 5k bodies | Closed-form position / velocity error | Step time, allocation, active body count |
| Broadphase sparse / dense | 100 / 500 / 1k / 5k bodies | Candidate-pair correctness, missed-pair count | Broadphase time, candidates, false positives |
| Contact stacks | 3 / 5 / 10 / 20 / 50 bodies | Penetration, COM drift, settling, energy / residual | Contact count, solver time, step time |
| Pyramids / mixed mass ratios | 10 / 50 / 100 bodies | Stability, penetration, angular growth | Solver iterations, residual / time |
| Friction slope | 1 / 10 / 100 bodies | Slide / rest threshold, friction residual | Contact solve time |
| Joints / constraints | 2 / 10 / 50 / 100 bodies | Constraint residual, drift, oscillation | Joint count, solve time |
| CCD projectile / thin wall | 1 / 10 / 100 projectiles | Tunneling count, time of impact, penetration | CCD step time |
| Sleep / wake | 10 / 100 / 1k bodies | Wake correctness, false wake/sleep | Sleep management cost |
| Ray / shape query | 100 / 1k / 10k bodies | Hit / miss correctness and nearest-hit identity | Query p50 / p95 |
| Deterministic replay | fixed scenario, repeated runs | State digest / tolerance agreement | Serialization and replay cost |

This workload matrix should become the machine-readable corpus under PHYS-VALID-06 / #460. It is not a claim that every row currently has an implementation.

## 6. Metrics and reporting

Report correctness before speed:

- Maximum and RMS position / velocity deviation from the declared reference.
- Maximum penetration depth and contact residual.
- Constraint error and drift.
- Momentum / energy drift only where the chosen model says the quantity should be conserved.
- CCD tunneling and missed-collision count.
- Deterministic replay agreement under an exact declared profile.
- p50 / p95 / p99 step time, throughput, and allocations.
- CPU/GPU, memory, and joules per simulated second where instrumented.
- Warm-up, repetition count, confidence intervals, and raw samples.

Use separate sparse and dense workloads. Body count alone is insufficient: also report active bodies, contact pairs, manifold points, islands, joints, and CCD candidates.

For contact-rich systems, do not assume exact terminal state equality across engines: stable contact arrangements can differ. Define physical observables and tolerances per scenario.

## 7. Suggested qualification gates

- **Q0 — local regression:** scenario and expected property exist; not an external comparison.
- **Q1 — analytical reference:** pinned inputs and analytic / high-accuracy reference; declared tolerances pass.
- **Q2 — differential lane:** Symtropy and a real external engine both execute the same scenario; all configurations and output mappings are recorded.
- **Q3 — stress qualification:** stacks, CCD, joints, friction, sleeping, and dense contact corpus pass stability thresholds.
- **Q4 — independent reproduction:** another operator reproduces the results from the scenario manifest and exact commit.
- **Q5 — domain validation:** relevant robotics / engineering / scientific stakeholder reviews model assumptions and external reference data.

A performance claim must cite a Q1–Q4 correctness context. A Q5 domain claim needs the domain-specific validation, not just a faster physics loop.

## 8. Priority sequence

1. Complete the real Rapier pipeline / differential runner in [#443](https://github.com/Luminous-Dynamics/symtropy/issues/443).
2. Instrument and resolve tall-stack instability in [#444](https://github.com/Luminous-Dynamics/symtropy/issues/444).
3. Freeze the broad scenario corpus in [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).
4. Add contact-heavy Criterion workloads and split broadphase / narrowphase / solver / CCD timing.
5. Qualify serial, SIMD, and parallel execution against identical semantics in [#453](https://github.com/Luminous-Dynamics/symtropy/issues/453).
6. Add independent reproduction and machine-readable result receipts.

Do not skip steps 1–3 to publish a generic “faster than Rapier” claim.

## 9. Reproducibility manifest

Every run should record:

- Symtropy repository and exact commit SHA.
- External engine, exact version, and binary / package digest.
- Compiler, Rust toolchain, target, OS, CPU, GPU, and driver.
- Solver profile, integrator, timestep, substeps, iterations, CCD, friction, restitution, and sleeping flags.
- Scenario ID and parameter digest.
- Seed and initial state digest.
- Reference source, license, uncertainty, and artifact digest.
- Raw output hash, parsed metrics, thresholds, verdict, and evidence tier.

The result should distinguish `PASS`, `FAIL`, `INCONCLUSIVE`, and `MEASUREMENT_ONLY`. Missing or unparsable output must not become zero.

## 10. Related work

- [PHYS-VALID-06 — Engine-wide physics qualification corpus (#460)](https://github.com/Luminous-Dynamics/symtropy/issues/460)
- [PHYS-DIFF-01 — Rapier differential reference runner (#443)](https://github.com/Luminous-Dynamics/symtropy/issues/443)
- [PHYS-SOLVER-02 — Tall-stack constraint scaling (#444)](https://github.com/Luminous-Dynamics/symtropy/issues/444)
- [PHYS-PERF-03 — Parallel-island / SIMD qualification (#453)](https://github.com/Luminous-Dynamics/symtropy/issues/453)
- [AERO-QUAL-001 — NASA TMR / SU2 NACA 0012 case (#2960)](https://github.com/Luminous-Dynamics/symtropy/issues/2960)
- [CFD-REF-001 — Deterministic SU2 case runner (#2894)](https://github.com/Luminous-Dynamics/symtropy/issues/2894)

The repository already contains a rigorous external CFD reference programme. This physics guide applies the same principles to rigid-body and N-dimensional workloads: frozen case definitions, explicit source semantics, reproducible inputs, bounded conclusions, and no automatic promotion of a reference solver to physical truth.
