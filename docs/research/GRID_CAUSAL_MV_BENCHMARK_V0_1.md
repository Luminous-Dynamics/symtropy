# Grid Causal M&V: Pre-Registered Design and Benchmark Programme v0.1

**Programme:** GRID-CAUSAL-MV-001 (#7347)
**Executable design gate:** `symtropy-grid-causal-design` / GRID-CAUSAL-DESIGN-AUDIT-001
**Research cutoff:** 24 November 2026
**Implementation status:** pre-registration and cluster-randomization auditor, strict schema, frozen fixture, CLI, integration test and dedicated CI workflow authored; exact-head CI pending.

## 1. The measurement-to-causality gap

A counterfactual model can estimate the no-dispatch load accurately on untreated holdout periods and still fail to identify what dispatch caused. A baseline forecast is a prediction of an unobserved quantity during treatment. Its residual calibration is important, but it is not equivalent to randomized assignment, a valid instrument, a defensible control group or a causal identification argument.

Keep four layers distinct:

1. **Observed outcome:** PCC meter and service telemetry under the actual event.
2. **Predicted counterfactual:** model estimate of the no-dispatch outcome, qualified on untouched no-dispatch periods.
3. **Causal effect:** difference attributable to treatment assignment under a stated identification design and assumptions.
4. **Operational benefit:** net energy/service/reliability/cost effect subject to reserve, recovery, network, safety and product constraints.

The new audit crate qualifies layer zero: whether the experiment's assignment and diagnostic plan were frozen and internally reproducible before treatment. It does not estimate layers 2–4.

## 2. Design ladder

### Tier 1 — Cluster-randomized parallel assignment

Prefer this when dispatch is safe and operationally permitted. Randomize at the largest shared interference boundary: depot/charge-inventory island, feeder, substation or other component where one unit's treatment can affect another unit's outcome. If several depots share a feeder or inventory pool, they must be one randomization cluster or the spillover must be explicitly modeled.

Primary estimand: **intention-to-treat (ITT)**, the effect of assignment/offer to the dispatch policy, regardless of subsequent compliance. Report treatment received and compliance separately. Do not claim treatment-on-treated without a credible instrument and explicit exclusion/compliance assumptions.

### Tier 2 — Randomized stepped-wedge rollout

Use when all eligible sites must eventually receive an intervention. Randomize deployment order, pre-register calendar and site effects, preserve not-yet-treated controls, and account for temporal trends and carryover. Dispatch and reserve-recovery washout periods must be explicit.

### Tier 3 — Contemporaneous controls and synthetic controls

Use untreated units observed over the same weather, price, workload, operating and calendar shocks. Predefine donor eligibility, weight estimation, overlap/support diagnostics, and sensitivity to removing each donor. A donor pool that is itself indirectly treated through feeder, queue, price or inventory spillovers is not a valid untreated control.

### Tier 4 — Quasi-experimental / observational identification

Difference-in-differences, event studies, double/debiased ML, regression discontinuity or instrumental variables may be used only with stated assumptions, overlap, pre-trend diagnostics, placebo windows/outcomes, negative controls and sensitivity to unobserved confounding. If those conditions do not hold, the result stays a model-based impact estimate, not a causal claim.

## 3. Executable design audit

The v0.1 Rust auditor currently supports one deliberately narrow design: cluster-randomized parallel assignment with ITT as the primary estimand.

Required frozen inputs:
- experiment, service product, primary outcome and PCC measurement boundary;
- protocol/pre-registration, covariate, baseline-partition and interference-map digests;
- seed commitment timestamp and digest;
- public-randomness source ID/value/digest and publication timestamp after the plan freeze;
- assignment-issued timestamp before dispatch;
- strata and exact treatment/control cluster counts;
- cluster ID, stratum, assigned arm, site IDs, asset IDs and declared interference domain;
- disjoint baseline-training, pretrend and placebo windows;
- dispatch and full recovery windows;
- at least one preregistered negative-control outcome.

Assignment is reproduced by sorting SHA-256 ranks of `experiment_id | seed_reveal | public_randomness_value | stratum_id | cluster_id` within each stratum. The predeclared treated count determines the assignment; the remainder become controls. The seed must match its pre-freeze commitment and the public randomness value must match its digest.

The audit rejects missing/duplicated cluster, site and asset IDs; reused interference-domain IDs across clusters; unexpected arm counts; assignment edits; seed/beacon digest mismatch; pre-registration/randomization time-order errors; overlapping baseline/pretrend/placebo windows; and missing negative controls.

**Trust boundary:** the auditor validates declared digests and timestamps, but does not authenticate the pre-registration document, timestamp authority, interference map or external randomness provider. Its output flags these as unauthenticated. A unique domain ID in a manifest does not prove the real-world interference graph is complete.

## 4. Required causal estimands

Pre-register before outcomes:
- ITT effect of dispatch offer/assignment on PCC import and service outcomes;
- dispatch-period gross reduction, recovery rebound and event-plus-recovery net energy;
- service success, mission/workload completion, reserve-recovery shortfall and queue delay;
- locationally deliverable net response at the exact PCC, under the declared network/contingency case;
- total cost and degradation per reliable net delivered kWh;
- compliance/uptake (descriptive first; treatment-on-treated only under justified identification).

Do not collapse a positive dispatch-period effect into a positive event-level benefit if recovery rebound offsets it. Do not treat a DC deliverability cap as a meter reading or a causal outcome.

## 5. Industry and research comparators

### 5.1 FERC / NAPDR baseline M&V

The FERC / National Action Plan on Demand Response M&V guidance describes how baseline windows, eligible-day exclusions, adjustment rules and load class affect measurement and settlement results. It is an important historical baseline-method comparator; it is not a proof that one historical baseline method is unbiased for every mobility or storage resource.

Reference: https://www.ferc.gov/sites/default/files/2020-04/napdr-mv.pdf

### 5.2 Quantile Generalized Synthetic Control (QGSC), November 2026

Shen and Bao, *Counterfactual Probabilistic Electricity Load Baseline Estimation for Demand Response Using Contemporaneous Non-Activated Customers*, Energies 19(22), 5442 (12 November 2026), use contemporaneous non-activated customer loads to update historical conditional quantiles and calibrate predictive intervals.

Against their CalTRACK-style regression comparator, the paper reports CRPS 170.440 to 164.458 kW (3.51% lower), nominal 90% interval coverage rising from 77.89% to 85.16%, and 9.79% wider normalized intervals. It uses measured C&I data with simulated activation/response and five overlapping 60-customer cohorts drawn from one city pool of 102 customers. It remains undercovered and does not demonstrate live operational DR performance or nominal 90% coverage.

Reference: https://doi.org/10.3390/en19225442

**Benchmark implication:** reproduce QGSC on the same frozen event/holdout cohorts. Compare CRPS, MAE, bias, worst-stratum coverage, normalized width, donor support and placebo false positives. Do not call the published 85.16% empirical coverage a 90% guarantee.

### 5.3 Randomized household field M&V

Fadhuile, Llerena and Roussillon, *Intrinsic Motivation to Promote Demand Flexibility: A Field Experiment From Household Demand*, The Energy Journal 46(4), 2025, report a two-year randomized field experiment involving 165 households and more than six million half-hour observations. Their difference-in-differences analysis reports a 21% peak-event reduction in that particular household intervention setting.

Reference: https://doi.org/10.1177/01956574251320363

This is a causal-design precedent, not an effect-size target for a mobility fleet, depot or BESS. Its value is the randomized intervention/control logic and a large observation history.

### 5.4 Rebound-aware M&V

Oh, *A Framework for Rebound-Aware Net-Reduction Measurement and Verification*, Sustainability 18(14), 2026, evaluates 3,960 ComStock buildings. It reports a median gross reduction of 62.7 kWh and median 2-hour rebound of 5.49 kWh, with substantial risk of treating event-window gross reductions as net savings. Rebound-aware accounting is necessary to expose recovery effects but not guaranteed to improve error in every method/building case.

Reference: https://doi.org/10.3390/su18147154

### 5.5 Distributionally robust BSS scheduling and safety layers

Xin et al., Energy 355 (15 July 2026), 141090, formulate two-stage distributionally robust battery-swapping station location/inventory planning with joint chance constraints, moment-based ambiguity sets, CVaR/SOCP reformulation and outer approximation.

Reference: https://doi.org/10.1016/j.energy.2026.141090

Du and Zhu, Applied Sciences 16(22), 11346 (16 November 2026), study DR-SAC heavy-truck battery-swapping scheduling. In their high-demand scenario DR-SAC reports 5.3 scarcity-violating periods/day versus SAC 2.9, TD3 12.4 and DDPG 21.7, while DDPG shows higher peak shaving but more violations. Across their full 31-day window DR-SAC averages 0.8 violations/day, above SAC's 0.4 and TD3's 0.3. The work uses one mining-area station and omits some charger-bay/degradation constraints; these figures are not transferable fleet results.

Reference: https://doi.org/10.3390/app162211346

These works reinforce that objective reward, peak shaving or profit must not dominate hard reserve, reliability and service constraints.

### 5.6 Cyber-physical resilience

Ghelichpour et al., Renewable Energy 275 (1 November 2026), 126219, include communication delays, false-data injection and network reconfiguration in smart-grid operation with EV swapping stations.

Reference: https://doi.org/10.1016/j.renene.2026.126219

Add the same threats to experiment execution: delayed treatment assignment, meter substitution, stale feeder mapping, assignment replay, telemetry loss and control contamination.

## 6. Benchmark tournament

All estimators receive identical pre-event features, eligibility rules, dispatch records, metrology, weather/price/workload context, event timestamps and recovery horizon. Hold out complete depots, feeders, seasons and hardware configurations; do not random-split time-adjacent observations.

| Comparator | Required score vector |
|---|---|
| Same-period historical mean / comparable-day | Bias, MAE/RMSE, heatwave and high-load residuals, placebo false reductions |
| CalTRACK-style regression / adjustment | Bias, MAE, RMSE, coverage, interval width, sensitivity to eligible-day rules |
| Workload-/mission-aware regression or GAM | Held-out regime accuracy, feature-time leakage, worst-stratum calibration |
| Physics-informed IT/cooling/thermal model | Out-of-sample residuals, physical-state consistency, transfer to new hardware |
| QGSC / contemporaneous non-activated controls | CRPS, interval coverage/width, donor overlap, placebo and donor-removal sensitivity |
| Randomized cluster ITT | Cluster-aware effect estimate, CI, compliance, spillovers, attrition and external validity |
| Stepped-wedge randomized rollout | Time/site effect, not-yet-treated support, carryover and calendar confounding |
| DiD / event study / synthetic control / DML | Pre-trends, overlap, placebo tests, sensitivity to unobserved confounding |

### 6.1 Metrics

- Untreated baseline bias, MAE, RMSE and CRPS.
- Nominal versus empirical coverage and normalized interval width, both pooled and worst-stratum.
- ITT effect estimate with cluster-robust uncertainty, plus assignment compliance and attrition.
- Placebo-window and negative-control false-positive rate.
- Spillover/interference and donor-contamination sensitivity.
- Gross dispatch reduction, rebound, event-plus-recovery net kWh.
- PCC lower-bound response and per-interval service/response compliance.
- Net deliverable MWh after reserve, recovery and DC/AC/network constraints.
- Cost, degradation and reserve opportunity cost per reliable net delivered kWh.
- False-safe admission, false abstention and service-level violations.

### 6.2 Power and sample-size discipline

Do not equate millions of telemetry rows with millions of independent experimental units. For cluster assignment, the effective information depends primarily on the number of independent clusters and intracluster correlation. Freeze the minimum detectable effect, alpha, power, cluster-size distribution, intracluster correlation, expected attrition and non-compliance before assignment. A conventional planning approximation uses design effect `1 + (m - 1) × ICC`; unequal cluster sizes and unequal assignment probabilities require a more appropriate design-based or simulation-based calculation.

The v0.1 auditor imposes only a minimal guard of two clusters per arm. That is not a power certificate. A real field study requires a domain-specific sample-size calculation and independent review.

## 7. How to outperform SOTA

Do not claim to beat QGSC because an internal estimator has lower MAE on a random split, or beat DR-SAC because a policy has higher reward while violating reserves. A meaningful result is a Pareto improvement on the same held-out event distribution: better net delivered service, equal or tighter valid uncertainty, lower false-positive causal credit, no degradation in worst-stratum service, and lower total cost per reliable net kWh, with hard safety-gate failures at zero.

Reproduce each comparator on its native protocol first. Then run the mobility-specific matched tournament with exact hardware/configuration, reserve, recovery, metrology, PCC topology and outcome definition frozen.

## 8. Integration and claim limits

- Baseline/model accuracy and rebound: #3802 / PR #3807.
- Dispatch response tracking: PR #3649.
- PCC metrology and uncertainty: PR #3772.
- Mobility reserve/commitment: #6802/#6803/#6809 and related lease/journal work.
- Locational DC network deliverability: #3826 / PR #3827 and mobility adapter PR #7313.
- Cross-report technical reconciliation: PR #7321.
- Pre-registration and assignment audit: this crate.

A passing design audit is not a causal effect. A baseline model pass is not causal identification. A technically reconciled event is not market settlement. A market or grid claim requires independently authenticated evidence, product-specific qualification and relevant operator/market authorization.
