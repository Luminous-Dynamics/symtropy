# Social Dynamics Benchmark Protocol v0.1

This protocol is a companion to the Social Dynamics Laboratory proposal. It is designed to prevent a common failure: reporting a plausible animation or a single favorable run as evidence that a social model predicts real outcomes.

## 1. Preregister each experiment

Before execution, publish a machine-readable manifest with:

- scientific question and intended use;
- primary outcome and scoring rule;
- treatment arms and baselines;
- scenario-generation rule and seed list;
- calibration/train window and untouched test window;
- exclusion, missingness, and failure rules;
- sample size and uncertainty procedure;
- subgroup/inequality metrics where lawful and justified;
- known threats to validity and stopping rule.

Do not select the primary metric after examining test results. Label exploratory analysis as exploratory.

## 2. Comparators: separate platforms from models

| Comparator | Role in comparison | What it can establish | What it does not establish |
|---|---|---|---|
| Null/simple rule model | Required floor for every claim | Whether added complexity earns its cost | Real-world validity by itself |
| Canonical formal ABM (for example, voter, bounded-confidence, simple contagion, public-goods) | Mechanism baseline | Reproduction of known qualitative/quantitative behavior | Validity for a different population or policy question |
| NetLogo | Mature model authoring and repeatable runtime reference | A fair reproduction/runtime comparator when model semantics are matched | That one simulation framework is universally more predictive |
| Mesa | Modular Python ABM implementation and analysis comparator | Ease of representation and equivalent-model outcomes | Equal speed unless hardware, code path, and workload are controlled |
| Repast Simphony / Repast HPC | Flexible spatial/network model and scaling comparator | Feasibility of spatial/network and larger-run designs | A predictive advantage without matched models and data |
| LLM-driven agents | Optional high-cost behavioral baseline | Whether natural-language reasoning adds measured predictive value | Human equivalence from realistic dialogue or evaluator preference |

Frameworks are not all competitors on model accuracy. Compare runtime only on equivalent semantics, data, hardware, implementation effort, and output requirements. Compare prediction only on the same held-out data, forecast horizon, target variables, and scoring rule.

## 3. Metric families

### Forecasting and calibration
- Log loss and Brier score for probabilistic event predictions.
- CRPS or a declared distributional scoring rule for continuous outcomes.
- Calibration curves and proper scoring rules, not accuracy alone.
- Prediction-interval coverage plus interval width.
- Temporal holdout performance and distribution-shift degradation.

### Network structure and evolution
- Degree distribution and degree assortativity.
- Reciprocity, clustering, path lengths, component sizes, and community structure.
- Tie formation/dissolution and survival prediction.
- Event-level link prediction evaluated with time-respecting splits.

### Collective outcomes
- Contribution and participation distributions.
- Cooperation persistence, free-riding, and recovery after shock.
- Time-to-recovery, service coverage, unmet demand, and cascade size.
- Inequality of access/outcomes and subgroup worst-case metrics; do not silently trade away severe harms for a better mean.

### Mechanistic and causal checks
- Ablations of the proposed mechanism and feedback paths.
- Negative controls expected not to respond.
- Parameter identifiability and sensitivity analysis.
- Prospective or held-out intervention validation when credible intervention data exist.
- Alternative model structures that can explain the same observations.

### Runtime and engineering
- Agents and events processed per second, with event and run definitions stated.
- Peak resident memory, result size, startup time, and wall time distributions.
- Scaling curve for agent count and edge/event density.
- Determinism/replay rate, crash rate, and invalid-state count.
- Build/version provenance and benchmark harness overhead.

Do not combine these into a single leaderboard score by default. A model can be fast but invalid, predictive but uncalibrated, or accurate on average while harming a subgroup.

## 4. Test design

1. Run invariants and unit tests before stochastic experiments.
2. Run canonical models at literature-documented parameter settings and compare expected summary statistics.
3. Use paired seeds where treatment arms can share random streams; document where treatment changes invalidate common-random-number assumptions.
4. Calibrate only on train/validation data. Keep a temporal or scenario holdout untouched until decisions are frozen.
5. Report distributions across seeds, not only the mean or the most attractive trajectory.
6. Include deliberately misspecified data, missing edges, noisy evidence, strategic manipulation, and distribution shift.
7. Run an ablation for each claimed novel component.
8. Publish negative results and failures alongside successful cases.

## 5. Fair performance comparison

Pin each implementation to an explicitly documented commit and compiler/runtime. Use the same machine, data, horizon, seed count, output collection, precision, and stopping condition. Separate headless simulation throughput from visualization overhead. Warm up compiled runtimes where applicable, report repeated-run variability, and do not omit slow runs without a declared outlier rule.

The first release must publish a reference-machine receipt before it may claim a percentage speedup. Because the local environment may differ across developers, a missing hardware result must remain “not measured,” never zero or pass.

## 6. Data governance

Start with fully synthetic data and canonical mathematical models. For real-world data, require provenance and permitted use, minimum necessary granularity, explicit retention limits, removal/re-identification risk review, and aggregate reporting. No raw private communications or inferred sensitive profiles are required by this benchmark protocol.

The intended output is a conditional scientific model, not a ranking engine for individual citizens. Any policy-facing deployment requires domain-specific validation and human governance independent of a passing software test suite.

