# Beverage adaptive-sensing benchmark v1

**Status:** research contract and synthetic policy corpus; not an implemented sampler, plant qualification, or measured improvement.  
**Owner surface:** Symtropy process-twin research.  
**Date:** 2026-10-21.

## Executive thesis: optimize information acquisition, not only predictions

The next useful shift is from **“sample on a fixed clock, then improve the model”** to **“choose the next measurement whose expected decision value justifies its cost and delay”**, under an immutable safety/quality sampling floor.

A fermentation twin can be wrong in two ways: it can infer the wrong state from available data, or it can request the wrong data at the wrong time. Optimizing only the estimator leaves the second failure mode largely unmeasured. A risk-aware adaptive-sensing policy should choose among available measurements, sample locations, recalibration checks, and bounded deferrals by balancing:

- how the measurement might change a decision;
- the consequences of acting or waiting under current uncertainty;
- sample, laboratory, instrument, technician, sanitation, and turnaround cost;
- information already provided by correlated channels;
- drift, missingness, spatial heterogeneity, and applicability-domain uncertainty;
- a hard, non-negotiable minimum schedule of process-specific required tests.

This is a **measurement-acquisition policy**, not a replacement for validated quality systems, required tests, process authority, or a safe operating procedure. The optimiser must never remove required tests or convert a model score into a food-safety diagnosis.

## Why now: industry and research context

Horváth et al. (2026), *Automated monitoring of alcoholic fermentation: trends and challenges*, reviews sensors, fermentation materials, reproducibility, and industrial feasibility. It notes that many automated solutions focus on few variables, frequently temperature; the reviewed studies are often isolated laboratory studies, not yet evaluated in industrial settings; and a public database combining fermentation-monitoring data from electronic noses, electronic tongues, and other sensors was still missing. DOI: https://doi.org/10.1007/s13197-025-06528-0.

Boulton, Nelson, and Knoesen's 2025 review of commercial wine-fermentation sensors describes nonlinear, time-varying signals, measurement challenges from solids/turbidity and tank stratification, and candidate measurements such as density, weight, refractive index, sugar consumption, ethanol/glycerol, redox potential, biomass, and viability. DOI: https://doi.org/10.3390/fermentation11060329.

A separate 2026 industrial abnormal-fermentation adaptation study used 30 contaminated fed-batch runs and reports that preferred parameter-fitting methods change with the length of the available data prefix. Its raw trajectories are confidential, so this is literature motivation, not a public replay dataset or an independently reproduced result. DOI: https://doi.org/10.1016/j.ceja.2026.101157.

These sources support a specific research gap to test—not a claim that no adaptive-sampling research exists: the field needs more reproducible, apples-to-apples evaluation of **when to measure, what to measure, and whether the resulting evidence arrives in time to change a decision**.

## Benchmark claim and evidence ladder

The benchmark separates four evidence levels:

1. **Synthetic contract tests.** Check policy invariants, scenario dispositions, leakage rejection, budget accounting, safety-floor preservation, and abstention behavior. They validate the evaluator's semantics only.
2. **Retrospective replay with dense references.** Run candidate policies against chronologically ordered, high-frequency reference trajectories and hide measurements that the policy did not request. Keep latent outcomes hidden from the policy. A replay needs a credible reference trajectory; missing or never-measured variables cannot be silently fabricated.
3. **Prospective shadow mode.** The sampler makes timestamped recommendations while the existing validated monitoring schedule continues unchanged. Compare its proposed choices with actual observations and decisions, without suppressing required measurements.
4. **Prospective controlled pilot.** Only after independent quality/process review, preregistration, data governance, and power justification may a site test reductions in *discretionary* measurements. Required process and safety tests remain in place.

A result at one level does not automatically qualify the next. A synthetic win is not retrospective evidence; retrospective replay is not prospective effectiveness; shadow mode is not authorization to change sampling or actuation.

## Policy ladder: equal information and cost constraints

Every comparator sees the same permitted history, timestamps, sensor inventory, calibration states, sample delays, and per-batch evaluation windows. Compare candidates at the same discretionary resource budget or along a complete cost-versus-risk frontier—not with a conveniently selected point after results are inspected.

| Comparator | Policy | What it tests |
|---|---|---|
| B0 | Existing site schedule / fixed-cadence baseline | Operational anchor; the required minimum schedule is always preserved |
| B1 | Simple threshold/event-triggered sampling | Whether a transparent trigger beats static extra measurements |
| B2 | Uncertainty-only acquisition | Whether prediction uncertainty alone selects useful samples |
| B3 | Mechanistic sensitivity / observability acquisition | Whether a measurement is chosen to distinguish process hypotheses or identify parameters |
| B4 | Decision-weighted value of information (VOI) | Whether a likely measurement result could change a decision enough to justify cost and delay |
| B5 | Risk-aware hybrid: fixed required schedule + VOI discretionary sampling + drift/out-of-domain fallback | Whether the full constrained policy adds value over simpler policies |
| B6 | Oracle / hindsight sampler | Upper bound for interpretation only; never an eligible deployable competitor |

