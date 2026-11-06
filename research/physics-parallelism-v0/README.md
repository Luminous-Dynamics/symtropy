# Symtropy single-world parallelism qualification v0

This programme separates three distinct kinds of parallel work:

1. **Independent simulations** — many separate worlds/environments, each advanced
   independently. PR #3657 measures this with replay-driven rollouts and
   sequential, spawned-thread, and persistent-Rayon execution modes.
2. **Inter-island solver parallelism** — disconnected components inside one
   physics world can be advanced/solved independently, provided all shared
   resources and event side effects are handled explicitly.
3. **Intra-island solver parallelism** — contacts/constraints within one
   connected component are scheduled in conflict-free stages (for example
   deterministic graph colors), with stable ordering and a declared numerical
   reproducibility contract.

A fast independent-world benchmark does not demonstrate either of the latter
two. Likewise, a fast inter-island benchmark says nothing about a single large
connected stack or articulated mechanism.

## Current Symtropy source state

- `island::build_islands` exists as a graph helper, but source inspection found
  no call from `PhysicsWorld::step`. The helper is not yet part of the live
  solver pipeline.
- PR #3883 canonicalizes emitted components by minimum stable `BodyHandle`,
  assigns contact/constraint edges once in a linear pass after union-find, and
  checks exactly-once edge ownership. It is a decomposition prerequisite, not
  a working parallel solver.
- The same PR adds a Criterion decomposition benchmark comparing the
  single-pass candidate against a benchmark-only reproduction of the prior
  per-component-rescan algorithm. Both outputs are canonicalized and compared
  for exact partition equivalence before timing. The legacy comparison is
  omitted for the 100k-isolated-body case, where its per-island HashSet
  allocations are intentionally pathological; the candidate is still timed.
- `PhysicsWorld::step` performs integration, CCD, broadphase, narrowphase and
  contact solving in serial loops.
- `PhysicsCallback` is mutable and includes force/impulse modulation, event
  notification, dissipation recording and work recording. Sharing it between
  workers would serialize or race side effects. The first experimental lane
  should keep `step_with_callback` sequential and begin with callback-free
  `step` only.
- Contact manifolds mutate two bodies and carry accumulated per-point lambdas.
  Manifold-level conflict graphs must use body ownership; contact points inside
  one manifold remain sequential initially.

## Industry reference lanes (version-pinned semantics)

### MuJoCo

- MuJoCo 3.10 introduced an `mju_threadpool` attached to `mjData`. Parts of
  the engine pipeline, including collision detection and constraint solving
  across islands, can run in parallel. The 3.10 changelog also notes moving
  island sparse-matrix construction into the multithreaded constraint-forward
  path.
- MuJoCo 3.17.0 (22 Oct 2026) adds `mjBatch`: many independent simulations of
  one model on a thread pool, with bit-identical output to a plain loop at any
  thread count. This is an independent-simulation comparator, not evidence that
  every solver operation inside one world is parallel.
- Benchmark the two MuJoCo capabilities separately: single-world engine
  threading and independent batch simulation.

Official sources:
- https://mujoco.readthedocs.io/en/3.17.0/changelog.html
- https://mujoco.readthedocs.io/en/3.17.0/APIreference/APIfunctions.html

### Rapier

- The v0.35.0-beta.0 changelog describes a PhysicsPipeline/PhysicsWorld thread
  pool, persistent islands, a persistent contact graph, staged multithreaded
  solving (including parallelism within one island), and a reworked broadphase.
- The changelog states that `enhanced-determinism` can be combined with
  `parallel` and produce bitwise-identical results across pool sizes when
  numeric features match. Cross-platform claims still depend on matching
  numeric features such as f32/f64 and block-solver settings.
- The current v0.35.3 stable tag (28 Aug 2026) is a separate release snapshot.
  Do not attribute every v0.35.0-beta.0 or Q3 report development change to that
  stable tag without checking the exact commit/tag contents.
- Dimforge's Q3 2026 report (25 Oct) describes a reworked Rapier parallel
  architecture using intra-island parallelism, persistent graph coloring and
  parallel broadphase. The report states up to 2x single-thread and 4x
  multithread improvements relative to their earlier implementation on an M4
  Max with 8 worker threads, but explicitly warns that the graphs in the report
  use early-August engine versions. Treat this as an engineering reference and
  development-commit lane, not a direct SOTA result to copy into our benchmark.

Official sources:
- https://github.com/dimforge/rapier/blob/master/CHANGELOG.md
- https://www.dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/

### Broader physics benchmark families

- **Physmark** (Dimforge) separates large-scale rigid-body stacks/piles,
  many-environment robotics simulation, and non-rigid MPM workloads. Adopt the
  scenario diversity, raw replay/video evidence and explicit timeout behavior;
  do not rely on a single body-count curve.
- **Project Chrono 10.0.0** is a comparator for multibody, FSI, peridynamics,
  flexible materials and vehicle scenarios where model semantics are
  equivalent; it is not a simple substitute for every rigid-contact test.
- **SUNDIALS 7.9.0** is a reference for matched ODE/DAE/multirate numerical
  systems, not an apples-to-apples rigid-body engine.

Sources:
- https://physmark.dimforge.com/
- https://chronoengine.info/news/
- https://computing.llnl.gov/projects/sundials/sundials-software

