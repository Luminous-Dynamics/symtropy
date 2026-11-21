# Luminous Cybernetics — Capability Frontier and Evidence Contract

Status: programme-level research and product-evaluation contract. It is not a claim that a current prototype has met any of the targets below.

## Thesis

The aim is not to maximize robotic capability in isolation. It is to increase a person's useful capability while reducing the effort, dependence and maintenance required to obtain that benefit, without taking agency away.

A product should move a four-dimensional capability frontier:

1. **Capability:** useful task success, precision, mobility, strength where needed, endurance and functional independence.
2. **Agency:** choice, refusal, cancellation, predictability, transparency, privacy and the ability to operate independently or hand control to a person.
3. **Burden:** physical effort, metabolic cost, cognitive workload, discomfort, weight, setup/don/doff, training and recovery.
4. **Sustainability:** affordability, energy, service life, repair time, calibration burden, spares availability, material efficiency and accessibility.

Safety, dignity, privacy and rights are hard constraints—not score axes that can be traded away.

## 1. Evaluation rule: no universal composite score

Do not combine the four axes into one weighted score. A weighted composite can hide that a device is faster but less reliable, stronger but more uncomfortable, or more autonomous but harder to stop.

Report each metric, its uncertainty, context and user/task stratum. Use Pareto dominance to compare designs:

- Candidate A dominates B only if A is no worse on every declared objective and strictly better on at least one, under matched conditions.
- If one design gains capability but loses agency or sustainability, it is a trade-off, not a universal improvement.
- Any violation of a safety, consent, privacy or accessibility gate makes the candidate ineligible regardless of its other results.

Show the frontier as a set of nondominated configurations rather than a league table with a hidden scalar weight.

## 2. Capability axis

Measure task success and quality at multiple levels.

### Component
- torque/force and velocity envelope;
- precision, repeatability, backlash, stiffness and hysteresis;
- tactile/contact sensing accuracy and uncertainty;
- energy and thermal derating;
- fatigue, impact resilience and ingress performance.

### Device
- POMDAR task correctness and throughput with official protocol/version;
- repeated task success, grasp retention, slip detection and recovery;
- task performance on held-out objects, orientations, materials and contact conditions;
- mobility and manipulation under matched payload/terrain conditions;
- stability, collision response and safe-stop performance.

### Integrated system
- task completion and stage completion;
- perception-to-action latency and source-clock freshness;
- robustness to occlusion, lighting, sensor dropout, clock drift and disturbances;
- coordinated loco-manipulation and whole-body control;
- simulation-to-hardware prediction residuals.

### Human function
- activities of daily living and meaningful occupation selected by users;
- task independence and assistance required;
- performance under fatigue and real environmental variation;
- user-defined goals, not only manufacturer-selected demonstrations.

A robot-only manipulation score cannot be promoted to a claim of improved human function.

## 3. Agency axis

Agency should be measurable and designed into the authority model.

| Metric | Operational definition |
|---|---|
| Refusal compliance | fraction of valid refusals followed without task continuation |
| Revocation latency | time from stop/revoke input to cessation of the relevant action |
| Unauthorized actuation | count of actuator commands lacking current authorization |
| Predictability | agreement between communicated intended action and observed action |
| Override availability | whether a user can pause, stop, disengage or hand off in each operating mode |
| Privacy control | recording/transmission only under purpose-bound, revocable permission |
| Calibration transparency | user can identify current mode, assistance level, confidence and fault state |
| Dependence burden | training, setup, account/cloud, operator and maintenance dependencies required for ordinary use |

Target: zero continuation after a valid refusal or revocation in the declared test matrix; zero unauthorized data transmission; all modes expose a reachable stop/withdrawal path. These are hard acceptance conditions, not a weighted user-experience preference.

## 4. Burden axis

Measure burden rather than assuming that increased motor assistance is inherently beneficial.

### Physical
- metabolic cost under matched speed, terrain, load and task;
- muscle activation and joint loading where valid instrumentation and protocols exist;
- perceived exertion, fatigue and recovery time;
- device mass, pressure distribution, skin-interface movement and discomfort;
- don/doff time, alignment time and daily setup effort.

### Cognitive and operational
- workload and attention demand;
- number of corrective interventions and failed attempts;
- control predictability and learning time;
- number of modes and configuration steps;
- dependence on a clinician, technician, remote operator or network.

A device that improves mechanical task performance while increasing discomfort or cognitive burden is not automatically better.

## 5. Sustainability axis

Track the entire service lifecycle.

- electrical energy at named boundaries (Wh/task, Wh/hour, standby Wh, measured regeneration);
- battery replacement, charging and thermal derating;
- mass and material use;
- component service life and failure modes;
- repair time and cost;
- parts availability and supplier dependency;
- calibration time/frequency and requalification scope;
- software/firmware support lifetime;
- access cost, repairability by local technicians and availability across user environments.

The preferred cost metric is cost per qualified useful hour/task over the expected service life—not purchase price alone.

## 6. Reliability and uncertainty protocol

