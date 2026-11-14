# Probabilistic Forest Forecast Benchmark v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Focused work:** [FOREST-PROB-FORECAST-001 #5277](https://github.com/Luminous-Dynamics/symtropy/issues/5277)  
**Status:** synthetic ensemble-scoring kernel and six-case arithmetic corpus; no empirical ecological forecasts

## 1. Why point accuracy is not enough

A point forecast hides uncertainty. A forecast system can have a small mean error and still be overconfident, underdispersed, poorly calibrated or useless in a high-impact tail. Conversely, a broad distribution can achieve nominal coverage but be too vague to inform a decision.

The evaluation objective is **sharpness subject to calibration**, assessed with proper scoring rules. This is standard probabilistic-forecasting practice, not a novel scoring rule invented by this project.

The NEON Ecological Forecasting Challenge accepts distributional or ensemble forecasts for horizons from one day to one year, scores with CRPS, and publishes climatology and persistence baselines. Its documented baseline comparisons are a useful reference architecture: https://projects.ecoforecast.org/neon4cast-ci/instructions.html

A synthesis of more than 100,000 freshwater forecasts in the NEON challenge describes the use of CRPS and a day-of-year reference forecast to compare model skill: https://doi.org/10.1002/eap.70004

The EFI 2026 abstract booklet describes the challenge infrastructure as having processed more than 82 million forecasts across 81 sites since 2021: https://projects.ecoforecast.org/meetings/ecoforecast/_assets/files/2026-07-27_EFI2026_AbstractBooklet.pdf

An October 2026 review of digital-twin forestry identifies tropical and boreal ecosystem under-representation, transferability constraints, data-assimilation uncertainty and operational compute costs as open problems: https://link.springer.com/article/10.1007/s44564-026-00029-8

## 2. Kernel scope

The Rust module `validator/src/forecast_kernel.rs` provides:
- empirical CRPS for equally weighted finite ensemble samples;
- nearest-rank median and central 50%, 80% and 95% prediction intervals;
- point-summary MAE;
- paired baseline CRPS and the CRPS skill score;
- interval coverage and average interval width;
- explicit abstention accounting and forecast coverage.

The six-case public corpus is `cases/forest-probabilistic-forecast-scoring-synthetic-v0.1.json`.

### CRPS

For ensemble members $x_1,\ldots,x_n$ and observation $y$:

`CRPS = \frac{1}{n}\sum_i |x_i-y| - \frac{1}{2n^2}\sum_i\sum_j |x_i-x_j|`

The implementation uses the sorted-sample identity for an $O(n\log n)$ sort plus linear pass, avoiding an $O(n^2)$ pairwise matrix. A point-mass forecast reduces to absolute error.

### Paired CRPS skill

This benchmark uses the positively oriented convention:

`CRPSS = 1 - \frac{\overline{CRPS}_{model}}{\overline{CRPS}_{baseline}}`

Positive values mean the model improves over the baseline; zero means equal mean score; negative values mean worse performance. Model and baseline means are computed on the same issued-forecast cases. Baseline performance over all eligible cases is reported separately. If the paired baseline score is zero, skill is undefined rather than divided by zero.

This convention is explicitly different from the signed difference used by some challenge reports. Every public result must state its convention.

### Intervals and calibration

This v0.1 implementation uses an empirical nearest-rank quantile for finite ensembles. It reports nominal 50%, 80% and 95% interval coverage, as well as mean widths for 80% and 95% intervals.

Coverage and width are reported separately. Coverage alone rewards arbitrarily wide intervals; width alone rewards overconfident forecasts. These six synthetic cases exercise the arithmetic but are far too small to establish statistical calibration, and finite ensemble quantiles are discrete. Production evaluation should add PIT/reliability analysis, interval scores or weighted interval score, and block-bootstrap uncertainty over site/campaign/time groups.

## 3. Abstention is part of the denominator

The summary reports:
- eligible cases;
- cases where a forecast was issued;
- cases where the model abstained;
- forecast coverage over all eligible cases;
- predictive scores on the issued cases;
- baseline score on the same paired subset and separately on all eligible cases.

Abstentions are not assigned zero CRPS and are not silently deleted from the coverage denominator. A system cannot make itself look better simply by refusing difficult regions or time horizons. This v0.1 slice records issued-versus-abstained status; a later version should add confidence thresholds and publish the full risk–coverage curve, including risk at 50%, 70%, 90% and 100% coverage.

