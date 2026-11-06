# Physics Qualification Corpus v0

This directory defines the first executable campaign that connects deterministic
replay, independent-rollout throughput and engine-qualification evidence.

It extends, rather than replaces, the existing `symtropy-physics` Criterion
benchmarks, `ReplayTape`, `WorldSnapshot`, and `InvariantSnapshot` APIs.
The family-level roadmap and exact status are indexed in
[`benchmark-family-index-v0.json`](benchmark-family-index-v0.json). Current
adjacent verification work includes [free-fall analytic convergence PR
#3870](https://github.com/Luminous-Dynamics/symtropy/pull/3870) and
[analytical CCD hardening PR
#3868](https://github.com/Luminous-Dynamics/symtropy/pull/3868). The first
implementation slice in this batch/replay PR adds:

- Stable rollout identity independent of worker scheduling.
- Canonical output ordering with fail-closed missing/duplicate/unexpected ID
  checks.
- A sequential-vs-parallel bitwise snapshot regression for 1, 2, 4 and 8
  independent workers.
- A Criterion batch-throughput benchmark for 64 replay-driven rollouts.

The current campaign is a reproducibility/performance foundation. It is not a
claim that the physical model is validated, that the solver is internally
parallel, or that Symtropy outperforms an external engine.

## Current frozen scenario

See [independent-rollout-batch-v0.json](manifests/independent-rollout-batch-v0.json).

The workload contains five 3D dynamic spheres in a fixed contact-rich layout,
a uniform gravity vector, deterministic per-rollout initial velocity offsets,
and three force commands delivered at fixed ticks through a `ReplayTape`.
Each rollout advances 90 fixed steps at 120 Hz. Rollout identity is a stable
integer from 0 through 63; the scenario's offsets depend on this ID, never on a
worker ID, thread ID, completion order or system clock.

The test version uses 16 rollouts, while Criterion measures 64. Both use the
same scenario-generation formulas.

## Result identity and ordering

A result is a pair of:

- `rollout_id`: stable campaign identity;
- `WorldSnapshot`: bitwise body state and collision-event snapshot.

Parallel completion order has no semantic meaning. Call
`order_batch_results(expected_rollouts, results)` before comparing results or
writing a receipt. The function rejects an empty campaign, out-of-range ID,
unexpected ID, duplicate ID or missing ID. It never silently shortens the
sample or lets a duplicate replace a missing run.

The snapshot sorts bodies by `BodyHandle` and collision events by the declared
stable key. This yields a reproducible canonical comparison for the captured
fields. It does not prove that every hidden solver cache or contact state is
captured for checkpoint/resume; replay/checkpoint state completeness remains a
separate acceptance item.

## Determinism classes

Keep the following claims distinct:

1. **Same-binary scheduling determinism** — the same frozen set of independent
   rollouts produces bitwise-identical per-rollout snapshots when run
   sequentially or scheduled across 1/2/4/8 workers on the same build and host.
   This is what the first implementation test exercises.
2. **Same-platform replay determinism** — a recorded command tape replayed on
   the same exact binary/config reproduces the per-tick state.
3. **Cross-platform determinism** — no cross-platform equality claim is
   made merely because the `deterministic-net` feature is enabled. That feature
   is a candidate deterministic profile, not proof. Exact equality may be
   claimed only after the complete execution path passes an explicit
   architecture/OS/compiler matrix with identical source/config/input digests.
   Until then, report the observed cross-platform error distribution and use a
   declared tolerance only when justified by the numerical method.
4. **Physical validity** — requires analytic solutions, invariant/constraint
   checks, reference-engine comparison and experimental validation where
   appropriate. Bitwise repeatability alone is not physical validity.

## Performance methodology

The Criterion benchmark group measures a complete batch of 64 replay-driven
rollouts with:

- a sequential baseline;
- standard-library worker counts 1, 2, 4 and 8, creating/joining threads inside
  each timed campaign;
- persistent Rayon worker pools of 1, 2, 4 and 8 threads, constructed outside
  the timed campaign;
- one frozen model/scenario definition and rollout-ID sequence;
- identical tick count, timestep, command schedule, body properties and output
  capture.

Criterion reports completed rollouts per second and batch latency. The two
parallel modes remain separate: `std_threads_spawned_*` includes worker
startup/join cost, while `rayon_persistent_*` measures scheduling and result
collection on an already-created work-stealing pool. A persistent-pool result
must not replace the end-to-end path when reporting application
startup-to-completion costs.

Run at least 20 independent campaign repetitions after warm-up for any
published comparison. Preserve all crashes, panics, missing results, non-finite
states and failed quality checks in the denominator. Report p50/p95 batch
latency, rollouts/s, worker count, peak memory, allocation bytes/rollout,
failure rate and energy per completed rollout when a supported energy meter is
available.

## Correctness and negative controls

The minimum gates are:

- Exact per-rollout snapshot equality against the sequential baseline.
- Complete ID coverage, stable canonical order and no duplicate IDs.
- Zero panics, missing results or invalid/non-finite snapshots.
- Explicit scenario/config digest and exact source commit.
- Negative controls for a missing rollout, duplicate result, out-of-range ID,
  perturbed rollout input/seed and changed model/configuration digest.
- Separate tests for replay/checkpoint state completeness and physical
  invariants; the current world snapshot is not assumed to contain every
  solver-internal contact cache.

## External reference baselines

Pin exact release versions and match model semantics before comparing:

- **MuJoCo 3.17.0 (22 Oct 2026)** — its batch API runs many simulations of one
  model on its own thread pool and documents bit-identical results to a serial
  loop across thread counts. This is the principal deterministic batch
  comparator. Its API supports per-simulation options, observation streams and
  trajectories. Important comparison caveats: default batches reuse one
  `mjData` per worker, so non-state fields/plugins outside the integration
  state may be shared/stale across simulations; a persistent batch keeps one
  `mjData` per simulation and is required for sleeping/IPC models. MuJoCo
  exposes per-simulation status/error results, and a failed simulation retains
  its previous state while other simulations may complete. Our comparison lane
  must capture these statuses and select the correct persistence mode, not only
  compare final state arrays.
- **Rapier 0.36.1 (4 Oct 2026)** — pin this as the latest tagged stable
  external Rust-engine release found. Rapier 0.36.0 added soft-body APIs and an
  experimental FEM solver; 0.36.1 added reduced-coordinate multibody state and
  motor bindings. The Dimforge Q3 2026 report describes a reworked parallel
  architecture with intra-island parallelism, persistent graph coloring and
  parallel broadphase. Keep that report's optimized development revision as a
  separate commit-pinned lane, not as an automatic property of the 0.36.1 tag.
  The vendor reports up to 2x single-threaded and 4x multithreaded speedups over
  its previous Rapier implementation on an M4 Max with 8 workers. It separately
  reports Nexus simulating 505K rigid bodies in about 14 ms/frame on an RTX
  5080. These are vendor-reported results for distinct engine revisions and
  workloads—not head-to-head measurements against Symtropy or universal
  body-count targets.
- **Box3D (30 Jun 2026 announcement)** — include as a major 3D game-physics
  comparator. Its architecture advertises C17, substepping, continuous collision,
  graph coloring for large islands, wide-SIMD contact solving, multithreading
  hooks/optional scheduler, large-world double positions, cross-platform
  determinism and recording/replay. Match API and solver settings carefully:
  these are project features, not proof of measured superiority.
- **Box2D 3.2 (18 Oct 2026)** — use for the 2D engine lane and as an algorithmic
  reference for contact recycling, SIMD dispatch, solver and broadphase
  optimization. The maintainer reports roughly 3x performance over 3.1 in its
  own workloads, with contact recycling, solver changes, AVX2 dispatch and
  broadphase each contributing. These are author-reported within-project gains,
  not cross-engine comparison data. Box2D 3.2 also includes recording/replay,
  world snapshots, native task scheduling, large-world double positions and
  continuous sensors.
- **Jolt Physics 5.6.0 (11 Jul 2026)** — include as a 3D rigid/soft-body and
  multithreading comparator, with GPU compute interfaces for DX12/Vulkan/Metal
  and strand-based hair simulation. Distinguish CPU/GPU backends and soft-body
  workloads from rigid-body solver comparisons.
- **Project Chrono 10.0.0 (27 Mar 2026)** — use for multibody, vehicle, FSI and
  deformable/material scenario families where model assumptions match. It adds
  checkpointing and updated implicit-integrator Jacobian strategies.
- **SUNDIALS 7.9.0** — use as an independent ODE/DAE/multirate numerical-solver
  comparator for matched equations, not as a rigid-body/contact-engine
  comparator.
- **PhysMark** — the Dimforge physics-engine benchmarking platform pins engine
  commits, builds each engine, runs identical scenes/timesteps/mass properties
  in separate processes, warms and synchronizes rollouts, checks physical
  sanity, and reports performance/memory curves and replay artifacts. Adopt
  these controls for our external engine lane; keep our analytic scenarios as
  the correctness oracle because a common renderer or sanity check is not an
  independent physical truth source.
- **ASME V&V 20** — use validation-point framing and account for both numerical
  solution uncertainty and experimental data uncertainty. Keep verification,
  validation and calibration separate.

Official reference links:
- MuJoCo 3.17.0 changelog: https://mujoco.readthedocs.io/en/3.17.0/changelog.html
- MuJoCo batch simulation: https://mujoco.readthedocs.io/en/latest/programming/simulation.html
- Rapier changelog: https://github.com/dimforge/rapier/blob/master/CHANGELOG.md
- Box2D 3.2 release notes: https://box2d.org/documentation/md_release__notes__v320.html
- Box2D contact recycling design: https://box2d.org/posts/2026/10/contact-recycling/
- Box3D announcement: https://box2d.org/posts/2026/06/announcing-box3d/
- Jolt releases: https://github.com/jrouwe/JoltPhysics/releases
- Dimforge Q3 2026 report: https://www.dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/
- Project Chrono 10.0.0: https://projectchrono.org/news/
- SUNDIALS release history: https://computing.llnl.gov/projects/sundials/sundials-software
- Dimforge Q3 2026 technical report: https://www.dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/
- PhysMark: https://github.com/dimforge/physmark
- ASME V&V 20: https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer

## Chaotic/contact systems: compare ensembles when trajectories decorrelate

Rigid-body contact trajectories can be highly sensitive to tiny state and
rounding differences. MuJoCo explicitly notes that small state differences can
grow significantly under contact and guarantees exact replay only within one
version on the same architecture, provided all required integration state is
saved. Therefore, a long-horizon cross-engine trajectory mismatch alone is not
a sufficient physical-validity verdict.

For systems with strong sensitivity, add an ensemble/observable comparison
alongside trajectory error:

- Freeze a distribution over initial conditions, material parameters and solver
  tolerances; bind every sample to a stable ID and digest.
- Compare distributions of physically meaningful observables: rebound height,
  settling time, contact impulse, penetration, energy change, support load,
  failure probability and final pose.
- Report means, quantiles, tail risk and confidence intervals.
- Where appropriate, compute a mathematically valid distribution distance
  (for example 1-Wasserstein on a declared, normalized observable vector).
  Do not call a scalar absolute prediction error "Wasserstein distance".
- Preserve failures and outliers; do not trim unstable runs from the ensemble.
- Separate numerical uncertainty (same model, solver variation) from model
  uncertainty (friction/restitution/material parameters) and measurement
  uncertainty (experimental sensor errors).

The acceptance question changes from "did every chaotic trajectory match
bitwise?" to "does the predicted observable distribution agree with reference
data within a declared uncertainty budget, while deterministic controls and
invariants pass?" This is a distinct qualification tier, not a relaxation of
analytic tests.

## 2026 GPU-physics and industry benchmark update

The comparator set has expanded beyond CPU rigid-body engines. Dimforge's
25 October 2026 Q3 report describes Nexus v0.7.0, a cross-platform GPU
multiphysics engine written in Rust shaders, and the Physmark benchmarking
suite. The report states a 505,000-rigid-body pyramid at approximately
14 ms/frame on an RTX 5080. This is a vendor-reported result for a particular
scene, engine revision and hardware; it is not an independently reproduced
Symtropy comparison and must not be treated as a directly comparable number.

The same report describes three distinct benchmark families used by Physmark:

1. Large-scale rigid-body scenes with tens/hundreds of thousands of bodies.
2. Many small independent environments for robotics/RL training.
3. Non-rigid MPM scenes.

It also describes Rapier parallelism work beyond independent islands:
intra-island parallelism, persistent graph coloring and parallel broadphase.
The reported up-to-2x single-thread / up-to-4x eight-worker speedups are
before/after vendor results on an M4 Max; they are not attributed to the tagged
Rapier 0.35.3 release unless a matching exact release/commit is verified.

This means the current 64-rollout CPU benchmark is only the **many-small-
environments CPU baseline**. It does not cover the large-single-world GPU or
MPM families. Track the extension in
[PHYS-GPU-001 #3866](https://github.com/Luminous-Dynamics/symtropy/issues/3866).

Required comparison rules:
- Separate bodies/frame, environments/second and particles/second; never
  aggregate unlike workloads into one score.
- Pin exact engine commit, solver/friction/contact mode, device backend,
  precision, worker count and scene digest.
- Report kernel-only and end-to-end time separately, including host/device
  transfer and synchronization.
- Preserve the full output/observable stream and failure IDs; do not silently
  omit timeouts or unsupported engines from the denominator.
- On current local RTX 2070 Laptop hardware, use the new workload family for
  correctness and profiling only. It is not hardware-equivalent to an RTX 5080.
- A GPU performance claim needs matched physical/numerical quality and actual
  supported energy measurements, not FLOP estimates.

Official research links:
- Dimforge Q3 2026 report:
  https://www.dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/
- Physmark:
  https://physmark.dimforge.com
- MuJoCo 3.17.0 batch API:
  https://mujoco.readthedocs.io/en/3.17.0/changelog.html

## Current analytic qualification work

The next physics-verification increments are tracked separately from the batch
throughput lane:

- [CCD helper invariance and exact TOI #3915](https://github.com/Luminous-Dynamics/symtropy/pull/3915):
  plane-equation scale invariance, invalid-input rejection and a stable
  sphere-sphere quadratic root. This qualifies analytical helpers, not the
  full world collision response.
- [Semi-implicit Euler work-precision #3916](https://github.com/Luminous-Dynamics/symtropy/pull/3916):
  a constant-acceleration closed-form fixture checks exact final velocity and
  first-order global position error. The symplectic guarantee applies only to
  the undamped conservative update; damping/non-conservative forces remove it.
- [World-level TOI response #3919](https://github.com/Luminous-Dynamics/symtropy/issues/3919):
  position clamping at exact TOI may leave a tangent sphere with no positive-
  depth contact manifold, so the impact impulse and remaining-step response
  must be verified separately.

A green replay test does not certify these physical properties. Keep output
determinism, analytic/numerical verification, cross-engine validation, physical
validation and performance as distinct acceptance layers.

## Future scenario families

1. Anisotropic rotational dynamics: asymmetric-top motion, off-axis products of inertia,
   long-rod torque response, compound parallel-axis mass properties and frame
   covariance. This is tracked in [PHYS-INERTIA-001 #3871](https://github.com/Luminous-Dynamics/symtropy/issues/3871); the current scalar-average inertia
   implementation is not a valid physical comparator for non-spherical bodies.
The first batch workload is only one lane. Extend the corpus with separately
frozen manifests for:

1. Smooth analytic dynamics: free fall, harmonic oscillator, Kepler orbit.
2. Rigid-body integration: asymmetric inertia, torque/rotation, gyroscopic
   motion.
3. Resting/contact: sphere/box/capsule on planes, stacks, mixed mass ratios,
   friction slopes, restitution and rotated contacts.
4. CCD: fast small projectile vs thin wall, rotation/translation and sleep/wake.
5. Constraints/joints: pendulum, chains, motors/limits and coupled structures.
6. Broadphase/query: static-heavy, sparse/dense dynamic scenes, ray/shape query
   counts and deterministic candidate ordering.
7. Lifecycle/replay: add/remove/reset/rebuild, snapshot/reload, checkpoint and
   replay continuation.
8. Scaling: 10/100/500/5,000/10,000 bodies; 1/2/4/8/physical-core workers;
   single-world and independent-rollout modes.

Each family gets task-specific physical observables and thresholds. Do not
aggregate them into one score until the per-family pass/fail and uncertainty
are visible.

## Proposed improvement targets

These are programme targets, not achieved results:

- 100% of benchmark runs preserve a frozen input/config/source identity.
- Zero missing, duplicated or silently discarded rollouts.
- Exact same-host per-rollout equality for the deterministic batch profile.
- At least 20 repeat campaigns for published performance claims.
- Compare rollouts/s and p95 latency at matched numerical/physical quality.
- For any claimed improvement, show a Pareto frontier across throughput,
  latency, memory, energy, numerical error and robustness.
- A candidate 10% improvement target is meaningful only when reproduced on
  three independent scenario families and no correctness/robustness gate regresses.

## Current status

The code branch adds the stable result contract, ordering validator, deterministic
thread-count tests and Criterion batch benchmark. No benchmark numbers have been
measured yet. The PR must compile, pass the exact tests and run the Criterion
campaign before any performance claim is made.
