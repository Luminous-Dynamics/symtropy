# Robotics Platform Demo Maturity Inventory v0.1

**Parent:** [Symtropy #5700 — Robotics Platform Simulation Portfolio](https://github.com/Luminous-Dynamics/symtropy/issues/5700)  
**Qualification workstream:** [Symtropy #5702 — Robotics Demo Qualification](https://github.com/Luminous-Dynamics/symtropy/issues/5702)  
**Inspection scene:** [Symtropy #5701 — Built-Asset Robot Simulation Article](https://github.com/Luminous-Dynamics/symtropy/issues/5701)  
**Snapshot basis:** public Symtropy `main` inspected 16 November 2026.  
**Status:** source inventory only; builds, launches, replay, model validation and physical trials not verified by this document.

## 1. Purpose and claim boundary

Symtropy already contains a family of Bevy robotics demo applications. This document records their existence and the tasks/models described by their source entry points so the portfolio can mature them into useful, reproducible simulations.

A Cargo manifest and a `main.rs` file establish that an application is present in the repository. They do **not** establish that it builds, launches, reproduces its results, implements the described model faithfully, or represents a safe/qualified physical robot.

Use the following independent maturity states; do not infer one from another:

- `ManifestExists`
- `EntryPointExists`
- `BuildVerified`
- `LaunchVerified`
- `ScenarioReplayed`
- `OracleChecked`
- `ModelValidated`
- `HILVerified`
- `PhysicalTrialQualified`

At this snapshot, all twelve applications have a manifest and an entry point. This inventory did not execute Cargo builds, launch the Bevy applications, replay a scenario, independently validate the physics models, run HIL or conduct physical trials. Those states remain `NotVerified`.

## 2. Existing application inventory

All paths below are repository-relative. The source object IDs identify the files inspected for this inventory; later file revisions must be re-audited.

| Application | Main source / task described | Manifest blob SHA | Entry-point blob SHA |
|---|---|---|---|
| `symtropy-auv-demo` | `crates/apps/symtropy-auv-demo/src/main.rs`; waypoint navigation under underwater currents; entry point describes a 6-DOF hydrodynamic simulator with added mass, quadratic drag and buoyancy | `0adbb923976464f7729c0806480a3410447aafb7` | `19e05a19d2d5a0986c62a3aa23441c8419990419` |
| `symtropy-exoskeleton-demo` | `crates/apps/symtropy-exoskeleton-demo/src/main.rs`; human gait plus exoskeleton assistance modes | `85360be8f9b1db584e6ace2123bcef39df2b6661` | `27c68d861649549269acd6c390d79606ab296a13` |
| `symtropy-flight-demo` | `crates/apps/symtropy-flight-demo/src/main.rs`; quadrotor waypoint flight with wind-gust disturbance; motor lag and aerodynamic drag described | `8813781b89957d2672494470e6a0158dc31e698e` | `2ba4da2af6013341e4144ac8dd707b822af62296` |
| `symtropy-gravcraft-demo` | `crates/apps/symtropy-gravcraft-demo/src/main.rs`; metric-perturbation visualization, not a conventional robot task | `bec144ae39b3b0b0e7729594c6ee29aaced18300` | `4fee157b75dc683e396a2b0c52454b0f2f678532` |
| `symtropy-helicopter-demo` | `crates/apps/symtropy-helicopter-demo/src/main.rs`; SAR hover / station keeping with rotor dynamics and wind | `133823355e0a9206010b3cc465c1cee205038e06` | `d46d3d26cc56694ac8e9c941558f8a8c6eb91624` |
| `symtropy-humanoid-demo` | `crates/apps/symtropy-humanoid-demo/src/main.rs`; 21-DOF standing balance under push disturbances | `d583aba205be52b6a130bc53bed172cb89365ffb` | `f6ea2e4a139ece4bdba041e4b161aaa6f816aa55` |
| `symtropy-manipulator-demo` | `crates/apps/symtropy-manipulator-demo/src/main.rs`; Franka Panda pick/place assembly and moving-human-obstacle comparison | `af6ba9b96b941445a9617093b2d9f624740fd15b` | `4cf5b879311d34bb9119e0bc6c22ff27d6455631` |
| `symtropy-orbital-demo` | `crates/apps/symtropy-orbital-demo/src/main.rs`; spacecraft-mounted 7-joint arm with reaction-wheel, solar-exposure and communication-window state | `32aaf8b80b8008e89d9a54f58875883dfc7e1f41` | `82b792d9a2d9c824d94f78ff8ca12d8ffbf68405` |
| `symtropy-quadruped-demo` | `crates/apps/symtropy-quadruped-demo/src/main.rs`; 12-DOF gait selection and terrain scene; source describes spring-damper foot contact | `43c27bcf5a90792e71c4315f7ddbeaf36b193a87` | `f1e6bc4dcc0159851c504b23db5d81de9a0f1960` |
| `symtropy-scavenger-demo` | `crates/apps/symtropy-scavenger-demo/src/main.rs`; current scene spawns a Scavenger agent and three reclaimable scrap-metal targets | `1ff552d7b025c0e23b1ae321861a71e75e22df94` | `c8a2d42b69aa7ab36dba39c7d3c9347b559cfca7` |
| `symtropy-surgical-demo` | `crates/apps/symtropy-surgical-demo/src/main.rs`; surgical manipulator, tissue-contact proxy, trocar constraint and energy-interlock demonstration | `5fc41b1d18dc5ccaa731a4c81a08638b3d73fda3` | `68d65d5f5c7d60a3314a25b9557e5549aec9d011` |
| `symtropy-vehicle-demo` | `crates/apps/symtropy-vehicle-demo/src/main.rs`; bicycle-model vehicle waypoint task; source describes tire saturation, weight transfer, drag, ice patches and crosswind | `d977102fd57f35725849ac9e0980788a29b1a400` | `46359c9172ac999f62110974d2d42a27e677d7ff` |

The physical details in the middle column are descriptions in the demo source, not independent model-validation results.

## 3. Build and workspace caveat

The root `Cargo.toml` deliberately excludes the robotics demo applications from the standalone root workspace. Several app manifests refer to sibling Symthaea crates through relative paths (for example, `../../../../symthaea/crates/domains/...`). Therefore:

1. a green `cargo test --workspace` in the public Symtropy checkout does not, by itself, compile or test these applications;
2. per-application build/launch validation must run in the intended monorepo layout where the required sibling repositories and compatible toolchains exist;
3. each app needs an explicit workflow/job and exact source SHA;
4. skipped, queued or absent jobs remain `NotVerified`, not PASS.

## 4. Robotics bridge taxonomy snapshot

The inspected permissively licensed `crates/bridges/symtropy-robotics-bridge-core/src/platform.rs` defines ten `PlatformType` variants:

- `Quadrotor`
- `Vehicle`
- `Humanoid`
- `Auv`
- `Helicopter`
- `Manipulator`
- `Surgical`
- `Orbital`
- `Quadruped`
- `Exoskeleton`

Its inspected blob SHA is `0f2046fbe5138c4872386ee669df57a157e8779f`.

The AGPL bridge conversion in `crates/bridges/symtropy-robotics-bridge/src/platform.rs` explicitly returns `UnsupportedPlatform` for Symthaea embodiment variants that have no bridge-core equivalent. The inspected blob SHA is `b72ee35d2f5defeeb92dbc66f3718bcb4805201a`.

The source explicitly lists physical and non-physical examples, including Subterranean, Infrastructure, Scavenger, Agribot, Biota, Clime, Detritivore, CareProvider, Browser, Phone and Desktop. This is not necessarily a defect: these may be different categories, and not all should become robot types. The next step is a semantic compatibility matrix, not an indiscriminate enum expansion.

In particular:
- Scavenger has a demo application but no `PlatformType::Scavenger` variant in the ten-member bridge-core enum.
- Gravcraft is a specialized simulation/visualization app and should not automatically be classified as a robot embodiment.
- A future Subterranean or Infrastructure platform may require task- and environment-specific capability profiles rather than a generic vehicle-like fallback.

Unsupported variants must remain explicit; never silently coerce to Humanoid, Vehicle or another default.

## 5. Immediate code-review findings to validate

### 5.1 Manipulator throughput claim

The manipulator demo entry-point comment says the adaptive safety-gradient approach “proves” a 20–40% throughput improvement over the ISO/TS 15066 SSM comparison.

The inspected `iso_comparison.rs` defines a simplified binary stop/go controller using a protective-distance formula and contains unit tests for that local formula. This alone is not a complete implementation or conformity assessment of ISO/TS 15066, and it does not substantiate a 20–40% end-to-end throughput claim.

Before retaining that claim, create a matched experiment with identical robot/task trajectories, obstacle trajectories, sensing uncertainty, speed/acceleration/deceleration limits, stopping dynamics, acceptance criteria, safety boundary and compute budget. Report task output alongside minimum separation, contact/violation events, protective stops, stop distance, false-negative events, uncertainty calibration and operator intervention. Safety failures are hard failures and cannot be compensated by throughput.

### 5.2 Quadruped locomotion scope

The quadruped entry point describes a CPG-driven simulator with spring-damper foot contact and gait-dictated forward velocity. This is suitable as a bounded controller/mode demonstration, but does not by itself establish high-fidelity terrain interaction, locomotion robustness, fall recovery or field access capability. Add those claims only after independent model and measurement comparisons.

### 5.3 Gravcraft visualization completeness

The Gravcraft source itself comments that amplifier-cone and geodesic-trail visual entities are not spawned even though the entry-point description promises those visual elements. Track this as an unfinished visualization item, separate from any physical-propulsion claim.

### 5.4 Scavenger scene scope

The current Scavenger startup scene spawns three reclaimable scrap-metal targets. That is a useful seed for a materials-recovery workcell, but it does not yet establish robust sorting, material identification, contamination handling, grasping, accepted recovered mass or residual-waste accounting.

## 6. Priority order for turning demos into useful platforms

### P0 — first task-oriented qualification

1. **Manipulator assembly cell:** accepted assembly units, part/fixture identity, grasp slip/drop, placement quality, dynamic obstacle uncertainty, interruption/recovery and the safety-comparison claim audit.
2. **Quadruped inspection/access:** traverse a controlled terrain course, reach inspection waypoints, acquire declared views, return with reserve; measure slip/fall, mission completion, energy and coverage.
3. **AUV inspection:** bounded subsea inspection route with current/depth variation, localization degradation, data coverage and return reserve.
4. **Multirotor inspection:** structure/site survey with view-angle coverage, wind, payload, GNSS/communications faults and return/abort.
5. **Vehicle logistics:** site route, docking, cargo handoff and receipt-backed delivery; arrival alone is not accepted delivery.

### P1 — specialized useful applications

- Humanoid: manipulation and tool-use tasks only after contact, locomotion and recovery are sufficiently validated.
- Exoskeleton: user-intent and assistance-mode experiments with wearer override and no medical/clinical inference.
- Helicopter: SAR route, search coverage, hover, payload and mission-reserve accounting.
- Orbital: payload capture/installation with reaction dynamics, contact stability, collision avoidance and communication windows.
- Scavenger: material sorting, recovery, contamination and accepted-material accounting.
- Surgical: research-only, non-clinical simulation; no patient-safety, clinical-performance or regulatory claims.
- Gravcraft: visualization and speculative physics research only; not a qualified robotics platform.

## 7. Benchmark and evidence requirements

For every platform/task pair, freeze:
- exact software, model, platform morphology and configuration;
- initial conditions, scenario IDs, seeds, environment, payload and sensor profile;
- observation channels available to the controller;
- evaluator-only ground truth and independent reference;
- task completion and functional acceptance predicates;
- baselines, compute/energy budget and trial exclusions.

Report per-task/per-condition results, not one cross-platform composite score:
- accepted functional units and first-pass acceptance;
- task success and stage reconstruction;
- geometric error, uncertainty and coverage;
- false completion, false-clear, missed defects and safe-abstention;
- collision/contact and safety events;
- recovery time, rework, duplicates and residual WIP;
- p50/p90/p95 elapsed time, energy and resource use;
- direct operator hours, support burden and total workflow cost;
- model residuals and uncertainty coverage.

Repeated sensor frames and repeated retries are not independent experimental units. Use held-out sites/assets/tasks and preregistered minimum detectable effects before making improvement claims.

## 8. Proposed qualification targets (not results)

- 100% deterministic replay on the frozen development corpus.
- 100% oracle agreement on finite public known-answer fixtures.
- Zero false accepted task completion on critical negative controls.
- Zero duplicate accepted functional units after retry/replay.
- Zero silent acceptance of stale robot/tool/sensor/calibration/configuration revisions.
- At least 98% held-out task-state reconstruction agreement, with confidence intervals and per-stratum reporting.
- At least 95% correct hold/abstention on unknown critical observations.
- Model verification/validation gates passed before claiming comparative physical performance.
- Publish p50/p90/p95 and total workflow cost, not only mean task success.

These are prospective engineering targets. They are not current results, safety limits, industry standards or guarantees.

## 9. External comparison anchors

Use the following as task-specific anchors, without treating unlike benchmark scores as directly comparable:

- [NIST Performance Assessment Framework for Robotic Systems](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems): compose task-driven perception, mobility, dexterity and safety measures.
- [BEHAVIOR 2026 evaluation rules](https://behavior.stanford.edu/challenge/evaluation.html): report partial goal-predicate completion and normalized efficiency measures separately; its task set is household robotics, not industrial/construction qualification.
- [LADDER](https://ladderrobotics.ai/): long-horizon manipulation and cost-per-completed-task comparator; this is not a physical construction benchmark.
- [MOST](https://arxiv.org/abs/2611.00734): task-progress and object-state memory as a distinct long-horizon design comparator.
- [RoboCasa-Mem](https://arxiv.org/abs/2611.06475): behavior-conditioned constraints generated from actual earlier interactions, relevant to process memory and alternate valid paths.
- [VDA 5050 v3.0](https://www.vda.de/en/topics/automotive-industry/vda-5050): interoperability for mixed mobile-robot fleets, including zones and path sharing.
- [AMRA-201:2026](https://amr-alliance.org/amra-201/): published general requirements/test methods for mobile robots operating on solid travel surfaces; explicitly does not cover underwater or airborne robots.
- [Dusty FieldPrinter specifications](https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs): manufacturer-declared ±1/16 in (±1.6 mm) print accuracy under accurate surveyed control; manufacturer conditions must be reproduced before any comparison.

For construction productivity, align machine/run attribution with established project and work-package productivity measurements. For material-flow interoperability, adopt relevant open interfaces where they fit rather than designing a private fleet protocol.

## 10. Next implementation tasks

1. Establish a working build environment for the twelve apps and capture exact-head per-package build output.
2. Add headless deterministic run paths to the P0 apps without coupling the evaluator to Bevy rendering.
3. Extract task scenarios and result reports from UI-only logic into testable domain interfaces.
4. Add known-answer and adversarial fixtures for the P0 tasks.
5. Validate each physics model against an independent analytical, published or instrumented reference appropriate to its domain.
6. Connect task success to the existing Symtropy authorities for accepted output, mission progression, sensor condition, configuration applicability and physical evidence.
7. Keep simulation, HIL and physical-trial claims as separate evidence levels.

## 11. Nonclaims

This inventory does not establish that any application builds, launches, passes an independent benchmark, accurately models a real robot, meets a safety standard, is ready for clinical/industrial/field operation, or surpasses SOTA. Each claim requires an exact revision, a reproducible test and evidence appropriate to the claim.
