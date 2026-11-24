# Service Decision-Impact Calibration Benchmark v0.1

**Prediction-calibration parent:** [#7240](https://github.com/Luminous-Dynamics/symtropy/issues/7240)  
**Decision-impact child:** [#7245](https://github.com/Luminous-Dynamics/symtropy/issues/7245)  
**Branch:** \`research/service-decision-impact-calibration-v0-1\`  
**Status:** research and executable contract specification; no decision-impact evaluator is claimed yet.

## 1. Why forecast coverage is not enough

The local calibration layer (#7240) audits intervals and probabilities by exact site, configuration, asset, metric and stratum. That is necessary, but a 90% interval coverage score does not tell an operator whether the model makes better mobility decisions.

The operational consequence of error is asymmetric:
- a 20-second mission-duration error far from a deadline may change nothing;
- a 20-second error near the deadline can change on-time admission;
- a 30 Wh pack-energy error far above reserve may be harmless;
- a 30 Wh error near a protected mission floor can produce a false-safe admission;
- an over-wide interval can reject safe work and strand capacity;
- a narrow interval with a low-frequency catastrophic miss can create a reserve breach.

A model can therefore look well calibrated marginally and still be unsuitable for a specific decision boundary.

## 2. Paradigm shift: calibration belongs at the policy boundary

The next layer evaluates the entire chain:

\`forecast -> policy proposal -> hard assurance gates -> admitted/rejected action -> measured outcome -> replayed decision\`

For every historical or synthetic opportunity, retain:
- the forecast as it existed at its original decision cutoff;
- the exact policy and threshold version;
- the resource/mission/site snapshot digest;
- the candidate admission/assignment decision;
- the hard-gate disposition from #6803/#6809/#6829/#6837;
- later independently measured outcome and service result;
- a replay using the realized information but the same policy/threshold version;
- the opportunity/cost denominator.

The original decision replay must not use future information. A second, explicitly labelled realized-information replay may use the later measured outcome to estimate whether a different decision would have been made under the same policy. It is a counterfactual benchmark, not information available to the live controller.

The #7129 exact assignment result remains a *clairvoyant upper bound*. It is not the online optimum and must not be used as the denominator for claims about online regret without qualification. #7133 remains the nonanticipative scenario-tree oracle.

## 3. Non-negotiable safety ordering

Decision utility never relaxes a hard invariant. Apply constraints in this order:

1. exact identity, custody, evidence cut-off and state transaction validity;
2. mission reserve, resource/occupancy exclusivity and physical compatibility;
3. recovery deadline and repeated-shock viability;
4. capability-lease uniqueness and unresolved-outcome reservation;
5. operational request/deadline policy;
6. only among candidates that pass the hard gates, compare expected utility, delay, cost and capacity use.

A false-safe decision is not traded against several successful missions. A hard reserve or evidence failure remains inadmissible even if a finite reward function says otherwise.

## 4. Decision-impact metrics

Report these independently; do not collapse them into a single unqualified score.

| Metric | Definition |
|---|---|
| False-safe admission | Forecast/policy admits; realized measured state violates a hard operational feasibility condition. |
| False abstention | Forecast/policy rejects; realized state would have passed all hard gates and deadline requirements. |
| Decision flip rate | Original forecast-based decision differs from same-policy realized-information replay. |
| Hard-gate bypass attempt | Candidate decision requests an action that a reserve/evidence/lease gate rejects. |
| On-time service regret | Utility lost relative to same-policy realized-information replay, with all terms explicit. |
| Clairvoyance gap | Difference between nonanticipative/online decision and #7129 perfect-information upper bound; never call it online regret. |
| Threshold-conditioned coverage | Forecast interval coverage among cases within configured margin bands of reserve, deadline, queue and PCC thresholds. |
| Risk-coverage curve | Delivered/admitted service as abstention threshold varies, plotted against false-safe and unsupported-risk rates. |
| Avoidable abstention | Safe opportunities rejected due to uncertainty, separated by reason and stratum. |
| Decision recovery | Number of subsequent observations required before a calibrated policy returns to its declared risk envelope after shift. |

For every rate publish sample and cluster denominators, Wilson IID-reference interval, and the dependence caveat. For temporal/common-cause traces, use block/cluster resampling or sequential martingale certificates rather than pretending every sortie is independent.

## 5. Cost decomposition

If a bounded utility comparison is used, publish the terms rather than hiding them in a single score:

\`L = L_mission + L_lateness + L_customer + L_energy + L_abstention + L_recovery + L_resource\`

Hard safety/evidence failures are not finite penalty terms. They are exclusion conditions. The loss vector should separately show:
- missed/late missions;
- customer delay and fallback;
- energy cost and peak demand;
- avoidable abstention and unused capacity;
- recovery shortfall/time;
- maintenance/degradation exposure;
- evidence/identity rejection and unresolved capacity lock time.

A weighted scalar may be reported only after showing the unweighted vector and the chosen weights/sensitivity analysis.

## 6. Executable v0.1 contract

Input records must bind:
- sample/opportunity ID, forecast snapshot digest and policy version;
- exact site, asset, resource configuration and forecast stratum;
- metric forecast intervals and later measured outcomes;
- declared reserve/deadline/queue/PCC thresholds and signed margins;
- original decision and same-policy realized-information replay;
- independent evidence for the actual outcome;
- hard-gate dispositions and their reason codes;
- service utility terms and observation cut-off.

For each case, the evaluator recomputes the following rather than trusting a supplied summary:
1. whether the original policy decision was admissible under contemporaneous inputs;
2. whether the realized outcome passed the applicable hard constraints;
3. whether the original action would have changed under the same policy with realized information;
4. whether any lost utility is due to forecast error, policy choice, unavailable capacity or a hard-gate rejection;
5. the distance from the relevant hard threshold at the original decision time.

Any counterfactual that is not identifiable from observed outcomes must be labelled unknown/partially identified; do not invent outcomes for rejected or unserved requests.

## 7. Matched baselines

Evaluate all methods on identical immutable traces, resource configurations, forecasts, evidence cutoffs and policy versions:

- point forecast + fixed conservative margin;
- raw quantile intervals;
- static split conformal (#7240);
- group/asset/site conditional conformal;
- ACI / strongly adaptive online conformal;
- online certificate-driven predict-then-update calibration;
- robust/MPC scheduler with no forecast abstention;
- forecast-risk-aware scheduler behind #6803/#6809/#6829/#6837;
- fixed conservative baseline that always preserves the same reserve margin;
- realized-information replay of the exact same policy;
- #7129 clairvoyant assignment upper bound, labelled separately.

The most important comparison is not raw RMSE. It is the Pareto frontier between successful on-time service, false-safe admission, avoidable abstention and lifecycle cost, under identical hard assurance gates.

## 8. Negative controls

- identical marginal coverage but different false-safe admission rate;
- small forecast error flips a critical reserve threshold;
- large forecast error far from every threshold does not count as a decision flip;
- over-abstention rejects actually feasible opportunities;
- the replay uses a different policy version or threshold from the original decision;
- future information leaks into the original decision replay;
- a hard invariant is converted into a finite economic penalty;
- missing counterfactual outcomes are imputed as successful;
- a clairvoyant #7129 assignment is labelled online-optimal;
- repeated sorties from one pack/weather event are treated as independent trials;
- duplicate opportunity, evidence or outcome receipt is reused;
- model/configuration or measurement boundary changes in the replay;
- global coverage masks a site/asset stratum's false-safe decisions.

## 9. Targets (not achievements)

- zero false-safe acceptance in the frozen hard-negative suite;
- 100% agreement with an independent bounded replay oracle;
- zero hard reserve/evidence/lease gate bypass;
- report false-safe and false-abstention rates by exact site/asset/decision class;
- show a strictly better risk-coverage Pareto frontier than point-forecast/fixed-margin baselines on held-out traces;
- research target: reduce false-safe decisions by at least 20% while adding no more than 5% avoidable abstention on matched held-out cases;
- publish decision regret relative to same-policy realized-information replay separately from clairvoyance gap to #7129;
- report sample counts, dependence clusters and uncertainty for all rates;
- all hard-negative cases remain rejected irrespective of reward/utility weights.

## 10. Ownership and claim boundary

- #7240 owns forecast/outcome calibration and deployment-shift audit.
- #7245 owns decision-impact metrics and same-policy counterfactual replay.
- #6909/#6999/#7004 own event-derived capacity opportunities.
- #7074 owns request admission against exact capacity receipts.
- #7129 provides a clairvoyant offline upper bound; #7133 provides a nonanticipative future-scenario model.
- #6803/#6809/#6829/#6837 remain hard flexibility, recovery, unique commitment and event journal gates.

A passing synthetic decision-impact evaluator would prove only its bounded replay arithmetic and metric definitions. It would not prove operational reliability, customer outcomes, flight safety, grid eligibility or commercial performance.

---

## 11. Executable v0.1 evaluator

The first executable increment lives in:

- tools/ev-passenger-cost-frontier/src/service_decision_impact.rs
- tools/ev-passenger-cost-frontier/src/bin/service_decision_impact_screen.rs
- tools/ev-passenger-cost-frontier/schemas/service_decision_impact_v0_1.schema.json
- tools/ev-passenger-cost-frontier/fixtures/service_decision_impact_v0_1.json

It consumes frozen paired forecast/realized records. A record binds site and site configuration, exact asset and asset configuration, operating stratum, forecast model/version, policy/version, decision time, target interval, threshold direction/value, prediction interval, measured outcome, hard-gate disposition, service-value units, and site/forecast/gate/outcome evidence envelopes.

### 11.1 Explicit decision semantics

The v0.1 policy is deliberately small and auditable:

- For a minimum-bound decision, propose admission only when the entire forecast interval is at or above the threshold (predicted_lower >= threshold).
- For a maximum-bound decision, propose admission only when the entire forecast interval is at or below the threshold (predicted_upper <= threshold).
- The realized-information replay applies the same direction, threshold and policy version to the later observed value. It is a retrospective oracle, never information available at the original decision.
- A hard-gate failed result blocks a proposal; unknown also blocks it and is reported as unsupported. A proposal cannot override either disposition.
- A false-safe proposal is not automatically a false-safe admission. Report proposals separately from those that remain admitted after the independent hard gate.
- False abstention is counted only when the hard gate passed and the realized value would have passed the same threshold policy.
- Service-value loss is counted only for avoidable abstention. Hard failures are never converted to finite utility penalties.

This is a bounded threshold policy, not a general scheduler. The evaluator does not infer source authenticity or independently establish the correctness of supplied hard-gate dispositions.

### 11.2 Frozen synthetic known answer

The fixture contains eight paired records across a stationary-storage lower-bound group and a charger-service upper-bound group.

| Output | Expected |
|---|---:|
| Candidate admission proposals | 6 |
| Proposals blocked by a failed hard gate | 1 |
| Unsupported proposals held for unknown hard-gate outcome | 1 |
| Hard-gate bypass attempts | 2 |
| False-safe proposals | 2 |
| False-safe admissions after hard gate | 2 |
| False abstentions | 2 |
| Same-policy replay comparisons | 6 |
| Decision flips | 4 |
| Avoidable service-value loss units | 200 |
| Group-local risk/coverage frontiers | 2 |

All fixture evidence is synthetic. The reference report must therefore set any_synthetic_evidence=true, all_evidence_source_grade=false, and all field-qualification, model-update and operational-authority flags to false. These counts are known-answer expectations for the synthetic fixture, not empirical mobility results. It also reports separate Wilson 95% reference intervals for false-safe admissions and false abstentions, with their denominators; these intervals explicitly assume IID samples and are not valid cluster-aware guarantees.

The module includes ten focused tests covering false-safe/false-abstention separation, hard-gate blocking, unknown-gate abstention, upper-bound semantics, record-order digest invariance, duplicate evidence rejection, synthetic evidence claim ceilings, forecast chronology, group-local risk/coverage monotonicity, and non-negotiable hard gates.

### 11.3 Risk/coverage output

For each exact group (site, asset, asset configuration, stratum, model/version, policy/version, direction and threshold), the evaluator sweeps the signed conservative-bound margin. It emits accepted-proposal count, coverage of gate-passing replay cases and false-safe rate at each distinct margin, including explicit zero-admission and full-admission endpoints. It reports the realized-feasible denominator for false abstentions and the operational-admission denominator for false-safe admissions. The accompanying Wilson intervals are IID-reference diagnostics only; repeated sorties, shared packs, weather events and site-level common causes require clustered or sequential inference.

This avoids pooling unlike sites, assets, policies or measurement units. The curve is an empirical diagnostic over the supplied cases; it is not a distribution-free risk guarantee. Temporal and common-cause dependence must be handled by the campaign design and inference method, not by assuming every observation is IID.

### 11.4 Verification boundary

The repository workflow now parses the strict schema and fixture, runs the CLI, asserts the frozen counts and retains the report and input artifacts. The exact-head Actions run must pass before the module is described as compiled or tested. A source commit, local arithmetic inspection, queued workflow or artifact from another head is not a pass.

The v0.1 evaluator does not train or recalibrate the forecast model, estimate arbitrary counterfactual outcomes for rejected/unserved requests, prove causal effects, calculate a universal economic utility, authenticate evidence signatures, qualify field hardware, or grant operational authority. It adds an executable audit of the decision boundary—not an operational controller.

## 12. Updated research comparison and tournament

The strongest relevant comparators are complementary rather than interchangeable:

- Static split conformal / conformalized quantile regression: calibration baseline, with chronology and held-out deployment strata retained. The 2026 battery deployment-shift study reports substantial SOC interval coverage collapse when moving from random-cycle splits to held-out cells, protocols, chemistries and EV-profile cells (Processes 14(21), 3557, DOI 10.3390/pr14213557). This motivates explicit abstention and per-domain reporting, not a claim that our evaluator repairs conformal guarantees.
- Adaptive conformal inference and rolling risk control: compare against the October 2026 ERCOT day-ahead reserve study, which reports approximately 90% coverage and 5.3% less reserve than uncalibrated requirements for its rolling risk-control method under the paper's deployment simulation. Reproduce the paper's timing protocol and metric before applying the idea to mobility.
- Hierarchical station calibration: compare against the October 2026 EV charging power forecasting study using 2,964 sessions across 336 stations and a larger 439-station snapshot. Retain cross-station holdouts and regime-specific metrics.
- Robust/MPC and DR-SAC scheduling: run the same forecasts, policy limits, hard-gate evidence and disturbances through every arm. No method may receive privileged information or weaker reserve constraints.

The matched evaluation arms should include point forecast with fixed conservative margin; raw quantiles; static split conformal; site/asset-conditional conformal; adaptive conformal/rolling risk control; online certificate-driven calibration; and each corresponding policy behind the exact same hard reserve, recovery and commitment gates.

The primary comparison is a Pareto frontier over successful on-time service, false-safe proposals, false-safe admissions after hard gates, avoidable abstention, recovery shortfall and lifecycle cost. Do not collapse a hard-invariant violation into a finite scalar penalty.

### 12.1 Research target ladder (not achieved)

| Benchmark | Target |
|---|---|
| Bounded paired replay oracle | 100% agreement |
| Frozen hard-negative false-safe admissions | 0 |
| Hard-gate bypass after independent gate | 0 |
| Unknown hard-gate disposition | 100% held/unsupported |
| Evidence/sample identity replay | 0 duplicate acceptance |
| Deterministic input digest | 100% invariant to record ordering |
| Matched risk/coverage frontier | Pareto improvement over fixed-margin baseline |
| False-safe reduction | ≥20% relative reduction without >5% added avoidable abstention |
| Runtime | report p50/p95/p99; initial research target p99 <10 ms on bounded cases |
| Deployment shift | publish each held-out site/asset/chemistry/protocol/regime separately |

Zero observed false-safe admissions in a finite corpus is not proof of zero real-world risk. Every rate must include the sample denominator, asset/site dependence groups, and an appropriate cluster-aware or sequential uncertainty analysis. A reject-all system fails the usefulness test.

## 13. External research references

- Zhang et al., Diagnosing Conformal Coverage Failure Under Battery Deployment Shift for Joint SOC and SOH Estimation, Processes 14(21), 3557 (2026), DOI 10.3390/pr14213557.
- Costilla-Enriquez, Weng & Ortega-Vazquez, Adaptive Calibration of Day-Ahead Flexibility Reserve Requirements Under Distribution Shift, arXiv:2610.16075 (12 October 2026).
- Huang & Wang, Session-level electric vehicle charging power forecasting under cross-station shift: A deployment-oriented routing and conformal calibration framework, Sustainable Energy, Grids and Networks (available online 15 October 2026), DOI 10.1016/j.segan.2026.102562.

These are external research comparators. Their results are not reproduced by the synthetic fixture and must not be represented as Symtropy results.