Every reported rate needs a frozen denominator, task version, configuration, trial-selection rule and confidence interval.

- Report per-task and macro-averaged results; do not let frequent easy tasks hide rare difficult tasks.
- Separate in-distribution and held-out object/environment/user strata.
- Report failed, aborted, timed-out and invalidated trials. Exclusions require a predeclared reason.
- Report calibration and uncertainty, not only point estimates.
- Publish exact hardware, firmware, policy, sensor calibration, model, task fixture and data hashes.
- Use independent negative controls and adversarial/fault-injection cases.
- Preregister acceptance criteria before examining the final evaluation set.

For a simple binomial reliability demonstration with zero observed failures, the approximate one-sided 95% upper bound on failure probability is 3/n (rule of three). Thus 30 failure-free trials only bound the failure probability to roughly 10%; they do not prove 99% reliability. Formal sample sizes must be chosen from the required failure bound, confidence level, task stratification and test cost.

## 7. Benchmark mapping

Use each benchmark only for the property it actually measures.

| Question | Primary reference | Additional evidence |
|---|---|---|
| Anthropomorphic hand dexterity | POMDAR | HAND ERC application/system/hand/component attribution |
| Human daily function | SHAP and user-defined activities | interviews, assistance level, user-reported outcomes |
| Cross-embodiment dexterous policy | HandsOn Core-50 / Transfer-16 | per-skill and held-out transfer diagnostics |
| Bimanual long-horizon policy | Bench2Dex | full 4-channel SR, LSCR and efficiency |
| OOD bimanual generalization | BiDex-100 | instruction, few-shot, long-horizon, memory and OOD variants |
| Humanoid baseline capability | NIST developing baseline | Fraunhofer IPA six-area application assessment |
| Rotary joint | JointLab | exact configuration, metrology and blind sim↔physical correlation |
| Exoskeleton mobility/energy | matched locomotion protocol | calibrated actuator loss maps and independent human outcomes |
| User control and safety | diplomatic interaction/resilience contract | refusal, stop, contact, fault and cyber-resilience test suites |

Simulation, hardware, human factors and clinical evidence remain distinct evidence classes.

## 8. Evidence ladder and claim promotion

Use explicit evidence classes:

- **E0 — concept:** sketches, assumptions and trade studies; no performance claim.
- **E1 — analytical/synthetic:** mathematical or synthetic fixtures; claims limited to the tested oracle and assumptions.
- **E2 — simulation:** a declared model path with exact configuration, convergence, sensitivity and differential comparison. No physical claim.
- **E3 — bench/hardware:** calibrated physical measurements, uncertainty budget, exact serial/configuration and independent acceptance.
- **E4 — supervised human use:** approved protocol, informed consent, safety oversight, adverse-event capture and meaningful user outcomes.
- **E5 — clinical/field evidence:** appropriately designed multi-user studies and post-deployment evidence within the approved intended-use scope.

Do not promote a claim just because the software version changed. A claim advances only when the evidence class and test method are sufficient for its context of use. A new motor, sensor, geometry, material, firmware, control model or calibration creates a scoped impact assessment and may require requalification.

## 9. Competitive target-setting

For each external benchmark:
1. reproduce the official baseline on the same task/fixture/version;
2. freeze training data, demonstration count, compute, action/observation interface, budget, evaluation seeds and timeouts;
3. set a target for both overall performance and the weakest task families;
4. include held-out object, environment, user and fault conditions;
5. compare distributions and confidence intervals;
6. request an independent replication before claiming SOTA.

Initial internal target directions (aspirational, not measured):
- at least 1.5× relative improvement over reproduced task-macro policy baselines on held-out tasks, without a worse weakest-skill floor;
- at least 95% repeated success on a declared high-value manipulation set, with sample size sufficient to bound failure probability at the required confidence;
- 100% refusal/revocation compliance in the declared interaction matrix;
- no safety envelope breach in the declared component/system test matrix;
- measurable reduction in Wh per successful task and maintenance minutes per qualified hour relative to a matched device baseline;
- blind physical response predictions inside a preregistered uncertainty envelope.

These targets are not interchangeable. Failure on any hard safety/agency gate cannot be offset by a better average task score.

## 10. Governance and ownership

- Symtropy owns physical simulation, contact mechanics, deterministic replay and measurement adapters.
- Sol-Atlas owns geometry, reference frames, clearances and spatial metrology provenance.
- Mycelix owns identity, authorization, serialized article/configuration provenance and repair-event references.
- Symthaea may propose configurations, experiments and task plans; it does not authorize hazardous motion or promote its own evidence.
- The local safety supervisor owns actuator permission and safe-state transitions independently of the high-level planner.
- The engineering lifecycle platform owns requirements, revisions, configuration baselines and manufacturing/service records.
- Human participants and users retain authority over goals, consent, refusal and meaningful success criteria.

The purpose is not to produce a robot that is maximally capable at any cost. It is to produce a system whose capability is useful, whose agency is respected, whose burden is low, and whose lifecycle can be afforded, repaired and trusted.
