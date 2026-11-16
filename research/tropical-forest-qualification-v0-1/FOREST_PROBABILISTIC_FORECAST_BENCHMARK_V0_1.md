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
- a full confidence-ranked CRPS risk–coverage curve with deterministic forecast-ID tie-breaking;
- Brier score for binary event probabilities, paired baseline Brier skill and fixed-width reliability bins;
- empirical CRPS for equally weighted finite ensemble samples;
- weighted interval score (WIS) over central 50%, 80% and 95% intervals, with paired baseline WIS skill;
- nearest-rank median and central 50%, 80% and 95% prediction intervals;
- point-summary MAE;
- paired baseline CRPS and the CRPS skill score;
- interval coverage and average interval width;
- explicit abstention accounting and forecast coverage.

The six-case public corpus is `cases/forest-probabilistic-forecast-scoring-synthetic-v0.1.json`; it includes a separate six-case selective-risk slice with confidence scores and known CRPS risks.
The binary-event extension is `cases/forest-binary-event-brier-synthetic-v0.1.json` and uses a separate event/horizon evaluation slice.

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

## Binary event forecasts: Brier and reliability

For an event probability $p$ and binary outcome $o\in\{0,1\}$:

`Brier = (p-o)^2`

The binary-event path also uses paired Brier skill:

`BSS = 1 - \frac{\overline{Brier}_{model}}{\overline{Brier}_{baseline}}`

Positive values mean lower model Brier than the paired reference. A zero-score baseline leaves skill undefined. The baseline is scored over all eligible cases and on the exact issued subset; the model score is evaluated only for emitted probabilities.

Five fixed reliability bins summarize mean predicted probability against observed event frequency. Empty bins remain empty/null. These bins are diagnostic summaries, not a formal proof of calibration; their usefulness depends on enough independent events and non-events per bin. A proper Brier score is not sufficient on its own to establish calibration or decision utility.

The six-case binary corpus includes correct event forecasts, a confident false alarm, an abstention and a non-event with low predicted probability. Its independent reference values are:
- five issued probabilities and one abstention (83.33% forecast coverage);
- mean model Brier 0.222;
- paired baseline Brier 0.250;
- Brier skill +0.112.

The small fixed example is solely a mathematical conformance suite. It does not estimate event prevalence, reliability, discrimination, calibration or forest disturbance skill.

### Weighted interval score

CRPS scores the full finite ensemble, while WIS summarizes the emitted central intervals and median. For central intervals with miscoverage levels $\alpha_k$ and median $m$:

`WIS = \frac{0.5|y-m| + \sum_k (\alpha_k/2) IS_{\alpha_k}}{K+0.5}`

where:

`IS_{\alpha} = (u-l) + \frac{2}{\alpha}(l-y)\mathbf{1}[y<l] + \frac{2}{\alpha}(y-u)\mathbf{1}[y>u]`

The current three intervals are central 50%, 80% and 95%. The benchmark reports model WIS, baseline WIS on the same issued subset, baseline WIS over all eligible cases, and paired WIS skill:

`WISSS = 1 - \frac{\overline{WIS}_{model}}{\overline{WIS}_{baseline}}`

Positive skill is better; a zero paired baseline leaves skill undefined. Abstentions are omitted from model WIS but remain in the coverage denominator, while the baseline is reported over all eligible cases and separately on the paired subset.

On the six synthetic cases, five forecasts are issued:
- mean model WIS 2.3143;
- mean paired baseline WIS 2.0000;
- WIS skill -0.1571.

The negative WIS skill agrees with the negative CRPSS direction: the synthetic model is worse than the baseline. WIS is not a calibration certificate by itself; pair it with coverage and interval-width diagnostics, and compare at matched realized coverage.

## 3. Abstention is part of the denominator

The summary reports:
- eligible cases;
- cases where a forecast was issued;
- cases where the model abstained;
- forecast coverage over all eligible cases;
- predictive scores on the issued cases;
- baseline score on the same paired subset and separately on all eligible cases.

Abstentions are not assigned zero CRPS and are not silently deleted from the coverage denominator. A system cannot make itself look better simply by refusing difficult regions or time horizons. This v0.1 slice records issued-versus-abstained status; a later version should add confidence thresholds and publish the full risk–coverage curve, including risk at 50%, 70%, 90% and 100% coverage.

### Confidence-ranked risk–coverage

The continuous corpus also declares six candidate forecasts and a confidence score per case. The scorer ranks cases in descending confidence (forecast ID breaks ties deterministically) and returns every prefix's selected-case count, actual coverage fraction and mean CRPS risk.

The curve is an offline evaluation of whether the submitted confidence score ranks lower-risk forecasts first. It does not infer confidence from ensemble spread and does not set an operational abstention threshold. The six-case curve is discrete; target coverage such as 70% maps to the next attainable prefix, 5/6 or 83.33%. The score is a synthetic ranking label and is not calibrated.

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
- PIT/reliability diagnostics and confidence intervals / calibration belts for reliability summaries;
- confidence intervals / calibration belts for Brier reliability bins and event-stratified uncertainty;
- confidence-score calibration and operational abstention thresholds;
- block-bootstrap confidence intervals and hierarchical region/site aggregation;
- distributional parameters beyond equally weighted finite ensembles;
- multi-year process forecast evaluation;
- real source-data adapters, rights adjudication, field calibration or prospective forecasts.

