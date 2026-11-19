# Symtropy Engine: Native Physics, Research Coupling and Model Federation

**Status:** engineering overview. This page distinguishes source-backed native capabilities, experimental coupling models and planned/external solver integrations. It is not a physical-validation or certification statement.

## What Symtropy is

Symtropy is a Rust simulation substrate whose native physics core focuses on N-dimensional geometry, rigid-body dynamics, contact and constraints, replay, and integration with Bevy. Its broader engineering direction is to coordinate domain-specific physical models through explicit interfaces and evidence-qualified composition.

The native engine is not a universal FEM, CFD, electromagnetics, electrochemistry, plasma, fusion, geoscience or circuit solver. Those domains require specialist models or future, separately qualified implementations.

## Terminology and scientific scope

The research layer uses the symbol Φ for a configured integrated-information metric associated with Integrated Information Theory (IIT). In this codebase, a computed Φ value is a model quantity produced by a particular estimator and configuration. It is not, by itself, evidence of subjective experience, sentience, a validated physical field, or a universal constitutive variable.

Likewise, “harmony”, “sanctuary”, “J/Φ” and “consciousness-physics” refer to named constructs in experimental simulation models. They should not be interpreted as established physical quantities or laws without independent operational definitions, dimensional analysis, mechanism and validation.

## Native architecture

```
symtropy-math
  N-dimensional geometric primitives, transforms and support-mapped shapes
       |
       v
symtropy-physics
  rigid-body world, collision/contact, constraints, CCD, raycasting, replay
       |
       +---- symtropy-bevy-core / Bevy adapters
       |
       +---- PhysicsCallback<D> (low-level application/research hook)
                  |
                  v
       symtropy-consciousness-physics
         experimental metric-coupled behaviors and energy bookkeeping

Separate composition paths:
  typed model ports + scenario identity + solver adapters + qualification
  specialist FEM / CFD / PIC / MHD / electrical / chemical / geoscience models
```

