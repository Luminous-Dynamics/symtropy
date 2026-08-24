# Symtropy Physics Excellence Program

## Purpose

Symtropy should not claim to be "better than PhysX" because it has one unusual
feature. The target is a physics platform that is simultaneously excellent for
games, credible for real-world simulation inside declared validity envelopes,
and unusually transparent about causality, conservation, determinism, and
numerical error.

The standard is evidence. A capability is not competitive until it passes a
published benchmark against an analytical solution, an independent engine, or a
well-defined production workload. A capability is not "best" until its advantage
survives matched conditions and the limitations are published beside the result.

## Current external reference set

The program should continuously compare against at least these independent
systems:

- NVIDIA PhysX: production rigid-body simulation on CPU/GPU, scene queries,
  joints, reduced-coordinate articulations, vehicles, character controllers,
  FEM soft bodies, SDF collision, PBD liquids/granular/cloth/deformables, Blast
  destruction, and Flow smoke/fire.
  Reference: https://developer.nvidia.com/physx-sdk
- Jolt Physics: high-performance multithreaded game rigid bodies, broad shape and
  constraint coverage, CCD, characters, vehicles, soft bodies, large-world
  double precision, and deterministic simulation under a documented execution
  contract.
  Reference: https://github.com/jrouwe/JoltPhysics
- MuJoCo: generalized-coordinate multibody dynamics, optimization-based contact,
  robotics, deformable flex elements, inverse/forward dynamics, and a mature
  validation culture.
  Reference: https://mujoco.readthedocs.io/
- NVIDIA Warp / Newton: GPU-native extensible simulation, differentiable kernels,
  sparse spatial structures, FEM/PDE tooling, and large-batch robotics/learning
  simulation.
  References: https://nvidia.github.io/warp/ and
  https://developer.nvidia.com/newton-physics

These are references, not enemies. Symtropy may use a different architecture and
still borrow benchmark ideas, file formats, or independently reproducible test
scenarios where licensing permits.

## North-star definition

A credible "best game/real physics engine" claim would require strength on four
axes at the same time:

1. **Production game physics** — robust, fast, predictable rigid bodies,
   collision, joints, queries, characters, vehicles, ragdolls, streaming worlds,
   debugging, and tooling.
2. **Physical fidelity** — explicit validity domains, conservation accounting,
   convergence, constitutive material laws, thermodynamics, deformables, fluids,
   fracture, and coupled multiphysics.
3. **Scale and performance** — multicore CPU, SIMD, GPU compute, adaptive fidelity,
   large worlds, deterministic scheduling, and graceful degradation.
4. **Auditability and causality** — replay, per-reservoir energy reconciliation,
   causal events, numerical residuals, solver-transition accounting, and evidence
   artifacts.

Symtropy's distinctive opportunity is axis 4 combined with 2, without sacrificing
1 or 3.

## Current strengths worth preserving

The current core already has useful foundations: const-generic dimensions,
GJK/EPA and analytical narrowphase paths, oriented collision support, a static
broadphase cache, CCD scaffolding, TGS-style iterative contact solving, multiple
joint types, deterministic replay, invariant snapshots, and a documented
research-validation protocol. The thermodynamic stack adds explicit sensible
energy, conduction, first-law and second-law auditing, external heat, measured
friction dissipation, an energy-transfer ledger, and state-versus-ledger
reconciliation.

The engine also has an articulated-chain builder, but this is currently a chain
of constrained rigid bodies rather than a reduced-coordinate articulation solver.
That distinction must remain explicit in public claims.

## Critical gaps before parity with top game engines

### P0 — rigid-body correctness before breadth

These are release blockers for any claim of top-tier rigid-body quality:

- Full body-space/world-space inertia tensors for asymmetric bodies.
- Gyroscopic coupling and torque-free asymmetric-top dynamics.
- Robust persistent multi-point manifolds across all production primitive pairs.
- General convex CCD with angular motion and time-of-impact islands.
- Better GJK/EPA degeneracy handling and a permanent adversarial corpus.
- Warm-start/contact-cache feature identity robust to manifold point changes.
- Static, dynamic, and rolling friction models with physically meaningful work.
- Restitution and solver-bias accounting that separates physical work from
  numerical stabilization.
- Constraint drift/error reporting per joint and per island.
- Stable stacking, high mass-ratio, thin-object, fast-object, and long-duration
  torture tests.

No advanced multiphysics feature compensates for a box stack that is less stable
than Jolt/PhysX.

### P0 — production game API

To compete as a game engine rather than a research crate:

- Complete scene queries: ray, overlap, shape cast/sweep, closest point, filters,
  query batching, and deterministic result ordering options.
- Character controller / physical character system.
- Ragdoll authoring and animation-to-physics pose driving.
- 6-DOF, cone/swing-twist, gear, rack-and-pinion, pulley, and path constraints.
- Vehicle dynamics: wheeled, tracked, suspension, tire model, drivetrain, and
  deterministic test tracks.
- Collision layers/groups and callbacks with production-grade filtering.
- Serialization/cooking, stable handles, world snapshots, versioned scene data.
- Physics debug visualization, contact/joint overlays, profiling, capture/replay.
- Origin shifting and/or double-precision large-world mode.

### P0 — performance architecture

Current feature breadth is not enough without scalable execution:

- Deterministic island scheduling independent of hash/ECS iteration order.
- Parallel broadphase, narrowphase, and independent island solving.
- SoA/hot-cold body storage where profiling shows benefit.
- SIMD-specialized 2D/3D kernels while keeping the N-D reference path.
- Allocation budgets and cache-miss measurements in representative scenes.
- GPU roadmap with an exact CPU reference implementation first.
- GPU broadphase, contact generation, constraint solve, and particle/continuum
  backends where batching wins.
- A reproducible CPU/GPU performance lab with fixed scenes and hardware metadata.

Performance claims must report physical error and determinism mode alongside
throughput. A faster unstable solver is not a win.

## Multiphysics roadmap beyond PhysX-like breadth

### Unified material and matter state

Long-term material identity should be separated from state and constitutive law:

`Matter = composition + thermodynamic state + mechanical history + representation`

Candidate state includes density, velocity, temperature, pressure, stress,
strain, phase fractions, damage, porosity, moisture, composition, and explicit
energy reservoirs where the active model requires them.

### Solver federation

Different regimes need different numerical methods. Do not force one solver to
simulate everything.

- Rigid bodies for mostly-rigid objects.
- FEM/XPBD for elastic/deformable solids and cloth where appropriate.
- MPM for soil, snow, granular/continuum transitions, and large deformation when
  evidence shows it is the right method.
- SPH/FLIP/PIC/grid hybrids for fluids depending on the target regime.
- Eulerian fields for smoke, atmosphere, combustion products, and heat diffusion.
- SDF/sparse voxel/implicit structures for editable large matter volumes.
- Reduced-order models for distant or low-causal-importance regions.

The architectural innovation should be conservative and auditable transitions
between representations, not merely placing many solvers in the same repository.

### Required representation-transition contract

Every promotion/demotion must declare:

- source and destination representations,
- conserved quantities,
- physical dissipation/source terms,
- projection/lifting error,
- numerical residual,
- deterministic ordering,
- hysteresis to prevent representation thrashing.

Example target:

`static terrain -> stressed continuum -> fracture -> fragments -> rigid bodies -> rubble/granular`

with mass and momentum conserved to tolerance and energy losses explicitly
assigned to fracture, plasticity, thermal energy, or numerical residual.

## Thermodynamics roadmap

The current lumped sensible-heat work is only the beginning. The next validated
regimes should be:

1. World-solver friction and collision dissipation routed through measured energy
   change, replacing dimensional heuristics.
