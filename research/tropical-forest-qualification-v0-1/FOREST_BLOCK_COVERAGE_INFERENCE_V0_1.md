# Realized Block-Coverage Inference v0.1

**Tracking:** [FOREST-COVERAGE-INFERENCE-001 #5319](https://github.com/Luminous-Dynamics/symtropy/issues/5319)  
**Conformal owner:** [FOREST-CONFORMAL-001 #5285](https://github.com/Luminous-Dynamics/symtropy/issues/5285)  
**Forecast owner:** [FOREST-PROB-FORECAST-001 #5277](https://github.com/Luminous-Dynamics/symtropy/issues/5277)  
**Status:** bounded Rust kernel and public synthetic arithmetic corpus; independent-block assumption is not verified

## 1. Why a second uncertainty quantity is needed

The block-conformal kernel returns a calibrated interval radius and an observed held-out block-coverage fraction. Those are not the same as uncertainty about the realized coverage estimate.

For the existing synthetic example, one of three test blocks is fully covered:

- observed whole-block coverage: 1/3 = 33.33%;
- number of independent test units available to estimate block coverage: at most three, if the declared units are in fact independent;
- within each block: ten pixel/stem-like residual observations, which do not turn three blocks into thirty independent coverage trials.

A point estimate alone hides how little the test sample can tell us. A within-block observation bootstrap would create pseudo-replication and artificially narrow the uncertainty.

This work adds an evaluation-side interval for the mean of held-out block-coverage indicators. It does not change the conformal threshold, prediction interval, or ecological model.

## 2. First executable baseline

For each held-out block b, define:

X_b = 1 if every scored observation in block b is covered, otherwise 0.

The point estimate is:

p_hat = (sum_b X_b) / n

Under independent block outcomes, Hoeffding's inequality gives a two-sided confidence interval for the average block success probability:

epsilon = sqrt(ln(2 / delta) / (2 n))

[max(0, p_hat - epsilon), min(1, p_hat + epsilon)]

where delta = 1 - confidence_level.

The bound does not require identically distributed block outcomes to bound their average expected success probability. Interpreting that average as a single common future-block coverage probability still requires a defensible sampling design. It is deliberately conservative and serves as a portable finite-sample baseline, not as the final forest-specific method.

The Rust API is coverage_inference_kernel::hoeffding_block_coverage_interval. The return value includes the block counts, observed fraction, confidence level, radius, clipped bounds, method, and the explicit status:

INDEPENDENT_BLOCK_ASSUMPTION_DECLARED_NOT_VERIFIED

Distinct block IDs are not proof of independence. The function cannot estimate spatial correlation length, campaign dependence, shared sensor ancestry, or temporal long memory. If the design cannot justify independent evaluation blocks, this interval must not be promoted as dependence-corrected uncertainty.

Pass block-level counts. Passing stem- or pixel-level counts for correlated records is pseudo-replication and invalidates the independent-block interpretation.

## 3. What the synthetic cases show

The frozen corpus is cases/forest-block-coverage-inference-synthetic-v0.1.json.

| Held-out outcomes | Point coverage | 95% Hoeffding interval | Interpretation |
|---|---:|---:|---|
| 1/3 blocks | 33.33% | [0%, 100%] | Three blocks cannot qualify nominal coverage |
| 0/3 blocks | 0% | [0%, 78.41%] | Zero observed successes still leaves substantial uncertainty |
| 3/3 blocks | 100% | [21.59%, 100%] | Three successes do not establish high population coverage |
| 18/20 blocks | 90% | [59.63%, 100%] | Twenty units remain weak evidence |
| 90/100 blocks | 90% | [76.42%, 100%] | One hundred units remain imprecise under this conservative bound |
| 702/738 blocks | 95.12% | [90.12%, 100%] | Hundreds of independent units are needed for a narrow distribution-free bound |

The radius at 738 blocks is approximately 4.999 percentage points at 95% confidence. This is a property of the conservative Hoeffding bound, not a universal minimum sample size: exact binomial or justified dependence-aware methods may be more efficient.

The corpus also includes a negative control that runs the same arithmetic on 28/30 observations. That output is intentionally marked REJECT_AS_BLOCK_COVERAGE_INFERENCE. Its smaller radius is not evidence; it is what pseudo-replication looks like.

The numeric corpus values are independent reference calculations of the stated formula. They are not reported Rust execution results.

## 4. Separate the three uncertainty questions

1. **Prediction uncertainty:** how wide should the interval for one future observation or block be? This is the concern of the conformal / forecast kernel.
2. **Calibration-conditional uncertainty:** how variable is the realized conditional coverage when the calibration sample and its fitted threshold change? This is a law over possible calibration samples.
3. **Evaluation uncertainty:** how much can the observed held-out block-coverage estimate vary because only a finite number of evaluation units were sampled? This is the concern of this kernel.

They are related, but one cannot substitute for another. A conformal radius is not a confidence interval for observed coverage. A binomial-style interval over test blocks is not a characterization of the random calibration threshold.

## 5. Competitive comparison and next statistical steps

| Comparator | What it contributes | Correct comparison boundary |
|---|---|---|
| Hoeffding block interval (this slice) | Finite-sample, distribution-free bound for independent bounded block outcomes | Conservative baseline; independence remains an external design assumption |
| Clopper–Pearson / exact binomial | Exact binomial coverage interval under iid Bernoulli block indicators | Compare sharpness and small-sample behaviour; do not use if the sampling design does not justify iid units |
| Wilson score interval | Practical approximate binomial interval | A comparator, not a substitute for exact or dependence-aware inference |
| Block bootstrap | Sampling uncertainty under an explicit resampling/dependence design | Resample the highest credible independent cluster or contiguous temporal block, never individual pixels by default |
| Transported Beta laws | Calibration-conditional coverage distribution; separates test-side shift from calibration dependence | Distinct target from test-set coverage uncertainty; include clustered calibration simulations |
| Temporal functional-dependence inference | Realized coverage standard errors / limiting laws under stated time-series assumptions | Use only for genuinely sequential data and its assumptions |
| Spatially localized conformal | Distance- or graph-weighted local calibration and spatial diagnostic strata | Compare conditional/spatial behaviour, boundary effects and residual Moran's I |
| Online conformal | Rolling coverage and interval-size control under changing streams | Report sustained / rolling undercoverage and efficiency; long-run averages can hide local failure |

Current research references:

- Ramos, Graziadei & Cabezas (2026), *Conformal Prediction via Transported Beta Laws*, PMLR 329: https://proceedings.mlr.press/v329/r-ramos26a.html. Studies calibration-conditional coverage distributions and how shift / clustered dependence deform the iid Beta reference.
- Zhai, Cheng & Wu (2026), *Conformal Coverage of Time Series: Validity and Inference*, arXiv:2609.33868: https://arxiv.org/abs/2609.33868. Studies realized coverage inference under temporal dependence, including block-based standard errors and long-memory cases.
- Barber & Pananjady (2026), *Predictive inference for time series: why is split conformal effective despite temporal dependence?*, ALT/PMLR 313: https://proceedings.mlr.press/v313/barber26a.html. Quantifies temporal-dependence effects using a switch coefficient and stationary beta-mixing settings.
- Liu, Dobriban & Orabona (2026), *Online Conformal Prediction via Universal Portfolio Algorithms*, ICML/PMLR 306: https://proceedings.mlr.press/v306/liu26y.html. Provides a parameter-free online conformal procedure with finite-time coverage and interval-size trade-off analysis.
- spconform 0.1.2, CRAN publication 19 October 2026: https://cran.r-project.org/web/packages/spconform/. Implements localized conformal calibration for point-referenced and areal spatial data, with spatial-stratum, boundary and Moran's I diagnostics.

These papers and tools establish a stronger comparator landscape; they do not prove that any one method is universally best for tropical-forest monitoring.

## 6. Benchmark protocol

For every nominal level in 50%, 80%, 90% and 95%:

- Freeze the ecological target, observation operator, spatial/temporal support, block geometry and test split before fitting.
- Count genuinely independent test units, not records, stems, pixels or repeated scans.
- Report point block coverage and its interval, nominal coverage, calibration block count, test block count, and within-block observation count separately.
- Report every predeclared biome, site, campaign, sensor, disturbance and acquisition-era stratum separately. Do not allow a large easy stratum to mask a small failing one.
- For time-ordered monitoring, report rolling-window coverage, longest consecutive undercoverage run and maximum rolling undercoverage.
- Compare interval width / WIS only at matched realized coverage; wider intervals are not a free win.
- Publish spatial residual diagnostics, block-size sensitivity, runtime, memory, calibration latency and field/data cost.

Proposed future acceptance target: on at least two independent regional holdouts, the 95% confidence lower bound for a predeclared 90% block-coverage target should be no lower than 85%, with adequate block counts and a justified design. This is a proposed empirical gate, not a result or a universal ecological threshold.

## 7. Limitations and explicit non-claims

Not implemented by this slice:

- Exact Clopper–Pearson, Wilson or calibrated Bayesian intervals.
- Block-bootstrap / dependent-time-series standard errors.
- Spatially weighted or graph-neighbourhood conformal.
- Calibration-conditional Beta-law simulation or transport-distance diagnostics.
- Hierarchical uncertainty across nested regions, sites and campaigns.
- A production receipt path that reads this corpus.
- Independence / exchangeability audits against actual ecological sampling designs.

A synthetic Rust unit-test pass will mean only that the bounded Hoeffding arithmetic and refusal controls behave as specified. No real field coverage, independence, distribution shift guarantee, ecological calibration, or SOTA win is established.
