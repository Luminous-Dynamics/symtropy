# CIVDYN benchmark crosswalk and staged targets (v0.1)

This document is a research comparison plan. It does not claim CIVDYN already outperforms any external benchmark. The external benchmarks below measure different constructs; raw scores must not be combined or compared as if they shared one scale.

## 1. Benchmark map: what each field currently measures

| Evaluation layer | External reference | Main measured construct | What CIVDYN should reuse | Remaining gap |
|---|---|---|---|---|
| Individual human behaviour | [SimBench (ICLR 2026)](https://proceedings.iclr.cc/paper_files/paper/2026/hash/2fc4d6a6400addda7493b9db258c34e0-Abstract-Conference.html) | Individual response/choice fidelity over 20 human datasets and diverse tasks | Match the same observation/prediction unit; report proper scores and task-specific results | Cross-population generalization, longitudinal mechanisms and institutional consequences |
| Population/statistical realism | [SSDataBench (PNAS 2026)](https://doi.org/10.1073/pnas.2538145123) | Univariate, bivariate, multivariate, life-event sequence and sequence/covariate statistics | Reproduce distribution, association, sequence and cohort-slice diagnostics | Policy intervention validity and resource/physical consistency |
| Cultural and norm dynamics | [LiveCultureBench (ACL 2026)](https://aclanthology.org/2026.acl-long.1247/) | Task completion, cultural-norm appropriateness and verifier uncertainty in a multi-agent town | Evaluate cross-cultural robustness and norm/outcome trade-offs; audit judge uncertainty | Resource-constrained policy consequences, institutions over time and real-world validation |
| Planning and adaptation | [Agent Planning Benchmark (EMNLP Findings 2026)](https://aclanthology.org/2026.findings-emnlp.1041/) and [DeepPlanning (ACL 2026)](https://aclanthology.org/2026.acl-long.335/) | Long-horizon plan correctness, feedback-conditioned planning, tool robustness and global constraints | Measure plan validity, infeasibility recognition, feedback adaptation and budget/constraint adherence | Causal mechanism diagnosis and whole-system side effects |
| Drift detection | [A Framework for Evaluating and Benchmarking Concept Drift Detection Methods (2026)](https://arxiv.org/abs/2606.07789) | Detection quality and timing across controlled drift types and real datasets | Measure detection precision/recall, delay and leave-one-dataset-out transfer | Causal classification of drift source and the quality of the ensuing intervention |
| Causal qualification | [CIVDYN causal-path reference](https://github.com/Luminous-Dynamics/symtropy/pull/3803) | Matched treatment/control, mediator continuity and independent physical outcomes | Keep matched counterfactuals, placebo controls and independent outcome receipts | External identification assumptions, unmeasured confounding and real interventions |
| Robust policy decisions | [CIVDYN robust-policy reference](https://github.com/Luminous-Dynamics/symtropy/pull/4024) | Finite-design feasibility and metric-specific regret under declared uncertainty | Keep native-unit regret, feasibility and unknown/missing outcomes separate | Adaptive policies over multi-stage reveals and distribution shift |
| Physical / socio-ecological viability | CIVDYN resource ledger, resilience and eco-flow references | Unit-specific conservation, service coverage, reserve, maintenance and environmental-flow delivery | Treat the shared resource ledger as the sole stock/flow authority | Calibrated hydrology, ecology, infrastructure networks and independent external datasets |
| Self-correction | This proposal | Evidence-backed transition from discrepancy to diagnosis to verified, constraint-respecting policy revision | Compose the above without creating a new simulator or a composite score | Human/expert policy references, real episodes, long-horizon prospective validation |

### Drift-detection standard and CIVDYN extension

The KDD 2026 benchmark by Cerqueira et al. compares 14 established drift detectors on seven real-world data streams, covering four distributional drift types (class-prior, label-swap, feature-permutation and feature-filtering) under both abrupt and gradual changes. It emphasizes timing-aware detection metrics and leave-one-dataset-out hyperparameter selection. This is an appropriate baseline protocol for statistical drift detection, but it does not by itself test whether a detected shift changed the causal effect of an intervention. [KDD 2026 / ACM DOI](https://doi.org/10.1145/3770855.3819070) · [arXiv](https://arxiv.org/abs/2606.07789)

CIVDYN's causal-drift reference therefore separates two questions:
1. **Signal:** did the observed distribution or association change? The first comparator is a two-sided CUSUM control. Future baselines should include ADWIN, Page-Hinkley, DDM/EDDM and other detectors from the KDD suite, with identical observation budgets and held-out-family tuning.
2. **Mechanism:** did the intervention's causal effect change, or did measurement/confounding/population composition change? A matched intervention probe or an explicit abstention protocol is required. CUSUM alone cannot answer this.

The current synthetic control deliberately includes a confounder-only shift: CUSUM detects a real observational change, but a signal-only policy disables a still-beneficial intervention. The matched-pair controller instead diagnoses a confounder shift and retains the intervention. This is a known-answer software fixture without sampling noise, not evidence of empirical causal identification or superiority to the 14-detector benchmark.

The next extension should add gradual/noisy shifts, delayed/partial outcomes, multiple detector baselines and leave-one-mechanism-family-out tuning before any performance claim.

### Important comparison rule

The published SimBench headline of 40.80/100 is a result on its own task suite and scoring protocol. It is not an attainable target for the CIVDYN self-correction benchmark. Similarly, SSDataBench's statistical-realism metrics, LiveCultureBench's norm/evaluator measures, and APB's plan grades are not interchangeable. Any direct comparison must reproduce the same dataset, split, prompt/configuration, unit of analysis, scorer, and compute budget.

## 2. What current industry practice tends to optimize

A useful competitor map is by discipline, not by vendor:

- **Social simulation and synthetic populations:** individual choice likelihood, aggregate distribution similarity, subgroup representativeness, and plausibility.
- **Agent planning and workflow systems:** task completion, plan correctness, tool-use reliability, constraint satisfaction, latency and cost.
- **Causal inference and experimentation:** identification assumptions, treatment/control comparability, mediator validity, placebo tests, and uncertainty.
- **Robust control and operations research:** feasibility under uncertainty, worst-case or tail performance, regret, reliability, and operational cost.
- **Resilience/adaptation planning:** service continuity, absorptive/adaptive capacity, recovery time, critical thresholds, lock-in and maladaptation.

CIVDYN should not claim to beat all five disciplines by reporting one high score. Its defensible research opportunity is to **compose the evidence contracts**: human/population validity + causal mechanism qualification + resource-constrained execution + robust decision analysis + traceable correction. This is an integration hypothesis to be tested, not a field-wide superiority result.

## 3. Paradigm shift: benchmark the correction process, not only the end state

The core unit should be a **correction episode**, with a complete, timestamped evidence chain:

`observe → detect → diagnose → discriminate → update → intervene → audit`

Each transition must have evidence and a declared uncertainty level. The evaluator must be independent of the controller. The controller must not be allowed to define success solely through its own reward or its own explanation.

A correction is not successful just because the target KPI improves. It must also:
- stay within declared resource and safety constraints;
- disclose who/what absorbed the cost;
- preserve the time horizon over which the externality is measured;
- disclose missing, censored or unsupported populations;
- reproduce from frozen inputs and source hashes;
- survive held-out mechanisms and contexts.

No composite “civilization score” should be introduced. Report a vector of native-unit and proper-scoring metrics plus any explicitly declared hard constraints.

## 4. Proposed metric vector

| Dimension | Primary metric(s) | Direction | Required disaggregation |
|---|---|---|---|
| Deviation detection | precision, recall, false-alarm rate, missed-event rate, detection delay | precision/recall up; false alarms/delay down | scenario, mechanism, cohort, severity |
| Probability quality | Brier score, log loss, calibration slope/intercept, reliability diagram | lower proper score; calibration near ideal | subgroup, horizon, mechanism |
| Mechanism diagnosis | top-1/top-k accuracy, posterior log score, false causal attribution rate, explicit abstention accuracy | accuracy up; false attribution down | candidate mechanism, evidence budget |
| Experiment/discrimination | expected information gain per unit cost; negative-control pass rate | information gain up; negative controls all pass | test/action cost, mechanism pair |
| Belief update | paired held-out log-score change; forecast calibration change | improves on unseen scenarios | pre/post, family, time horizon |
| Policy quality | hard-constraint violation rate, metric-specific regret, intervention cost, policy instability | violations/regret/cost down | each objective in native units |
| Human/population fidelity | matched Brier/log loss; total variation; Jensen–Shannon divergence; subgroup and trajectory diagnostics | proper scores/divergence down | context, cohort, site, time |
| Physical viability | conservation residual per resource; unmet demand; service-floor breach; reserve and maintenance deficit | residuals/shortfalls down | water, energy, food, region, tick |
| Externality displacement | marginal burden by group, region, sector and future time window | no hidden burden transfers | recipient/donor, current/future |
| Recoverability | service/reserve/environmental-flow recovery latency and post-shock stock | recovery faster, reserves sustained | shock sequence and resource |
| Institutional memory | provenance completeness; replay success; evidence-retention and justified-revision rates | replay/retention up, unjustified revision down | decision episode and policy version |
| Efficiency | cost per qualified correction: time, tokens, energy, currency, memory | down at equivalent evidence quality | model/hardware/controller |

A metric must have an explicit denominator, missingness rule, unit, direction, confidence/uncertainty procedure and aggregation rule. Report medians, tails and paired uncertainty where applicable; avoid averaging incomparable units.

## 5. Staged targets — proposed gates, not achieved performance

Targets below are intentionally staged. They should be frozen before evaluation and revised only through a versioned protocol change.

### Gate A — deterministic conformance (current synthetic reference)

- 100% pass on known-answer positive and negative controls.
- 0 false physical interventions in the sensor-drift sham.
- 0 unexplained water/energy/food conservation residuals beyond the shared ledger's unit-specific tolerances.
- Full seven-stage correction trace for a claimed mechanism-aware correction; incomplete traces must not qualify.
- Exact deterministic replay of each controller/scenario receipt.
- No hidden subgroup, maintenance-energy or industrial-demand omission in the output schema.

### Gate B — seeded mechanism suite

Before claiming a stochastic benchmark result, create at least five mechanism families: sensor drift, causal sign reversal, confounding, delayed externality/maintenance debt, and population/context shift.

Proposed acceptance targets:
- At least 100 declared seeded episodes per family and controller, with seeds and all failures retained.
- False-alarm rate ≤1% on sham/no-change episodes.
- Detection recall ≥95% for predeclared material deviations, with median detection delay ≤1 tick and p95 delay ≤2 ticks where the scenario cadence makes that attainable.
- Diagnosis top-1 accuracy ≥90% on the declared closed-set mechanism suite; otherwise the controller must abstain rather than fabricate certainty.
- Calibration error (predeclared ECE binning) ≤0.05 and a proper-score improvement over the no-update baseline on held-out mechanisms.
- 100% physical constraint and ledger-integrity compliance; policy failures remain failures, not dropped rows.
- Report all disaggregated burdens. No objective may be treated as protected unless the protocol explicitly marks it as a hard constraint.

These values are aspirational evaluation gates for a controlled suite, not claims about current systems or real-world detectability.

### Gate C — generalization and policy robustness

- Hold out entire mechanism families, study sites, time blocks and declared populations; never randomly split rows from the same participants/episode across train/test.
- Require positive paired improvement in proper predictive score on the held-out set, with a cluster-appropriate uncertainty interval that excludes zero.
- Report worst-case and p95 metric-specific regret over the declared uncertainty design; do not infer continuous robustness from a finite sample.
- Compare frozen, outcome-reactive, model-free adaptive, mechanism-aware, mechanism-aware-with-independent-audit, and documented human/expert policies at matched observation, action and compute budgets.
- Include placebo intervention, sham drift, evidence withholding, population replacement and metric-substitution controls.
- Demonstrate replay from frozen scenario, controller, source and configuration digests.

### Gate D — external benchmark and empirical validation

- Re-run the relevant upstream protocols for SimBench, SSDataBench and LiveCultureBench where data licences, ethics and implementation permit.
- Publish independent per-dataset and per-stratum results, not an aggregate cross-benchmark score.
- Validate real-world episodes prospectively or on untouched historical periods, with independent domain experts and calibration diagnostics.
- For a catchment application, use source-backed hydrological, ecological-flow, energy/pumping and infrastructure data; the current synthetic pump relationship is not an acceptable substitute.
- Publish all pre-registered failures, exclusions, uncertainty intervals and evidence limitations.

## 6. Comparison baselines and ablations

Every mechanism family should include the following equal-budget baselines:

1. Frozen policy: no adaptation.
2. Outcome-reactive: respond to the latest KPI threshold, without diagnosis.
3. Model-free adaptive: update action from a declared online rule.
4. Mechanism-aware: maintain competing causal explanations and choose discriminating tests.
5. Mechanism-aware + independent verifier: refuse to claim correction unless the path and physical ledger reconcile.
6. Human/expert reference: only when there is a documented, ethically appropriate protocol.

Ablate one capability at a time: secondary sensor, counterfactual test, uncertainty estimate, maintenance reserve, subgroup audit, institutional memory, or independent ledger. This shows what actually contributes rather than attributing the full improvement to a single model.

## 7. Current CIVDYN baseline and work still required

| Capability | Current implementation/reference | Honest status |
|---|---|---|
| Unit-specific resource conservation | `civdyn-resource-ledger` | Deterministic software reference |
| Micro/population/trajectory fidelity | `civdyn-human-fidelity` and #4092 | Synthetic conformance; not a SimBench/SSDataBench reproduction |
| Paper-era HumanStudy native result adapter | #4090 | Importer/schema/CLI CI passes; no new model run or PAS/ECS uplift demonstrated |
| Causal path with matched controls | #3803 | Separate draft branch; not yet composed into this PR's base |
| Robust policy feasibility/regret | #4024 | Separate draft branch; finite-design claim only |
| Collective-risk physical replay | #4037 | Separate draft branch |
| Repeated-shock reserve and environmental-flow obligations | #4123 | Synthetic ledger reference, exact-head CI passes |
| Self-correction reference | #4149 | Initial synthetic implementation; strict CI still being completed |

The current self-correction reference intentionally exposes a policy trade-off: the mechanism-aware controller protects the household water floor by investing energy in maintenance and curtailing industrial water during the repeated shock. Industrial unmet demand and maintenance-energy unmet obligations are reported separately. This is not a universal optimum.

## 8. Highest-value next research steps

1. Finish strict software conformance of #4149; then extend from two deterministic scenarios to a seeded mechanism suite.
2. Add proper-score calibration and uncertainty intervals to the controller output, while keeping the outcome evaluator independent.
3. Compose #3803 and #4024 only after their dependency branches are reviewable/integrated; do not duplicate their causal or robust-policy kernels.
4. Add held-out cross-population and cross-context transfer on lawful datasets, with privacy-safe splits.
5. Introduce source-backed operational data only after units, rights, calibration, spatial resolution and validation windows are frozen.
6. Run blinded expert review on mechanism diagnosis and correction traces; use structured labels and measure inter-rater reliability rather than relying solely on an LLM judge.

## 10. Noise-aware confirmatory self-correction benchmark

The seeded extension adds a separate development/confirmatory protocol rather than treating deterministic controls as performance evidence. Development seeds 1–512 select the CUSUM configuration under a declared false-alarm budget; confirmatory seeds 10,001–11,000 are untouched. The evaluator reports Wilson 95% intervals for signal/diagnosis rates, normal-approximation intervals for mean utility, and retains all 20,000 episode receipts.

This closes part of the gap between a software fixture and a statistical benchmark, but the generator is still synthetic. The intervals quantify finite-seed Monte Carlo variation only; they do not account for structural model uncertainty, real-world measurement error, cultural heterogeneity, historical confounding, or ecological model misspecification. The matched-probe intervals are conditional on the declared bounded-uniform noise model.

Before comparing against external drift or causal-inference SOTA, the next steps remain:
- reproduce detector baselines on the exact public streams used by the target benchmark;
- test multiple noise families and unbounded/heavy-tailed errors;
- add delayed, missing and selectively observed outcomes;
- tune by held-out mechanism family/site rather than only seed split;
- use empirical interventions or defensible natural experiments where lawful, with independent domain review.

The current noise-aware implementation is an internal confirmatory harness, not a reproduction of KDD 2026's 14-detector, seven-dataset benchmark and not an empirical civilizational policy evaluation.

## 9. Claim boundary

This crosswalk defines a research plan and proposed targets. It is not an empirical comparison, benchmark leaderboard, independent peer review, or demonstration of SOTA superiority. Synthetic reference results establish deterministic software behaviour under declared inputs only.
