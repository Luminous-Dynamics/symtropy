# Robotics Multiphysics Digital-Twin World Contract V0.1

**Status:** Research/architecture proposal; documentation only  
**Date:** 2026-10-13  
**Companion:** SUBTERRANEAN_ROCK_PHYSICS_INTEGRATION_CONTRACT_V0_1.md  
**Related:** Symtropy physics-composition proposal PR #487; Symthaea rock roadmap PR #7438

## 1. Purpose and decision

Symtropy should become the common, replayable robotics digital-twin world in which our robotics projects can be composed and tested. This does not mean forcing every physical phenomenon into the rigid-body engine. It means defining one world contract, scenario format, robot/platform contract, clock model, evidence/provenance scheme, and qualification vocabulary, while allowing domain-specific physics and sensor solvers to be selected explicitly.

The twin must distinguish:
- **canonical scenario truth**: what the scenario declares to exist;
- **simulated physical state**: the state computed by each declared solver and fidelity profile;
- **sensor observation**: a modelled sensor output with calibration/noise/latency/occlusion;
- **inference**: Symthaea's estimate from admitted observations;
- **measured physical evidence**: data captured from actual instruments;
- **rendering/presentation**: how the state is displayed.

These are different kinds of data, never interchangeable. A beautifully rendered crack is not evidence that a crack solver predicts the right crack; a simulated acoustic emission is not an independent physical measurement.

## 2. Core responsibilities

| Layer | Responsibility | Must not claim |
|---|---|---|
| Symtropy world/scenario host | World identity, composition, clocks, deterministic replay, solver orchestration and artifact manifests | That orchestration makes every contained solver physically valid |
| Symtropy rigid-body physics | Robot/tool rigid motion, contact, joints and rigid fragment motion inside its declared envelope | Continuum stress, elastic deformation, crack growth or realistic fragmentation without a qualified coupled model |
| Domain physics adapters | Thermal, acoustic, electromagnetic, electrical, fluid, continuum stress and fracture for specified phenomena | One universal solver is accurate for all modalities |
| Symtropy sensor/world adapters | Physics-based sensor projections, latency, noise, sampling and dropouts | Synthetic observation equals ground truth or an independent witness |
| Symthaea | Beliefs, multimodal evidence fusion, active experiment choice, planning, anomaly/discrepancy assessment and mission-level cognition | Inferred state is a direct physical measurement or provides actuator authority |
| Platform-specific control | Robot kinematics/dynamics, actuator constraints, state estimation and candidate controls | Safety or field readiness from simulation-only evidence |
| Independent safety/authority | Hazard interlocks, operation envelope, operator restrictions, recovery and emergency stop | That cognition, rendering, solver confidence or stale data can widen authority |

## 3. The Digital-Twin World Contract

Every world is versioned and has a stable identity. A world manifest should declare:

- world/scenario schema version, stable scenario ID, seed and initial-state digest;
- coordinate frames and transform graph, world origin/datum, gravity and unit system;
- geometry/mesh/CAD references, source digests, revisions and licensing/access provenance;
- material and rock-mass records, parameter distributions and calibration state;
- robots/platforms, kinematic trees, mass/inertia, joint/actuator constraints, end-effectors, tool geometry and wear state;
- environmental domains: terrain, tunnel topology, boundaries, groundwater, temperature, gas, pressure, radiation/EM/acoustic fields where relevant;
- selected solver modules and versions, model parameters, fidelity profile, step sizes and coupling schedule;
- sensor definitions, physical modality, calibration, noise model, sampling rate, latency, bandwidth, saturation, dropouts, coordinate frame and fault injection;
- canonical physical quantities and their declared owning solver at each scale/time interval;
- scenario outcomes, assertions, observables, tolerances and failure/abort conditions;
- artifact and output digests, runtime/environment information, and exact evidence level.

Manifests must not embed mutable floating labels as physical truth (for example, only “rock hardness = 0.9”). They should reference dimensional properties and uncertainty. Normalized indicators may be derived for cognition, but cannot substitute for physical parameters.

## 4. Multi-physics modalities

Physics modalities are separate models with explicit coupling variables. They should not be added as one generic damage factor. A modality is available only when a model contract and validation cases define its input variables, output variables, energy/mass/charge exchange where applicable, limitations, and evidence tier.

### 4.1 Mechanical cutting and percussion — first baseline