2. Contact-derived conductance from material properties and contact geometry.
3. Spatial heat diffusion with explicit stability diagnostics.
4. Temperature-dependent heat capacity and conductivity.
5. Enthalpy-based solid/liquid phase change with latent heat.
6. Thermal expansion and thermoelastic stress.
7. Temperature-dependent yield, fracture toughness, viscosity, and friction.
8. Fluid heat advection and convection.
9. Radiation and environmental exchange.
10. Chemistry/combustion only after the lower energy accounting is trustworthy.

## Deformables, fracture, terrain, and fluids

PhysX already exposes FEM soft bodies, PBD fluids/granular/cloth/deformables,
Blast destruction, and Flow smoke/fire, so Symtropy cannot claim multiphysics
leadership based on plans alone.

The differentiator should be coupling quality:

- fracture changes collision topology and material state,
- fracture work is assigned to new-surface energy, plastic work, and heat,
- water enters pores/cracks and changes effective stress,
- freezing/melting consumes/releases latent energy,
- erosion transports actual represented mass,
- rubble can demote to a granular or reduced-order representation,
- terrain excavation changes both geometry and mass accounting.

Each coupled demo must first be decomposed into small analytical/independent
validation cases.

## Robotics / articulated dynamics gap

The existing `ArticulatedChain` is useful game/robotics scaffolding but does not
replace reduced-coordinate dynamics. To compete with PhysX articulations and
MuJoCo:

- generalized coordinates,
- articulated-body or equivalent O(n) dynamics,
- loop-closure strategy,
- inverse dynamics,
- actuator/sensor models,
- joint-space limits and friction,
- Jacobians,
- stable contact-rich robotics benchmarks,
- independent comparison against MuJoCo.

This should be a separate backend behind the same causal/energy interfaces rather
than rewriting the rigid-body solver around robotics requirements.

## Differentiable simulation

Differentiability is strategically valuable for control, system identification,
optimization, and learned policies, but it should not distort the production game
solver. Build it as an optional research mode/backend after deterministic forward
physics is trustworthy.

Targets:

- finite-difference reference Jacobians first,
- analytic/automatic differentiation for smooth subsystems,
- explicit contact nonsmoothness policy,
- gradient validation against finite differences,
- GPU batch execution where useful.

## Adaptive physical fidelity

A major opportunity beyond conventional engines is to choose representation and
resolution from causal importance rather than camera distance alone.

A fidelity score may include:

- distance,
- kinetic/thermal/elastic energy,
- instability or impending failure,
- gameplay relevance,
- uncertainty,
- causal connectivity to high-priority regions,
- expected representation-transition error.

The scheduler must expose why fidelity changed and preserve a declared error
budget. This is essential for planetary/large-world simulation where full local
fidelity everywhere is impossible.

## Benchmark ladder

### Gate R1 — rigid-body analytical correctness

Required: free fall, constant force, elastic/inelastic collision, harmonic
oscillator, pendulum, torque-free spherical and asymmetric rotation, rolling,
friction incline, gyroscope, and orbital cases where applicable. Show convergence.

### Gate R2 — collision robustness

Required: adversarial GJK/EPA corpus, transformed primitives, deep/shallow
penetration, near-degenerate geometry, thin-wall CCD, rotational CCD, persistent
manifold stability, and no-nonfinite fuzz/property campaigns.

### Gate R3 — production game scenes

Run matched scenes against Jolt and PhysX where practical:

- 1k/10k/100k falling bodies,
- tall/high-mass-ratio stacks,
- ragdoll pile,
- vehicle test track,
- character crowd/controller obstacle course,
- mixed static-mesh/convex scene,
- high-speed projectile scene.

Report throughput, memory, penetration/error, constraint drift, determinism mode,
and failure counts.

### Gate R4 — robotics

Matched articulated pendulums, manipulators, humanoids, and contact tasks against
MuJoCo/PhysX articulations. Report trajectory error, constraint/joint error,
energy/work budgets, stability, and runtime.