## 7. Acceptance report template

Every result report should include:

- Claim tested:
- Baseline(s):
- Train/validation/test split:
- Number of runs and seeds:
- Primary metric and uncertainty interval:
- Calibration and subgroup results:
- Ablation/negative-control results:
- Runtime/memory receipt:
- Data/model/configuration digests:
- Failures and excluded cases:
- Validity boundary:
- Verdict: pass / fail / inconclusive / not run.

“Not run,” “queued,” and “inconclusive” are not equivalent to “pass.”

## 8. Validation ladder: earn stronger claims incrementally

Do not treat a green unit test, a replicated toy model, a match to one aggregate statistic, and an empirically useful intervention forecast as equivalent evidence. Use an explicit validity ladder:

| Tier | Evidence required | Permitted claim |
|---|---|---|
| V0 — implementation invariants | Deterministic replay, feasibility/bounds, resource-accounting checks, failure injection | The implementation meets its declared software contract |
| V1 — canonical replication | Independent reproduction of at least two named reference models, including published parameter settings and multiple output summaries | The implementation can reproduce those specific reference mechanisms |
| V2 — mechanism discrimination | Paired intervention arms, ablation, negative controls, sensitivity checks, and uncertainty across seeds | The tested mechanism changes the declared synthetic or experimental outcomes under stated conditions |
| V3 — empirical distributional validation | Calibrated train/validation split and untouched temporal or scenario holdout; distributional outcomes and subgroup errors reported | The model predicts the tested empirical target within the stated population and horizon |
| V4 — prospective intervention validation | Pre-registered forecasts committed before independently observed intervention outcomes, credible comparator, post-hoc analysis disclosed | Evidence of predictive utility for the specific intervention class and context that was tested |

A model must not skip from V0/V1 directly to policy recommendations. Passing a tier is not a universal certificate: each claim needs a target population, mechanism, time horizon, intended use, and documented validity boundary.

## 9. Distribution-first evaluation

Average outcomes are not enough. Empirical agent-based cooperation research found that a parsimonious deterministic model could predict average public-goods contributions while failing badly on the full contribution distribution; a stochastic model was needed to represent bimodality. Therefore, wherever micro-level distributions are available and their use is ethical, report the distribution—not just its mean.

Recommended metrics by target:
- Binary event probabilities: Brier score and log loss.
- Continuous/distributional outcomes: CRPS or a predeclared equivalent proper score, quantile coverage and interval width.
- Population outcomes: mean, variance, quantiles, tail probabilities, and a declared distribution-distance metric when appropriate.
- Network dynamics: time-respecting link formation/dissolution predictions plus degree, reciprocity, clustering, and component summaries.
- Unequal impacts: subgroup calibration and worst-group errors, alongside aggregate scores where lawful and adequately powered.

Do not tune to each metric independently and then present the best result only. Predeclare one primary outcome, report the complete metric family, and separate confirmatory from exploratory analysis.

## 10. Current research implications

Recent work makes validation itself a central challenge for LLM-agent social simulation. An ICML 2026 position paper calls for shared benchmarks, open data, and reproducible evaluation rather than further expansion without common validation practice. A 2025/2026 review similarly warns that face validity and weakly coupled outcome metrics do not establish operational validity. A 2026 ODD-based replication study reports that LLM-generated ABMs can be executable yet still require statistical validation to establish replication. This project should compete by making these requirements executable, not by assuming Rust or an LLM policy is inherently more correct.

- Puelma Touzel et al. (ICML 2026), *Position: Time to Close the Validation Gap in LLM Social Simulations*: https://proceedings.mlr.press/v306/puelma-touzel26a.html
- Larooij & Törnberg (published 2025; volume year 2026), *Validation is the central challenge for generative social simulation*: https://doi.org/10.1007/s10462-025-11412-6
- *Can large language models implement agent-based models? An ODD-based replication study* (Ecological Modelling 517, July 2026): https://doi.org/10.1016/j.ecolmodel.2026.111624
- Grimm et al. (2025), ODD and NetLogo replication exercise; 15 of 18 selected models were replicated: https://doi.org/10.1016/j.ecolmodel.2024.110967
- Wunder, Suri & Watts (2013), *Empirical agent based models of cooperation in public goods games*: https://www.microsoft.com/en-us/research/publication/empirical-agent-based-models-of-cooperation-in-public-goods-games/

For candidate external datasets, licensing gates, grouped study-level holdouts, and an empirical prediction protocol, see [EXTERNAL_VALIDATION_PLAN.md](EXTERNAL_VALIDATION_PLAN.md).

## 11. Interpreting the prototype's paired bootstrap

The current runner compares each treatment to the no-commitments baseline on the same declared synthetic seed list, then uses a deterministic paired bootstrap to estimate a 95% interval for relative reduction in simulated unmet need. Positive values mean a lower simulated loss.

This interval quantifies variation across this prototype's stochastic runs under its assumptions. It is **not** a confidence interval for effects in real people, not a causal estimate from a human experiment, and not evidence of empirical prediction. The experiment remains below V3 until fitted and evaluated against permitted external observations; it remains below V4 until predictions are committed prospectively before intervention outcomes are observed.
