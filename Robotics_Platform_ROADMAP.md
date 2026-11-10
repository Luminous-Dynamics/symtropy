# Symtropy Robotics Platform Roadmap

**Last reviewed:** 2026-11-09  
**Status:** engineering programme roadmap; all gates are evidence-dependent.

## North star

Develop reusable, maintainable and evidence-qualified robotic platforms for real-world environments. Humanoids are one embodiment, not the only product category and not a reason to make every subsystem humanoid-specific.

The programme should optimize for useful capability across a declared operating envelope, including reliability, repairability, environmental resilience, energy, safety, accessibility and lifecycle cost—not only demonstration performance.

## System ownership

| System | Authority |
|---|---|
| **Luminous engineering platform** | Product identity, requirements, engineering decisions, EBOM/MBOM/service BOM, revisions, technical documents, change impact, manufacturing and as-built records |
| **Symtropy** | Physical models, mechanics, dynamics, energy/thermal models where implemented, scenario execution and simulation evidence |
| **Sol-Atlas** | Spatial intelligence, geometric relationships, scene representations, localization, terrain and spatial provenance |
| **Symthaea** | Cognitive reasoning, perception, belief, planning and human interaction |
| **Mycelix** | Identity, permissions, trust, provenance, attestations and governance |

A component's CAD authority, product revision, approval and manufacturing status belong to the engineering lifecycle system. Symtropy consumes exact references and returns bounded physical-analysis evidence; it does not become the PLM or safety authority.

## Programme principles

1. **Requirements before hardware selection.** Freeze intended use, environment, task, loads, duty cycle, hazards and acceptance criteria before choosing actuators, dimensions or materials.
2. **Capability before claims.** Use the exact engine capability/model-path census. A generic API or demo is not proof of a qualified physical model.
3. **Matched baselines.** Compare candidates on the same mission, payload, terrain, controller budget, observation set, environment and physical boundary.
4. **Nominal and degraded operation.** Co-design morphology, actuation, sensing, power and recovery around credible faults, not just nominal locomotion.
5. **Repairability is a design property.** Replaceable seals, gaiters, wipers, covers, cables, bearings and actuator cartridges should have explicit replacement and requalification procedures.
6. **Evidence survives configuration change.** Geometry, material, firmware, actuator, supplier or calibration changes trigger explicit impact analysis; old evidence is not inherited without review.
7. **Human dignity and control.** Robots should assist people without hiding uncertainty, overriding legitimate human control or making unreviewed safety-critical decisions.

## Phase A — Engineering and simulation foundation

**Primary gate:** a reproducible engineering scenario and evidence chain exist before expensive hardware commitments.

