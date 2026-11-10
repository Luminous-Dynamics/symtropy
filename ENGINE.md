# Symtropy Engine: Architecture, Scope and Research Boundaries

**Last reviewed:** 2026-11-09  
**Status:** architectural guide; not a qualification report.

Symtropy is a Rust-based physical-computation and simulation workspace. Its reusable core includes N-dimensional mathematics, collision and rigid-body dynamics, constraints, replay primitives and integrations for robotics and domain-specific research.

It is an evolving research and software project. The existence of a type, API, example, feature flag or passing unit test does not establish that every physical regime involving that type is numerically verified, experimentally validated, qualified for engineering use or safe for deployment.

For the current, narrower capability envelope, see:
- [Physics research validation protocol](crates/core/symtropy-physics/RESEARCH_VALIDATION.md)
- [Oriented-collision validation](crates/core/symtropy-physics/ORIENTED_COLLISION_VALIDATION.md)
- [Engineering evidence model](docs/tech/ENGINEERING_EVIDENCE_MODEL.md)
- [Exact engine capability/model-path census issue #1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472)
- [Engine-wide physics qualification corpus issue #460](https://github.com/Luminous-Dynamics/symtropy/issues/460)

## 1. Architectural layers

The system is intentionally layered so that general physical computation does not depend on experimental cognition or presentation features.

| Layer | Responsibility | Authority boundary |
|---|---|---|
| `symtropy-math` | N-dimensional geometric primitives, transforms, rotations and support-mapped shapes | Mathematical and geometric operations only |
| `symtropy-physics` | Rigid-body state, integration, collision queries, contacts, joints, constraints, CCD and replay primitives | Only the exact implemented and qualified model paths |
| Domain crates | Robotics, terrain, energy, aerospace, industrial and other bounded models | Each domain declares assumptions and validity limits |
| Bridges | Bevy, robotics, external physics engines, rendering and ecosystem interfaces | Adapters must preserve units, frames, identity and limitations |
| Experimental research domains | State-coupled dynamics, Φ-related experiments and other exploratory models | Research hypotheses; never implicitly promoted to engineering authority |

The root workspace and crate manifests are the source of truth for package names, dependency direction and enabled features. This guide is not a substitute for reading the exact source revision and capability census.

## 2. Physical capability and claim discipline

A generic interface does not imply that every implementation behind it has the same physical fidelity.

Examples:
- A const-generic shape type does not prove every dimension and shape pair is qualified.
- A CCD API does not imply rotational CCD or general convex-pair CCD.
- An articulation or joint type does not establish complete reduced-coordinate multibody dynamics or inverse dynamics.
- A sensor-overlap event is not a calibrated physical sensor model.
- A deterministic replay within one fixed build is not cross-platform bitwise determinism.
- A successful simulation is not physical qualification.

The current physics validation protocol distinguishes credible paths from paths that remain unqualified. It documents, among other limits, the restricted CCD world paths, limitations in asymmetric inertia, and the absence of production FEM/MPM or fluid solvers in the core physics crate.

A backend capability report should distinguish at least:
- supported exact-under-declared-profile;
- supported numerical model;
- supported approximation;
- supported through a declared provider;
- partially supported;
- unsupported;
- unknown or unassessed.

Unknown must not be interpreted as supported. An unsupported query must not silently turn into a valid physical miss or a zero-valued observation.

## 3. General physical computation

The general-purpose core is intended to support bounded simulation tasks such as:
- geometric transforms and support queries;
- convex collision detection;
- contact and rigid-body response;
- translational and rotational state integration within documented model limits;
- joint and constraint experiments;
- ray/shape queries;
- deterministic scenario and replay experiments under a declared execution profile.

Numerical correctness must be established with analytical or manufactured solutions, metamorphic tests, convergence studies, independent implementations and physical measurements as appropriate.

The [physics validation protocol](crates/core/symtropy-physics/RESEARCH_VALIDATION.md) defines the validity ladder and known limitations. The [PHYS-VALID-06 corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460) is the shared destination for machine-readable engine qualification evidence.

The analytic and numerical oracle work is coordinated through [PHYS-VALID-07 #3749](https://github.com/Luminous-Dynamics/symtropy/issues/3749). Cross-backend matched comparisons additionally require [PHYS-SEMANTIC-EQUIVALENCE-001 #4366](https://github.com/Luminous-Dynamics/symtropy/issues/4366); its validator is a draft and is not yet a merged production gate.

## 4. Robotics and engineering use

Symtropy can support robotics research and engineering by providing a physical-analysis runtime and adapters. It is not, by itself:
- a CAD authoring system;
- a universal FEA, CFD, electromagnetics or battery-chemistry solver;
- an EDA, CAM, MES or PLM system;
- a certified safety controller or safety PLC;
- a substitute for calibrated instrumentation, physical testing or engineering approval.

For the Luminous humanoid programme:
- the Luminous engineering platform owns requirements, product configuration, EBOM/MBOM, revisions, technical documentation and production/service records;
- Symtropy owns the physical models and simulation execution it actually supports;
- Sol-Atlas owns spatial intelligence and spatial provenance;
- Symthaea supports perception, reasoning and planning;
- Mycelix supplies identity, permissions, provenance and attestations.

The first physical demonstrator is the repairable rotary-joint work package [#4337](https://github.com/Luminous-Dynamics/symtropy/issues/4337). It explicitly assigns specialist FEA and thermal tools where Symtropy does not have a qualified model path.

## 5. Experimental state-coupled and Φ-related research

The `symtropy-consciousness-physics` domain explores coupling between declared state metrics and simulated physical dynamics. Some experiments use Φ-related quantities associated with Integrated Information Theory; other coupling experiments can use application-defined scalar metrics.

These mechanisms remain experimental.

They should not be described as:
- established physical laws;
- proof of subjective experience, sentience or qualia;
- a validated causal account of biological consciousness;
- a qualified real-world motor-authority or human-safety mechanism;
- proof of universal thermodynamic closure.

A `ThermodynamicLedger` records the quantities and accounting rules implemented by its model. It does not establish that every modeled action captures all physical energy flows or that a proposed Joules-per-Φ quantity is a universal physical invariant. A Landauer-bound comparison is a theoretical reference, not proof that a physical device obeys a particular implemented budget. CEMI-inspired or harmony-field experiments are explicitly model-specific hypotheses.

The right scientific posture is to publish:
- exact Φ definition and estimator;
- system boundary and partition;
- approximation and computational complexity;
- null and negative controls;
- sensitivity to estimator choices;
- comparison with alternative metrics and no-coupling baselines;
- repeatability and uncertainty;
- claims limited to the tested simulation.

These experiments should remain modular and must not weaken the evidence or safety requirements of the general physics and robotics stack.

## 6. Performance reporting

Performance measurements are meaningful only with their full execution context:
- exact source commit and crate/profile;
- compiler and target;
- CPU/GPU, driver and power state;
- workload size, geometry and contact distribution;
- solver, precision, timestep, iterations and backend;
- warm-up and repetition policy;
- latency distribution, memory, allocations and failures.

Historical microbenchmarks are not general claims of superiority. A few nanoseconds for a primitive collision query do not establish faster end-to-end contact simulation, articulated robotics or engineering analysis.

The benchmark programme is described in [SYMTROPY_COMPETITIVE_BENCHMARK_STRATEGY_2026-11-09.md](docs/research/SYMTROPY_COMPETITIVE_BENCHMARK_STRATEGY_2026-11-09.md).

## 7. Interoperability

Symtropy should interoperate with specialist tools while preserving their authoritative outputs. Candidate interchange surfaces include:
- STEP AP242 for managed product geometry and engineering data;
- URDF and MJCF for articulated robotics descriptions;
- OpenUSD for composed scenes and simulation assets;
- FMI/FMU for dynamic-model exchange and co-simulation;
- solver-specific formats and raw result artifacts where specialist tools remain authoritative.

Format import is not semantic equivalence. Every adapter must specify units, frames, topology, mass properties, material/solver assumptions, unsupported fields and provenance. Pairwise simulator comparisons are gated by [PHYS-SEMANTIC-EQUIVALENCE-001 #4366](https://github.com/Luminous-Dynamics/symtropy/issues/4366).

## 8. Build, licensing and contribution

Use the workspace manifests and the current [README](README.md) for build instructions. Pure library crates have different system requirements from the full Bevy launcher.

Symtropy has a multi-crate, multi-license model. Do not infer a crate's license from the repository default. Consult [LICENSING.md](docs/LICENSING.md) and each crate manifest before embedding a crate in a product.

Contributions should:
- make a focused change;
- add regression tests and negative controls;
- state the exact model path and supported domain;
- preserve unknown and inconclusive results;
- report what was actually run;
- distinguish CI evidence from numerical validation and physical qualification.

**The governing rule:** Symtropy may help produce engineering evidence, but a solver output does not grant itself authority over a physical design.