Represent tool geometry, rotational speed, torque, thrust, feed, impacts, tool compliance, wear, contact loads, debris/spoil, and cooling. A fast model may predict bounded cutting response; a rigid-body engine models the tool and robot motion. Calibration must use measured force/torque, removed volume and wear, with held-out specimens or sites.

### 4.2 Acoustics and ultrasonics — sensor plus treatment are distinct

There are two distinct use cases:

1. **Acoustic diagnostics:** acoustic-emission (AE) sensors observe transient wave activity associated with crack events; active ultrasonic pulse-echo may estimate interfaces, cracks or elastic-wave behavior. A sensor model needs source, geometry, propagation speed, attenuation, reflections, coupling/contact, frequency response, noise, sampling and thresholding.
2. **Ultrasonic-assisted cutting/percussive action:** high-frequency mechanical excitation changes the tool-rock interaction. This requires a coupled actuator/tool model and an empirically calibrated cutting response. It is not equivalent to merely injecting a simulated AE event.

Initially, a low-order wave/attenuation model may be sufficient for sensor and exploration tests. Use a numerical acoustic wave solver only for experiments that require wave interference, guided waves, complex geometry or high-frequency detail. AE generated by a model must not then be used as independent proof that the same model predicts fracture correctly.

### 4.3 Thermal methods and thermal spallation

Model heat input, contact/convective/radiative transfer as applicable, temperature-dependent properties, gradients, thermal stress, water saturation/boiling where relevant, cooling, and affected-zone estimates. Thermal spallation is demonstrated as a plausible hard-rock drilling method in experimental work, but efficiency and damage depend on material and conditions. The model must distinguish surface melting, thermal cracking, and spall removal; it cannot use temperature alone as a removed-volume proxy.

A validated thermal or thermo-mechanical solver should own temperature/stress fields at the declared fidelity. Symtropy owns the represented tool/body dynamics and moving geometry; a coupling adapter records the boundary energy/force transfer.

### 4.4 Microwave-assisted heating

Microwave interaction depends on frequency, field distribution, dielectric properties, mineralogy, moisture, exposure, and geometry. Couple an electromagnetic energy-deposition model to thermal conduction and a validated damage/strength-change model only when parameters/data support it. Do not model microwave treatment as a blanket temperature increment or universal strength-reduction percentage.

### 4.5 Electrical discharge, electro-pulse and plasma jet

These are different mechanisms, and should be represented as different modes:

- **Electro-pulse / high-voltage discharge:** electrical breakdown/channel formation plus dynamic thermal and mechanical response;
- **Plasma jet:** heat flux, temperature distribution, thermal spallation/melting and material removal;
- **Ultrasonic/electrical combinations:** only through a combined model and validation campaign that supports the interaction.

The required model may couple electric fields/conductivity or breakdown, plasma/thermal transfer, stress/fracture and tool geometry. Do not add a “plasma multiplier” to hardness. This modality has high model, power-electronics and validation complexity. Begin with offline, literature/reference-case reproduction; physical tests require specialist engineering review, electrical safety systems, containment and approved test equipment.

### 4.6 Hydraulic and fluid-assisted methods

For water jetting, circulation, cooling, erosion or fracture-fluid interactions, specify pressure, flow, fluid properties, boundaries, porous-medium coupling and pressure loads. Distinguish borehole/tunnel-scale fluid behavior from small-scale near-tool multiphase/cavitation effects; one global fluid resolution will not serve every regime.

### 4.7 Sensing modalities throughout

Simulated cameras, lidar, IMU, encoders, torque/force sensors, vibration, AE, ultrasonic, thermal, pressure, flow, gas, and electrical measurements need modality-specific calibration/noise/latency/saturation/failure models. Noise should be seeded and reproducible in nominal qualification campaigns, with separate randomized campaigns for robustness. A noisier sensor must not increase confidence or count as an independent source when derived from another sensor.

## 5. Solver composition and physical ownership

Each physical quantity has one declared canonical owner per spatial/temporal domain. Multiple solvers may provide estimates or refinements but cannot independently claim conflicting physical truth.

