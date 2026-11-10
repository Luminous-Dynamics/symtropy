# Maritime control-aware diagnosis benchmark v0.1

**Owner:** MARITIME-CONTROL-AWARE-DIAGNOSIS-001 #4628  
**Composes with:** #4444, #4510, #4535, #4609, Symthaea #11185 and Mycelix #5974  
**Status:** protocol and signal-role contract; no engine control dataset or physical validation claimed  
**Date:** 10 November 2026

## Objective

Test whether control intent, controller/actuator state and physical plant response improve marine-engine fault diagnosis when represented as distinct evidence roles.

The key distinction is:

- **Intent:** what the operator/controller requested.
- **Actuation/state:** what the controller and actuator report they did.
- **Response:** what independent physical measurements show the engine actually did.

A setpoint is not an observed engine speed. A requested fuel quantity is not delivered fuel flow. A governor command is not actuator-position feedback. Actuator feedback is not combustion or shaft response.

## Research basis and claim ceiling

Lin et al., *Fault diagnosis of marine diesel engine via control-aware domain-adversarial network: A control-diagnosis collaboration perspective*, first published 24 September 2026, DOI [10.1177/01423312261487500](https://doi.org/10.1177/01423312261487500), proposes control/state feature fusion and a multi-class maneuvering-regime discriminator. The accessible abstract states that experiments use a thermodynamic simulation model and that real-engine validation is absent. The abstract claims improved robustness when control sensors drop out but supplies no independently reproduced physical-engine result.

This benchmark treats the paper as a simulation reproduction lead. It must not be described as real-engine SOTA.

## Signal-role model

Each signal has exactly one primary semantic role:

| Role | Examples | What it can establish |
|---|---|---|
| CONTROL_INTENT | speed/load/torque setpoint; requested fuel/air command | Requested target or command only |
| CONTROLLER_STATE | selected mode; limiter state; interlock; controller output | Declared controller state/output only |
| ACTUATOR_FEEDBACK | measured rack/valve/actuator position; drive feedback | Physical actuator-position evidence within its own calibration |
| PHYSICAL_RESPONSE | independently measured speed, pressure, temperature, flow, torque, vibration | Plant response within sensor and configuration validity |
| DISTURBANCE_CONTEXT | ambient conditions; sea/load context; fuel quality; external load | Context covariate, not actuator or plant proof |
| DERIVED_RESPONSE | model residual; estimated load; computed feature | Derived evidence with full parent lineage; not an independent sensor |

Role is assigned from an instrument/I-O register and engineering review, not guessed from the variable name. An absent or semantically ambiguous signal is NOT_EVALUATED.

## Command-conditioned response

Learn or specify the response envelope conditional on:

- exact engine, controller and installed configuration;
- prior command/setpoint history and controller mode available at prediction time;
- actuator feedback and declared response lag;
- load, ambient and disturbance context;
- measurement quality, calibration, units, clock domains and synchronization uncertainty.

The residual is evaluated against a training-only or independently qualified response envelope. It must retain model uncertainty, operating-domain applicability and timing uncertainty. A command-response mismatch is a candidate anomaly, not automatic isolation of an actuator fault. Alternative explanations include a biased physical sensor, unmodelled load, controller mode change, saturation, stale command, communication delay, configuration mismatch or unsupported regime.

Do not use post-event controller state, future setpoints, operator actions or maintenance outcomes as pre-event features.

## Matched model tournament

Compare on the same fault taxonomy, run grouping, operating domains, input availability, model capacity and compute budget:

1. state-only classifier / residual baseline;
2. control-only baseline;
3. naive control + state concatenation;
4. gated/weighted control-state fusion;
5. control-aware domain-adversarial model with multi-class maneuvering-regime discrimination;
6. binary source/target domain-adversarial baseline;
7. physics/rule-based command-response residual;
8. calibrated selective predictor with explicit unknown/abstain states.

Reproduce the 2026 study under its own simulation protocol before claiming replication. Recover its exact training/test split, regime definitions, fault labels, dropout patterns, parameter count and metric table. Then run selected baselines on common whole-run and held-out-regime splits.

## Required benchmark tracks

### A. Signal-role and provenance audit

Reconcile signal roles to a controller I/O map, sensor register, acquisition path and configuration revision. Record units, ranges, calibration, clocks, latency, sampling rate, hold semantics, shared dependencies and derived parentage. If a role cannot be established, preserve UNKNOWN/NOT_EVALUATED.

### B. Counterfactual discrimination

Include matched cases:

- same command, different physical response;
- same physical response, different command;
- correct response under changing maneuver;
- actuator lag or saturation with otherwise valid plant response;
- actuator feedback normal but plant response abnormal;
- sensor bias that mimics under-actuation;
- load/ambient disturbance that explains response variation;
- controller-mode change that changes the expected response envelope.

### C. Signal dropout and common causes

Remove individual control-intent, controller-state, actuator-feedback and physical-response channels. Remove shared controller power, gateway, clock and communication dependencies. Recompute qualification after each outage. Missing command, missing feedback and missing physical response are different states and must not be silently imputed as normal.

### D. Transfer and continual qualification

Send signal-role and maneuvering-regime metadata to #4510. Report within-engine held-out regime results separately from whole-engine transfer. After each model update, re-evaluate all previous regimes, fault classes, residual calibration and channel-dropout scenarios.

### E. Engineering evidence handoff

The output is read-only evidence for #4444 and downstream engineering owners. It may report a candidate response mismatch and affected function. It cannot write to a controller, issue an actuator command, authorize maintenance, clear a restriction or change a vessel mode.

## Metrics

Report per fault, regime, asset and signal-availability stratum:

- event recall and false alarms per valid engine-hour;
- fault isolation, ambiguity-class accuracy and false isolation;
- command-conditioned residual calibration and prediction-interval coverage;
- tracking-error detection precision/recall and latency;
- worst-regime/worst-fault performance;
- false-known/OOD rate and selective risk-versus-coverage;
- degradation after each control, actuator and response-channel dropout;
- common-cause dependency loss;
- transfer penalty and prior-domain forgetting;
- model parameter count, compute, memory, inference latency and signal bandwidth.

Group confidence intervals by independent engine/campaign/run. A large number of rows or overlapping windows is not a large independent-engine sample.

## Frozen falsification cases

1. A setpoint is presented as proof of actual engine speed, torque or fuel delivery.
2. A requested actuator command is presented as measured actuator position.
3. A derived load estimate is counted as independent evidence from the command used to calculate it.
4. A dropped control channel is filled with a nominal setpoint.
5. Missing actuator feedback is interpreted as successful actuation because a command was sent.
6. A physical sensor bias is isolated as an actuator fault.
7. Load or ambient change is attributed to degradation without conditioning on available context.
8. Controller saturation or mode change is omitted from the response explanation.
9. Future command or retrospective controller mode leaks into an earlier prediction.
10. The regime discriminator uses engine identity or fault labels as a shortcut.
11. Control and response share a clock/gateway failure but are counted as independent.
12. Simulation-only dropout robustness is promoted to physical-engine performance.
13. A recommendation causes a controller write, mode transition, maintenance order or restriction clearance.

Hard-gate failures cannot be compensated by macro-F1 or accuracy.

## Proposed internal targets

These are aspirations, not achieved results or operational limits:

- zero command-to-measurement role promotions in frozen tests;
- zero future-command, retrospective-regime or controller-state leakage;
- zero false physical actuation confirmation when feedback/response is missing;
- at matched event recall and signal availability, target at least 15% fewer false alarms per engine-hour than the reproduced state-only baseline on sufficiently sampled physical regimes;
- at matched capacity and target-data access, improve worst-regime macro-F1 and command-response interval calibration over the reproduced control-aware paper baseline;
- deterministic degradation or abstention under control/actuator/state-channel dropout;
- prior-regime macro-F1 degradation within the #4510 target after continual updates.

No target is an OEM limit, safety threshold or class requirement.

## Dataset constraints

The Aalto MU323DGSC record describes about 70 measured and derived channels, but accessible record metadata does not establish that setpoints, governor outputs, actuator feedback or controller modes are present. The ZIP payload and variable dictionary must be audited before asserting this dataset supports control-aware tests.

If control signals are absent or their semantics cannot be confirmed, Aalto remains useful for the physical-response arm and the control-aware arm is NOT_EVALUATED. Do not invent or infer controller semantics. The control-aware paper's data are described as available from the corresponding author upon reasonable request; acquisition rights and exact simulation assets remain to be established.

Physical validation requires synchronized and permissioned controller intent, actuator feedback and independent plant-response records from a testbed or vessel, with expert-reviewed fault/event labels.

## Acceptance

- typed signal-role definitions and strict JSON Schema;
- positive and negative fixtures covering role confusion, missingness, timing, units and common causes;
- recovered paper protocol and simulation reproduction record;
- matched state-only/control-only/fusion/control-aware/physics-residual tournament;
- independently reviewed controller I/O and measurement map;
- dropout campaign with per-regime performance and uncertainty;
- composition with #4510 transfer/continual reports and #4444 read-only handoff;
- exact model/data/split/evaluator digests and deterministic replay;
- physical testbed evidence before any real-engine performance claim.

## References

- Lin et al. (2026), [Fault diagnosis of marine diesel engine via control-aware domain-adversarial network](https://doi.org/10.1177/01423312261487500), first published 24 September 2026.
- Xie, Mo & Jia (2026), [Multi-source thermal and performance data for thermal fault diagnosis in marine diesel engines](https://doi.org/10.1038/s41597-026-07924-7), *Scientific Data*, 22 August 2026; relevant virtual-to-real reproduction lead, not evidence that Aalto includes controller I/O signals.
- Shi et al. (2026), [Synaptic regularization for continual transfer](https://doi.org/10.1016/j.energy.2026.141124), *Energy*, 15 July 2026; continual adaptation comparison owned by #4510.
- Altosole et al. (2026), [Literature Review Analysis: Fault Diagnostics of Marine Engines](https://doi.org/10.3390/jmse14211988), *Journal of Marine Science and Engineering*, 22 October 2026.

---

A passing contract/schema check establishes only signal-role consistency for the declared case. It does not establish control-system correctness, engine diagnostics, real-engine transfer, shipboard effectiveness, seaworthiness, class approval, maintenance authority or permission to operate.