Existing work:
- [#1172 — deterministic engineering scenario and replay substrate](https://github.com/Luminous-Dynamics/symtropy/issues/1172)
- [#1472 — exact engine capability/model-path census](https://github.com/Luminous-Dynamics/symtropy/issues/1472)
- [#460 — engine-wide physics qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460)
- [#3749 — analytic physics oracles, scenario V&V and solver-level benchmarks](https://github.com/Luminous-Dynamics/symtropy/issues/3749)
- [#443 — Rapier differential reference runner](https://github.com/Luminous-Dynamics/symtropy/issues/443)
- [#4366 — cross-simulator semantic equivalence](https://github.com/Luminous-Dynamics/symtropy/issues/4366)

Deliverables:
- exact engine/profile capability declarations;
- scenario and run identity with immutable input/output digests;
- analytical, metamorphic and negative-control physics cases;
- numerical conditioning, sparse-constraint and solver-stability challenges under #3749;
- independent reference lanes with explicit semantic mapping;
- uncertainty, applicability and evidence-class metadata;
- stale-evidence detection when a relevant configuration changes.

**Exit criteria:** every published comparison binds exact source/configuration and has an admitted semantic mapping. Unknown or unsupported physical paths remain visible and cannot pass a matched qualification gate.

## Phase B — Repairable rotary-joint demonstrator

**Primary gate:** select a joint architecture only after a reviewed requirements baseline and matched trade study.

Existing work:
- [#4337 — repairable sealed rotary-joint demonstrator](https://github.com/Luminous-Dynamics/symtropy/issues/4337)
- [#4179 — actuator loss-map qualification](https://github.com/Luminous-Dynamics/symtropy/issues/4179)
- [#4044 — locomotion energy accounting](https://github.com/Luminous-Dynamics/symtropy/issues/4044)
- [#460 — physics qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460)

Candidate architectures to compare:
1. frameless motor with planetary reduction;
2. frameless motor with strain-wave reduction;
3. cycloidal reduction where shock and service trade-offs justify it;
4. direct drive/high-torque motor where mass and thermal envelopes permit.

Model and test dimensions:
- continuous and peak torque-speed envelope;
- current, voltage and electrical/mechanical efficiency;
- backlash, torsional stiffness, bearing load and reflected inertia;
- heat generation, heat path, temperature sensing and thermal derating;
- seal, gaiter and interface protection;
- ingress, corrosion and condensation as separate physical tests;
- repair time, replaceable-unit granularity, calibration and requalification.

Use specialist multibody, FEA and thermal solvers where Symtropy's exact model path is not qualified. Keep measured, simulated, analytical and assumed data separate.

**Exit criteria:** frozen requirements, competing architectures, traceable geometry/material baselines, calibrated fixture, held-out prediction/correlation results, service demonstration and post-service requalification procedure. No ingress rating or product-safety claim without applicable physical testing and competent assessment.

## Phase C — Locomotion and fault-tolerant embodiment

**Primary gate:** demonstrate useful locomotion under matched nominal and degraded conditions.

Existing work:
- [#3357 — matched locomotion benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/3357)
- [#4358 — actuator-fault diagnosis/reconfiguration/recovery campaign](https://github.com/Luminous-Dynamics/symtropy/issues/4358)
- [#4360 — fault-tolerant morphology co-design](https://github.com/Luminous-Dynamics/symtropy/issues/4360)
- [PR #4361 — morphology co-design protocol](https://github.com/Luminous-Dynamics/symtropy/pull/4361)
- [PR #4362 — morphology Pareto auditor](https://github.com/Luminous-Dynamics/symtropy/pull/4362)

Compare:
- conventional quadruped/biped baseline;
- functionally redundant appendages;
- passive compliant or overload-slip mechanisms;
- modular/reconfigurable limb options;
- isolated power/drive architectures;
- diagnosable actuator and sensor layouts.

Measure nominal task success, payload, speed, energy, contact quality, falls, fault diagnosis latency, recoverability, worst-decile performance, maintenance burden and failure containment. Keep hard viability and safety gates outside any weighted score.

**Exit criteria:** held-out scenarios and fault cells are evaluated on exact hardware/simulation configurations; improvements survive matched controller, compute and payload budgets; no safety or recovery gate is bypassed by a high nominal score.

## Phase D — Environmental resilience and service lifecycle

**Primary gate:** verify that exposure, wear and repair do not silently invalidate capability claims.

Develop:
- replaceable environmental barriers and sealing cartridges;
- thermal and moisture management;
- protected electrical/communication interfaces;
- corrosion, dust, water, temperature and cleaning protocols;
- condition monitoring with sensor uncertainty;
- maintainable fasteners, connectors and inspection access;
- component identity, service lineage and as-maintained configuration.

A Robot Resilience Passport should report the exact approved configuration, environmental envelope, inspections, calibration, known degradation and operational restrictions. A detected seal or joint-cover fault narrows the permitted envelope until inspection and requalification are complete.

## Phase E — Integrated humanoid platform

Only after component and mobility foundations have passed their gates:
- lower-body and torso structural architecture;
- power distribution, thermal management and battery service isolation;
- modular hands, feet, sensors and end effectors;
- clothing and external protection as replaceable engineered subsystems;
- integrated perception, planning and human interaction;
- local safety controller and independent emergency-stop/energy-isolation paths;
- cybersecurity, privacy and update/rollback lifecycle evidence.

Do not assume that one embodiment or actuator architecture is optimal for every application.

## Phase F — Human-centred deployment

Potential application tracks include:
- industrial assistance and inspection;
- maintenance and hazardous-environment support;
- humanitarian logistics and accessibility;
- multilingual communication and telepresence;
- human-guided collaboration and mediation.

Human studies require explicit consent, privacy protections, clear human control, independent safety review and predeclared outcome measures. Human-likeness is not a success metric by itself.

## Benchmark and release gates

Report conventional performance and lifecycle performance separately.

| Domain | Required reporting |
|---|---|
| Mobility | Task success, payload, speed, terrain, contact/slip, falls, recovery and worst-decile results |
| Energy | Electrical input, mechanical output, losses, auxiliaries, thermal state and complete duty-cycle boundary |
| Environmental | Exact assembled configuration, exposure protocol, pre/post inspection and function |
| Safety | Hazard analysis, application-specific contact/control limits, independent assessment and fail-safe behaviour |
| Reliability | Trial hours, task exposure, failures, repair events, confidence intervals and censoring |
| Maintainability | Diagnosis/replacement/requalification time, tools, spare availability and return-to-service |
| Spatial intelligence | Held-out geometry, changed instructions, uncertainty calibration, active observation value and execution success |
| Cybersecurity | Threat model, secure update/rollback, identity/authorization and independent assessment |
| Engineering evidence | Exact configuration, source digests, solver profile, assumptions, uncertainty, test traceability and evidence freshness |

Targets such as runtime, availability, ingress protection, accuracy and repair time must be set only after the product requirements, duty cycle, measurement method and baseline are frozen. A 100-hour demonstration is a screening milestone, not an industrial MTBF claim.

## External humanoid task benchmark alignment

NIST's Humanoid Robot Baseline Performance Benchmark is currently a proposal being developed with industry and research partners, not a finalized certification or universal ranking. Its proposed task families cover basic mobility and manipulation/dexterity, coordinated loco-manipulation, confined-space whole-body awareness/control, and minimal task/scene reasoning. Track it as an **application-level task benchmark**, and update the task manifest when NIST publishes consensus revisions.

Keep two linked scorecards:
- **Model and solver evidence:** semantic equivalence, analytical/oracle error, convergence, numerical stability, uncertainty and physical correlation.
- **Robot task outcomes:** task success/failure, time, payload, environment, energy, recovery, intervention and exact robot/controller configuration.

Task success cannot validate simulator fidelity: a controller may compensate for model error. Conversely, a numerically accurate simulator does not prove perception, controller or robot-system performance.

Reference: [NIST Humanoid Robot Baseline Performance Benchmark proposal](https://www.nist.gov/el/intelligent-systems-division-73500/humanoid-robot-baseline-performance-benchmark).

## Explicit non-goals

- Do not build a full humanoid before validating key components and engineering infrastructure.
- Do not duplicate CAD, FEA, CFD, EDA, PLM, MES, spatial mapping, safety authority or provenance ledgers.
- Do not treat simulated success as physical qualification.
- Do not call the system certified, public-safe, all-weather or production-ready without the relevant testing and approval.
- Do not use consciousness/Φ-related experimental coupling as a real-world safety mechanism.

The programme succeeds when a physical robot can be reconstructed, understood, repaired and requalified from its exact engineering and field evidence—not when a demo looks convincing.