A solver adapter must expose:
- request identity and exact input/world-generation digest;
- simulation-time interval, time step and coupling contract;
- input/output units and coordinate frames;
- owning physical quantities and coupling boundary;
- calibration/parameter data provenance and model version;
- uncertainty/numerical error, convergence and validity envelope;
- typed results such as Completed, Warning, OutOfDomain, InsufficientEvidence, NonConvergence, Timeout, Invalid, or Cancelled;
- resource/time bounds, cancellation, replay/evidence manifests, license and data access constraints.

Late/stale solver results are rejected after relevant geometry, material, action or hazard state changes. If a high-fidelity solver fails, the host must not silently reuse its last result as current or manufacture a nominal success. A qualified fallback can only use the previously reviewed envelope and must be recorded.

Physics transfer (force, displacement, work/energy, mass/water, heat, charge) must be signed, unit-bound and time-bounded. Coupled solvers need explicit conservation/reconciliation tests before two-way coupling is promoted. Renderer frame rate, camera distance, compute load or asynchronous scheduling may not change canonical scenario semantics.

## 6. The fidelity pyramid

Use multiple fidelity levels rather than running a prohibitively expensive solver for every body and every frame.

- **F0 — Semantic/reference world:** fixed/synthetic geometry, scripted obstacles, fast deterministic robot/contact models. Great for API, mission and regression tests; not a material physics claim.
- **F1 — Calibrated reduced-order:** measured tool/rock response, simple thermal and acoustic response, material-property ranges and uncertainty.
- **F2 — Domain solver:** selected continuum, thermal, fluid, acoustic or EM solver for a declared subproblem with its own exact benchmark.
- **F3 — Coupled high-fidelity region:** fine model of the localized tool/rock/water/thermal/fracture region, composed with a larger low-cost world. Transfer boundaries and convergence must be tested.
- **F4 — Robot/workcell twin:** exact robot/tool geometry, actuators, hardware sensor models, controller-in-the-loop and independent metrology.
- **F5 — Bounded field twin:** a named, instrumented site/platform/configuration with measured environmental and operational data; no automatic generalization to new sites.

A higher visual fidelity or more complex solver does not automatically imply better predictive accuracy. Promotion is based on the declared observable and its validation evidence.

## 7. One robotics test envelope for every project

Every robotics project that opts into the world contract should publish an explicit Platform Profile with:

- kinematic/joint graph, mass/inertia and physical dimensions;
- actuator capabilities, continuous/peak loads, speed/thermal limits, backlash/compliance and emergency-stop behavior;
- end-effectors/tools and their state, wear and supported materials;
- battery/power source, energy model, operating temperature and degraded-power policy;
- sensor suite with calibration, sample timing, frames and known failure modes;
- control frequency, state estimator and command interface;
- required hazards, environment classes and explicit out-of-domain cases;
- allowed physics fidelity per evaluation claim;
- references to test suites and failure behavior.

A shared scenario pack must define initial conditions, seed, mission, hazards, injected faults, expected invariants, metrics and evidence outputs. Core cross-project cases should cover: nominal motion and contact, actuator saturation, sensor bias/dropout/latency, localization drift, collisions and jams, thermal load, energy depletion, communications partition, tunnel/terrain uncertainty, material changes, unsupported boundaries, solver timeout/non-convergence, stale results, and recovery/stop behavior.

Robotics code can then be tested in the same Symtropy world through platform adapters rather than building bespoke simulators per robot. A scenario that is not supported by a platform/solver must be marked unsupported, not passed by omission.

## 8. Digital-twin validation ladder

For each test, keep these outcomes separate:

1. schema and manifest validation;
2. deterministic replay;
3. solver analytical/reference benchmark;
4. cross-solver coupling/conservation tests;
5. sensor-model calibration;
6. held-out specimen/site predictive evaluation;
7. controller-in-the-loop closed-loop test;
8. hardware-in-the-loop and independent metrology;
9. bounded field evidence.

Report predictions with uncertainty and worst-case/residual breakdown, not only average error. Freeze material/tool/site splits before calibration, preserve negative controls, and identify exact source/model/data/solver/environment digests. Test-source existence is not execution; queued CI is not a pass; a simulator pass is not real-world validation.

## 9. Practical build sequence

### W0 — Contract and scenario spine
Finish the world manifest, platform profile, solver request/result, time/frame/unit and evidence contracts. Choose one canonical physical owner for every field. No domain solver dependency yet.

