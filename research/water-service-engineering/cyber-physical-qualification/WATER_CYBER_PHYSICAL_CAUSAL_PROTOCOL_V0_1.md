# WATER-CYBER-PHYSICAL-CAUSAL-001 — matched integrity-to-service experiment protocol v0.1

**Tracking:** [Issue #5095](https://github.com/Luminous-Dynamics/symtropy/issues/5095)  
**Composes:** [#5018 pump-control robustness](https://github.com/Luminous-Dynamics/symtropy/issues/5018), [#3510 / PR #3513 event evaluation](https://github.com/Luminous-Dynamics/symtropy/issues/3510), [PR #3412 paired experiment core](https://github.com/Luminous-Dynamics/symtropy/pull/3412), [PR #4934 observation-integrity triage](https://github.com/Luminous-Dynamics/symtropy/pull/4934), [#5045 observability](https://github.com/Luminous-Dynamics/symtropy/issues/5045), [#5027 service floors](https://github.com/Luminous-Dynamics/symtropy/issues/5027), [#5069 restoration verification](https://github.com/Luminous-Dynamics/symtropy/issues/5069), and [#5075 matched-input fairness](https://github.com/Luminous-Dynamics/symtropy/issues/5075).

**Status:** research contract and synthetic design fixture only. No detector, controller, hydraulic, HIL, utility or field performance is qualified.

## 1. Research question

Under identical exogenous conditions, what happens when the observation/control path is integrity-disturbed; what independent evidence can distinguish the disturbance; does a separately specified protection policy contain unsafe proposals; and what physical/service outcome can be independently supported?

The unit of evaluation is a **matched causal trajectory**, not a telemetry row or classifier score:

`integrity condition → received observations → decision proposal → independent protection disposition → external physical trajectory → service-floor result → independently qualified restoration`.

This protocol composes existing owners. It must not create another anomaly detector, alarm matcher, hydraulic solver, state estimator, evidence ledger, authorization service, actuator controller, work-order lifecycle or restoration scorer.

## 2. Why this is a frontier

- Ranganathan et al. (May 2026), *Journal of Water Process Engineering*, DOI [10.1016/j.jwpe.2026.110114](https://doi.org/10.1016/j.jwpe.2026.110114), report 47,000+ Net3 EPANET/WNTR scenarios and over 95% binary detection, while leading ensemble methods report around 75–77% Top-1 localization in that synthetic setting. Detection, localization, transfer and service impact are different capabilities.
- *Breaking the flow and the bank: Stealthy cyberattacks on water network hydraulics*, *Water Research* 283 (1 September 2025), DOI [10.1016/j.watres.2025.123719](https://doi.org/10.1016/j.watres.2025.123719), motivates evaluating physically constrained measurement corruption through its hydraulic/operational consequences. This repository has not reproduced that paper.
- Neji et al. (23 October 2026), *AquaSense-ICS*, [arXiv:2610.32661](https://arxiv.org/abs/2610.32661), [data DOI 10.5281/zenodo.23390541](https://doi.org/10.5281/zenodo.23390541), is a useful temporal leakage and data-integrity comparator. Its authors disclose that the real sensor node and simulated network are independent, attack perturbations are data-level rather than closed-loop control-path actions, water-quality labels are not validated with laboratory grab samples, TDS units/calibration are not established, and attack segments are limited. It is **not** a physically coupled cyber-to-hydraulic or potability benchmark.
- NIST's water/wastewater cybersecurity project (24 June 2026) is a practical sector comparator for risk reduction across utilities of different sizes: [NIST publication](https://www.nist.gov/publications/cybersecurity-water-and-wastewater-sector).

## 3. Four-arm matched factorial

For each registered scenario, define four logical arms:

| Arm | Integrity condition | Protection policy |
|---|---|---|
| A | nominal | declared baseline |
| B | disturbed | declared baseline |
| C | nominal | evidence-gated protection |
| D | disturbed | evidence-gated protection |

Use four pre-registered contrasts:
1. A vs B: effect of the registered integrity disturbance under the baseline policy.
2. C vs D: effect of the disturbance under the evidence-gated policy.
3. B vs D: effect of protection policy under the same disturbed condition.
4. A vs C: effect of protection policy under nominal observations.

Within each contrast, exactly one registered factor changes. Freeze network/topology/geometry, initial state, demand and physical disturbance traces, seed, sensor set, sampling/missingness/time grid, units, horizon, service-floor profile, actuator envelope, local protection contract and decision deadline. The integrity disturbance is a declared treatment factor, not an unrecorded mismatch. Baseline resource budgets may differ, but must be disclosed using #5075/#5079.

The four arms are an evaluator design, not permission to run a live controller. The evidence-gated path remains a policy/interface proposal until a separately approved isolated HIL test exists.

## 4. Truth, observations and evidence separation

The candidate receives only its admissible observation projection and permitted metadata. It must not receive evaluator-only latent cause labels, hidden schedules, future covariates or physical outcomes.

Keep separate:
- evaluator-owned latent cause and event truth;
- candidate-visible observations;
- independent physical, metrology or forensic evidence;
- modelled physical trajectory;
- measured physical outcome, if later obtained;
- restoration verifier output.

An anomaly is not proof of an attack. A physically plausible residual does not identify an actor or intent. Two sensor IDs do not establish independence: relevant source, acquisition path, gateway, trust root, time root, calibration authority and configuration lineage must be considered. If instrument fault and in-path alteration produce the same received trace and no admissible side evidence separates them, preserve an observational-equivalence set and abstain.

A digest establishes artifact identity only. It does not authenticate source truth, signatures, actor identity, calibration, event adjudication or independence.

## 5. Frozen synthetic scenario families

The companion T0 fixture defines twelve semantic controls:
1. Same received fingerprint, different latent causes: no unique cause attribution.
2. Authenticated but stale/replayed observation: reject for current-state use.
3. Plausible multi-channel integrity disturbance: evaluate external consequence without encoding operational attack recipes.
4. Communications partition/availability loss: local protection remains bounded; remote loss cannot expand authority.
5. Shared gateway/time/trust/calibration root: redundant channels are not independent corroboration.
6. Command acknowledgement versus actuator/flow disagreement: command state is not physical operation.
7. Physical source/demand shock without forensic evidence: no cyber attribution.
8. True incident plus nuisance alarms: retain event-level one-to-one matching and alarm capacity.
9. Repair ticket closes while a required area remains below its floor: not restored.
10. Integrity event without physical propagation: detection and service impact are distinct.
11. Physical consequence without attack evidence: impact may be real while cyber cause remains unknown.
12. Apparent recovery followed by recurrence or insufficient dwell: no restoration promotion.

All IDs, references, seeds and digests in the T0 fixture are synthetic placeholders. The fixture contains no real network geometry, control map, credentials, exploit payload or operational timing.

## 6. Dataset lanes

| Lane | Permitted use | Claim ceiling / exclusions |
|---|---|---|
| AquaSense-ICS | Public hybrid dataset for data-level attack/fault, temporal leakage and admission-method research. Keep real, generated and simulated rows distinct. | Not physically coupled; not closed-loop control; not laboratory-validated potability; TDS unit/calibration unresolved; limited attack segments. Verify exact archived release and per-file rights before accession. |
| Net1 / Net3 EPANET-WNTR | Synthetic matched control and modelled physical consequence. | Synthetic only. Pin network, sensor layout, solver, calibration, event definition and assumptions. |
| BATADAL / C-Town | Historical simulated-SCADA chronological detection/transfer. | Simulated, not real utility operation. Do not pool with physical testbeds. |
| WDSEventDB / ACG-Water-Testbed | Controlled sensor-fault and cyber-to-physical laboratory/HIL evaluation once access, rights, configuration and event truth are qualified. | Testbed result only; no utility-scale inference. |
| Babela et al. Slovak utility corpus | Retrospective hourly anomaly/fault-context transfer where data-level licence and label semantics are admitted. | Not attack ground truth or pipe localization by default. |

Dataset rights and admission belong to the existing corpus registry. A repository licence does not automatically establish the data-level licence. Do not use derived/augmented copies or leaked labels in a confirmatory test population.

## 7. Metric lanes — no composite score

**Event:** consume #3513 outputs: per-class event-onset precision/recall, false-alarm episodes per system-day, P50/P95 delay, fragmentation, alarm duration/duty cycle and class confusion.

**Integrity:** consume #4934 semantics: stale/replay rejection, source/path/gateway/trust/time/calibration independence, evidence completeness, equivalence-set size, abstention coverage and unsupported-attribution rate.

**Physical/service:** consume a named external solver/testbed trajectory. Report per-area delivered-demand fraction/volume, pressure/storage deficit degree-time where supported, worst-area floor, breach duration, time to recovery and recurrence. Keep model discrepancy and uncertainty separate. Do not compare modelled and observed numbers as if they were one evidence class. Do not infer potability from hydraulic quantity or generic proxy signals.

**Protection/recovery:** report accepted out-of-envelope or unauthorized simulated proposals, containment latency and false safe-state transitions. Consume #5072 restoration states: `verified_restored`, `candidate_restored`, `not_restored`, `breached_with_indeterminate`, `indeterminate`, `invalid`. Alarm clearance and ticket closure do not prove service.

**Resources:** report comparable online P50/P95/worst-case latency, memory, communication, energy and sensor lifecycle cost; disclose training population/split, offline compute/memory, tuning trials and engineering time under #5075/#5079.

Publish a Pareto surface and all hard-gate failures. Do not collapse these lanes into one weighted resilience number.

## 8. Baselines and aspirational targets

Baseline ladder: fixed threshold; hysteresis/persistence; CUSUM/EWMA; transparent RF/XGBoost/LightGBM where compatible; temporal/graph model where information access is matched; process-residual baseline; declared local-protection baseline; evidence-gated abstention path.

Targets, not results:
- On a frozen, matched holdout, seek either +5 percentage points event-onset recall at a preregistered false-alarm budget, or 30% fewer false-alarm episodes at matched recall, with no regression in P95 delay, calibration, worst stratum or hard service/safety gates. The alarm budget must be established through #3510/operator context, not invented as a universal limit.
- Zero unsupported cyber attributions on observational-equivalence negative controls.
- Zero accepted unsafe/out-of-envelope simulated proposals in the declared finite protected-policy corpus. Report denominator and exact confidence interval; do not extrapolate a finite fixture into a field guarantee.
- Any service-improvement claim requires exact exogenous parity, a paired uncertainty interval and the applicable physical/outcome evidence tier. Synthetic/modelled service outcomes must be labelled as such.

## 9. Staged qualification

- T0: schemas, fixtures, deterministic semantic checks, negative controls and claim-boundary validation.
- T1: matched Net1/Net3 or C-Town simulation with frozen solver/model and external physical-oracle receipts.
- T2: approved isolated HIL/testbed with independent interlocks, physical reference instruments, repeatable disturbances and rollback.
- T3: rights-authorized retrospective utility data with independent incident adjudication and temporal integrity.
- T4: supervised field evaluation with site/operator/regulatory approval, independent metrology and human authorization.

Each tier is a separate result population. T0 does not qualify T1–T4.

## 10. Safety and claim ceiling

At this stage: offline replay and synthetic simulation only. No PLC/SCADA writes, production utility network, credentials, exploit instructions, automatic dispatch or physical actuation. No inference of actor, motive or intent from a telemetry anomaly. This benchmark grants no authority to ration water, change service, isolate assets or initiate a repair.

**Current claim ceiling:** synthetic contract semantics and dataset-admission design only. No detector performance, control safety, hydraulic validity, restoration outcome, laboratory result, utility service or SOTA superiority is established by this protocol or its fixtures.