The current codebase exposes several supported shapes and constraints. Exact pairwise contact routes, dimensional specializations, CCD coverage, numerical approximations and replay guarantees vary by implementation path. Consult the machine-readable capability work in [#1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472); do not infer full fidelity from a generic trait or type signature.

## Native physics capabilities and limits

The documented native substrate includes:
- const-generic N-dimensional geometry and rigid-body types;
- sphere, capsule, hyperbox, half-space and convex-hull shape families;
- GJK/EPA collision machinery with specialized contact paths;
- contact manifolds, warm-starting, broadphase collision filtering and sleeping;
- bounded continuous-collision paths;
- joint/constraint and motor-related primitives;
- raycasting, overlap/sensor events and deterministic replay structures;
- an optional fixed-point quantization profile for a declared networking subset.

Important scope limits include:
- Generic shape abstraction does not mean every shape pair follows an equally exact or equally tested contact path.
- CCD support is shape-pair and profile dependent; it is not universal for every rapidly moving convex body.
- Some rotational-inertia paths are approximations.
- Sensor overlap events are not calibrated camera, lidar, radar, IMU, thermal or force/torque sensor models.
- Replay determinism is limited to its declared execution profile; it does not imply cross-platform bitwise identity for all floating-point or multithreaded runs.
- Long contact stacks, high stiffness, severe mass ratios and difficult constraint systems require dedicated stress and convergence qualification.
- A successful build or test suite does not establish predictive validity against physical measurements.

The current package-level public capability audit is [#1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472). That census is the source of truth for exact feature paths and should be kept synchronized with code.

## The PhysicsCallback extension point

`PhysicsCallback<D>` is a low-level hook invoked in the rigid-body simulation path. It can let an application or research model modify intended forces, contact impulses and friction coefficients, and observe collisions/dissipation.

This hook is useful for controlled experiments and policy-conditioned actuation. It is **not** the general multiphysics federation interface.

A callback that changes friction as a function of an arbitrary metric is implementing a model rule. It is not automatically describing a measured material law. To claim a physical effect, a model needs:
1. an operationally defined quantity and units;
2. a proposed physical mechanism and constitutive relationship;
3. parameter provenance and a stated validity range;
4. analytic or manufactured verification where possible;
5. independent validation against relevant physical observations;
6. uncertainty and failure/abstention behavior.

### Research coupling examples

The research crate contains experimental behaviors such as:
- metric-conditioned motor-authority mappings;
- model-specific energy reservoirs and flows;
- collision prediction-error feedback;
- spatial harmony/resonance fields;
- convergence and statistical analysis helpers.

These mechanisms should be described as experiments under explicit parameters, not as general laws of thermodynamics, friction, gravity or consciousness. The details and current limitations are in [FORMAL_SPECIFICATION.md](FORMAL_SPECIFICATION.md).

## Performance measurements

Historical local Criterion measurements on NixOS / Rust stable / an AMD Ryzen CPU (single-threaded) include:

| Operation | Reported time |
|---|---:|
| GJK sphere-sphere 3D | 156 ns |
| GJK capsule-capsule 3D | 173 ns |
| GJK HyperBox-HyperBox 3D | 116 ns |
| GJK HyperBox 4D | 101 ns |
| GJK ConvexHull tesseract 4D | 359 ns |
| EPA sphere-sphere 3D | 30 ns |
| Raycast 100 bodies | 14 μs |
| Physics step, 10 bodies | 5.8 μs |
| Physics step, 100 bodies | 135 μs |
| Physics step, 500 bodies | 910 μs |

These are local microbenchmarks from a particular host, compiler, feature set and workload. They are not a matched comparison against MuJoCo, Newton, Chrono, Rapier, Isaac Sim or a commercial multiphysics suite. They do not establish physical accuracy or SOTA superiority.

For fair comparisons, match equations and model semantics, geometry, solver settings, integrator, timestep, observables, hardware and resource budget. Report single-world latency separately from batch throughput, and compare error/convergence/uncertainty alongside speed. See [REALITY_BENCHMARK_PORTFOLIO.md](../ops/REALITY_BENCHMARK_PORTFOLIO.md).

## Research results and interpretation

The repository contains many experiments on cooperation, clustering, resource allocation and metric-coupled behavior. Results such as tighter clustering, convergence of a J/Φ ratio or collapse/recovery under selected energy parameters are conditional outcomes of the specified simulation model.

They do not by themselves establish:
- that cooperation is a universal thermodynamic necessity;
- that J/Φ is a validated biological or physical constant;
- that the model's energy bookkeeping is a general proof of thermodynamic closure;
- that Φ is a measured physical field or an explanation of subjective experience.

Each result should be accompanied by the exact code/configuration, seed policy, sample size, statistical analysis, negative controls and independent replication status.

## Reality-physics composition

The long-term platform strategy is documented in:
- [Reality Physics Platform](REALITY_PHYSICS_PLATFORM.md): domain portfolio, model passports, typed ports, solver federation, ownership and claim boundaries.
- [Reality Benchmark Portfolio](../ops/REALITY_BENCHMARK_PORTFOLIO.md): matched industry comparisons, benchmark cases, VVUQ, uncertainty and resource metrics.

Existing owners to compose:
- [#4487 multi-fidelity admission and escalation](https://github.com/Luminous-Dynamics/symtropy/issues/4487)
- [#1467 engineering twin projection and synthetic boundary](https://github.com/Luminous-Dynamics/symtropy/issues/1467)
- [#1472 capability/model-path census](https://github.com/Luminous-Dynamics/symtropy/issues/1472)
- [#5779 typed physical ports and coupling semantics](https://github.com/Luminous-Dynamics/symtropy/issues/5779)
- [#4275 fluid-structure coupling](https://github.com/Luminous-Dynamics/symtropy/issues/4275)
- [#6156 FMI co-simulation qualification](https://github.com/Luminous-Dynamics/symtropy/issues/6156)
- [#1685 solver-neutral plasma adapter](https://github.com/Luminous-Dynamics/symtropy/issues/1685)

The core competitive proposition is not to replace every specialist solver. It is to coordinate the least costly model that is admissible for a declared physical question, escalate when the validity envelope is exceeded, and abstain when no model is sufficiently supported.

## Bevy integration

Bevy provides the application/ECS/rendering integration. FixedUpdate scheduling and frame presentation should be distinguished from the physical model's own timebase. The exact Bevy and crate versions in the current workspace manifests take precedence over stale examples in older documentation.

## Related documents

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [REALITY_PHYSICS_PLATFORM.md](REALITY_PHYSICS_PLATFORM.md)
- [FORMAL_SPECIFICATION.md](FORMAL_SPECIFICATION.md)
- [REALITY_BENCHMARK_PORTFOLIO.md](../ops/REALITY_BENCHMARK_PORTFOLIO.md)
- [ROADMAP.md](../ops/ROADMAP.md)
