# CIV-INFRA Fleet Resilience Benchmark v0.1

**Status:** Proposed benchmark contract and synthetic corpus only; not an executed benchmark.  
**Tracking issue:** [CIV-INFRA-FLEET-RESILIENCE-001 #1843](https://github.com/Luminous-Dynamics/symtropy/issues/1843).  
**Related capability contract:** [CIV-INFRA Robot Capability Matching v0.1](../civ-infra-robot/CAPABILITY_CONTRACT_V0_1.md).  
**Synthetic fixture:** [fleet-resilience-v0.1.json](fixtures/fleet-resilience-v0.1.json).

## 1. The paradigm shift

A construction plan must optimize **verified functional output over the full task lifecycle**, not nominal robot movements or scheduled machine-hours.

`command executed != task completed != inspected output accepted`

Every accepted unit must trace through work-package requirements, the exact capability profile, current site/material state, prerequisite and inspection evidence, and a receipt that says what the result does and does not establish.

A platform that cannot recover from a tool failure, explain its state, or be repaired locally may be less useful than a slower but serviceable system. Therefore repair time, material waste, human intervention, uncertainty and safe stopping are first-class measures—not a postscript to throughput.

This benchmark starts synthetic and deterministic. It provides no real robot actuation authority and establishes no field productivity or construction-safety claim.

## 2. Scope and composition

Compose existing owners; do not establish parallel sources of truth.

- **Symthaea:** design intent, candidate planning, robot design/evidence lineage, uncertainty interpretation and bounded recommendations.
- **Symtropy:** synthetic physical/resource semantics, execution evidence composition and reference benchmark harness.
- **Sol Atlas:** read-only spatial/source projections with explicit provenance/currentness.
- **Mycelix:** authorized organizational commitments, material/work provenance and approval records.
- Existing domain owners continue to own construction lifecycle, hydrology, soil/geotechnical assumptions, site safety and specialist engineering.

Relevant work:
- Capability matcher: [Symtropy #1834 / PR #1835](https://github.com/Luminous-Dynamics/symtropy/pull/1835)
- Civil corridor verifier: [Symtropy #1732 / PR #1756](https://github.com/Luminous-Dynamics/symtropy/pull/1756)
- Specialist hydraulic evidence boundary: [Symtropy #1764](https://github.com/Luminous-Dynamics/symtropy/issues/1764)
- Robotics-design evidence boundaries: [Symthaea #4795](https://github.com/Luminous-Dynamics/symthaea/issues/4795)
- Decision-value-guided investigation: [Symthaea #7727](https://github.com/Luminous-Dynamics/symthaea/issues/7727)
- Modular repairable timber/construction robotics: [Symtropy #1818](https://github.com/Luminous-Dynamics/symtropy/issues/1818)
- Construction approval/provenance: [Mycelix #5149](https://github.com/Luminous-Dynamics/mycelix/issues/5149)

The first host scenarios are a fictional road, trench and utility corridor. The same benchmark methodology may later be profiled for buildings, factories, power, data, water and thermal systems, but task physics, safety requirements and acceptance criteria remain domain-specific.

## 3. Comparable policy lanes

All policies receive identical initial state, task graph, resource budget, evidence, event schedule and evaluation corpus.

- **B0 Fixed schedule:** follow a precommitted schedule with only declared safe-stop and abort behavior.
- **B1 Rule-based recovery:** deterministic substitution/buffer/inspect/retry rules frozen before evaluation.
- **B2 Evidence-bounded replanning:** replan from currently admissible observations and explicit capability envelopes; hard constraints remain hard.
- **B3 Finite-instance oracle:** exhaustive optimum over tiny declared scheduling/recovery state spaces. This is a reference bound, not a deployable planner.
- **B4 Human-operated comparator:** populate only from a genuine matched study. No synthetic human speed assumption may be presented as measured reality.

No policy may access a disturbance before its declared observation time. A hidden simulator state is available to the evaluator/oracle but is not available to the policy until a legitimate observation exposes it.

## 4. Event and state semantics

Each injected event needs a stable ID, exact scenario revision, injection time, visibility/observation time, affected asset/task/resource references, evidence invalidation list, recoverability class and required recovery evidence.

A state transition must preserve:
- old observations as historical records rather than rewriting them;
- currentness and revisions of site, geometry, material, tool, controller and calibration data;
- the distinction between missing, bounded-unknown, unbounded-unknown, stale, conflicting, measured and independently qualified evidence;
- the difference between safe stop, aborted work, incomplete work, failed inspection, repaired work and accepted work;
- explicit ownership for every state mutation.

If a task's site/material/tool/calibration assumptions change, downstream admissions depending on those assumptions must be invalidated or explicitly re-evaluated. A cached approval cannot become current simply because the network is unavailable.

## 5. Frozen disturbance families

The versioned JSON fixture includes cases covering:
- localization loss;
- stale calibration after a tool change;
- delayed communication/telemetry;
- unavailable robot/tool;
- late or incorrect material;
- inspection rejection and rework;
- changed site geometry after scheduling;
- capacity-one resource conflict;
- low-energy recovery infeasibility;
- unbounded unknown underground utility;
- an explicitly repairable failure;
- human approval decline/unavailability;
- a stale approval/capability revision.

These are disturbance definitions and expected invariants, not a claim that a full simulator or planner exists. The executable harness must validate the case schema, references and invariant expectations before evaluating policies.

## 6. Required invariants

1. Required safety, site-access, operator, evidence, resource and domain-engineering gates cannot be soft objectives.
2. Unknown never becomes clear, safe, compatible, zero-cost or complete without new admissible evidence.
3. A safe stop is correct safety behavior but is not counted as productive task completion.
4. Failed inspections remain rejected until required corrective work and reinspection succeed.
5. Task/revision drift invalidates stale assignments and receipts.
6. Unobserved state changes may not be used by the adaptive planner before observation.
7. Time and resource use includes setup, charging, travel, inspection, rework, maintenance and recovery—not only active tool time.
8. Failed, missing, unexecuted or non-converged evaluations are unavailable/unknown, never implicitly zero or pass.
9. Reports include all predeclared cases, including adverse and negative controls.
10. The reference implementation derives expected findings independently from the candidate policy's own status and digest.

## 7. Metrics (do not collapse into one scalar)

### Accepted useful work
- accepted functional units after required verification;
- first-pass acceptance rate and rework fraction;
- escaped-defect count in the declared evaluation scope;
- time to first accepted unit and end-to-end time to acceptance.

### Resilience and recovery
- work/service retained through each disturbance;
- time to detect, safely stop, isolate, diagnose, repair, recalibrate, reinspect and resume;
- P50/P90/P99 and worst-case recovery time where samples justify quantiles;
- aborts, deadlocks, cascading failures, repeated retries and lost-work fraction.

### Resources and maintainability
- total energy/material per accepted functional unit;
- damaged, wasted, recovered and reused material;
- operator attention and intervention count/time;
- tool-change/setup time, spare-part variety and maintenance burden;
- time-to-repair and fraction of faults locally repairable, when genuinely measured.

### Evidence and uncertainty
- false acceptances relative to the finite reference oracle;
- stale revision/calibration admissions;
- unknown-to-clear promotions;
- missing inspection receipts;
- policy decisions made before required observations;
- model prediction residuals compared with physical measurements where available.

Always report per-case dispositions, denominators, source revisions, corpus/config digest, toolchain, scenario seed, unknown count, and claim ceiling. For physical experiments, use appropriate uncertainty intervals. For tiny exhaustively enumerated synthetic cases, report exact finite agreement.

## 8. Proposed C0 goals (not achieved claims)

- 100% of seeded stale-revision, missing-inspection and capacity-one-overcommitment controls are rejected.
- Zero unknown-to-clear transitions without new admissible evidence.
- Zero accepted-complete outcomes without the required inspection chain.
- Zero use of not-yet-observed disturbance information by the evaluated policy.
- Exact agreement with exhaustive enumeration for the explicitly bounded test instances.
- Identical normalized output for identical admitted input bytes, configuration, model/profile revisions and seed.
- No predeclared disturbance or negative control is silently omitted from the report.
- Every policy is compared on the same corpus; failed runs are recorded and not dropped from comparison without an explicit missingness analysis.

Only after the baseline is executed should we preregister practical improvement targets. A future pilot may test whether evidence-bounded replanning lowers time/cost per accepted unit relative to rule-based recovery **without worsening required quality or safety gates**. Do not pick an attractive percent and treat it as evidence.

## 9. Data and external grounding

- [NIST Agility Performance of Robotic Systems](https://www.nist.gov/programs-projects/agility-performance-robotic-systems) discusses repeatable metrics for retasking/reconfiguration, robot performance test methods, information models, datasets and combined virtual/physical testbeds.
- [Hilti–Trimble SLAM Challenge 2026 dataset](https://hilti-challenge.com/dataset-2026) provides construction-site visual-inertial sequences and floor-plan priors for localization evaluation. The perception track should consume only the dataset's documented truth and input fields; check its terms before repackaging or redistribution. Localization scores are not construction-task acceptance.
- [ISO 10218-1:2025](https://www.iso.org/standard/73933.html) covers industrial robot safety requirements, while robot integration and application hazards need suitable system/task analysis. This benchmark is not a standards conformance assessment.
- [OSHA Robotics](https://www.osha.gov/robotics/) notes accident risk during non-routine activities such as setup, maintenance, testing and adjustment.
- [OSHA Trenching and Excavation](https://www.osha.gov/trenching-excavation) emphasizes excavation protection and inspection because conditions can change. The software benchmark cannot replace competent-person inspection or applicable local legal duties.

These are references for benchmark design, not proof that the proposed benchmark or any robot is valid, safe, cost-effective or field-ready.

## 10. Stage gates

1. Schema-valid deterministic event corpus; verify unique/resolving IDs and ensure observation time is not earlier than injection time unless the event is explicitly pre-observed.
2. Independent reference model for tiny schedule/recovery cases.
3. Baseline and policy runs on exactly the same events and resources.
4. Adversarial testing of stale state, event ordering, duplicate events, dropped observations, schedule boundary semantics, missing outputs and forged completion.
5. Add construction-site SLAM data as a separate localization-input track after terms and formats are reviewed.
6. Only after synthetic correctness, define a supervised low-consequence physical pilot with calibrated sensors, safeguarding, competent operators, site permissions and independent review.
7. Compare measured end-to-end accepted output, repairability and cost with equivalent baselines before making real-world productivity claims.

## Non-goals

No live dispatch or motor authority; no structural or hydraulic solver replacement; no claim of building-code compliance; no inference that a synthetic event models every physical failure; no claim of real-world productivity from a simulation; no conversion of consciousness scores, HDC similarity, or planner confidence into authority; no redistribution of third-party datasets without rights review.
