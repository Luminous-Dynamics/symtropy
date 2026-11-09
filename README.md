# Symtropy

**A Rust physics, geometry and simulation substrate for evidence-qualified engineering and embodied systems.**

Symtropy provides reusable N-dimensional mathematics, collision and rigid-body simulation, articulated constraints, robotics adapters, and research domains. Its purpose is to make physical models composable, inspectable, reproducible and progressively accountable to measured reality.

The long-term direction is not to replace CAD, FEA, CFD, EDA, PLM or manufacturing execution systems. Symtropy should connect to specialist tools, preserve their authoritative artifacts, execute bounded physical models, and emit machine-readable evidence about assumptions, numerical behaviour, uncertainty, validation and applicability.

> **Claim boundary:** Symtropy is an evolving research and software project. A passing software test, simulation, benchmark fixture or synthetic engineering study is not by itself physical validation, product qualification, functional-safety certification or authorization to manufacture or deploy hardware.

## What Symtropy is

Symtropy is organized around a reusable physical-computation core and separable domain integrations:

- **Geometry and mathematics:** const-generic N-dimensional primitives, transforms, rotors and shape interfaces.
- **Rigid-body physics:** collision detection, contact handling, continuous collision detection, constraints, joints and simulation stepping.
- **Simulation infrastructure:** deterministic ordering where specified, snapshots/replay, bridges and benchmark tooling.
- **Robotics integration:** adapters and reference applications for embodied platforms, locomotion and manipulation research.
- **Domain research:** terrain, energy, aerospace, industrial and other bounded models.
- **Experimental state coupling:** optional research mechanisms that couple declared per-entity metrics to simulated dynamics.

The core should remain useful without the experimental consciousness-physics layer, Bevy presentation, networking, or any one robotics platform.

## Engineering philosophy

Symtropy treats simulation as an evidence-producing activity, not an oracle.

A consequential result should be accompanied by enough context to answer:

1. **What was evaluated?** Exact model, configuration, geometry, materials, solver, software revision and input data.
2. **Under which conditions?** Units, coordinate frames, boundary conditions, loads, initial state, environment and operating domain.
3. **What was assumed?** Explicit assumptions, omissions, parameter sources and evidence classes.
4. **How credible is the computation?** Numerical tolerances, convergence, conservation/residual checks, sensitivity, uncertainty and known solver limitations.
5. **What supports the model?** Analytical solutions, manufactured solutions, reference data, independent solvers, physical measurements or other clearly identified evidence.
6. **What does the result authorize?** A result may support exploration or a design comparison without supporting qualification or release.

A simulation result must not silently become a qualification result. Missing inputs remain missing; out-of-domain results remain out of domain; stale evidence remains stale after a relevant model or configuration change.

See [the engineering evidence model](docs/tech/ENGINEERING_EVIDENCE_MODEL.md) and [competitive benchmark strategy](docs/research/SYMTROPY_COMPETITIVE_BENCHMARK_STRATEGY_2026-11-09.md).

## Place in the Luminous ecosystem

The projects have complementary authorities. They should exchange versioned contracts rather than duplicate each other's source of truth.

| Project | Primary responsibility |
|---|---|
| **Symtropy** | Physical models, mechanics, dynamics, thermal/environmental analysis, numerical methods and simulation evidence |
| **Sol-Atlas** | Spatial intelligence, geometric relationships, scene representations, localization and spatial provenance |
| **Symthaea** | Cognitive reasoning, perception, planning, belief and human interaction |
| **Mycelix** | Identity, trust, permissions, provenance, attestations and governance |
| **Luminous platform** | Product lifecycle, requirements, engineering decisions, BOMs, technical documentation, revisions, production and service records |

For a humanoid or other engineered product, the Luminous platform owns the lifecycle and configuration baseline. Symtropy evaluates the physical questions assigned to it. Sol-Atlas owns spatial interpretation and spatial provenance. Symthaea may help explore and reason about candidates, but cannot silently change an approved baseline. Mycelix supports identity, authorization and attestations.

Symtropy should consume and emit stable, versioned engineering contracts. It should not become the owner of the product BOM, requirements database or manufacturing release.

## Architecture at a glance

The repository is a multi-crate Rust workspace. The high-level dependency and integration map is:

```text
symtropy-math
    |
    +-- symtropy-physics
    |       |
    |       +-- robotics / Bevy / rendering / networking bridges
    |       |
    |       +-- domain-specific physical models and tools
    |
    +-- experimental research domains
            |
            +-- consciousness-physics and state-coupling experiments
```

The exact crate graph, features and supported APIs evolve. Consult the manifests and crate-level documentation for the authoritative current details:

- [Architecture guide](docs/tech/ARCHITECTURE.md)
- [Engine notes](ENGINE.md)
- [Crate and feature licensing](docs/LICENSING.md)
- [Experimental features and claim boundaries](docs/EXPERIMENTAL_FEATURES.md)
- [Current physics validity envelope and known limitations](crates/core/symtropy-physics/RESEARCH_VALIDATION.md)
- [Oriented-collision validation](crates/core/symtropy-physics/ORIENTED_COLLISION_VALIDATION.md)
- [Robotics platform roadmap](Robotics_Platform_ROADMAP.md)
- [Cross-simulator semantic equivalence contract](docs/research/CROSS_SIMULATOR_SEMANTIC_EQUIVALENCE_V0_1.md)

