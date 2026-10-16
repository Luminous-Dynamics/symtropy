# CyPort Benchmark Tasks and Evaluation Contract v0.1

**Status:** frozen task definitions and evaluation safeguards; no model has been fitted or validated.  
**Machine-readable contract:** [CYPORT_BENCHMARK_TASKS_V0_1.json](./CYPORT_BENCHMARK_TASKS_V0_1.json)  
**Verifier:** `cargo run --locked -p maritime-data-receipt -- --validate-cyport-benchmark-tasks docs/tech/CYPORT_BENCHMARK_TASKS_V0_1.json`  
**Upstream source profile:** [CyPort Interaction Table Profile v0.1](./MARITIME_CYPORT_INTERACTION_PROFILE_V0_1.md)  
**Frozen split:** [CYPORT_EPISODE_SPLIT_V0_1.json](./CYPORT_EPISODE_SPLIT_V0_1.json)

## Why this changes the research framing

Treat the job as a **time-indexed decision problem over dependent port–storm episodes**, not a generic row-classification contest. It separates (1) whether the historical table records a coherent impact/recovery marker triplet, (2) the rounded equivalent-days-lost value, and (3) recovery duration as a descriptive outcome pending censoring review. This prevents a common metric trap: the majority zero class makes a constant-zero classifier look good by raw accuracy, while four rows in the pinned table have zero-rounded `total_impact` alongside present recovery markers and nonzero impact proxies.

The next major risk is **information unavailable at forecast time**. The CSV is retrospective. Even a physically relevant variable may be a realized measurement rather than a forecast available when a port operator must act. Accordingly, the feature whitelist is deliberately empty in v0.1. No predictive experiment should begin until each candidate field has a point-in-time availability record, provenance, units, missingness policy, and transformation history. IDs are join/group keys, not default predictors; outcome-derived fields and recovery markers are prohibited as predictors.

## Frozen split and statistical unit

All records sharing a cyclone SID stay in one partition. The split is frozen before model fitting:

| Partition | Years | Storm groups | Port–storm rows |
|---|---|---:|---:|
| Train | 2015–2020 | 59 | 1,471 |
| Calibration | 2021 | 13 | 308 |
| Test | 2022–2023 | 18 | 148 |
| Total | 2015–2023 | 90 | 1,927 |

Fit every imputer, scaler, feature selector, encoder, prior, and model on train only. Use calibration only for probability calibration and decision-threshold selection. Lock the experiment protocol before opening the test metrics; run the temporal test once per registered candidate. Bootstrap uncertainty by SID, not by treating 148 rows as independent. Always report how many independent test storms contain positive outcomes; suppress confirmatory claims when positive cases are concentrated in fewer than ten storm clusters.

This dataset and its labels are public, so the temporal test is **not a secret blind holdout**. The partition mainly measures a forward-in-time, storm-grouped generalization test under a reproducible protocol.

## Four distinct tasks

### 1. Impact-window marker state — proxy target

Predict whether all three recovery fields are present. Treat a mixed field triplet as a data-quality failure. Describe the target as *source-recorded marker state*, not as physical impact truth. Report PR-AUC, precision/recall, specificity, recall at a fixed false-alarm budget, Brier score, log loss, and calibration error when calibrated probabilities are available.

Baselines: train-prevalence constant, train-only port/season climatology, regularized logistic regression. Research aim after adequate data and point-in-time features are established: at least +0.05 absolute PR-AUC over the strongest non-leaking train-only climatology baseline, with storm-cluster uncertainty and the false-alarm trade-off published.

### 2. Rounded impact occurrence

Predict `total_impact > 0`, strictly labelled as occurrence of **nonzero rounded source outcome**. It is not synonymous with true no-impact versus impact. Use the same imbalanced classification metrics and baselines as task 1, plus a hurdle occurrence component.

Research aim: positive Brier skill against the training-prevalence baseline and at least +0.05 absolute PR-AUC against the best non-leaking train-only baseline, without a misleading accuracy headline.

### 3. Rounded impact severity

Predict the validated non-negative integer `total_impact` as equivalent days lost. Report both all-row performance and positive-only performance so the excess-zero distribution cannot conceal failure on disrupted rows. Use MAE, RMSE and count deviance plus storm-cluster bootstrap intervals. Compare train-only median and mean baselines before regularized count and hurdle models. A Random Parameter Negative Binomial-Lindley implementation belongs in a separate, fidelity-checked reproduction track.