A learned VOI model, Bayesian sampler, active learner, mechanistic method, or hybrid should not win by receiving more observations, extra tuning budget, future labels, unavailable channels, or a friendlier data split. Report both the actual site-incumbent comparator and the strongest pre-registered policy baseline.

### Auditable value-of-information gate

For each discretionary sample, express expected losses and costs in the same pre-registered decision-loss units. Record the estimated result-availability time and the current decision deadline. The sample should be eligible as a positive-value recommendation only when:

`net_value = expected_loss_without_sample - expected_loss_with_sample - sampling_cost - delay_cost`

is strictly positive, and the expected result arrives no later than the deadline for the decision it is supposed to inform. If the result is late, the policy may still request it for a future decision or investigation through a separately declared pathway, but it must not claim the sample reduces loss for the already-due decision.

The terms are forecasts, not ground truth. The replay evaluator must score these forecasts against held-out outcomes and report calibration, decision regret and cost sensitivity. Value weights must be frozen before evaluation, published with sensitivity analyses, and must not compensate for required-test omissions, missed-event guardrails, or other hard limits. When the expected-loss terms cannot be defensibly estimated, the policy should abstain or fall back rather than fabricate precision.

The policy output is a bounded decision object:

- **SAMPLE:** request a named available channel or location, with reason, priority, expected turnaround, cost class, and expiry time;
- **DEFER:** defer only a discretionary measurement until a stated review time;
- **ESCALATE:** route to the named human/process authority and preserve the safe incumbent procedure;
- **ABSTAIN:** insufficient calibration, observability, or domain support;
- **REJECT:** malformed, temporally leaked, infeasible, or safety-floor-violating policy output.

Every decision records the model/policy/config digest, observed-data prefix digest, decision timestamp, available channels, requested action, expected value/cost estimates, uncertainty, applicable guardrails, and the reason for choosing or deferring the sample. A request for a sample is not evidence that the result is already available.

## Experimental protocol

### 1. Freeze the measurement contract

Before scoring policies, publish the site/product-family scope, channel inventory, measurement units, sample-location definitions, sensor and laboratory calibration status, assay-result latency distribution, measurement cost basis, required sampling floor, quality endpoints, incident severity classes, and decision deadlines. Maintain distinct costs for assay fees, technician time, instrument wear, sanitation burden, and delay; do not hide unlike costs in an undocumented scalar.

Freeze definitions of a batch, independent experimental unit, abnormal-event onset, event severity, a detected event, a false alarm, a missed event, and an acceptable delay. Respect privacy and commercial confidentiality without weakening data lineage or validation.

### 2. Avoid temporal leakage and selection bias

At each decision time, reveal only information actually available at that time. Features may not include later laboratory results, retrospectively corrected labels, post-batch summaries, or sensor data from the future. Include lab request time and result-available time separately. Split at batch level and, where data allow, hold out campaigns, seasons, sites, product families, or strains to quantify transfer rather than relying on random row splits.

For offline evaluation, the policy cannot see the dense reference values used by the evaluator to score the consequences of not sampling. If no adequate dense reference or validated counterfactual method exists, mark the corresponding result **INDETERMINATE**; do not call reduced observation count a proved efficiency gain.

### 3. Match the budget and retain required measurements

Keep the required sampling schedule identical across all policies. The discretionary-sampling budget may be expressed as measured assay count, direct financial cost, technician minutes, or an explicitly justified common cost basis. Publish all of these separately when available. Include sample transport, laboratory turnaround, and unusable/invalid tests. An event-driven policy may use a protected escalation reserve, but may not exhaust it on low-value routine samples.

### 4. Stress the policy, not just nominal trajectories

The synthetic corpus covers stable batches, high-consequence uncertainty, channel redundancy, sensor drift, calibration uncertainty, tank stratification, result delay, missing observations, budget exhaustion, product/strain shift, future-data leakage, unavailable channels, lack of a dense reference, non-reproducible decisions, difficult-batch starvation, and miscalibrated confidence.

For measured data, report results by disturbance type and severity where those labels are independently supportable. Include sensor failure and missingness mechanisms, not just clean random noise. Simulate no physical hazard as an excuse to reduce a validated real-world test schedule.

## Metrics: fewer samples is only one axis

### Primary safety and process guardrails

- Missed high-severity event rate, with batch-level uncertainty and a pre-registered non-inferiority/superiority criterion.
- Detection delay from the first independently labelled event onset to the first policy-eligible detection, including median, p95, worst batch, and event-severity strata.
- False alarms per batch and false-alarm burden per operating hour; report the detection-delay / false-alarm trade-off.
- Required-test compliance and hard-limit violations. Any policy that cancels a required test is a contract failure regardless of its efficiency score.
- Product-quality endpoints, rejected/scrapped output, and saleable yield only when measured and linked with a valid design. Never infer a food-safety or sensory outcome from a model residual.