## Research tracks

### 1. General physical computation

Improve correctness, robustness, performance and usability of geometry, collision detection, contact resolution, constraints, integration, and replay. Prefer well-defined numerical contracts and independently checkable reference cases over feature-count growth.

### 2. Robotics and embodied systems

Develop reusable models and adapters for articulated systems, locomotion, manipulation, actuator dynamics, fault injection, terrain interaction, energy and thermal constraints. Compare candidates under matched tasks, payloads, environments, controller budgets and evidence tiers.

### 3. Engineering model credibility

Build the verification and validation discipline around every solver and domain model:

- analytical and manufactured-solution tests;
- convergence and mesh/time-step studies where applicable;
- conservation, residual and invariant checks;
- sensitivity and uncertainty propagation;
- independent reference comparisons;
- simulation-to-test correlation;
- explicit applicability domains and model-discrepancy records.

### 4. Experimental state-coupled physics

The `symtropy-consciousness-physics` domain explores Φ-related and other state-coupled mechanisms. These are research hypotheses and configurable simulation mechanisms, not established physical laws or validated robot-safety controls. Claims about IIT, subjective experience, thermodynamic closure, or real-world motor authority require separate definitions and independent evidence.

## What Symtropy is not

Symtropy is not intended to be:

- a CAD authoring or product-data-management replacement;
- a universal FEA, CFD, multibody, electromagnetics or battery-chemistry solver;
- an EDA, CAM, MES or PLM replacement;
- a safety PLC, certified controller, or independent safety authority;
- a substitute for physical testing, metrology, inspection or competent engineering review;
- evidence that a robot, vehicle, aircraft or industrial system is safe or production-ready.

Instead, Symtropy should interoperate with authoritative specialist tools and make the lineage and limitations of exchanged data explicit.

## Benchmarks: measure credibility as well as speed

Performance matters, but throughput alone is not a sufficient measure of an engineering simulator. Benchmark suites should be versioned, reproducible and task-specific.

| Benchmark family | What to measure |
|---|---|
| Numerical correctness | Error against analytical/manufactured solutions; residuals; invariant and conservation drift |
| Collision and contact | Detection precision/recall against reference cases; penetration and contact error; stability under difficult geometry |
| Dynamics | Trajectory/state error, energy and momentum behaviour, constraint drift, sensitivity to step size |
| Articulated systems | Joint-limit behaviour, constraint residuals, contact transitions, closed-chain stability |
| Robustness | Adversarial geometries, extreme mass ratios, near-degenerate contacts, high-speed impacts and malformed inputs |
| Determinism | Repeatability on a frozen platform/toolchain; explicit cross-platform reproducibility limits |
| Performance | Throughput and latency distributions, scaling curves, memory and allocation profiles, not isolated best-case timings |
| Model credibility | Calibration error, held-out prediction error, uncertainty coverage and domain-shift detection |
| Physical correlation | Prediction-versus-measurement error under matched configurations and documented sensor uncertainty |
| Engineering workflow | Reproducibility from immutable inputs, provenance completeness, stale-evidence detection and change-impact coverage |

Every public comparison should publish the exact commit, toolchain, hardware, fixture corpus, configuration, warm-up/repetition policy, statistical summary and raw results. Do not compare unlike tasks or mix synthetic, simulated and physical evidence into one score.

The first priority is not an unsupported claim to beat every incumbent. It is a trustworthy public benchmark corpus and repeatable results against established reference problems and appropriate specialist tools.

## Getting started

The workspace uses Rust and includes Nix/Just development support. Start with the toolchain and contributor documentation:

```bash
rustc --version
cargo metadata --locked --no-deps --format-version 1
```

For focused library checks, use the relevant package names and feature set from the current workspace:

```bash
cargo test -p symtropy-math --locked
cargo test -p symtropy-physics --locked
```

Some applications and optional integrations require platform libraries, additional features or dependencies that are not needed by the pure math/physics crates. Consult the crate manifests and CI workflow before building the full launcher.

## Contributing

Contributions should preserve explicit scope boundaries and make claims reproducible.

- Keep changes focused and reviewable.
- Add regression tests and negative controls for new behaviour.
- Bind evidence to exact inputs and software revisions.
- Do not weaken a gate to make a workflow green.
- Distinguish `queued`, `not run`, `failed` and `passed`.
- Report what was actually tested; do not infer physical validation from CI.
- Preserve uncertainty and unknown states rather than silently substituting defaults.

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Symtropy uses a multi-crate, multi-license arrangement. **Do not infer a crate's license from the repository default.** Consult [LICENSING.md](docs/LICENSING.md) and the individual crate manifests before incorporating a crate into a product.
