# Physics Qualification Corpus v0

This directory defines the first executable campaign that connects deterministic
replay, independent-rollout throughput and engine-qualification evidence.

It extends, rather than replaces, the existing `symtropy-physics` Criterion
benchmarks, `ReplayTape`, `WorldSnapshot`, and `InvariantSnapshot` APIs.
The first implementation slice adds:

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
3. **Cross-platform deterministic profile** — exact equality across CPU
   architectures/compilers is claimed only when the complete execution path is
   under an explicitly verified deterministic profile such as
   `deterministic-net`; it must not be inferred from same-host results.
4. **Physical validity** — requires analytic solutions, invariant/constraint
   checks, reference-engine comparison and experimental validation where
   appropriate. Bitwise repeatability alone is not physical validity.

## Performance methodology

The Criterion benchmark group measures a complete batch of 64 replay-driven
rollouts with:

- a sequential baseline;
- standard-library worker counts 1, 2, 4 and 8;
- one frozen model/scenario definition and rollout-ID sequence;
- identical tick count, timestep, command schedule, body properties and output
  capture.

Criterion reports completed rollouts per second and batch latency. The current
parallel path creates and joins the worker threads inside each batch, so thread
setup cost is included. This is an end-to-end baseline, not a persistent-pool
lower bound. A later persistent-worker implementation should be benchmarked as
a separate execution mode, never silently substituted in this benchmark.

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
  trajectories. MuJoCo's 3.14 IPC flex-contact mode has a separate replay caveat:
  contact multipliers are not represented by ordinary state snapshots, so
  exact replay is unsupported for that mode.
- **Rapier 0.35.3 (28 Aug 2026)** — pin the latest tagged release found for the
  stable external Rust engine lane. Its release notes cover recent contact,
  multibody and solver changes. A Dimforge Q3 2026 report describes a reworked
  parallel architecture with intra-island parallelism, persistent graph
  coloring and parallel broadphase; treat this as a separate commit-pinned
  development lane, not automatically as the 0.35.3 release baseline.
- **Project Chrono 10.0.0 (27 Mar 2026)** — use for multibody, vehicle, FSI and
  deformable/material scenario families where model assumptions match. It adds
  checkpointing and updated implicit-integrator Jacobian strategies.
- **SUNDIALS 7.9.0** — use as an independent ODE/DAE/multirate numerical-solver
  comparator for matched equations, not as a rigid-body/contact-engine
  comparator.
- **ASME V&V 20** — use validation-point framing and account for both numerical
  solution uncertainty and experimental data uncertainty. Keep verification,
  validation and calibration separate.

Official reference links:
- MuJoCo 3.17.0 changelog: https://mujoco.readthedocs.io/en/3.17.0/changelog.html
- MuJoCo batch simulation: https://mujoco.readthedocs.io/en/latest/programming/simulation.html
- Rapier changelog: https://github.com/dimforge/rapier/blob/master/CHANGELOG.md
- Dimforge Q3 2026 report: https://www.dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/
- Project Chrono 10.0.0: https://projectchrono.org/news/
- SUNDIALS release history: https://computing.llnl.gov/projects/sundials/sundials-software
- ASME V&V 20: https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer

## Future scenario families

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