## Decomposition microbenchmark

Run the new target with:

```sh
cargo bench -p symtropy-physics --bench island_decomposition_bench
```

It creates each frozen fixture outside the timed region and measures only
island construction and edge-to-island assignment. Workloads include 1k/10k
isolated bodies, 1k/10k disjoint contact pairs, contact chains and mixed
contact/constraint edges; the single-pass candidate also runs at 100k isolated
bodies. For the <=10k cases, the same fixture is run through a benchmark-only
reproduction of the previous per-island rescan algorithm. Before timing, the
benchmark asserts equality of canonical body/contact/constraint membership
and sleeping flags. This gives an apples-to-apples algorithm comparison, not a
whole-world or solver speedup claim. No performance result is recorded until
the benchmark is run on a frozen machine/toolchain and its run receipt is
stored.

## P0: make decomposition a live, inspectable stage

Before threading, integrate the island helper as an opt-in/read-only analysis
stage in `PhysicsWorld::step`:

- Build the graph after narrowphase has produced contacts and before iterative
  contact/constraint solve.
- Include dynamic bodies and define explicit policy for static/kinematic
  endpoints, sensors, isolated bodies and invalid/stale handles.
- Verify every eligible dynamic body appears once; each contact/constraint
  with known endpoints belongs to exactly one component; fully unknown edges
  are counted as rejected rather than silently lost.
- Publish a canonical per-step manifest: stable island ID, sorted body handles,
  contact IDs, constraint IDs, sleeping/awake status, size histogram and
  component count.
- Keep this stage read-only initially. Compare partition membership against a
  slow independent graph oracle in tests.
- Add negative controls: duplicate body, omitted contact, stale handle,
  cross-island edge, reversed contact insertion order, randomized HashMap seed,
  and reordered constraints.

## P1: inter-island execution

Only after P0 coverage passes:

- Add an opt-in parallel feature with a persistent worker pool.
- Start with callback-free `step`; mutable callback steps remain sequential.
- Each island owns its dynamic body writes. Static/kinematic bodies are shared
  read-only and must not be mutated by separate islands.
- Produce island-local event/diagnostic buffers. Merge in canonical island ID
  order after all workers complete.
- Do reductions in a deterministic fixed order over island-local summaries;
  no shared floating-point atomics/reductions in the numeric path.
- Prove sequential/parallel equivalence at identical solver iterations,
  dt, contact ordering and numerical features.

## P2: intra-island graph coloring

Once inter-island scheduling is stable:

- Model every contact manifold and constraint as a graph edge/operation with
  the set of body handles it mutates.
- Deterministically greedy-color in stable operation-ID order. Operations of
  one color may execute concurrently only when their mutable body sets are
  disjoint.
- Process color phases in fixed order and retain sequential Gauss-Seidel order
  where dependencies exist.
- Keep contacts within one manifold sequential initially; their lambda state
  and both bodies are shared.
- Compare graph-coloring setup cost with available parallelism. Tiny islands or
  low-edge-count stages should stay serial.

## Benchmark matrix

Use frozen manifests and test these regimes separately:

1. **Many islands:** 1k / 10k / 100k small bodies in disconnected pairs or
   small stacks. This is the inter-island scaling case.
2. **Few large islands:** 1 / 2 / 4 dense stacks, chains and constrained
   mechanisms. This exposes load imbalance and intra-island opportunities.
3. **One giant island:** tall resting stacks, mixed mass ratios, high-friction
   contacts and joint chains. This cannot be made faster by simply running
   independent islands.
4. **Broadphase-heavy:** static-heavy scenes, high static collider counts,
   sparse/dense dynamic candidates and insertion/removal lifecycle.
5. **Many environments:** thousands of small independent worlds, reported as
   the separate batch lane in PR #3657.

Run 1/2/4/8 workers plus physical-core count when distinct. Include one-thread
parallel mode as overhead control. Warm up; run at least 20 measured campaigns;
interleave baselines/candidates. Report p50/p95 step latency, bodies/s,
contacts/s, constraints/s, speedup, parallel efficiency, load imbalance,
island count/size histogram, allocation bytes/step, peak memory, numerical
residuals and event mismatch. Report joules/step only when a supported
hardware-energy receipt exists and its sensor scope is declared.

## Acceptance targets (programme goals, not achieved results)

- 100% eligible-body and edge ownership coverage.
- Stable component IDs and membership across repeated graph builds and input
  insertion-order variations.
- Same-host bitwise equality across worker counts for a declared deterministic
  profile; if a solver stage is not bitwise deterministic, quantify and
  predeclare a numerical tolerance with invariant/constraint error bounds.
- A 1.5x target at 4 workers on many-island workloads after instrumentation
  overhead, not merely a best-case anecdote.
- A 1.2x target on one large island only after intra-island coloring exists.
- No more than 5% regression in one-worker p95 step latency.
- Repeat a claimed gain on three scenario families and report confidence
  intervals, failure rate and memory cost.

## Claim boundary

A correct island partition is not a solver speedup. A solver speedup is not
physical validation. Same-host bitwise replay is not cross-platform
reproducibility. Each claim gets a distinct evidence class and exact source,
scenario, toolchain, platform and settings receipt.
