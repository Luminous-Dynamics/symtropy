# Subterranean Rock Physics Integration Contract V0.1

**Status:** Research and architecture proposal; documentation only  
**Date:** 2026-10-13  
**Related:** Symtropy solver-composition work (draft PR #487); Symthaea rock-mechanics roadmap (draft PR #7438)

## 1. Decision

**Use Symtropy as the robot/tool rigid-body dynamics and contact layer, not as the authoritative rock-fracture solver.**

Symtropy currently supplies N-dimensional geometry, rigid-body dynamics, collision detection/contact manifolds, joints, replay, and robotics bridges. Its published scope explicitly excludes soft-body physics from the core. Those capabilities are useful for vehicle motion, cutter/tool pose, collision, articulated machinery, tunnel clearance, fragment transport, and replayable robotic scenarios. They do not, by themselves, calculate real rock stress/strain, crack nucleation, fracture propagation, or calibrated cutting resistance.

Symthaea should own cognition, geological evidence fusion, uncertainty, active probing, mission/task planning, counterfactual selection, discrepancy monitoring, and policy proposals. Specialist geomechanics solvers should own the numerical solution of the physical mechanisms they are qualified to represent.

This document is not a claim that a solver adapter or end-to-end robot model already exists.

## 2. Scope boundaries

| Concern | Primary owner | Explicit limit |
|---|---|---|
| Rigid robot, vehicle, cutter housing, support equipment and loose fragment motion | Symtropy physics | Correct contact/dynamics for represented rigid bodies does not establish rock constitutive behavior |
| Collision, clearance, articulated motion, machine pose, replayable action traces | Symtropy | Requires calibrated geometry, masses, inertia, contact parameters and timestep envelope |
| Lithology/material hypotheses, evidence lineage, uncertainty and active observations | Symthaea + domain evidence contracts | Hypothesis is not ground truth or site certification |
| Fast drilling/cutting response estimate | A calibrated, versioned reduced-order model | Must state calibrated materials/tools/ranges and return out-of-domain when unsupported |
| Stress redistribution, deformability, pore pressure and coupled heat/water/geomechanics | A validated specialist numerical solver | No general solver is assumed correct for every process |
| Crack growth, joint opening, fragmentation or progressive failure | A solver qualified against mechanism-specific tests | A rigid-body collision engine is not a fracture solver |
| Human/operator authority, hazard interlocks, emergency stop and recovery | Independent safety/authority layer | Neither learned confidence nor a physics prediction may widen authority |
| Provenance, deterministic manifests, validation tier and change impact | Shared contract; evidence owner retains authority | Artifact digest does not prove physical correctness |

### Non-negotiable physical-truth rule

A cognition metric, Phi estimate, trust score, or inferred agent state must not alter rock density, elastic constants, strength, fracture toughness, contact coefficients, gravity, or a solver's constitutive law unless the quantity has a declared physical measurement/actuation meaning and a separately validated model.

Such metrics may inform which plan an agent proposes or whether policy allows a requested action. They are not a substitute for material mechanics. A consciousness-coupled simulation experiment must be labelled as such and kept separate from engineering predictions of real matter.

## 3. Composed architecture

~~~text
Sensors + survey + sample/test records
    |
    v
Evidence admission: source identity, units, frames, time, calibration, provenance
    |
    v
Symthaea geological belief + uncertainty + active probing
    |
    +-------------------------+
    |                         |
    v                         v
Rock/rock-mass model      Robot/task plan
    |                         |
    v                         v
Specialist solver result -> Symthaea candidate action
    |                         |
    +-----------+-------------+
                v
     Independent hazard/authority arbiter
                |
                v
     Symtropy robot/tool rigid-body step
                |
                v
     Command and predicted response record
                |
                v
     Independent measured observations
                |
                v
     Residual/discrepancy and model requalification
~~~

The order can vary in a specific implementation, but authority boundaries must not. Symtropy's physical-body state, Symthaea's inferred state, and a specialist solver's synthetic state remain explicitly distinct. A simulated sensor must never be presented as an independently observed physical sensor.

## 4. External rock-physics candidates

Use the least complex model that answers a declared engineering question.

### Initial candidate: a transparent reduced-order cutter model

Build or adapt a small, deterministic, unit-aware model for penetration, torque, thrust, mechanical power, removed volume, thermal load and cutter wear. Calibrate it against controlled drilling/cutting records and hold out independent specimens or sites. Keep its domain narrow and versioned. This is the first practical bridge to Symtropy because it can inform a cutting plan without requiring a real-time high-fidelity fracture solution.

Mechanical specific energy (MSE) is a useful measured diagnostic when input power and removed rock volume are consistently defined. It is not a universal rock-strength constant; reported MSE depends on tool geometry, operating regime, wear, measurement quality and the selected energy accounting.

### Candidate for continuum and coupled subsurface physics: OpenGeoSys

OpenGeoSys is an open-source finite-element framework for thermo-hydro-mechanical-chemical (THMC) simulation in porous and fractured media. Its public benchmark collection covers thermo-hydro-mechanics and phase-field fracture, including analytical/reference comparisons and fractured-sample cases.

That makes it a plausible external candidate for bounded questions such as deformation under declared boundary conditions, thermal/hydraulic coupling, or selected fracture benchmark classes. It does not automatically make OpenGeoSys a qualified rock-cutter contact or excavation-wear model. A candidate solver must be chosen per target mechanism and benchmark; do not integrate it solely because its feature list sounds broad.

### Interoperability option: FMI co-simulation

The Functional Mock-up Interface (FMI) is an established model exchange/co-simulation standard. A solver that already supports a suitable FMI export can be connected through an FMI adapter in a later phase. Do not force FMI onto a solver that does not expose the right variables or semantics. First define the canonical evidence/result contract; then implement a native process/API adapter and/or FMI adapter as justified.

### Rust-native acoustic candidates for an early prototype

Two current Rust crates are worth testing as bounded acoustic components rather than adopting blindly:

- [math-fem](https://docs.rs/math-fem/latest/math_fem/) documents a finite-element Helmholtz solver with 2-D/3-D meshes, impedance/absorption boundary conditions, PML, frequency sweeps and a room-acoustics CLI.
- [math-bem](https://docs.rs/math-bem/latest/math_bem/) documents a boundary-element solver for acoustic scattering/room acoustics, with analytical validation examples and structured outputs.

These are candidates for early sensor-propagation experiments and software integration. Room acoustics or a frequency-domain Helmholtz solve does **not** automatically model transient acoustic emission from a crack, wave propagation through heterogeneous fractured granite, ultrasonic tool-rock coupling, or acoustic-emission sensor performance. Each intended use requires a benchmark that matches geometry, scale, boundary conditions, wave band and measured outputs.

Current Rust FEM crates should also be treated as components, not turnkey hard-rock solvers. For example, a linear-elastic FEM implementation is not a fracture-propagation model. Keep the first Rust-native test small and analytically verifiable; delegate coupled THMC/fracture/electromagnetic work to mature specialist solvers when their models and benchmarks fit.

### Multiphysics candidates for heavier domains

- [OpenGeoSys](https://www.opengeosys.org/stable/) is a candidate for bounded subsurface THMC, poromechanics and selected fracture problems, backed by a substantial benchmark library.
- [Elmer FEM](https://github.com/ElmerCSC/elmerfem) documents structural mechanics, fluids, heat transfer and electromagnetics in a general finite-element suite.
- [MOOSE](https://mooseframework.inl.gov/modules/) provides modular applications for electromagnetics, heat transfer, solid mechanics, phase field, peridynamics and porous flow. It is a candidate framework for composing physics, not a ready-made plasma-rock model.

For microwave/electro-pulse/plasma-jet work, no one small Rust crate surfaced as an established end-to-end rock-removal solver. The likely path is a specialist solver plus a narrow Rust adapter, a frozen benchmark and independent validation. Do not equate “has an electromagnetic module” with “predicts breakdown or plasma-rock erosion correctly.”

### Keep the selection test-driven

A solver choice record must state: target question, represented mechanisms, omitted mechanisms, constitutive model, calibration data, boundary conditions, failure modes, reproducible case, license/access constraints, and what result would reject the candidate.

## 5. Canonical interface contract

The first bridge should be a versioned Rust-host contract. Solver-specific file formats and native bindings sit behind adapters.

### Request envelope

Every request identifies:

- schema and interface version;
- run/case ID and monotonic request sequence;
- simulation-time interval and requested coupling cadence;
- subject/platform, tool, scene/region and geometry identities;
- coordinate-frame and unit-system identifiers;
- material and rock-mass property references, with source digests;
- initial and boundary conditions, including the exact represented stress, groundwater and thermal assumptions;
- input model/solver/parameter versions and environment digest;
- accepted validity envelope and requested observables;
- cancellation/deadline policy and resource bounds.

### Result envelope

Every result identifies:

- exact request/run ID and input digests;
- solver binary/build/environment, model and parameter digests;
- start/end simulation time and step/coupling information;
- output quantities with explicit units and frames;
- uncertainty/error estimate, convergence/tolerance state and residuals;
- warnings, represented/omitted mechanisms, and validity-domain assessment;
- provenance class: synthetic, analytical, literature-derived, lab-observed, or field-observed;
- a typed disposition.

Suggested result dispositions:

- **CompletedWithinEnvelope**
- **CompletedWithWarnings**
- **OutOfDomain**
- **InsufficientInputEvidence**
- **NumericalNonConvergence**
- **DeadlineExceeded**
- **InvalidResult**
- **Cancelled**

A result that times out, does not converge, refers to stale inputs, or exits its calibrated envelope is never silently replaced by a nominal success. A result is advisory prediction data, not actuator authority.

### Units, frames and time

- Use SI units at the integration boundary unless a field contract explicitly demands another unit.
- Reject unknown or conflicting units and coordinate frames; never guess conversion.
- Carry observed time and received time separately for physical observations.
- Separate wall-clock time from simulated time.
- Bind each result to the exact input state/generation it predicts.
- Reject or quarantine late results that no longer apply to the live state.
- Treat interpolated or resampled fields as derived evidence with an explicit lineage, not new independent observations.

## 6. Multi-rate coupling and real-time boundary

The rigid-body controller and an external continuum/fracture solve need not, and often should not, run at the same rate.

1. Symtropy advances robot/tool mechanics at a bounded step.
2. A rock model is requested at a declared event, cadence or substep boundary.
3. The solver runs out of process or behind a bounded adapter when required for fault isolation and runtime containment.
4. The returned result is validated against run ID, model generation, time window, input digest, units, frames, convergence and validity envelope.
5. Symthaea updates its belief/planner only from results that passed admission.
6. A safety arbiter independently checks commands against current measured hazards and authority.
7. The real-time path uses a previously qualified conservative fallback when the solver is late, invalid or unavailable.

Do not block a safety-critical control loop indefinitely waiting for a mesh solver. Do not extrapolate an old result without a tested bound. Do not permit rendering rate, camera distance, CPU/GPU load or asynchronous solver scheduling to change canonical physical semantics. Performance optimization (threads, SIMD, GPU) is a separate backend property; it does not change the model profile or turn an unvalidated solver into ground truth.

## 7. Exchange of physical effects

The coupling rules must be defined before two simulations exchange forces or state.

- A rock solver can return displacement, stress, damage/fracture indicators, pore-pressure/temperature fields, or other outputs only when those quantities are actually produced by that solver.
- Symtropy returns robot/tool state, contact events and measured/estimated boundary loads only to the extent represented by its validated dynamics.
- The coupling layer must declare which side owns each physical quantity at a given resolution and time interval.
- No two solvers may independently advance duplicate copies of the same conserved reservoir and then both claim canonical ownership.
- If coupled exchange transfers work, energy, mass, water or heat, record the signed transfer, interval, units, ownership transition and reconciliation residual.
- If a physical coupling law is not implemented, mark that interaction unavailable rather than inventing an effect from proximity or a normalized hardness value.

For a first MVP, avoid two-way fully coupled fracture/body contact. Start with a narrow, one-way workflow: a calibrated cutter/rock model returns bounded loads or expected cutting response to planning; Symtropy simulates the rigid robot/tool; independent measurements score the result. Only introduce two-way exchange once conservation, convergence, time alignment and boundary consistency have a dedicated benchmark.

## 8. Evidence and validation gates

Evidence tiers should remain distinct:

- **P0 — Specified:** interface, assumptions, units and non-claims reviewed.
- **P1 — Source validated:** input/result schemas and adapter error paths inspected.
- **P2 — Deterministic tests executed:** malformed inputs, unit/frame mismatch, stale response, timeout, non-convergence and replay rejection actually executed.
- **P3 — Solver benchmark reproduced:** exact solver case compared against an analytical result or published, traceable benchmark.
- **P4 — Coupling benchmark reproduced:** cross-solver time alignment, boundary transfer and conservation/error tolerances independently checked.
- **P5 — Lab-held-out:** predictions evaluated against independent physical specimen data, with error and uncertainty coverage.
- **P6 — Controlled robot/workcell:** prediction and tool behavior assessed with independent metrology under an approved test plan.
- **P7 — Bounded field evidence:** specific site, platform, operating envelope and independent engineering review.

A higher tier does not attach automatically to a neighbouring component. A Symtropy contact test cannot qualify an external fracture model; an OpenGeoSys benchmark cannot qualify the robot's return planner; a subsystem pass cannot qualify the complete composition.

### Initial adversarial corpus

1. Granite and basalt records with identical labels but conflicting or missing strength data: the result must retain uncertainty.
2. A unit typo (MPa versus Pa) or coordinate-frame mismatch: reject, do not normalize heuristically.
3. Solver result for an old request generation arrives after a new hazard/scene revision: reject as stale.
4. Solver timeout and numerical non-convergence: no nominal result and no authority expansion.
5. A rigid fragment hits a cutter in Symtropy: this proves a contact response for that represented body, not that a real fracture would create that fragment.
6. A rendered fracture visualization disagrees with solver result metadata: metadata/evidence integrity gate fails.
7. The same sensor is duplicated through two adapters: it counts as one physical source.
8. A stronger material parameter raises predicted cutting resistance only within the stated model scope; any apparent improvement must be explained by independently changed geometry/conditions, not silently averaged away.
9. A missing solver result cannot reuse the last successful output outside its declared freshness/validity window.
10. Enabling Phi/consciousness coupling changes exploratory policy in an experiment but must not rewrite material properties or engineering ground truth.
11. Thread/GPU load changes execution timing: scenario identity and canonical state-transition semantics remain within declared numerical tolerance.
12. A model outside its calibrated material/tool/scale range returns OutOfDomain rather than a confident extrapolation.

## 9. Implementation sequence

### S0 — Architecture and schema review

- Adopt this responsibility split alongside the general solver-composition contract.
- Define request/result schemas, units/frames and evidence statuses before adding a solver dependency.
- Record license and source-data rules for every candidate benchmark.

**Gate:** review confirms no duplicate physical authority and no claim that rigid contact equals fracture mechanics.

### S1 — Symtropy reference workcell

- Create a deterministic synthetic scene with a rigid robot chassis, articulated tool, tunnel clearance geometry, and explicitly labelled rigid obstacles/fragments.
- Add replayable motion and contact scenarios with measured outputs, not a visually plausible-only acceptance criterion.
- Keep fragment creation fixed/synthetic; do not claim it models fracture.

**Gate:** exact scene/seed/config replay, unit and frame tests, contact invariant tests and run manifest.

### S2 — Reduced-order rock/tool adapter

- Build a Rust-side adapter contract and simple calibrated cutting response.
- Bind all output to versioned material, tool and calibration references.
- Feed the response into Symthaea planning while leaving Symtropy responsible for represented rigid-body motion.

**Gate:** held-out measurements, interval coverage, unit/monotonicity tests and explicit unsupported-input handling.

### S3 — External continuum/fracture benchmark adapter

- Select a single target benchmark, not an entire solver category.
- Reproduce it independently with a pinned solver build, input files, boundary conditions and digest manifest.
- Define whether a native adapter or FMI interface best matches the chosen solver and runtime needs.

**Gate:** benchmark results and numerical error tolerance reproduced before coupling to a moving robot.

### S4 — Bounded multi-rate composition

- Add request/result lifecycle, cancellation/deadline, freshness and exact-generation checks.
- Define physical ownership and record transferred loads/energy/heat/water if present.
- Run stale-result, timeout, numerical-failure and mass/energy reconciliation tests.

**Gate:** external solver failure cannot bypass safety or produce unqualified control inputs.

### S5 — Laboratory and robot-in-the-loop validation

- Compare simulated cutter loads, penetration, wear and removed volume against independent metrology.
- Freeze calibration and hold-out splits by specimen/site.
- Capture exact source, model, tool, specimen, sensor/calibration and environment identities.

**Gate:** predefined predictive-error and intervention criteria met; independent engineering review approves the next operating envelope.

## 10. Licensing and implementation hygiene

Symtropy's core uses an Apache-2.0 OR MIT track, while some research/integration crates use AGPL. OpenGeoSys advertises BSD-3-Clause. Keep solver adapters and dependency graphs explicit, review license obligations before distributing a coupled artifact, and do not silently pull a copyleft research crate into a permissive product target. This is a project engineering checklist, not legal advice.

Prefer Rust for host-side request/result validation, evidence manifests, replay and orchestration. A domain solver may use another language where mature scientific software requires it; isolate it behind a documented adapter rather than translating a specialist solver into an unvalidated partial reimplementation just to make the stack Rust-only.

## 11. Research references

1. Symtropy repository and stated scope: https://github.com/Luminous-Dynamics/symtropy
2. General solver composition / physical ownership architecture, proposed in Symtropy PR #487: https://github.com/Luminous-Dynamics/symtropy/pull/487
3. Symthaea rock mechanics roadmap, proposed in Symthaea PR #7438: https://github.com/Luminous-Dynamics/symthaea/pull/7438
4. OpenGeoSys: coupled THMC simulation in porous/fractured media: https://www.opengeosys.org/stable/
5. OpenGeoSys phase-field fracture benchmarks: https://www.opengeosys.org/stable/docs/benchmarks/phase-field/
6. OpenGeoSys GREAT-cell hydro-mechanical fracture benchmarks: https://www.opengeosys.org/stable/docs/benchmarks/phase-field/greatcellhm_vpf/
7. OpenGeoSys thermoelastic-stress analytical benchmark: https://www.opengeosys.org/stable/docs/benchmarks/thermo-mechanics/thermomechanics/
8. International Society for Rock Mechanics and Rock Engineering (ISRM) test-method recommendations: https://isrm.net/isrm/page/show/305 and https://w.isrm.net/page/show/1305
9. Functional Mock-up Interface (FMI), model exchange/co-simulation: https://fmi-standard.org/

These references support a design candidate and verification strategy, not a statement that an external solver is integrated, correct for a specific use case, or qualified for operation.

## 12. Non-claims

This proposal does not add runtime code, modify Symtropy's physical laws, implement a rock/fracture solver, validate granite/basalt property sets, qualify any robot, or authorize excavation. It proposes a reviewable boundary so those capabilities can be implemented and measured without conflating rigid-body simulation, specialist geomechanics, cognition and physical authority.
