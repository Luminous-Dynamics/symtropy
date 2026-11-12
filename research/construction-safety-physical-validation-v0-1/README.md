# Construction Safety Physical Validation V0.1

**Status:** research protocol / draft qualification specification  
**Repository:** public `Luminous-Dynamics/symtropy`  
**Issue:** [CONSTRUCTION-ROBOT-SAFETY-003 / #3505](https://github.com/Luminous-Dynamics/symtropy/issues/3505)

This document specifies how to progress from synthetic construction-robot safety contracts to independently measured physical evidence. It is deliberately documentation-first. It does not implement a safety function, certify a machine, authorize work, or claim that any robot is safe.

## 1. Research objective

The benchmark asks:

> What has actually been measured, on which machine and operating domain, against what independent reference, with what uncertainty—and what remains unknown?

The required unit of evaluation is not a frame, a model score or a successful simulation. It is a **revision-bound physical encounter**:

`machine + as-built + tool + payload + task + site + ODD + safety configuration + sensing + reference metrology + trial protocol`

A measurement without these bindings is not a transferable qualification result.

### 1.1 Compose existing owners

| Owner | Existing responsibility | Physical-validation relationship |
|---|---|---|
| [#3317](https://github.com/Luminous-Dynamics/symtropy/pull/3317) | Snapshot safety-envelope evaluator; evidence-bound operating state and spatial coverage | Consumes a measured encounter as one source of evidence; does not establish physical detection or stopping performance |
| [#3499](https://github.com/Luminous-Dynamics/symtropy/pull/3499) | Event-time reduction and per-cell temporal evidence | Supplies event chronology; a valid chronology is not proof the sensor observed the physical world |
| [#2672](https://github.com/Luminous-Dynamics/symtropy/issues/2672) | Mission-level fault-to-disposition oracle | Consumes mission evidence and keeps physical conformity unevaluated until justified |
| [#1595](https://github.com/Luminous-Dynamics/symtropy/issues/1595) | Shared scenario identity, world revisions and fault schedules | Supplies portable scenario IDs and declared observables |
| [#1172](https://github.com/Luminous-Dynamics/symtropy/issues/1172) | Deterministic scenario execution and replay identity | Supplies reproducibility metadata, not physical truth |
| [#1834](https://github.com/Luminous-Dynamics/symtropy/issues/1834) | Machine/work-package capability matching | Binds the exact tool, material, reach, load and task envelope |

This programme owns the **physical test protocol, metrology qualification, ODD evidence matrix and external adjudication boundary**. It must not create a second simulator, mission evaluator, event bus, event store, construction ledger or safety authority.

## 2. Paradigm shift: evidence provenance is not evidence validity

A source digest proves only that bytes can be identified. A timestamp does not prove clock accuracy. Two sensor labels do not prove independent fault domains. A deterministic replay proves repeatability under the replay contract, not physical validity. A vendor performance statement is not an independently reproduced result.

Each claim therefore carries two separate descriptions:

1. **Provenance class** — where the evidence came from.
2. **Admissibility and limitations** — whether the evidence can support this particular claim, under this ODD and measurement uncertainty.

### 2.1 Evidence classes

| Class | Meaning | Claims it may support |
|---|---|---|
| `Synthetic` | Authored case with declared expected result | Software-contract behavior only |
| `Simulated` | Output from a named simulation/model and declared fidelity | Model-conditional behavior; never physical truth by itself |
| `BenchMeasured` | Instrumented physical rig or surrogate test | Measured bench behavior within the tested setup |
| `HILMeasured` | Actual hardware/software components exercised against a controlled rig | Component/interface behavior under the tested integration |
| `FieldObserved` | Supervised observations on a declared site | Observations within that site's measured conditions |
| `IndependentlyAdjudicated` | Evidence reviewed by an assessor independent of the evaluated controller/vendor | Only the specifically adjudicated claim and scope |

These are not ordinal scores. An independent review of a weak measurement does not make it strong; a bench result does not become field evidence; a field observation does not become a certification.

### 2.2 Result states

Use explicit states rather than a binary PASS:

- `MeasuredWithinDeclaredCriterion`
- `MeasuredOutsideDeclaredCriterion`
- `BlockedBySafetyGate`
- `Indeterminate`
- `NotEvaluated`
- `InvalidEvidence`
- `UnsupportedODD`

Do not convert missing measurements, failed reference instrumentation, excluded trials, unknown conditions or out-of-domain runs into success. Keep the reason and denominator impact.

## 3. Physical ground-truth contract

### 3.1 Independent reference system

The reference system must measure the actual physical quantities used to adjudicate the claim. Depending on task and site, candidates include calibrated optical motion capture, surveyed total-station measurements, high-speed reference cameras, independent position/velocity encoders, calibrated force or load instrumentation, and independently surveyed geometry.

No single instrument is prescribed for every construction task. The protocol must justify the chosen reference system, its coverage, uncertainty, sampling rate and failure modes.

At minimum, characterize:

- robot base pose and velocity;
- tool/end-effector pose and swept volume;
- target/mannequin trajectory and geometry;
- fixed and temporary hazard boundaries;
- brake command, brake onset and actual rest state;
- coordinate-frame transforms and their uncertainty;
- clock synchronization and bounded timestamp error.

The reference must not depend on the robot's own perception output. It must not share the relevant sensing, power, network, time source or processing failure domain with the evaluated evidence path. Where complete independence is impossible, disclose the shared dependencies and downgrade the claim.

### 3.2 Metrology suitability

Before testing, publish the reference instrument's calibration status, traceability, uncertainty budget, operating range, sampling characteristics, synchronization method and known exclusions.

A reference result is admissible only when it can resolve the decision being tested. As an initial **protocol hypothesis**, require expanded reference uncertainty (U_{95}) no greater than one-third of the smallest asserted decision margin. This is a proposed study gate, not a requirement quoted from an ISO standard. A qualified metrologist and responsible safety engineer must approve the actual budget for the measurement.

Likewise, time-base uncertainty must be materially smaller than the smallest latency distinction claimed. Where reference uncertainty overlaps both the pass and fail regions, return `NotEvaluated` or `Indeterminate`.

### 3.3 Identity and calibration binding

Each trial must bind:

- manufacturer, platform/model, serial or privacy-preserving stable asset identifier;
- as-built and modification revision;
- tool/end-effector, attachment and payload/ballast;
- firmware, controller, safety configuration and enabled protective devices;
- sensor identifiers, firmware, calibration certificate/revision and calibration epoch;
- brake/drive state, maintenance condition and relevant thermal state;
- task, work-package and material revision;
- site snapshot, coordinate frame and geometry source;
- applicable jurisdiction, standards profile and ODD profile;
- test protocol, evaluator/oracle version, reference instrumentation and assessor identity.

A changed brake, tool, firmware, sensor, calibration, payload envelope, machine configuration or ODD can invalidate transfer of previous results. Requalification is a declared decision—not an automatic inheritance.

## 4. Operational design domain (ODD) matrix

ODD labels must resolve to measured or explicitly bounded conditions. A case described only as `dusty`, `wet` or `low light` is not reproducible.

### 4.1 Dimensions to characterize

| Dimension | Example strata / measurements |
|---|---|
| Task | Survey/layout; obstacle-aware navigation; material transport; inspection; placement; excavation; lifting; drilling/cutting; masonry assembly |
| Visibility | Illuminance, glare direction, shadow, contrast, dust concentration/particle distribution, water or mud on optics, occlusion geometry |
| Weather | Precipitation, ambient temperature, wind where relevant, humidity and transition conditions |
| Contact/terrain | Surface material, wet/dry condition, slope, cross-slope, roughness, traction/friction measurement and transitions |
| Dynamic site | People/vehicles/plant, temporary barriers, moving obstacles, site changes, map age, work-zone geometry changes |
| Human surrogate | Height/shape, standing/crouching/kneeling/low-profile posture, crossing/approaching/retreating path, carried object, single/multiple targets |
| Equipment state | Payload, speed/mode, brake temperature, battery state, sensor health/calibration, tool wear, suspension/track/tire state |
| Communications/time | Delay, jitter, packet loss, disconnect/reconnect, clock offset/drift, stale/reordered/duplicated event cases |

Use representative ranges from the intended task/site risk analysis. Do not choose arbitrary universal dust, rain, friction or temperature thresholds and then imply they qualify every machine.

### 4.2 Fault campaigns

Include isolated and selected combined faults:

- degraded or misaligned calibration;
- dirt, dust, mud, rain or glare obscuring sensors;
- lidar/camera/radar/ultrasound modality-specific failure;
- clock drift, timestamp discontinuity and bounded clock uncertainty;
- delayed, dropped, duplicated, reordered, replayed or conflicting events;
- stale coordinate frame, map, tool or site snapshot;
- localization degradation or disagreement between independent sources;
- communications partition, controller restart or power-domain failure;
- changed payload, tool, brake state or surface traction;
- reduced braking performance and actual stopping-distance deviation;
- obstruction of a required view or a hazard-zone cell;
- human track lost, split, merged or assigned an unsupported state.

Every critical fault needs a paired **benign control** in which the hazard precondition is absent. This measures overblocking and ensures the benchmark does not reward a system that simply stops for every input.

### 4.3 Human surrogates and human subjects

Use instrumented mannequins, articulated surrogates and controlled target rigs for initial physical campaigns. Exercise low-profile and partially occluded target geometries; a standing high-visibility mannequin alone is not representative coverage.

Human-subject experiments are a separate stage requiring appropriate ethics review, informed consent, task-specific risk assessment, physical protective safeguards and independent supervision. Never expose people to intentional collision, braking-limit or hazardous-stop tests. The research evaluator must not control the robot. Minimize collection and retention of identifiable imagery; face recognition or biometric identification is not required by this benchmark.

## 5. Physical metrics

All metrics are reported per encounter, machine/configuration, task and ODD stratum. Preserve raw case results and tails.

### 5.1 Detection and tracking

- encounter-level true/false detections and misses;
- time-to-detect from independently measured entry into the declared detection region;
- track continuity, track loss and recovery;
- target position/velocity residual against the reference;
- per-cell spatial coverage and per-cell maximum temporal observation gap;
- performance by posture, occlusion, lighting, dust/water and target multiplicity.

Frame-level classification accuracy is supplementary only. Thousands of video frames from one encounter are not thousands of independent encounters.

### 5.2 Motion and stopping

Measure separately:

1. physical target/hazard event time from the independent reference;
2. sensor observation and source timestamp;
3. decision/evaluator timestamp;
4. brake or protective-stop request;
5. physical brake onset;
6. measured zero-motion/rest condition.

Report source-to-decision, decision-to-brake, and brake-to-rest intervals separately. Measure stopping distance using independent pose/velocity data at declared speed, payload, direction, slope, contact surface, mode, braking/thermal state and relevant degradation.

Do not substitute nominal data-sheet braking distance for measured stopping distance. Preserve slip, skid, bounce, tool overhang, trailing attachments and swept volume.

### 5.3 Separation and clearance

Compute the minimum clearance between independently measured target geometry/trajectory and the robot-plus-tool swept volume. Include uncertainty bounds for both. Report minimum clearance, time-to-contact where meaningful, worst observed encounter and the margin to the declared criterion.

The reference trajectory and the evaluated detector's own track must remain separate data streams. Never grade a detector using its own unverified track as ground truth.

### 5.4 Human factors and interventions

Measure operationally observable quantities: operator warning acknowledgement, intervention latency, takeover success, handover/recovery time, alarm frequency, alarm comprehension and task interruption. Workload questionnaires (for example, NASA-TLX) are optional research instruments subject to suitability and consent; they do not replace objective task or safety measures. Do not infer emotion, cognition or worker intent from camera or telemetry data.

### 5.5 Productivity and quality

After safety hard-gates are reported, measure first-pass acceptance, placement/layout residuals, rework, task cycle time, setup, maintenance, interventions, useful output per shift and crew-hours per accepted unit. Match the functional unit to the task. Do not compare blocks/hour, piles/day and printed-line tolerance as if they were one performance measure.

## 6. Statistical design and blind evaluation

### 6.1 Denominators

Define the encounter before collecting data. Publish:
- number of independent encounters and number of independent sites/configurations;
- number of trials per stratum and per fault/control pair;
- exclusions, missing reference data and invalid trials;
- false-negative and false-positive counts;
- confidence intervals and the sampling model;
- repeated-measures and clustering structure.

A replayed recording or repeated frame is not a new independent trial. If trials share a site, weather event, worker, recording, firmware build or calibration, account for clustering and do not use the raw trial count as the effective independent sample size.

### 6.2 Zero observed misses is not zero risk

For (n) independent Bernoulli encounters with zero observed misses, the one-sided exact 95% upper bound on the miss probability is:

`p_upper = 1 - 0.05^(1/n)`

For illustration, zero misses in 3,000 independent encounters yields an upper bound of approximately 0.10%. This does not prove zero risk, does not cover untested ODD strata, and is not a universal acceptance threshold. Applicable risk acceptance criteria must come from the machine-specific risk assessment, required standard and competent safety authority.

Do not pool a strong easy stratum with a weak difficult stratum to manufacture a good overall number. Report the per-stratum result, including cases with too little data to estimate reliably.

### 6.3 Development, public holdout and blind holdout

- **Development:** may be used for implementation and tuning.
- **Public held-out-by-generation:** protocol/cases are withheld from tuning by process, but all bytes/labels are public. Do not call it blind.
- **Blind independent holdout:** an independent custodian retains raw inputs and labels until source, evaluator, protocol and output commitments are frozen.

Split by site, robot family/configuration, environment and source recording. Do not randomly split frames from the same clip, encounter or scene across development and test. Freeze protocol, adjudication, denominator, metrics and exclusion rules before unblinding.

## 7. Baselines and fair industry comparison

Use identical hardware/configuration, task, payload, site conditions, reference instrumentation and adjudication:

| Baseline | Description | Comparison question |
|---|---|---|
| P0 | OEM safeguarded configuration and documented operating envelope | What does the deployed protective configuration do under the same test? |
| P1 | Fixed conservative distance boundary | What is gained/lost by a simple static rule? |
| P2 | #3317 mode-aware evidence-bound snapshot evaluator | Does richer state binding reduce unsafe classifications without unacceptable nuisance stops? |
| P3 | #3499 event-time and temporal per-cell evidence reducer | Does temporal handling catch replay, stale evidence and blind intervals? |
| P4 | #2672 mission-level independent oracle | Are whole-mission faults and terminal claims classified correctly? |
| Referee | Independent physical reference / external assessor | What actually happened, within measured uncertainty? |

The referee is not another candidate controller. Keep software correctness, physical sensing performance, braking performance, task quality and human factors as separate scorecards. Publish Pareto comparisons rather than one composite robot score.

### 7.1 Vendor and programme references

Existing references in #3317 include HP SitePrint, Dusty FieldPrinter 2, Built Robotics/ARENA and FBR Hadrian. Their reported figures concern different tasks and are vendor/programme statements. They are useful for designing task-specific productivity and quality questions, not matched safety benchmarks.

For each public comparator, record:
- task and functional unit;
- hardware/tool/configuration and payload;
- site and environmental conditions;
- stated measurement method, uncertainty and acceptance criterion;
- first-pass quality and rework if published;
- safety functions/standards actually documented;
- detection coverage, stopping distance and intervention rate only when published or independently measured.

Use `Not publicly disclosed` for missing fields. Do not infer a vendor's safety performance from its productivity or positioning tolerance.

## 8. Standards and measurement-science applicability

Select standards by actual robot type, use and jurisdiction. This is a scoping aid, not a compliance assessment.

| Reference | Potential relevance | Boundary to preserve |
|---|---|---|
| ISO 19014-1 to -5:2026 | Functional safety of safety-related control systems on in-scope earth-moving machinery defined by ISO 6165; MPLr/MPLa, environmental, software/data and required-level material | Not a universal construction-robot standard; MPLr/MPLa are not ISO 13849 PLr/PL or IEC 61508 SIL |
| ISO 17757:2019 | Autonomous/semi-autonomous earth-moving and mobile-mining systems | Apply only within its scope and machine context |
| ISO 10218-1/-2:2025 | Industrial robots and industrial robot applications/cells | Check scope and exclusions for mobile platforms, underground systems and the actual application |
| ISO 3691-4:2023 | Driverless industrial trucks and their systems | Do not apply automatically to every construction mobile robot; the third edition is under development, not a published replacement |
| ANSI/A3 R15.08-1-2020 (R2026) / R15.08-2:2023 | Industrial mobile robot manufacturer and application/integration considerations | Confirm applicability to the exact machine/application |
| AMRA-201:2026 | Mobile robot general requirements and test methods on solid travel surfaces | A measurement reference, not a substitute for functional-safety risk assessment |
| ISO 12100:2010 / ISO 13849-1:2023 | Machinery risk assessment/design principles and safety-related control-system design | Requires machine-specific hazard analysis, architecture and validation; this protocol calculates no PL/SIL |
| NIST mobility performance programme | Mobile-robot measurement methods, adaptability, efficiency, completion quality and assurance | Research measurement programme; not a machine-specific certificate |

Official and research starting points:
- [ISO 19014 series](https://www.iso.org/search.html?q=ISO%2019014)
- [ISO 17757:2019](https://www.iso.org/search.html?q=ISO%2017757)
- [ISO 10218:2025](https://www.iso.org/search.html?q=ISO%2010218)
- [ISO 3691-4:2023](https://www.iso.org/search.html?q=ISO%203691-4)
- [ANSI/A3 R15.08](https://www.automate.org/robotics/standards)
- [AMRA standards](https://www.amra-standards.org/)
- [NIST robotics programme](https://www.nist.gov/programs-projects/robotics)
- Wang & Xu (2026), [Toward intelligent safety assurance in construction human-robot collaboration](https://doi.org/10.1016/j.autcon.2026.107288).
- 2026 review of human factors in construction HRC, [Journal of Safety Research](https://doi.org/10.1016/j.jsr.2026.08.009).

Wang & Xu reviewed 215 studies and reported that 54% remained at controlled-laboratory validation, with limited integration of uncertainty across sensing, prediction, control and supervision. This motivates the staged bridge; it does not establish that any specific product or standard is deficient. NIST's measurement work is a useful complement for mobile-robot capability and task metrics.

## 9. Benchmark receipt: minimum data contract

A physical trial receipt should be machine-readable and versioned. This example is illustrative, not a finalized schema:

```json
{
  "schema": "construction-robot-physical-trial/v0.1",
  "trial_id": "site-a-surrogate-crossing-0001",
  "claim": {
    "id": "human-detection-and-stop-envelope",
    "ceiling": "bench-measured",
    "result": "not-evaluated"
  },
  "subject": {
    "platform_revision": "declared-revision",
    "tool_revision": "declared-tool",
    "payload_kg": 0,
    "firmware_revision": "declared-build",
    "safety_configuration_digest": "sha256:..."
  },
  "task": {
    "scenario_id": "shared-scenario-id",
    "work_package_revision": "declared-revision",
    "site_snapshot_digest": "sha256:...",
    "coordinate_frame": "site-local"
  },
  "odd": {
    "profile_revision": "odd-rev-1",
    "stratum_ids": ["indoor", "low-illumination", "partial-occlusion"],
    "measurements": []
  },
  "reference": {
    "instrument_ids": [],
    "calibration_records": [],
    "expanded_uncertainty": [],
    "clock_sync_method": "declared-method",
    "maximum_clock_error_ns": null,
    "independence_review": "pending"
  },
  "evidence": {
    "raw_record_ids": [],
    "raw_byte_digests": [],
    "sensor_calibration_revisions": [],
    "fault_schedule_digest": "sha256:...",
    "protocol_digest": "sha256:..."
  },
  "metrics": {
    "detection": {},
    "latency": {},
    "stopping": {},
    "clearance": {},
    "interventions": {},
    "task_quality": {}
  },
  "adjudication": {
    "assessor_role": "independent",
    "decision": "not-evaluated",
    "reasons": ["reference-calibration-not-reviewed"]
  }
}
```

The schema must eventually define units, numeric encodings, null semantics, canonical serialization, timestamp clock domains, digest scope and trust verification. A digest string by itself is not a signature, calibration certificate or trusted source assertion. Raw evidence custody, issuer identity and signature verification belong to the designated authoritative evidence/provenance path; do not duplicate it here.

## 10. Qualification stages and promotion rules

| Stage | Work | Exit evidence | Does not establish |
|---|---|---|---|
| 003A Protocol | ODD schema, trial/receipt contract, metrology plan, fault/control pairs, frozen analysis plan | Reviewed protocol and independent adjudication rules | Physical performance |
| 003B Surrogate bench | Calibrated reference geometry/time, mannequin/target rigs, braking/trajectory measurements | Repeatable bench dataset with uncertainty budget | Human-safe field operation |
| 003C HIL | Real hardware/software components and bounded injected faults on isolated rigs | Exact configuration and measured component/interface results | Whole-machine field safety |
| 003D Shadow operation | Supervised observation with research evaluator unable to control the robot | Matched predictions vs independent field observations | Permission to control or operate autonomously |
| 003E Controlled field | Externally reviewed, site-approved, safeguarded low-consequence tasks | Independent, scoped trial report | General certification or universal ODD coverage |
| 003F Replication | Held-out sites/configurations and requalification after material changes | Independent replication and change-impact record | Transfer beyond tested scope |

Promotion is claim-specific. An unsuccessful or incomplete stage remains visible. A later stage requires separate approval and cannot be inferred from a successful prior stage.

## 11. Acceptance gates and reporting

Targets below are prospective protocol requirements, not achieved results:

1. Every result binds exact hardware/software/tool/site/ODD/reference revisions and raw evidence.
2. The independent reference passes its metrology and time-sync suitability gate before the encounter is scored.
3. All seeded critical mutations remain detected in the synthetic software suites.
4. No false-safe on the frozen physical challenge set; publish per-stratum denominators and confidence bounds.
5. Unsupported/unmeasured ODD conditions remain unsupported or NotEvaluated.
6. Critical outcomes have independent ground-truth adjudication and retained counterexamples.
7. Benign controls measure false stops and unnecessary holds.
8. Results retain null/unknown/invalid/indeterminate cases; no silent omission or denominator manipulation.
9. Material configuration changes trigger an explicit requalification decision.
10. Productivity and quality cannot offset a safety hard-gate failure.

A benchmark report must publish raw per-case outcomes or an independently reviewable protected-data access path, exact corpus/protocol/evaluator identities, hardware and calibration, environmental strata, exclusions, statistical bounds, uncertainty budget, all failures, and claim ceiling. If privacy or commercial restrictions prevent full raw data release, publish enough metadata and an independent audit path to make the limitation explicit.

## 12. First execution sequence

1. Freeze 003A protocol and case taxonomy; link each scenario ID to #1595 rather than inventing a second scenario namespace.
2. Select one low-consequence task and one exact robot/tool configuration. Do not start with a universal construction-robot campaign.
3. Design reference metrology and uncertainty budget with a qualified measurement specialist.
4. Build only a surrogate/bench setup; validate the reference system itself before evaluating the robot.
5. Pre-register the ODD strata, faults, paired benign controls, sample plan and adjudication rules.
6. Compare OEM safeguarded operation and the research evaluators only in shadow/bench conditions; do not let this research package control the robot.
7. Publish all failures and unknowns; decide on the next stage through an independent review.

## 13. Limits

This research package is not a safety function or controller. It cannot issue commands, stop or reset machinery, authorize a task, certify a robot, calculate MPLr/MPLa/PLr/PL/SIL, or replace qualified engineering and conformity assessment. Human-subject tests require separate approval and protections. Synthetic agreement, deterministic replay, a benchmark score, and a digest do not prove physical safety.

The meaningful result is not a larger number of synthetic cases. It is a traceable, independently adjudicated measurement whose scope and uncertainty are narrow enough that another laboratory can reproduce or falsify it.