### Information and economics

- Discretionary lab assays and total measurement events per batch.
- Direct test spend, technician minutes, sample-handling burden, and result latency reported separately.
- Calibration and prediction interval coverage/width by batch, phase, channel, and disturbance class; use proper scoring rules where applicable.
- Decision regret or loss under declared consequence weights, with all weights preregistered and sensitivity analyses published.
- Added value from each chosen sample: did it reduce uncertainty, change a feasible decision, detect a labelled event, or merely duplicate a correlated measurement?
- Policy-decision latency (median/p95), abstention/escalation rate, invalid requests, and the proportion of decisions that arrive before their real decision deadline.

### Robustness and distribution

Report per-batch distributions, lower-tail / worst-batch outcomes, confidence intervals, and subgroup/campaign performance. Do not treat thousands of correlated timestamps from one tank as thousands of independent fermentation runs. Show the strongest incumbent and baseline results, not just the model's preferred comparator.

## Proposed stretch targets (not achieved results)

The initial target is to reduce **discretionary** paid assay count or a predeclared direct measurement-cost metric by at least **20% versus the strongest pre-registered fixed/threshold baseline**, while satisfying all of the following:

- no missed high-severity events in the qualified sample, plus an appropriately powered and pre-registered bound on the missed-event rate; a small sample with zero misses is not proof of zero risk;
- the upper confidence bound on detection-delay degradation remains within a pre-registered operational margin (an initial comparison margin may be one incumbent sampling interval, but a stricter site requirement takes precedence);
- no required-schedule violations and no increase in hard process/quality constraint violations;
- false alarms remain under the site-defined, pre-registered ceiling;
- calibration, worst-batch, and held-out campaign/site results do not reveal a material collapse hidden by the aggregate mean.

These are research targets, not universal plant-appropriate limits. The target must be re-specified with process owners for each product/site and justified against event severity, operating deadlines, the baseline schedule, measurement cost, and available independent batches. If statistical precision or a valid reference is insufficient, return **INDETERMINATE**.

A positive simulation result is not enough to claim 20% fewer assays safely. A credible industrial claim needs a locked prospective protocol, independent review, evidence that required controls remained intact, and an analysis of the actual measurements and outcomes. Do not say the policy beats SOTA until it has been evaluated against a named, reproducible baseline on a comparable task and split.

## How to compare with industry

Use three comparisons instead of a single leaderboard:

1. **Operational comparator:** the actual site's existing schedule and alert procedure, frozen before prospective evaluation.
2. **Published / reproducible comparator:** published algorithms or open implementations that can be re-created with the same timestamps, channels, permitted history, budget, and evaluation protocol.
3. **Cross-site transfer:** held-out product/campaign/site performance, with sensor stack, tank size, raw-material mix, and assay capability declared.

Commercial deployments may not expose policy internals, raw data, or costs. In those cases, identify the comparison as unavailable rather than infer a competitor's score. Report both absolute measurement burden and normalized costs; context such as plant type, vessel size, and local resource constraints matters. A benchmark score is not a substitute for a plant-specific business case.

A defensible initial industrial win is not “our model predicts better.” It is: **at equal measurement budget and with the same mandatory tests, the policy produces earlier or equally timely detection, no worse safety/quality guardrails, calibrated uncertainty, fewer discretionary assays, and reproducible improvements on held-out batches—then passes prospective shadow-mode and controlled validation.**

## Scenario corpus

- beverage-adaptive-sensing-scenarios-v1.json contains 22 synthetic cases.
- beverage-adaptive-sensing-scenarios-v1.schema.json defines the Draft 2020-12 contract.
- Cases include fixed safety-test preservation, future-information leakage, unavailable channels, missing-as-zero errors, sensor drift, spatial gradients, delayed assays, insufficient dense-reference evidence, policy auditability, non-positive expected net value, and results arriving after the decision deadline.

These files define expected evaluator behavior. They do not contain empirical fermentation trajectories, a qualified sampler implementation, or a result showing any policy wins.

## References

- Horváth, T. et al. (2026). *Automated monitoring of alcoholic fermentation: trends and challenges*. Journal of Food Science and Technology 63, 224–237. https://doi.org/10.1007/s13197-025-06528-0
- Boulton, R., Nelson, J., & Knoesen, A. (2025). *A Review of Sensors for the Monitoring, Modeling, and Control of Commercial Wine Fermentations*. Fermentation 11(6), 329. https://doi.org/10.3390/fermentation11060329
- Kim et al. (2026). *Effect of data availability on in-line adaptation of fermentation mechanistic models under abnormal conditions*. Chemical Engineering Journal Advances 26, 101157. https://doi.org/10.1016/j.ceja.2026.101157