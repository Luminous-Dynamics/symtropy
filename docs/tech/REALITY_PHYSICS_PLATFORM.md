# Symtropy Reality Physics Platform

**Document:** REALITY-PHYSICS-PLATFORM-001  
**Version:** 0.1  
**Research baseline:** 2026-11-19  
**Status:** architecture and portfolio specification; not a claim that the listed physics domains are implemented or qualified.

## Executive decision

Symtropy should become a **deterministic, evidence-aware coordinator for composable physical models**, not a monolithic solver that reimplements all of mechanics, fluids, electromagnetism, chemistry, plasma and geoscience.

The core product is the trustworthy composition of:
- Symtropy's native N-dimensional rigid-body and scenario execution capabilities;
- specialist numerical solvers where their equations and regimes are a better fit;
- typed model ports, time coordination and conservation-aware exchanges;
- capability and applicability admission, multi-fidelity escalation and explicit abstention;
- replay, provenance, numerical-health diagnostics and independent qualification evidence.

The competitive objective is not to win every domain-specific solver leaderboard. It is to deliver the **lowest-cost result that is actually admissible for a declared engineering question**, while exposing uncertainty, unsupported semantics and model disagreement.

A completed simulation is not by itself a credible prediction, a physical observation, a design approval, a certification or permission to actuate hardware.

## 1. Current baseline and claim boundaries

