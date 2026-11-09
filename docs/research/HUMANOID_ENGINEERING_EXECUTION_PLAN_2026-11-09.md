# Symtropy: evidence-first robotics and engineering execution plan

**Review date:** 2026-11-09  
**Scope:** Symtropy's physics/simulation responsibility in the Luminous humanoid programme and reusable engineering runtime  
**Status:** Research and implementation plan. It is not an approved product specification, qualified model, safety case or production release.

## 1. Executive direction

Do not try to make Symtropy beat MuJoCo, Newton, Isaac Lab, Rapier and specialist CAE systems at every task. Make Symtropy the **evidence-producing, backend-neutral scenario and physics integration layer** that can identify what a particular model can and cannot represent, execute a reproducible scenario, compare results to independent references, and preserve the exact evidence needed by the Luminous engineering digital thread.

The competitive thesis is not "we have a more accurate physics engine." It is:

> A consequential engineering result should be reproducible, revision-bound, model-path-aware, uncertainty-qualified and independently falsifiable—and should remain connected to the physical component and its service history.

This can become useful beyond humanoids: vehicles, aircraft, energy systems, construction robotics, industrial equipment and infrastructure.

## 2. System authority boundaries

| System | Owns | Must not silently take over |
|---|---|---|
| **Luminous engineering platform** | Product lifecycle, candidate and product identity, requirements, engineering decisions, EBOM/MBOM/service BOM, controlled documents, revisions, change-impact graph, production/as-built/service records | Physics calculations, CAD solver internals, source identity or trust authority belonging to specialist systems |
| **Symtropy** | Physics model execution, simulation scenarios, supported model paths, numerical diagnostics, deterministic replay within a declared profile, simulation output artifacts | Product-lifecycle authority, approval/attestation authority, CAD/FEA/EDA/MES, human-robot safety authority |
| **Symthaea** | Cognitive reasoning, beliefs, plans, experiment selection, active information gathering and human interaction; emits typed, expiring intent | Direct unrestricted actuator torque, independent hard safety limits or qualification approval |
| **Sol-Atlas** | Geometry and spatial representations, frames, topology, semantics, time-varying scene state and spatial provenance/uncertainty | Product configuration authority or physics-solver truth |
| **Mycelix** | Identity, authorization, provenance, custody, attestations and governance | Physical inspection, measurement quality or engineering acceptance by itself |

The existing Luminous platform should connect CAD/FEA/EDA and manufacturing systems by durable references, content digests, exchange formats and revision/effectivity links. It should not duplicate their native authoritative design files.

**Implementation status (2026-11-09):** this authority is a target architecture, not a complete system already present in the workspace. The inspected `symthaea-engineering` facade composes requirements/reviews with other domains; `symthaea-digital-twin` currently models telemetry, health, residuals and uncertainty; `symthaea-sim-bridge` has solver-neutral requests/results and evidence classes; and `symthaea-fabrication-kernel` has valuable geometry/process/release/audit machinery. The inspection did not find a unified Product/ConfigurationItem/EBOM/MBOM/ServiceBOM/EngineeringChange/Effectivity/AsBuilt object model. That missing authority is being specified as an internal Luminous engineering-digital-thread work package.

The current `symthaea-humanoid` domain is the main humanoid dynamics/control integration (with separate MuJoCo oracle/release evidence); the smaller humanoid demo in this Symtropy repository must not be mistaken for the whole Luminous humanoid stack. Keep the model authority explicit and use independent solver references rather than duplicating a controller or asserting that this physics core is the only simulation path.

## 3. Findings from the repository audit

The public repository is substantially more than the earlier small balance demo: it contains the N-dimensional math/physics crates, transformed collision validation, replay primitives, robotics bridge crates, a humanoid demo, and active scenario/benchmark work.

The key claim boundaries are nevertheless important:

1. The public `symtropy-physics` core offers const-generic rigid-body physics, GJK/EPA collision paths, contact/constraint machinery, joint primitives, query and replay facilities. It is not yet a generally qualified articulated-humanoid multibody solver.
2. The canonical physics validation notes identify real limitations: asymmetric inertia/gyroscopic behavior, general rotational CCD, some rotated-contact manifold paths, mesh-path validation, reduced-coordinate multibody/inverse dynamics, and cross-architecture bitwise replay guarantees.
3. The robotics bridge's `UniformGainPlanner` is a scalar baseline. The generic body-spawn helper is a coarse proxy, not an articulated CAD-derived robot.
4. The humanoid demo's controller is explicitly a PD controller holding joints near a neutral standing pose, with 21 actuators receiving a uniform gain. It does not demonstrate walking, manipulation, whole-body control or industrial task competence.
5. The correct cognitive/control/safety boundary is already specified in [#2828](https://github.com/Luminous-Dynamics/symtropy/issues/2828), with an active implementation sequence beginning at [PR #2829](https://github.com/Luminous-Dynamics/symtropy/pull/2829).
6. The permissive `symtropy-robotics-bridge-core` manifest depended on the AGPL `symtropy-consciousness-physics` crate solely to reuse a tier enum. This branch removes that dependency and defines a local attenuation enum. The local helper is explicitly not a safety-rated function; the full Symthaea/consciousness integration remains in the AGPL sibling crate. This licensing correction is proposed and still requires independent review and build verification.

Read the source-backed limits before using a feature in an engineering context:

- [Physics research validation](../../crates/core/symtropy-physics/RESEARCH_VALIDATION.md)
- [Transformed collision validation](../../crates/core/symtropy-physics/ORIENTED_COLLISION_VALIDATION.md)
- [Engine-wide qualification corpus #460](https://github.com/Luminous-Dynamics/symtropy/issues/460)
- [Engine capability census #1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472)

## 4. Paradigm shifts worth pursuing

### 4.1 From "simulation success" to model-validity envelopes

A model should declare its capability path for each requested operation: supported under exact implementation semantics, supported numerical model, declared approximation, provider-backed, partial, unsupported or unknown. Add a context-of-use validity envelope: geometry family, dimensions, velocity/load/temperature ranges, contact regime, timestep, solver profile and known failure modes.

Keep five statements separate:

1. Capability is present in code.
2. The exact path has been exercised by a regression.
3. The numerical implementation agrees with an analytical or independent reference within a declared tolerance.
4. The model agrees with calibrated physical measurements within a stated context of use.
5. The complete product has passed applicable independent qualification.

None implies the next automatically.

### 4.2 From one simulator to federated multi-fidelity execution

Use the least-cost model that can answer the engineering question, then escalate uncertainty or risk:
- Symtropy for its validated rigid-body, geometry, custom-dimension and deterministic-scenario use cases.
- [MuJoCo](https://mujoco.readthedocs.io/en/stable/overview.html) for articulated dynamics, contact-rich control, inverse dynamics and system identification.
- [Newton](https://developer.nvidia.com/newton-physics) and Isaac Lab where GPU throughput, differentiable physics, multiphysics or robot-learning infrastructure is appropriate.
- Rapier/Jolt/other independent engines for differential rigid-body references.
- Domain-specific FEA/CFD/thermal/electrical tools for specialist analyses.

A federation must preserve units, frames, sign conventions, boundary conditions, time bases, solver settings, material revisions and provenance. Two simulators agreeing is not physical truth; a disagreement is a diagnostic, not a reason to select the more convenient result.

### 4.3 From optimizing average performance to active falsification

Use uncertainty and consequence to choose the next experiment. Search for counterexamples at regime boundaries: grazing contacts, high friction, rapid thermal drift, backlash transitions, sensor dropouts, stale commands, actuator saturation, water/dust ingress and post-maintenance degradation. Preserve failures as named regression fixtures; never discard them from an aggregate.

### 4.4 From robot capability to recoverable capability

A robot should not merely report "vision confidence is low." It should know which sensing channels remain valid, whether they share a common-cause failure, which fallback behaviors are admitted for that actual evidence set, and whether the transition preserves a recoverable physical state. Re-entry after a sensor recovers requires revalidation and re-admission.

Compose the existing [#4332 multimodal failover](https://github.com/Luminous-Dynamics/symtropy/issues/4332) and associated contact, viability, uncertainty and safety-vector work. Do not build a duplicate OOD classifier or viability solver.

### 4.5 From nameplate efficiency to measured lifecycle energy

Calibrate actuator maps at the component boundary before claiming whole-robot efficiency. Record DC-bus energy, driver losses, motor losses, gearbox losses, mechanical joint work, auxiliaries and battery energy as separate nested boundaries. Negative mechanical work is not automatically regenerative electrical energy.

Compose [#4044 energy accounting](https://github.com/Luminous-Dynamics/symtropy/issues/4044) and [#4179 actuator loss-map qualification](https://github.com/Luminous-Dynamics/symtropy/issues/4179).

### 4.6 From sealed-for-life to repairable environmental protection

Treat external gaiters, seal cartridges, wipers, bellows, covers and replaceable interface modules as controlled product components. Qualify not only new ingress resistance but also the same protection after removal, resealing, maintenance cycles, contamination and field exposure. Every repair changes the as-maintained configuration and may invalidate affected qualification evidence.

### 4.7 From product documentation to evidence continuity

A component's identity should connect requirement → design revision → material/lot → process → inspection/NDT → acceptance → custody → installation/as-built → operational exposure → maintenance → requalification. Signatures prove who attested to what; they do not make a bad measurement or out-of-tolerance part conforming.

Compose with [#3331 component digital thread](https://github.com/Luminous-Dynamics/symtropy/issues/3331) and the Luminous lifecycle platform.

## 5. Benchmark strategy: answer different questions with different scoreboards

### 5.1 Physics and numerical correctness

Use the current [#460 physics corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460), analytical fixtures and independent engines. Report per scenario and solver profile:
- State and trajectory error.
- Contact classification agreement, penetration and contact-normal error.
- Constraint residual and drift.
- Momentum/energy accounting where the model supports those invariants.
- CCD misses/false positives within declared semantics.
- Replay divergence, non-finite state and failure counts.
- Runtime distributions, allocations, memory and solver iteration counts.

**Target:** no unreported known failures; zero false PASS on the registered negative controls; numerical acceptance thresholds preregistered per fixture; convergence and error-vs-cost curves published. Do not set a universal "1% accurate" requirement across different physical quantities.

### 5.2 Articulated dynamics and controller benchmarks

Compare against MuJoCo first for matched articulated systems. Include Newton/Isaac Lab for the solver/compute regimes they are designed to serve. Keep identical URDF/MJCF/USD geometry, mass and full inertia, joint axes/limits, actuator model, contact parameters, control rate, initial states and solver budgets.

Measure:
- Joint position/velocity/torque error against the independent lane.
- Constraint and energy residuals.
- Contact impulse, slip and recovery.
- Trajectory error and task success.
- p50/p95/p99 step time and missed real-time deadlines.
- Model-to-hardware residual where calibrated physical tests exist.

Do not claim superiority from speed alone if the engines are solving different contact or constraint models.

### 5.3 Locomotion, recovery and energy

Use [#3357](https://github.com/Luminous-Dynamics/symtropy/issues/3357), which pins FlyGym 2.1.1 and specifies CPG, rule-based, hybrid, learned and PID/reflex baselines. Keep train/test splits and causal interventions explicit. Report task success, falls, recovery probability/time, foot slip, support margin, joint/actuator saturation, thermal state, energy, OOD degradation and tail risk.

Use [#4332](https://github.com/Luminous-Dynamics/symtropy/issues/4332) to add fault combinations, failover latency, false transition/re-entry, recovery feasibility and sensor/actuator common-cause faults.

### 5.4 Spatial reasoning and active perception

Track at least:
- Spatial QA accuracy and calibration.
- Object/target grounding accuracy and localization error.
- Instruction-change success in unchanged and changed layouts.
- Intervention efficiency: observations/actions required to resolve ambiguity.
- Execution success, wrong-target action rate and recoverability.
- Map/geometry uncertainty calibration and provenance completeness.

Use three complementary benchmarks, without pooling their scores:
- [ESI-Bench](https://arxiv.org/abs/2605.18746) evaluates action-conditioned spatial evidence acquisition across 10 task categories and 29 subcategories. Its authors report that active exploration can outperform passive views and that random multi-view capture can add noise; action blindness is a central failure mode.
- The [BEHAVIOR-Spatial October 2026 preprint](https://arxiv.org/abs/2610.34051) reports 41.6% held-out configuration success and 16.1% when the requested target changes while the layout is held fixed. These are benchmark-specific simulated baselines, not universal robot capability numbers.
- [VA-Bench (September 2026)](https://arxiv.org/abs/2609.19554) evaluates visual demonstration, active camera selection, metric spatial control and feedback-driven revision. Its authors report a three-run macro-average task success of 53.93 ± 3.17% for their best model condition, a matched active-camera comparison improving from 27.86% to 57.50%, and significant degradation under held-out geometry. No strict long-horizon episode was completed in the reported evaluation.

Research milestones: first establish fully versioned splits and reproducible baselines; then aim for 70% held-out configuration and 50% changed-instruction success on BEHAVIOR-Spatial as stretch targets; separately aim for ≥30% fewer observation actions at equal-or-better task success on ESI-Bench/VA-Bench versus passive or random-view baselines. These are proposed internal milestones, not achieved performance. Publish confidence intervals, task success definitions, false-free/wrong-target rates, action/observation budgets and abstention/recovery outcomes.

### 5.5 Humanoid application readiness

Use NIST's [Humanoid Robot Baseline Performance Benchmark](https://www.nist.gov/el/intelligent-systems-division-73500/humanoid-robot-baseline-performance-benchmark) as a developing baseline proposal, not as a completed certification scheme. Commission independent assessment against [Fraunhofer IPA's humanoid benchmark](https://www.ipa.fraunhofer.de/en/press-media/press_releases/benchmark-for-humanoid-robots.html), which divides application readiness into six areas: capability, complex tasks, cleanroom suitability, functional safety, cybersecurity and energy efficiency.

The Fraunhofer G1 evaluation provides a useful duty-cycle reference: the tested EDU-4 configuration reported 2 h 49 min standing and 1 h 49 min for a typical standing/walking scenario. Those values are meaningful only when payload, hands, firmware, gait, speed, floor, temperature and test profile are matched.

**Manufacturing and workcell integration comparator.** Figure reported in April 2026 that its BotQ facility had produced more than 350 third-generation humanoids and had moved its reported F.03 line rate from one robot per day to a one-per-hour target cycle. In June 2026, Figure reported F.03 entering a BMW Spartanburg logistics/sequencing workflow, with Helix 02 coordinating the hands, arms, torso and feet during cart handling. These are **manufacturer-reported milestones**, not independently audited sustained throughput, field availability or end-to-end productivity. They are nevertheless useful design-for-manufacture and workcell-integration references. A fair comparison should request delivered serialised units, accepted first-pass yield, sustained shift throughput, cycle time, human interventions, uptime, maintenance labour, safety incidents and cost per successful task. Sources: [Figure production ramp announcement, 29 April 2026](https://www.figure.ai/news/ramping-figure-03-production) and [F.03 at BMW, 30 June 2026](https://www.figure.ai/news/f-03-at-bmw).

[DEEP Robotics advertises](https://en.deeprobotics.cn/en/index/dr02.html) DR02 full-body IP66 and an operating range of -20°C to +55°C. Treat these as **manufacturer claims and competitive targets**, not independently verified facts about our own design. A Luminous claim requires a complete assembled configuration, a defined test profile and independent test evidence.

## 6. First physical work package: the rotary-joint cartridge

Tracked in [HUMANOID-JOINT-CARTRIDGE-001 #4337](https://github.com/Luminous-Dynamics/symtropy/issues/4337).

The correct order is:

1. Freeze intended application, duty cycle, load combinations, torque/speed envelope, mass/volume, electrical bus, thermal environment, ingress exposure and maintenance needs.
2. Compare direct-drive PMSM/BLDC, planetary, strain-wave and cycloidal alternatives against the same requirements. Do not select a topology on peak torque alone.
3. Define the CAD/interface baseline and revision-bound exports. Luminous owns the EBOM and configuration.
4. Admit only supported model paths. Use Symtropy for validated scopes; use MuJoCo/other qualified multibody models and specialist FEA/thermal tools where its current semantics are insufficient.
5. Build a calibrated bench measurement plan for torque, speed, voltage/current, temperature, backlash, stiffness, vibration and defined environmental exposure.
6. Correlate the exact simulated configuration with physical measurements, publish discrepancies and update the validity envelope.
7. Demonstrate replacement/reseal/calibration and requalification, then bind the serialized module to its as-built/service record.

The first deliverable is a reviewed requirements and interface baseline—not an arbitrarily chosen motor or a production CAD drawing.

## 7. Recommended implementation order

### P0 — claim and evidence integrity
- Keep the README and crate licensing truthful.
- Finish the core capability census (#1472) and engineering scenario envelope (#1172).
- Ensure unsupported model paths are visible and fail closed in engineering profiles.
- Preserve exact source/configuration identities and portable semantic observables.
- Land and independently review the typed robotics intent, assurance, dispatch and outcome sequence (#2828 / PR #2829 and its follow-ons).

### P1 — first reusable engineering demonstrator
- Open and review #4337 as the reference humanoid component.
- Integrate the Luminous requirements/configuration/EBOM record with Sol-Atlas spatial interfaces and Symtropy scenario manifests.
- Calibrate an actuator map through #4179.
- Compare a selected articulated joint chain against MuJoCo with exact matched model inputs.
- Commission a physical fixture only after requirements, hazard review and measurement plan approval.

### P2 — lower-body mobility
- Integrate qualified joint modules, contact evidence, terrain/spatial uncertainty, thermal/energy-aware control and recoverability gating.
- Benchmark on held-out terrains and fault combinations, not only nominal scripted routes.
- Keep a physical lower-body testbed separate from the humanoid demo until the model and controller are calibrated.

### P3 — integrated humanoid and field service
- Expand torso, arms, hands, sensor and power architecture under configuration control.
- Qualify application-specific contact, functional safety, cybersecurity, ingress, thermal, runtime, reliability and maintainability.
- Track each serialized robot's exact hardware, firmware, model, calibration, maintenance and current operating restrictions with a Robot Resilience Passport.

## 8. Standards and interoperability posture

- **[ISO 10303-242:2025 (STEP AP242)](https://www.iso.org/standard/84300.html):** managed model-based 3D engineering and product-data exchange. Use it as a CAD/PMI/configuration exchange format where supported; it does not replace the PLM configuration authority. The 2025 edition is published; successor revision work is underway.
- **[IEC 81346-14:2026](https://webstore.iec.ch/en/publication/81613):** structuring and reference designations for manufacturing/processing systems and their technical documentation. It does not assign the serial identity of individual manufactured products; keep product-instance IDs in Luminous PLM.
- **[ISO 23247-6:2026](https://www.iso.org/standard/87426.html):** digital-twin composition patterns (integrated, unified and federated). This supports composing specialist twins rather than forcing all tools into one simulator.
- **[ISO 10007:2017](https://www.iso.org/standard/70400.html):** current published configuration-management guidance; a revision is under development. Apply change control, configuration identification, status accounting and audit practices without claiming certification.
- **[ISO/FDIS 13482, Edition 2](https://www.iso.org/standard/83498.html):** at approval stage 50.20 on 2026-11-09 and intended to replace ISO 13482:2014. It addresses service robots and physical human contact; its scope does not cover industrial or medical robots. Determine the compliance route from intended use and classification rather than assuming one robot standard covers all deployments.

Standards are design inputs. They do not certify the robot or substitute for a competent conformity assessment.

## 9. Proposed internal milestone register

These are **proposed research and engineering targets**, not current results, industry standards, or certification thresholds. Freeze the test method, configuration, measurement uncertainty, pass/fail rule and independent reviewer before using any target as a release gate.

| Domain | Proposed milestone | Required qualification caveat |
|---|---|---|
| Environmental protection | IP6X + IPX6 on the complete assembled configuration, with a -20°C to +55°C operating design envelope | DR02 advertises IP66 and this temperature range; verify our own complete robot in an independent lab. Repeat affected ingress tests after defined seal/service cycles. IP66 is not an immersion rating. |
| Mixed-duty runtime | ≥2 h 30 min as an initial stretch target under a preregistered mixed standing/walking/manipulation profile | Measure at battery and DC bus; compare with Fraunhofer's 1 h 49 min G1 mixed-duty result only under matched payload, hands, firmware, gait, speed, terrain and temperature. |
| Spatial reasoning | ≥70% held-out configuration and ≥50% changed-instruction success on BEHAVIOR-Spatial as stretch milestones | Report separate benchmark task splits, uncertainty, confidence intervals, target-grounding errors and real action success; never pool with ESI-Bench or VA-Bench. |
| Active perception | ≥30% fewer observation actions for equal-or-better success than passive/random-view baselines on ESI-Bench and VA-Bench | Preregister the task subset, action/latency budget and quality constraint; report cases where active sensing increases risk or fails to resolve ambiguity. |
| Simulation validity | Zero false PASS on registered negative controls; no silent execution of unsupported model paths | For each physical observable, set a context-specific acceptance envelope using combined prediction and measurement uncertainty; a generic universal error percentage is not appropriate. |
| Initial reliability screen | ≥95% operational availability over a defined 100-hour field screening profile | Report planned/unplanned downtime, intervention time, fault counts and operating envelope. This is only an initial screening milestone—not an MTBF claim or industrial reliability demonstration. |
| Extended reliability | A multi-unit, ≥500-hour field pilot with failure-mode, maintenance and confidence-interval reporting | Use actual duty-cycle exposure and censoring rules; do not infer population MTBF from a single robot or a short demonstration. |
| Manufacturing readiness | 100% traceability for designated critical components; measure first-pass yield, rework, cycle time and process capability | A future Cpk ≥1.33 target may be set for stable, capable processes on selected critical dimensions; it is not meaningful before process stability and measurement-system analysis. |
| Maintainability | Replace designated external protection/service modules without opening the protected core, then complete calibration and requalification | Measure service time, tools, skills, errors, seal damage, replacement-part traceability and post-service test results. Do not assume a universal reseal cycle count before the design is selected. |
| Human/functional safety | Independent assessment against the hazard analysis and standards applicable to the intended application | Do not adopt a universal collision-force threshold from another robot's test. Limits depend on body region, contact type, duration, speed, protective measures and application. |
| Cybersecurity | Signed firmware/model artifacts, secure update and rollback, SBOM/model manifest, and no unresolved critical/high findings at release | Include update-chain, key management, remote interface, telemetry/privacy, vulnerability disclosure and recovery tests. |

The strongest “better than SOTA” claim should be made only on a named benchmark lane, under a matched protocol and resource budget, with a reproducible baseline and statistical uncertainty. A faster simulation with different contact semantics, a longer advertised battery runtime on a different duty cycle, or a higher score on a different task split is not a fair comparison.

## 10. Paradigm-level success criteria

Symtropy should aim to outperform the status quo on **reproducibility, capability transparency, cross-tool traceability, falsification efficiency and measured simulation-to-physical correlation**—not claim a universal physics advantage.

The programme is succeeding when:
- Every consequential run can be recreated from the archived source, scenario, configuration and toolchain identity.
- Every requested model operation declares exact, approximate, provider-backed, unknown or unsupported semantics.
- All known negative controls remain visible and cannot be silently promoted to PASS.
- Benchmark comparisons are matched and include uncertainty, tails and resource budgets.
- The next physical experiment is selected to reduce consequential uncertainty, not merely to generate more data.
- The exact simulated component revision can be linked to its manufactured serial number, inspection, installed state and service history.
- Service/repair changes trigger an explicit requalification-impact analysis.
- The robot can reduce its operating envelope when evidence or component condition degrades.

No target percentage substitutes for this evidence chain. Any claim of being better than SOTA must be tied to a public protocol, a matched baseline, an independently reproducible result and—where the claim concerns physical behavior—physical measurements.