### Gate M1 — thermodynamics

Complete the declared conduction, external heating, phase-change, radiation, and
thermo-mechanical campaigns with first/second-law audits.

### Gate M2 — deformables/fluids/fracture

For each solver, require analytical or published reference cases before combined
demos. Examples: beam/cantilever, wave propagation, hydrostatic column, dam break,
Taylor-Green vortex where appropriate, granular angle of repose, Stefan phase
boundary, and fracture benchmarks.

### Gate S1 — solver-transition conservation

Promote/demote the same physical region through multiple representations and
measure mass, momentum, angular momentum, energy accounting, geometric error,
and hysteresis behavior.

### Gate D1 — determinism

Define levels rather than one vague promise:

- D0: replay within one process/build.
- D1: repeatable across runs on one binary/platform.
- D2: reproducible across supported CPU architectures under a documented mode.
- D3: rollback/lockstep-safe networking subset.
- D4: GPU reproducibility contract, which may be tolerance/envelope based rather
  than bit-identical if performance requires it.

### Gate P1 — performance

No benchmark result is accepted without scenario source, revision, compiler,
features, hardware, thread count, timestep, solver settings, and physical-error
metrics.

## Claims policy

Use four labels:

- **Implemented** — code exists and local tests pass.
- **Validated** — an analytical or independent comparison campaign passes.
- **Competitive** — matched external benchmarks are within the declared target.
- **Leading** — the same matched benchmark demonstrates a reproducible advantage
  in fidelity, performance, determinism, auditability, or a combination thereof.

"Best" should only be used for a precisely named benchmark class, never as an
unqualified marketing statement.

## Recommended execution order

### Phase 1 — make rigid physics boringly excellent

1. Full inertia tensor and gyroscopic dynamics.
2. Replace world friction-dissipation heuristic with measured work/energy.
3. General convex + rotational CCD.
4. Contact/manifold robustness and stack torture campaign.
5. Complete production joint/query coverage.
6. Multicore deterministic island execution.
7. Character, ragdoll, and vehicle reference implementations.

### Phase 2 — make the engine measurable at scale

1. Cross-engine benchmark harness.
2. Determinism matrix.
3. Profiling/debug capture format.
4. Large-world/double-precision strategy.
5. SIMD/SoA optimization from profiling evidence.
6. GPU reference/fast-path split.

### Phase 3 — make matter unified

1. Material/composition/state/constitutive interfaces.
2. Spatial thermal cells.
3. One validated deformable solver.
4. One validated fluid solver.
5. Fracture/topology transition.
6. Conservative representation transitions.
7. Enthalpy/phase change and thermo-mechanical coupling.

### Phase 4 — make it uniquely capable

1. Causal adaptive-fidelity scheduler.
2. Cross-solver matter/energy ledger everywhere.
3. Reduced-coordinate robotics backend.
4. Optional differentiable backends.
5. Planetary/large-scale reduced-order simulation.
6. Complete `rock -> fracture -> rubble -> water infiltration -> collapse`
   vertical experiment with small-case evidence for every constituent coupling.

## Immediate next PRs

The highest-value next implementation work is deliberately less glamorous than
adding a new solver:

1. Replace the current contact-solver friction dissipation heuristic with measured
   pre/post work/energy accounting and distinguish physical friction work from
   stabilization/restitution/numerical residual.
2. Add full 3D inertia tensor and asymmetric-top validation.
3. Add an engine-vs-engine rigid-body benchmark manifest and adapters, beginning
   with Jolt/Rapier/PhysX scenarios where integration/licensing allows.
4. Harden general convex CCD and create a projectile/tunneling corpus.
5. Design deterministic parallel island scheduling before broad GPU expansion.

If these fail, fix them before adding more breadth. If they pass, they become the
foundation on which the multiphysics architecture can credibly outperform
conventional engines rather than merely being more ambitious.