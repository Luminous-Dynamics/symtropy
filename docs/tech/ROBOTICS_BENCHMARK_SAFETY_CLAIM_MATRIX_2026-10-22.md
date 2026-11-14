# Robotics Benchmark and Safety-Claim Matrix — 2026-10-22

**Status:** research design, not a conformity assessment, safety case, risk assessment, or authority to operate physical machinery.

**Scope:** the platform portfolio and synthetic corpus in this PR, plus downstream work on mobile robotics, manipulation, forestry, timber handling, and construction support.

## 1. Core distinction: performance is not safety

Keep three evidence tracks separate; do not collapse them into one score:

1. **Task performance:** what qualified work was completed, with which material, tool, load, environment, sensors, and time budget?
2. **Software/process safety contract:** did every modeled transition remain authorized, preserve uncertainty, reject stale/mismatched evidence, and reach a declared disposition under the tested faults?
3. **Physical safety and conformity:** did the actual built system satisfy applicable hazard controls under physical test and independent review for its intended use?

A system can perform well and violate its software contract. It can pass both in simulation and still fail physically. A receipt proves the identity/result of a specified computation, not physical assumptions or truth of sensor inputs.

Every report must label evidence as **synthetic contract**, **simulation result**, **bench/HIL measurement**, **controlled field observation**, or **independent conformity/regulatory evidence**. A result must never be promoted to a higher evidence class by inference.

## 2. Standards and industry reference map

The following public references were checked on 2026-10-22. Before any physical prototype, map the applicable edition and national adoption to the actual robot, task, installation, market, and jurisdiction.

