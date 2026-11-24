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