### W1 — Baseline all-robot workcell
Build deterministic robot chassis/articulation/tool scenes, terrain/tunnel geometry, rigid obstacles and labelled synthetic fragments. Add replay and common sensor outputs. Do not call these fragments simulated fracture.

### W2 — Acoustic diagnostic MVP
Add a reproducible AE/ultrasonic sensor channel using a documented low-order propagation model plus noise, latency, saturation and dropouts. Validate sensor timing and observable semantics; keep physical crack inference uncertain until compared against independent experiments.

### W3 — Thermal/tool MVP
Compose the reduced-order calibrated cutter model with thermal state and tool wear. Ensure energy accounting, temperature limits and tool degradation affect behaviour through explicit equations and measurements.

### W4 — First external solver benchmark
Select one relevant OpenGeoSys benchmark for a thermal/mechanical or phase-field subproblem and reproduce its published reference outcome with a pinned executable/input manifest. Do not begin by coupling every solver.

### W5 — Offline electro-pulse/plasma/microwave research lane
Compare feasibility and data availability modality by modality. Require one reproducible physical benchmark per mechanism and an explicit safety review before any high-energy hardware work. Do not expose these modes as operational actuators while unsupported.

### W6 — Multi-rate coupled test
Add one narrow coupling across the validated physical boundary, with stale-result, timeout, non-convergence and conservation tests.

### W7 — Cross-robot qualification campaign
Run the same scenario pack across all supported platform adapters, recording capability differences and unsupported conditions. Add randomization only after deterministic baselines are stable.

### W8 — Physical workcell
Compare against independently calibrated instrumentation and controlled specimen tests. Use site-specific review and a defined stop criterion before any field escalation.

## 10. Research sources

- Symtropy code and stated scope: https://github.com/Luminous-Dynamics/symtropy
- Symtropy rock physics integration contract, companion proposal: https://github.com/Luminous-Dynamics/symtropy/pull/1586
- Symthaea rock mechanics roadmap: https://github.com/Luminous-Dynamics/symthaea/pull/7438
- OpenGeoSys coupled subsurface physics and benchmark library: https://www.opengeosys.org/stable/ and https://www.opengeosys.org/stable/docs/benchmarks/
- OpenGeoSys phase-field fracture benchmarks: https://www.opengeosys.org/stable/docs/benchmarks/phase-field/
- Ultrasonic-assisted rock-breaking review, *Energies* (2022): https://doi.org/10.3390/en15228394
- Ultrasonic percussive drilling dynamics in hard rock, *Journal of Sound and Vibration* (2005): https://doi.org/10.1016/j.jsv.2003.12.045
- Plasma-jet rock-breaking thermal mechanism, *Petroleum Science* (2025): https://doi.org/10.1016/j.petsci.2024.12.017
- Electropulse fracture modeling of granite, *International Journal of Rock Mechanics and Mining Sciences* (2020): https://doi.org/10.1016/j.ijrmms.2020.104238
- High-voltage electropulse and mechanical drilling coupled fragmentation model, *Geoenergy Science and Engineering* (2026): https://doi.org/10.1016/j.geoen.2025.214128
- Microwave-assisted underground excavation: tests including norite, granite and basalt, *Journal of Rock Mechanics and Geotechnical Engineering*: https://doi.org/10.1016/j.jrmge.2015.10.004
- Thermal-spallation field test, *Journal of Petroleum Science and Engineering* (2020): https://doi.org/10.1016/j.petrol.2020.107005
- NASA/JPL NeBula robotics: https://www.jpl.nasa.gov/capabilities/robotics/nebula/
- MuJoCo actuator/sensor modeling documentation: https://mujoco.readthedocs.io/en/stable/overview.html
- NVIDIA Isaac Sim sensor simulation documentation (reference only; not a mandatory dependency): https://docs.isaacsim.omniverse.nvidia.com/latest/sensors/index.html

The papers demonstrate investigated mechanisms and some test cases, not a universal performance advantage. Efficiency depends on rock composition/condition, geometry, energy input, treatment time, tooling and operating environment. Source data, reproducibility and applicability must be reviewed for each selected benchmark.

## 11. Non-claims

This proposal does not add runtime code, implement an acoustics/plasma/microwave solver, qualify any fracture method, create realistic fractures in the rigid-body core, or establish the twin's accuracy. It defines a composable, testable path to a shared world in which all robot work can be evaluated with explicit physical scope and evidence boundaries.