Next extensions should add PIT diagnostics, calibration intervals for the confidence score, block-bootstrap uncertainty and richer distributional methods. It should compose with the shared forecast and uncertainty owners rather than creating a second ecological forecasting engine.

## 8. Claim ceiling

A successful `PASS_SYNTHETIC_FORECAST_SCORING` receipt means only that the committed ensemble CRPS, WIS, nearest-rank intervals, paired baseline scores/skills, risk–coverage and abstention accounting agree with the synthetic known-truth cases. It does not establish ecological calibration, forecast skill, regional transfer, operational utility, conservation impact or SOTA superiority.


## November 2026 benchmark alignment: NEON ecological forecasting

The NEON Ecological Forecasting Challenge is a useful external evaluation pattern: it reports CRPS at each site and each forecast horizon, includes an overall summary, and compares submissions against a null model. CRPS is a proper scoring rule for predictive distributions/ensembles, rewarding both accuracy and useful spread. See the public evaluation documentation: https://projects.ecoforecast.org/neon4cast-docs/Evaluation.html

The forest tournament should adopt that granularity, then go further where the forest dependence structure requires it:

- report CRPS by site, variable and horizon, not only one pooled average;
- include persistence, climatology and simple statistical/process baselines;
- preserve paired test cases when comparing candidate and baseline scores;
- show worst-site / worst-region skill and calibration;
- add whole-block coverage and uncertainty around empirical block coverage;
- report interval width/WIS at matched realized coverage;
- report shift/abstention and per-regime results without pooling away a failing site;
- freeze the forecast distribution and input/source digests before the validation observation is revealed.

The competitive target is not to invent a new proper score. It is to apply existing proper-scoring discipline to a stricter evidence, dependence, transfer and decision-value contract.


## 16 November 2026: decision-specific useful forecast horizons

A horizon score alone does not state whether a forecast remains useful for a particular decision. The new [#5485 usefulness-limit workstream](https://github.com/Luminous-Dynamics/symtropy/issues/5485) composes this scorer with block qualification and a unitful decision tolerance. It reports a variable-/regime-specific horizon limit as the longest initial contiguous sequence of horizons that beats its frozen baseline, meets interval-coverage and issuance requirements, and stays within decision tolerance in every declared block.

The synthetic development fixture has stable-canopy and post-disturbance mortality contexts, both with a deliberately misleading later recovery after an intermediate failure. It shows why a single pooled CRPS or the longest isolated passing horizon is not enough. The 365-day score may recover after a failed 180-day or 90-day evaluation; the first failing horizon still bounds the qualified prefix for that forecast origin/regime.

See:
- [Usefulness-limit protocol](FOREST_FORECAST_USEFULNESS_LIMIT_V0_1.md)
- [128-case synthetic fixture](cases/forest-forecast-usefulness-synthetic-v0.1.json)
- [Rust adapter over the shared CRPS kernel](validator/src/forecast_usefulness_kernel.rs)

The useful-horizon policy is not itself a confidence guarantee. Empirical limits require rolling-origin held-out hindcasts, adequate independent blocks/sites, uncertainty on block coverage, a preregistered decision tolerance and separate pre-/post-disturbance evaluations.


## 16 November 2026: as-of-time integrity and data latency

The new [#5487 as-of forecast integrity gate](https://github.com/Luminous-Dynamics/symtropy/issues/5487) separates ecological valid time, collection time, source release, QA/QC release, rights-effective time, system ingestion, forecast reference, information cutoff and issue/publication time. A source value can be collected before a forecast but still be unavailable at its issue time because a laboratory assay, satellite composite, quality check or licence release arrived later.

The protocol uses the source/version that was actually available at the fold cutoff. A later correction cannot overwrite the historical input state, and realized future meteorology cannot masquerade as a forecast driver. A driver forecast is allowed only when its own issue time precedes the information cutoff and its target support covers the forecast target.

Methodological anchors:
- ecological forecasting identifies collection-to-availability latency as an operational constraint, especially for lab processing and QA/QC: https://pmc.ncbi.nlm.nih.gov/articles/PMC5816139/
- NEON documents target latency and records submission publication time: https://projects.ecoforecast.org/neon4cast-docs/CronTiming.html
- ecological forecast conventions distinguish `reference_datetime`, target `datetime` and publication time: https://doi.org/10.1002/ecs2.4686

Artifacts:
- [As-of integrity protocol](FOREST_AS_OF_FORECAST_INTEGRITY_V0_1.md)
- [12-case synthetic corpus](cases/forest-as-of-forecast-integrity-synthetic-v0.1.json)
- [Rust as-of validator](validator/src/forest_as_of_integrity_kernel.rs)

All timestamps and source versions in v0.1 are synthetic. The gate does not establish AFEX data release times or a real historical hindcast. Missing availability or rights evidence is NOT_ESTIMABLE/REFUSED, not assumed available at collection time.