The public Symtropy main branch at the research baseline is commit [bdf7f04](https://github.com/Luminous-Dynamics/symtropy/commit/bdf7f04d44d4906edc281fe5ca07c4e202951549). This section summarizes source-audited boundaries from the public project and existing work items; the exact runtime census remains owned by [#1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472).

### Native engine

The current native core is an N-dimensional rigid-body/contact engine with:
- const-generic geometry and collision paths, with public documentation centered on 2D, 3D and 4D;
- GJK/EPA collision machinery, contact manifolds, warm starting and broadphase acceleration;
- continuous collision detection for a bounded set of collision paths, not every possible shape pair;
- articulated constraints and motor-related primitives;
- raycasting, overlap/sensor events, replay and numerical diagnostics;
- an optional fixed-point quantization profile for a declared deterministic-networking subset.

The native implementation has known scope limits that must remain visible in the capability census. The public model-path audit describes semi-implicit translational integration, SO(D) orientation integration, pair-specific contact routes, bounded CCD shape coverage, approximate asymmetric rotational-inertia handling, and limitations in long contact stacks. A generic shape trait or a successful build must not be interpreted as identical fidelity for every shape pair or dimension.

The engine's sensor overlap events are not calibrated physical sensor models. The replay primitives and deterministic subset do not establish cross-platform bitwise identity for all profiles. See [#1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472) and [#1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467).

### Domain-specific work already present

The broader repository contains domain-specific research and benchmark work. It must be reused rather than duplicated:
- multi-fidelity physics admission and escalation: [#4487](https://github.com/Luminous-Dynamics/symtropy/issues/4487), with implementation/research work in [PR #4492](https://github.com/Luminous-Dynamics/symtropy/pull/4492);
- deterministic engineering scenario and replay substrate: [#1172](https://github.com/Luminous-Dynamics/symtropy/issues/1172);
- engineering twin projection and counterfactual boundary: [#1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467);
- model-path capability census: [#1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472), with the fresh exact-source reference/qualifier tranche in [PR #6248](https://github.com/Luminous-Dynamics/symtropy/pull/6248);
- integrator portfolio and numerical qualification: [#3975](https://github.com/Luminous-Dynamics/symtropy/issues/3975);
- analytic/manufactured numerical oracles and V&V: [#3749](https://github.com/Luminous-Dynamics/symtropy/issues/3749);
- semantic equivalence for backend comparisons: [#4366](https://github.com/Luminous-Dynamics/symtropy/issues/4366), with executable receipt/schema work in [PR #4369](https://github.com/Luminous-Dynamics/symtropy/pull/4369);
- differential fuzzing and counterexample minimization: [#4446](https://github.com/Luminous-Dynamics/symtropy/issues/4446);
- the shared physics qualification corpus: [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460);
- campaign-level quality-qualified throughput and failure-denominator preservation: [#4559](https://github.com/Luminous-Dynamics/symtropy/issues/4559), with an evaluator prototype in [PR #4564](https://github.com/Luminous-Dynamics/symtropy/pull/4564);
- multiscale model coupling: [#5772](https://github.com/Luminous-Dynamics/symtropy/issues/5772);
- typed subsystem ports, units, time and balance semantics: [#5779](https://github.com/Luminous-Dynamics/symtropy/issues/5779);
- fluid-structure interaction coupling and rollback: [#4275](https://github.com/Luminous-Dynamics/symtropy/issues/4275);
- solver-neutral plasma scenario integration: [#1685](https://github.com/Luminous-Dynamics/symtropy/issues/1685);
- FMI energy-boundary and time-synchronization benchmark: [#6156](https://github.com/Luminous-Dynamics/symtropy/issues/6156).

These owners are the current work decomposition. This document is an index and architectural synthesis, not a replacement owner for their implementations.

### Explicit scientific boundary

A software feature, a numerical solver, a replay, an attractive visualization, an internal agreement between solvers, or a queue/CI result is not independent physical validation.

Each model must state its:
- intended context of use;
- governing equations and constitutive assumptions;
- supported regimes and validity envelope;
- discretization, solver and numerical tolerances;
- boundary and initial conditions;
- known approximations, exclusions and failure modes;
- parameter and data provenance;
- uncertainty representation and evidence;
- unsupported conditions and abstention behavior.

No synthetic trajectory may be silently promoted to a physical observation or current engineering-twin state. No model result grants design acceptance, certification or physical-control authority.

## 2. Architectural north star: one coordinator, many models

The platform should compose model families at the level required by the question. It should not force every phenomenon into rigid-body dynamics or into one global scalar called fidelity.

### Layer A — physical scenario and asset semantics

Own stable scenario identity, entities, geometry references, initial/boundary conditions, operating profiles, event schedules, disturbances and exact configuration digests. Preserve distinctions among as-designed, as-built, as-tested, as-operated, inferred and assumed states.

### Layer B — specialist model federation

Expose specialist solvers through explicit adapters or model packages. Each model retains its own executable identity, solver state, numerical method and model passport. FMI 3.0.2 and SSP 2.0.1 are the first interoperability references where their model-exchange, co-simulation, scheduled-execution and system-structure semantics fit.

A standard interface is an interoperability contract; it does not certify the equations, input data, solver accuracy or coupled-system result. FMI's co-simulation specification also leaves the importer/master algorithm to the integrating tool. Symtropy must qualify its own synchronization and coupling behavior.

### Layer C — typed coupling graph

Use explicit ports and edges for physical states, conserved fluxes, control signals, mechanical exchanges, boundary conditions and observations. Every port and edge must declare units/dimensions, sign convention, direction, timebase, update cadence, mapping/conversion, uncertainty, provenance, balance group and validity limits.

The port semantics in [#5779](https://github.com/Luminous-Dynamics/symtropy/issues/5779) are the foundation. String-named variables alone are not executable coupling semantics.

### Layer D — numerical coordination

Coordinate integrator selection, solver subcycling, event ordering, communication steps, rollback/retry, interpolation/hold rules, checkpoint state, asynchronous delivery, and coupling stability. A federated run must preserve each solver's exact version and clock identity.

### Layer E — evidence and qualification

Bind each run to frozen inputs, source/model identities, exact adapters, toolchain, solver options, resource profile, outputs and receipts. Distinguish:
1. code verification;
2. solution verification;
3. model validation against independent references;
4. uncertainty and sensitivity qualification;
5. context-of-use admission.

A deterministic replay establishes reproducibility of the declared computation, not physical predictive validity.

## 3. Physical-domain portfolio

The table is a planning map, not a capability advertisement. “Native” means there is an identified Symtropy implementation path; it does not mean every use case in that domain is validated. Specialist solver choice remains profile-specific.

| Domain | Current posture | Candidate specialist/reference paths | Project relevance |
|---|---|---|---|
| Rigid-body mechanics and contact | Native N-D engine; bounded shape/contact/CCD paths; source census required | MuJoCo, Newton, Project Chrono, Rapier, Drake where semantics match | robotics, vehicles, construction, aerospace |
| Articulated and compliant robotics | Native articulation/motor primitives plus platform-specific models; fidelity varies by profile | MuJoCo, Newton, Chrono, Drake, Pinocchio for appropriate kinematics/dynamics tasks | manipulators, humanoids, quadrupeds, vehicles |
| Continuum solids, soft bodies and fracture | Do not claim a general native FEM or fracture solver | MOOSE, Chrono peridynamics, specialist FEM/meshless solvers | structures, aerospace, construction, manufacturing |
| Fluids and CFD | Public SPH code includes scaffold/placeholder limitations; not admitted for engineering prediction without qualification | OpenFOAM, MOOSE Navier–Stokes, Chrono FSI, other regime-appropriate CFD solvers | aerodynamics, water, HVAC, pumps, turbines, maritime |
| Fluid-structure interaction | Dedicated coupling owner exists; not a blanket all-domain capability | Chrono FSI, OpenFOAM coupling, MOOSE, domain-specific partners | aeroelasticity, marine structures, pumps, turbines |
| Thermal transport | Multiple project-level thermal models exist; no universal all-regime thermal solver claim | MOOSE heat transfer, OpenFOAM energy equations, EnergyPlus for buildings | batteries, data centres, homes, geothermal, plasma exhaust |
| Electrical networks and power electronics | Project-level energy contracts and system models; no general circuit/EM solver claim | FMI-connected Modelica tools, GridLAB-D, SPICE-family tools, specialist EM solvers | grids, microgrids, motors, storage, aircraft |
| Electromagnetics | Adapter/integration work required for full-field models | MOOSE electromagnetics, dedicated FEM/EM solvers | motors, wireless power, RF, plasma |
| Chemistry and electrochemistry | Domain-specific models only; reaction/property provenance required | MOOSE geochemistry/chemical reactions, specialist battery/electrochemistry tools | batteries, fuel cells, electrolysers, cleaning, corrosion |
| Plasma and fusion | Solver-neutral plasma scenario/receipt boundary; Symtropy is not itself a validated plasma PDE/PIC solver | WarpX for supported kinetic PIC cases; Zapdos for appropriate low-temperature plasma fluid cases; regime-specific fusion codes | plasma research, fusion integration, plasma processing |
| Geology and porous media | Domain models and benchmark consumers; no general subsurface solver claim | OpenGeoSys THM/THMC, MOOSE Porous Flow, specialist reservoir tools | geothermal, groundwater, mining, underground structures |
| Atmosphere, ocean and environmental fields | Composition through source-bound environmental and domain models | Weather/ocean model outputs, CFD and geophysical tools as appropriate | renewable energy, buildings, maritime, planetary models |
| Acoustics, optics and RF propagation | Planned/adapted by question; sensor models must remain separate from latent truth | Domain-specific wave solvers and calibrated sensor models | robotics, aerospace, sensing, communications |
| Biological and physiological systems | Nested model atlas and coupling contracts are under development | CellML/SBML-compatible models, JOS-3 and domain-specific physiology models | human systems, buildings, ergonomics, environmental exposure |
| Manufacturing and process physics | Narrow domain-specific research; not a universal manufacturing solver | NIST AM-Bench references, MOOSE, OpenFOAM, Chrono and specialist process models | additive manufacturing, machining, fabrication, bootstrapper |

Important current limitations:
- The repository's fluid implementation must not be described as engineering-grade CFD. The current cryogenic-fluid audit [#6187](https://github.com/Luminous-Dynamics/symtropy/issues/6187) records placeholder SPH pressure/viscosity force gradients and an empty GPU fluid-plugin build hook. The work item explicitly composes #4487 instead of creating another fluid engine.
- The plasma adapter [#1685](https://github.com/Luminous-Dynamics/symtropy/issues/1685) is a solver-neutral scenario/evidence boundary. It does not implement PIC, MHD, plasma chemistry or fusion physics.
- The physics callback used by the research coupling layer is not the general-purpose multiphysics port API. Application metrics and agency policies must not be treated as intrinsic material laws without a separately evidenced mechanism.

## 3A. First composed engineering vertical slice: geothermal–fusion

The public GeoFusion work is a concrete example of the proposed architecture and should be treated as the first composed energy-system slice, not a claim of a functioning geothermal/fusion plant or validated multi-physics solver:

- [#6142 — GEOFUSION-INTEGRATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/6142) owns the integration question and architecture.
- [PR #6143](https://github.com/Luminous-Dynamics/symtropy/pull/6143) supplies the system architecture, 27-case benchmark plan and analytic/synthetic energy scenarios.
- [PR #6154](https://github.com/Luminous-Dynamics/symtropy/pull/6154) adds dependency-free Rust contracts for typed heat/electricity streams, source allocation, first-law arithmetic and source-bound wellhead property state.
- [PR #6162](https://github.com/Luminous-Dynamics/symtropy/pull/6162) adds evidence-envelope and source-allocation receipt schemas and synthetic fixtures.
- [#6172 — GEOTHERMAL-PROPERTY-VV](https://github.com/Luminous-Dynamics/symtropy/issues/6172) owns qualification of water/steam property providers against IAPWS; pure-water IF97 must not silently stand in for saline brine.
- [#6156 — GEOFUSION-BENCH-001](https://github.com/Luminous-Dynamics/symtropy/issues/6156) owns FMI time synchronization and multi-component co-simulation qualification. [PR #6198](https://github.com/Luminous-Dynamics/symtropy/pull/6198) begins a bounded clock/step/retry/barrier trace-verifier sub-slice.

This separation is intentional: stream arithmetic, property-state validity, co-simulation clocks, FMU physics, geothermal reservoir behavior and plasma physics are distinct model/evidence obligations. The current contracts and synthetic fixtures do not calculate reservoir productivity, wellbore two-phase flow, fusion plasma, net fusion generation or plant operation.

## 4. Paradigm shifts worth pursuing

### 4.1 From one fidelity slider to a model-admission vector

A universal “high/medium/low fidelity” scalar hides the actual reason a model is suitable or unsuitable. Admission must consider equation set, constitutive law, geometry, contact/event semantics, dimensions, boundary conditions, units/frames, resolution, solver error, parameter/model-form uncertainty, evidence, applicability and compute budget.

The policy in [#4487](https://github.com/Luminous-Dynamics/symtropy/issues/4487) already defines the direction: choose the least costly candidate that satisfies all hard conditions; escalate or abstain when no candidate is qualified. A more expensive model is not automatically more correct.

### 4.2 From loose data exchange to typed, conservation-aware physical ports

Every connection must state what physically crosses the boundary and how it is accounted for. A scalar called “power” is insufficient without unit, sign, boundary, carrier, time interval, source identity and allocation semantics.

Energy, mass, momentum, electric charge and species balances are separate groups. Do not sum unlike carriers across a conversion device unless the conversion relation and losses are represented. Reject duplicate allocation, missing sinks, stale state, incompatible units and inconsistent intervals before mutation.

### 4.3 From fixed-rate federation to qualified multi-rate coupling

Mechanical contact, electrical switching, thermal diffusion, chemical kinetics and atmospheric forcing operate on different timescales. Use subcycling or scheduled execution only where stability and error are understood. Bound interpolation/hold error; detect late, duplicate, skipped and out-of-order events; support rollback/retry where the federation semantics require it; preserve coupling-interface work/energy residuals.

### 4.4 From a simulator that “finishes” to a simulator that can abstain

The useful result is not always a number. Explicit outcomes should include completed-within-profile, approximate-within-envelope, incomplete, unsupported, non-converged, stale-input, invalid-input, out-of-envelope, disagreement-unresolved and abstained-insufficient-evidence.

Unknown is not zero. Missing material parameters, sensor values, reaction data, boundary conditions or solver capabilities must remain missing or trigger a no-go disposition.

### 4.5 From model calibration to active model discrimination

Symthaea can propose competing physical hypotheses and identify observations that distinguish them. Symtropy can generate bounded counterfactual plans and estimate expected observables for each admitted candidate. Physical experiments remain independent; only measured outcomes can qualify the hypothesis. Keep calibration, validation and blind challenge data separated by specimen/article, configuration, batch and campaign.

### 4.6 From “faster simulation” to the selective engineering-answer frontier

Measure how many questions receive admissible answers at each risk ceiling and resource cost. Report coverage, selective error, false-admission rate, abstention quality, uncertainty calibration and cost per independently admissible answer.

This can outperform a fixed-fidelity solver portfolio operationally without claiming that Symtropy's individual numerical kernels beat every specialist solver.

### 4.7 From solver agreement to independent evidence

Two solvers can share equations, constitutive assumptions, source data, meshes or bugs. Agreement is not independent validation. Track common-cause dependencies and reference lineage; prefer analytic/manufactured solutions and held-out experimental data for qualification. Unresolved disagreement stays visible.

## 5. Model passport and admission contract

Each model/backend/profile should carry a versioned passport containing:

| Field | Required content |
|---|---|
| Identity | Model, solver, adapter, release/commit, build and executable digests |
| Equation family | Governing equations, constitutive models and physical regime |
| Subject | Geometry, material, species, topology, dimensionality and exact configuration |
| Initial/boundary conditions | Values, units, coordinate frames, clock and provenance |
| Numerical profile | Discretization, mesh/particle resolution, timestep, integrator, tolerances and convergence criteria |
| Supported envelope | Regime, parameter range, horizon, observables and exclusions |
| Approximation register | Reduced-order, surrogate, contact, constitutive, interpolation and numerical approximations |
| Evidence | Analytic anchors, code/solution verification, independent validation and uncertainty receipts; separate measurement, parameter, numerical, model-form and surrogate uncertainty |
| Resource profile | Hardware, CPU/GPU, memory, wall time and energy where measured |
| Failure handling | Non-convergence, invalid states, event loss, stale data, OOD conditions and abstention |

Use categorical model-path statuses rather than one boolean:
- supported exact under the declared mathematical profile;
- supported numerical model;
- supported approximation;
- supported via a declared external provider;
- partially supported;
- unsupported;
- unknown / not yet censused.

“Exact under profile” means exact relative to that declared implemented contract; it does not mean exact physical truth.

The admission policy consumes this passport, the requested observable set, context of use, hard constraints, uncertainty/error limits and resource budget. Evidence effort scales with model influence and decision consequence. It must refuse semantic mismatches and unqualified extrapolation. Follow the risk-based metrology-aligned UQ guidance in [NIST AMS 100-85 (2026)](https://doi.org/10.6028/NIST.AMS.100-85): measurement uncertainty is not parameter uncertainty, numerical error, or model-form discrepancy, and these terms must not be silently collapsed into one confidence score.

## 6. Typed port and coupling contract

A port or coupling edge must identify:
- stable port/edge identity and semantic version;
- owning model, source variable and exact model revision;
- quantity class: conserved flux, state, control signal, mechanical, boundary condition or observation;
- physical meaning, SI unit or explicitly symbolic unit, dimensional exponents, direction and sign convention;
- valid range and source-to-destination mapping;
- conversion formula and conversion provenance;
- event time, availability time, clock domain, cadence, solver step and interpolation/hold rule;
- uncertainty, precision and parameter provenance;
- balance group and expected closure definition;
- dispositions for missing, delayed, duplicate, stale and out-of-order events;
- validity label, assumptions, unsupported conditions and validation receipts.

Maintain two distinct channels:
1. **Plant coupling:** internal model states and physical fluxes exchanged between declared subsystem models.
2. **Observation exchange:** measurements exposed to an estimator or agent, with capture time, availability time, calibration, sensor health, modality and source lineage.

Hidden simulated state must not leak into an observation channel through IDs, timestamps, missingness, diagnostics or data availability. A synthetic model output remains synthetic even when initialized from a measured value.

## 7. Interoperability and specialist solver strategy

### Standards and packaging

- **FMI 3.0.2:** first reference for model exchange, co-simulation and scheduled execution where suitable. FMI defines interfaces and execution semantics; it does not certify physics or prescribe one universal master algorithm.
- **SSP 2.0.1:** reference for packaging and describing connected systems, component models and parameterization.
- **ISO 23247 series:** relevant manufacturing digital-twin and composition context; use as alignment references, not as a certification claim.
- Use explicit adapters for native Rust crates, process-isolated specialist solvers and FMUs. Pin every executable and dependency. Preserve original solver artifacts and raw outputs.

### Solver adapters

An adapter must:
- declare exact executable and version;
- validate all inputs, units, frames and model capabilities before launch;
- execute in a bounded, isolated process with explicit resource/time limits;
- capture stdout/stderr, exit status, solver convergence, warnings, resource usage and all output artifacts;
- preserve raw outputs and normalize only declared quantities;
- reject missing or unexpected fields and stale input/output identity;
- emit a receipt that records execution integrity, not physical truth;
- never silently substitute a toy result, fallback solver or approximate field for an unavailable specialist calculation.

### Candidate solver portfolio

| Workload | Candidate references | Selection rule |
|---|---|---|
| Rigid contact, articulated robotics, deformables | MuJoCo 4.0.1; Newton 1.8.0; Project Chrono 10.0.0 | Match model family, contact/material law, integrator, observables and hardware |
| CFD, heat-transfer flow and combustion | OpenFOAM; MOOSE modules | Match equation set, turbulence/chemistry closure, mesh and boundary conditions |
| Multiphysics continuum and materials | MOOSE; Chrono; specialist FEM codes | Compare only matching constitutive laws and coupling semantics |
| Plasma PIC | WarpX 26.04 examples and supported cases | Match particle shape/order, field solver, boundary conditions, species and normalization |
| Low-temperature plasma fluid | Zapdos where its validated regime matches | Do not substitute a fluid model for kinetic/fusion regimes |
| Geothermal and porous THMC | OpenGeoSys TH2M benchmarks; MOOSE Porous Flow | Compose the same process subset and property/boundary models |
| Buildings and energy systems | EnergyPlus, GridLAB-D, FMI-capable Modelica tools | Match weather, occupancy, equipment, controls and time intervals |
| Additive manufacturing | NIST AM-Bench 2025 data and challenge cases | Use challenge-specific physical observables and held-out measurements |

The solver names are candidates, not endorsements of universal accuracy. License, current release, supported regime, model assumptions and evidence must be checked for the exact proposed use.

## 8. Qualification lifecycle

Every model path and composed run should progress through evidence levels:

**Q0 — Contract and capability:** identity, units, schema, provenance, supported-path and negative-control checks. No numerical accuracy claim.

**Q1 — Code/equation verification:** analytic solutions, manufactured solutions, dimensional analysis, limiting cases, positivity/boundedness, invariants and independent implementation oracles.

**Q2 — Solution verification:** iterative convergence, mesh/particle refinement, timestep refinement, observed order, solver tolerances, conservation residuals and numerical error estimates.

**Q3 — Cross-solver comparison:** semantic-equivalence gate, matched cases, independent numerical methods, raw disagreement retained, no “vote by solver count”.

**Q4 — Physical validation:** comparison to independent, calibrated, held-out measurements with reference uncertainty and complete measurement provenance.

**Q5 — Context-of-use qualification:** uncertainty, sensitivity, identifiability, OOD limits, consequence/risk tier, external review and the exact engineering decision boundary.

A model can be Q2 for one observable and Q0 for another. Evidence does not automatically transfer across regimes, geometry, materials, operating ranges or intended uses.

## 9. Work sequence

This sequence composes existing owners and deliberately avoids a second generic physics engine.

### Immediate — repository and contract baseline

- Keep this architecture and benchmark portfolio current with exact repository owners.
- Audit README and docs claims against main-branch source.
- Link the capability census and model admission policy.
- Reconcile public documentation for engine counts, dimensions, joint types, solver paths and known approximations.
- Treat unimplemented or CI-pending work as pending, not complete.

### Next — close composition semantics

- Advance #1472 capability census and #4487 admission/escalation together.
- Advance #5779 typed ports before creating new cross-domain runtime wiring.
- Reuse #3975 integrator portfolio, #3749 analytic/manufactured oracles, #4366 semantic equivalence, #4446 differential fuzzing and #460 qualification corpus.
- Qualify multirate coupling and FSI rollback through #4275 and #6156.

### First domain slices

Prefer small vertical integrations with clear external references:
1. rigid-body/contact comparison against MuJoCo and Newton on the same frozen cases;
2. one thermal/structural coupon with analytic and external FEM references;
3. one fluid channel/heat-transfer case with a verified OpenFOAM reference;
4. one typed FMI multi-component energy network under #6156;
5. one solver-neutral plasma run receipt under #1685 using a supported, pinned external case;
6. one THMC/geothermal benchmark hierarchy using OpenGeoSys reference cases.

Do not implement all six simultaneously. First complete the contract and qualification path, then choose vertical slices based on project demand and available independent data.

## 10. Cross-project consumers

| Project family | Required coupled models |
|---|---|
| Plasma and fusion | regime-specific plasma model, electromagnetics, heat, radiation, materials, coolant and energy balances |
| Geothermal and underground systems | porous flow, heat transport, geomechanics, chemistry, fractures and well/interface models |
| Aerospace and eVTOL | rigid/articulated dynamics, aerodynamics, structures, batteries, motors, thermal and controls |
| Construction and mining robotics | articulated contact, terrain/soil, granular material, fracture, hydraulics and sensing |
| Grid and energy infrastructure | circuits, power flow, converters, thermal ratings, storage, generation and multi-vector allocation |
| Data centres and buildings | electrical distribution, IT load, heat generation, cooling-fluid paths, weather and service demand |
| Household and water systems | fluid networks, heat, surface chemistry, material wear, filtration and biological processes where justified |
| Manufacturing and Mk0 | machining, extrusion, additive process physics, material properties, energy and resource logistics |
| Maritime | hydrodynamics, waves, structure, propulsion, corrosion, acoustic propagation and sensing |
| Human and ecological systems | separately validated subsystem models, transport and exchange ports, environment and observation process |

The goal is a shared substrate and common evidence language, not one universal high-resolution model of every atom, person, machine and ecosystem.

## 11. External references

Standards and scientific references are time-sensitive. Pin the exact version and retrieval date in each benchmark record.

- [FMI 3.0.2 specification](https://fmi-standard.org/docs/3.0.2/) and [FMI downloads / tool support](https://fmi-standard.org/downloads/).
- [SSP 2.0.1 specification](https://ssp-standard.org/docs/2.0.1/).
- [NASA-STD-7009B active standard record](https://standards.nasa.gov/standard/NASA/NASA-STD-7009).
- [ASME V&V standards portfolio](https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty) and [V&V 10-2019 (reaffirmed 2025)](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019).
- [MuJoCo 4.0.1 changelog](https://mujoco.readthedocs.io/en/4.0.1/changelog.html).
- [Newton releases](https://github.com/newton-physics/newton/releases).
- [Project Chrono 10.0.0 release notice](https://projectchrono.org/news/).
- [OpenFOAM documentation](https://www.openfoam.com/documentation/) and [verification/validation cases](https://doc.openfoam.com/2306/examples/verification-validation/).
- [MOOSE physics modules](https://mooseframework.inl.gov/modules/).
- [WarpX 26.04 examples](https://warpx.readthedocs.io/en/26.04/usage/examples.html).
- [OpenGeoSys TH2M benchmark hierarchy](https://www.opengeosys.org/stable/docs/benchmarks/th2m/).
- [NIST AMS 100-85 (5 Oct 2026): risk-based UQ harmonized with metrology](https://doi.org/10.6028/NIST.AMS.100-85).
- [NIST AM-Bench](https://www.nist.gov/ambench) and [AM-Bench 2025 measurements/challenges](https://www.nist.gov/ambench/am-bench-2025-measurements-and-challenge-problems).

## 12. Non-goals

This architecture does not:
- promise one universal solver or universal fidelity scalar;
- replace specialist FEM, CFD, PIC, MHD, chemistry, geoscience or circuit solvers;
- claim physical validity from code presence, replay, CI or solver agreement;
- turn Phi, trust, health, wealth or agent-state metrics into intrinsic material laws;
- infer missing measurements or silently default missing physical values to zero;
- make Symtropy the canonical engineering twin, evidence authority, model-update authority, certification body or physical-control authority;
- authorize fabrication, energization, experiments, commissioning or operation.

**Success means more correctly bounded, independently qualified engineering answers per unit of compute and engineering effort—not a universal-looking renderer.**