## 4. Synthetic reference result

The six-case corpus contains five issued forecasts and one abstention. An independent JavaScript reference calculation reproduces all case-level scores and the aggregate summary:

| Metric | Synthetic result |
|---|---:|
| Eligible cases | 6 |
| Issued forecasts | 5 |
| Abstentions | 1 |
| Forecast coverage | 83.33% |
| Mean model CRPS | 2.45 |
| Mean paired baseline CRPS | 2.00 |
| CRPSS | -0.225 |
| Mean absolute error of ensemble mean | 2.20 |
| 50% interval coverage | 60% |
| 80% interval coverage | 60% |
| 95% interval coverage | 60% |
| Mean 80% interval width | 2.40 |
| Mean 95% interval width | 2.40 |

The toy model is worse than its baseline on the paired subset. This is expected and desirable as a conformance check: the evaluator must not force every result to be a win.

These are hand-designed arithmetic cases, not observations, field forecasts, calibration measurements or model skill estimates.

## 5. Fair empirical tournament

Compare each candidate against at least:
1. seasonal/day-of-year climatology;
2. persistence or random-walk baseline;
3. a predeclared strongest single-process model;
4. a process-model ensemble;
5. an observation-informed / data-assimilation model;
6. a hybrid process + ML model;
7. an abstaining/selective model.

Freeze input cutoff, source versions, observation operators, spatial and temporal support, forecast submission time, horizons, ensemble size, compute budget, baseline implementations, split identities and score code. The paired comparison must use the same eligible cases, observations and target support.

Do not pool different variables or horizons into one scalar without an independently justified normalization and weighting contract. Report each variable, unit, horizon, region and disturbance regime separately, then provide transparent macro- and micro-aggregates if useful.

Required holdouts:
- forward-time rolling origin;
- leave-site-out;
- leave-campaign-out;
- leave-region / biome-out;
- disturbance-regime holdout where sample size permits.

Every split must group related plots, acquisitions, scenes, deployments and common upstream evidence to avoid leakage.

## 6. Proposed targets (not achieved results)

These are acceptance targets for a later empirical tournament, not present performance:

| Dimension | Proposed target |
|---|---|
| CRPSS | At least +0.10 against the strongest paired baseline in at least two independent regional holdouts, with a block-bootstrap 95% lower confidence bound above zero |
| Calibration | 80% and 95% interval coverage within ±5 percentage points of nominal across sufficiently large independent groups, with confidence intervals and sample sizes reported |
| Sharpness | Lower interval width than the strongest baseline at matched empirical coverage; never trade away calibration without declaring the change |
| Selective prediction | Publish the entire risk–coverage curve; report CRPS at 50/70/90/100% coverage and area under the curve |
| Transfer | Report worst-region CRPSS and calibration as well as pooled scores; no single-region win may conceal a transfer collapse |
| Horizons | Score 1-, 30-, 90-, 365-day and multi-year horizons separately wherever qualified target observations exist |
| Operational cost | Report runtime, memory, energy, data latency, storage and field crew-hours alongside forecast quality |

A +0.10 CRPSS target is meaningful only against a strong baseline, frozen paired cases and independently held-out regions. It is not a claim that one model can reach it on every forest variable.

## 7. Limits and next work

Not implemented:
- empirical PIT and reliability diagnostics;
- weighted interval score / interval score;
- Brier score and event-probability reliability for disturbance or mortality events;
- confidence-ranked risk–coverage curves;
- block-bootstrap confidence intervals and hierarchical region/site aggregation;
- distributional parameters beyond equally weighted finite ensembles;
- multi-year process forecast evaluation;
- real source-data adapters, rights adjudication, field calibration or prospective forecasts.

The next extension should add Brier/reliability for binary events and risk–coverage curves based on a declared acceptance score. It should compose with the shared forecast and uncertainty owners rather than creating a second ecological forecasting engine.

## 8. Claim ceiling

A successful `PASS_SYNTHETIC_FORECAST_SCORING` receipt means only that the committed ensemble CRPS, nearest-rank intervals, paired baseline score, skill, and abstention accounting agree with the synthetic known-truth cases. It does not establish ecological calibration, forecast skill, regional transfer, operational utility, conservation impact or SOTA superiority.