Research aim: at least 10% lower held-out MAE than the train-only median baseline and improved count deviance. This remains aspirational until positive test storm-cluster counts make the uncertainty interpretable.

### 4. Recovery duration — descriptive only for now

The meaning of censoring, at-risk origin and observation-window end must be resolved before fitting a recovery predictor. Do not fill missing duration with zero; do not call absent markers right-censoring without an explicit cutoff. First publish duration quantiles, missingness by year/port and construction provenance. After those semantics are documented, a future external forecast benchmark should target 90% interval coverage of 85–95%, with at least 30 independently observed storm episodes; those counts are a minimum research gate, not proof of operational safety.

## Research roadmap: benchmark tiers

| Tier | Required benchmark | Why it matters |
|---|---|---|
| T0 — integrity | 100% exact source/split identity; zero storm-ID overlap across partitions; zero post-outcome feature leakage; deterministic rerun from a recorded config | A high score with leakage is no progress |
| T1 — useful prediction | Beating train-only prevalence/climatology on PR-AUC and Brier skill; severity MAE/deviance beat simple baselines; report storm-cluster uncertainty | Demonstrates incremental skill, not just a complicated model |
| T2 — calibrated decisions | Reliability plots, calibration slope/intercept, recall at an explicitly bounded false-alarm budget, decision-curve / expected-cost analysis over operator-defined costs | A port operator needs actionable risk trade-offs, not just ranking |
| T3 — transfer | Forward-time validation plus at least two independently sourced datasets and multiple hazard/port settings; no tuning on external test labels | Separates generalization from dataset-specific pattern matching |
| T4 — network resilience | Predict service loss on port–route–cargo graphs; compare node-only, static graph and time-varying graph baselines; stress-test alternate routes and correlated port failures | Moves from single-port scores to freight-service consequences |
| T5 — operational rehearsal | Replay historical forecast issue times with only information available then; evaluate lead-time slices (e.g. 24/48/72 h where true forecast snapshots exist), costs, misses, warning burden and recovery decisions with humans in the loop | Tests utility in the actual decision context |

The 24/48/72-hour lead-time benchmark is **future work only**: the interaction table by itself does not establish point-in-time forecast availability. Do not backfill official forecast values from post-event observations and pretend they were known in advance.

## Baselines before complexity

1. **Null baselines:** training prevalence, train-only median/mean, port/season climatology.
2. **Transparent tabular baselines:** regularized logistic/Poisson or negative-binomial models; hurdle model for excess zeros.
3. **Strong non-neural challenger:** gradient-boosted trees, only after field timing is audited and grouped validation code exists.
4. **Paper reproduction:** match its documented filtering and random 80:20 protocol in isolation before comparing its reported model with our chronological storm-grouped benchmark. Do not present the two protocols as like-for-like.
5. **Graph/network models:** only once time-varying port connectivity and route/service outcomes are properly joined with point-in-time provenance. Graph neural networks are not automatically superior; require ablations proving a gain over node-only baselines.

Symthaea may help with schema audits, explanation, scenario generation and hypothesis triage, but the first numeric forecasters should be simple, calibrated, falsifiable baselines. Any Symthaea-generated rationale must remain separate from the measured predictive output and should never be treated as ground truth.

## Interpretation boundary

The frozen CyPort split has only 18 storms and 148 interaction rows in test. Report scores as preliminary, expose uncertainty and per-storm outcomes, and avoid sweeping “revolutionary” claims. Strong evidence means reproducible gains over simple baselines, no leakage, useful calibration, meaningful external transfer, and measurable improvement in port/freight outcomes—not model novelty by itself.

References:
- Kuai et al. (2026), *U.S. port disruptions under tropical cyclones: Resilience analysis by harnessing multiple-source dataset*, *Transportation Research Part D*, 157, 105420. [doi:10.1016/j.trd.2026.105420](https://doi.org/10.1016/j.trd.2026.105420)
- Dataset repository: [ChenchenMobility/Maritime-Data-CyPort](https://github.com/ChenchenMobility/Maritime-Data-CyPort)
- A contemporaneous AutoML study reports high raw accuracy on a different 1,214-incident global-hazard dataset; that number is not comparable to our temporal CyPort test and illustrates why split, label and metric parity must be checked. [Marine Pollution Bulletin study, doi:10.1016/j.marpolbul.2025.118477](https://doi.org/10.1016/j.marpolbul.2025.118477)