| Reference | Relevant use | Important limitation |
|---|---|---|
| [ISO 10218-1:2025](https://www.iso.org/standard/73933.html) | Industrial robot safety requirements for the robot itself | Does not qualify the complete workcell/task by itself |
| [ISO 10218-2:2025](https://www.iso.org/standard/73934.html) | Integration/application and robot-cell safety | Not blanket approval for every woodworking tool, lift, or construction activity |
| [ISO 13482:2014](https://committee.iso.org/standard/53820.html) | Published safety reference for covered service/personal-care robots | Not an industrial-cell or medical-robot qualification |
| [ISO/FDIS 13482, Edition 2](https://www.iso.org/standard/83498.html) | Standards-watch item for service robots | As of this review, the ISO catalogue lists it in the approval phase and says it will replace the 2014 edition; do not describe it as a published final edition yet |
| [ISO 18646-2:2024](https://www.iso.org/standard/82643.html) | Ground-contact mobile service-robot navigation: pose accuracy/repeatability, obstacle handling, path deviation, narrow passage, mapping accuracy | Does not cover aerial navigation, manipulation safety, or full mission qualification |
| [ISO 18646-5:2026](https://www.iso.org/standard/86850.html) | Legged-robot locomotion-performance methods | ISO explicitly says it is not intended to verify or validate safety requirements |
| [NIST grasping/manipulation/contact-safety program](https://www.nist.gov/programs-projects/grasping-manipulation-and-contact-safety-performance-robotic-systems) | Measurable manipulation outcomes and test artifacts | Research/measurement work, not a universal safety certification |
| [NASA-STD-7009B](https://standards.nasa.gov/standard/NASA/NASA-STD-7009) and [NASA-HDBK-7009B (2026)](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009) | Credibility assessment and disciplined use of simulation in consequential decisions | Does not automatically establish credibility of Symtropy's present model |

These references are complementary, not one combined certification recipe. In particular, ISO 18646-5:2026 is a performance reference—not safety validation.

## 3. Minimum benchmark dimensions by platform

| Profile | Performance metrics | Non-tradeable gates |
|---|---|---|
| **P0 aerial Forest Scout** | Required-feature observation completeness; pose/feature uncertainty; revisit yield; coverage per energy/minute; recovery availability | Site/airspace permission, geofence, weather envelope, wildlife stand-off, privacy, link-loss disposition; no unauthorized contact or payload release |
| **Ground survey/logistics rover** | Pose error/repeatability; path deviation; verified target/area completeness; energy efficiency; recovery success | Traversability evidence, payload/stability envelope, localization confidence, braking/recovery envelope, return reserve, obstacle exclusion, stale-map/link-loss response |
| **Metrology/woodworking cell** | Dimensional error distribution; first-pass yield; fixture repeatability; rework/material damage; qualified output per total cycle | Correct tool-machine-task pairing, current calibration, fixture/load confirmation, guarding/extraction/interlock prerequisites, human authorization; block motion/cutting if required evidence is absent |
| **Timber handling/positioning** | Alignment error; presentation time; interventions; recovery after aborted alignment | Qualified load/rigging plan, known support/fixture, load/stability limits, exclusion zones and human lift approval; no implicit structural or lifting authority |
| **Reuse/remanufacture assessment** | Defect detection performance on labelled cases; uncertainty calibration; usable-material yield; inspection cost | Unknown defects, hidden fasteners, and grade remain unknown; permission/habitat/biosecurity gates remain explicit; scan geometry alone never authorizes structural reuse |

Keep incompatible service units separate. Do not rank an aerial observer and a manipulator by one weighted score; compare them only on genuinely shared tasks or resource dimensions.

## 4. Fault-to-disposition benchmark

Score the complete episode, not just whether a planner returns a path. Each trace should bind the platform/tool/controller profile, world and calibration identity, evidence freshness, authorization, uncertainty, resource budget, intervention state, and terminal disposition.

Inject each fault with a paired benign control:

- stale/wrong calibration, tool or payload profile after planning;
- changed world, obstacle, coordinate frame or exclusion zone after plan generation;
- declining localization/perception confidence beyond the declared operating envelope;
- communications loss, controller timeout, missing actuator acknowledgement;
- battery/energy reserve falling below return or safe-completion requirements;
- human decline, open interlock, protective-stop/E-stop request, attempted resume;
- duplicate, reordered, stale or replayed completion/command events; receipt mismatch;
- missing/contradictory sensor evidence and out-of-domain tasks;
- a simulator stop being misreported as physical stop performance.

Freeze one expected disposition per case: **complete**, **degraded-with-explicit-limits**, **blocked**, **safe-recovery-required**, or **indeterminate**. Blocking can be correct behavior. Unsafe continuation and falsely claimed completion are critical failures.

Never interpret a simulator tick as physical E-stop timing. Synthetic tests can check modeled software transitions only. Physical stopping time/distance must be measured for a specified speed, payload, tool, surface, slope, controller, hardware configuration, and calibrated measurement setup.

## 5. Proposed acceptance targets (not achieved results)

| Metric | Prospective target | Required reporting |
|---|---:|---|
| Forbidden hard-gate escapes in frozen synthetic cases | 0 | Every escape blocks qualification; publish counterexample |
| Detection of declared critical software mutations | 100% | Per-invariant result, fault operator, paired control |
| False blocks on labelled benign synthetic controls | 0% in the frozen suite | Report corpus size and disagreements; do not generalize to field false-alarm rates |
| Receipt/claim-boundary tamper detection | 100% | Bind source, corpus, config, model, executable and outputs |
| Replay repeatability for declared deterministic runs | 100% | State whether semantic or byte-identical replay is required |
| Coverage of declared critical invariants | 100% | Positive/negative controls, missingness and out-of-domain cases |
| Critical-counterexample disclosure | 100% | Per-case and worst-case results; do not average defects away |
| Physical stop/force behavior | No universal target yet | Preregister task-specific criteria, instrumentation, uncertainty and environment before HIL/field work |

These are engineering targets, not measured results or universal industry acceptance thresholds. Zero failures in a finite synthetic suite does not imply zero real-world risk. Set speed, energy, precision and throughput targets only after baseline measurements on pinned hardware and a declared operating envelope.

## 6. Fair comparison against industry

A superiority claim is admissible only for the same task class and comparable protocol. Freeze baseline version, hardware, tool, load, environment, sensor assumptions, human involvement, run budget and score definition. Include a transparent baseline, a relevant standardized test method where available, and an independent checker for safety-critical outcomes.

Publish qualified task completion, blocked/degraded/indeterminate counts, per-case and worst-case results, error distributions, energy and total cycle cost (setup, calibration, cleanup, rework, recovery), interventions, safe dispositions, false-safe outcomes, held-out performance, sensor/calibration uncertainty, and simulation-to-bench discrepancy. Preserve critical counterexamples and exact source/config/corpus/build identity. Prefer Pareto trade-offs over a composite score that can hide a safety failure behind speed or cost.

## 7. Evidence ladder

1. **Synthetic contract:** deterministic schema, lifecycle and fault-mutation tests; frozen input and expected outcome.
2. **Independent oracle:** separately authored expected-outcome model/specification, held-out mutations, paired controls, all disagreements retained.
3. **Simulation credibility:** intended-use statement, assumptions, sensitivity, uncertainty, calibration and validation evidence under a project-specific credibility plan.
4. **Bench/HIL:** qualified hardware, measured sensing/actuation latency, relevant stopping/force tests, fault injection, calibrated instruments and independent review.
5. **Controlled field:** permissions, task-specific hazard controls, qualified operators, declared operating envelope, incident/near-miss reporting and rollback.
6. **Conformity/deployment review:** applicable standards and regulations assessed for the actual system and jurisdiction.

Each gate binds an exact configuration. Changes to morphology, payload, tool, controller, braking, firmware, calibration, or operating domain require an explicit qualification-impact decision; identical product names do not preserve qualification automatically.

## 8. Immediate implementation sequence

1. Preserve the existing 29-scenario corpus as a versioned baseline; never regenerate expected outcomes from the candidate under test.
2. Add evidence class and standard-reference metadata to reports without asserting compliance merely because a standard is linked.
3. Add paired benign/fault controls and a held-out split whose generation is not controlled by the candidate planner.
4. Report safety disposition, task performance and simulation credibility separately.
5. Proceed to bench/HIL only after the independent oracle and exact-head receipts pass, with qualified robotics/safety engineers. This research slice grants no motor, flight, cutting, lifting or dispatch authority.

## References

- ISO 10218-1:2025 — https://www.iso.org/standard/73933.html
- ISO 10218-2:2025 — https://www.iso.org/standard/73934.html
- ISO 13482:2014 — https://committee.iso.org/standard/53820.html
- ISO/FDIS 13482 (approval-stage status) — https://www.iso.org/standard/83498.html
- ISO 18646-2:2024 — https://www.iso.org/standard/82643.html
- ISO 18646-5:2026 — https://www.iso.org/standard/86850.html
- NIST robotics measurement program — https://www.nist.gov/programs-projects/grasping-manipulation-and-contact-safety-performance-robotic-systems
- NASA-STD-7009B — https://standards.nasa.gov/standard/NASA/NASA-STD-7009
- NASA-HDBK-7009B (2026) — https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009
