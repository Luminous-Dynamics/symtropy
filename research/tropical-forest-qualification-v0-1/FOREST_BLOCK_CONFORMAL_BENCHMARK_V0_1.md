# Block-Aware Forest Conformal Calibration v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Forecast-scoring owner:** [#5277](https://github.com/Luminous-Dynamics/symtropy/issues/5277)  
**Focused work:** [FOREST-CONFORMAL-001 #5285](https://github.com/Luminous-Dynamics/symtropy/issues/5285)  
**Status:** synthetic block split-conformal kernel; designer-declared exchangeability, no ecological calibration

## 1. Research question

A forest dataset may contain thousands of pixel, stem or sensor records but only a handful of genuinely separate site/campaign units. If errors are correlated within a site, counting every stem as an independent calibration point can make the calibration sample look much larger than it is.

This benchmark changes the unit of uncertainty calibration. The v0.1 score is the **maximum absolute residual within each equal-sized spatial/campaign block**. Split conformal then calibrates across block scores, not across individual stems/pixels. The resulting radius is shared across every observation in a future block. That is conservative by design: it asks whether the whole block is covered, not merely whether the average pixel is covered.

This is not a universal fix for dependence. It is valid only when calibration and future blocks are exchangeable under the declared experimental design. Separate group IDs and no leakage are necessary checks, not proof of exchangeability or independence.

## 2. Method and finite-sample rank

Given calibration block scores $s_1,\ldots,s_n$ and target miscoverage $\alpha$, the split-conformal rank is:

`k = \lceil (n+1)(1-\alpha) \rceil`

Sort the scores and choose the $k$th order statistic. If $k>n$, return an unbounded interval (represented as no finite radius), rather than inventing a finite quantile.

This immediately sets a minimum calibration-block count:
- 90% target ($\alpha=0.10$): at least 9 exchangeable blocks;
- 95% target ($\alpha=0.05$): at least 19 exchangeable blocks;
- 99% target ($\alpha=0.01$): at least 99 exchangeable blocks.

These counts refer to calibration **blocks**, not the number of stems, pixels, GEDI footprints or individual observations inside them.

For each equal-sized calibration block $b$, the score is:

`s_b = \max_{i\in b} |y_i - \hat y_i|`

The calibrated prediction interval for every observation in a held-out block is:

`[\hat y_i - q,\; \hat y_i + q]`

where $q$ is the conformal quantile of block scores. If calibration and future blocks are exchangeable, the block-maximum score targets marginal coverage of a whole future block. It does not guarantee conditional coverage for every biome, species, disturbance state or site.

## 3. Why compare pooled and block calibration?

The public synthetic fixture has nine calibration blocks of ten residuals each. Each block contains nine residuals of 0.1 and one block-specific maximum from 1 through 9.

At nominal 90%:
- naïve pooled observation-level calibration uses 90 residuals and returns radius 1;
- block-maximum calibration uses nine block scores and returns radius 9.

The pooled quantile treats within-block residuals as if they were exchangeable calibration units. The block quantile treats the block as the unit. The width increase from 2 to 18 is not an automatic victory: it is the cost of targeting all-observations-in-block coverage under this particular synthetic dependence structure. A real tournament must compare interval score / WIS at matched empirical coverage and on independent held-out blocks.

## 4. Synthetic test blocks and shift control

Three held-out test blocks have ten observations each:
- in-domain A: all ten residuals fall inside the block radius;
- in-domain B: nine of ten fall inside, but one exceeds it;
- known-shift C: nine of ten fall inside, but the final shifted residual exceeds the radius.

The independent arithmetic oracle yields:

| Measure | Pooled radius 1 | Block radius 9 |
|---|---:|---:|
| Interval width | 2 | 18 |
| Observation coverage | 90.00% | 93.33% |
| Whole-block coverage | 0/3 = 0% | 1/3 = 33.33% |
| In-domain whole-block coverage | Not separately meaningful for pooled method | 1/2 = 50% |
| Known-shift whole-block coverage | Not separately meaningful for pooled method | 0/1 = 0% |

The test set is deliberately tiny and engineered; those percentages are not estimates of any forest's coverage probability. The shift result demonstrates only that a calibrated in-domain radius does not automatically transfer to a changed regime. The fixture labels the shifted block from known synthetic truth; it does not implement a prospective shift detector.

The kernel also reports the number of calibration blocks, observations per block, both interval widths, both observation-coverage measures, both all-in-block coverage measures, and separate declared in-domain / known-shift group coverage.

## 5. Leakage and comparability gates

Version 0.1 requires:
- training group IDs disjoint from calibration group IDs;
- training group IDs disjoint from test group IDs;
- calibration group IDs disjoint from test group IDs;
- unique calibration block IDs and group IDs;
- unique test block IDs and group IDs;
- finite, nonnegative calibration residuals;
- finite predictions and observations;
- equal observation counts across calibration and test blocks.

Equal-sized blocks are restrictive. Real plots and EO tiles often differ in support and sample count. A production version should either define a comparable block summary/score with a justified target unit or implement weighted/conservative block procedures; it must not silently compare maxima from blocks with different numbers of opportunities for an extreme residual.

The exchangeability basis is carried as an experimenter declaration. The receipt explicitly returns `DESIGNER_DECLARED_NOT_STATISTICALLY_VERIFIED`.

## 6. Industry / SOTA comparison

The benchmark should compare methods by data regime and stated assumptions, not by method name alone:

| Method family | Strength | Key qualification / comparison |
|---|---|---|
| Ordinary split conformal | Simple finite-sample marginal coverage under exchangeability | Unit of exchangeability matters; naïve per-pixel split is a negative control for block-correlated data |
| Block-maximum split conformal | Targets simultaneous coverage of a whole block under exchangeable block scores | More conservative, requires enough independent blocks and comparable block sizes |
| Spatial conformal (e.g. `spconform`) | Localized spatial weighting for point-referenced or areal data; includes spatial-strata, boundary and Moran's I diagnostics | Compare both marginal and spatial-conditional diagnostics; do not assume identical guarantees across methods |
| EnbPI | Sequential ensemble prediction intervals without data splitting | Published theory relies on assumptions including strongly mixing stochastic errors; benchmark only in suitable temporal settings |
| SPCI / time-series conformal | Models time-ordered residual dependence | Requires a genuinely sequential holdout and clear temporal assumptions |
| Adaptive Conformal Inference | Online adaptation under distribution shift | Must expose sustained local miscoverage and interval efficiency; signed long-run averages can hide persistent undercoverage |

Relevant references:
- Xu & Xie, *Conformal Prediction for Time Series*, IEEE TPAMI 2023: https://doi.org/10.1109/TPAMI.2023.3272339
- Spatial/spatio-temporal conformal `spconform` diagnostic and workflow: https://cran.r-project.org/web/packages/spconform/vignettes/spconform-intro.html
- Zhai, Cheng & Wu, *Conformal Coverage of Time Series: Validity and Inference* (September 2026 preprint): https://arxiv.org/abs/2609.33868
- Vaze, *Simultaneous Coverage and Efficiency Guarantee in Online Conformal Prediction* (July 2026 preprint): https://arxiv.org/abs/2607.26577
- 2025/2026 CP survey and data-regime taxonomy: https://doi.org/10.1145/3736575

## 7. Proposed empirical benchmark targets

These are future goals, not achieved results:
1. Evaluate 90% and 95% whole-block coverage on sufficiently many independent held-out blocks, with block-bootstrap 95% confidence intervals.
2. Require empirical block coverage within ±5 percentage points of nominal in at least two regional holdouts before describing transfer as qualified.
3. Report interval width / WIS at matched realized block coverage; never reward coverage obtained only by arbitrarily wide intervals.
4. Publish marginal observation coverage and whole-block coverage side by side.
5. Report longest run and maximum rolling-window amount of undercoverage, not only aggregate averages.
6. Report worst-site, worst-campaign, worst-region and disturbance-regime coverage.
7. Compare block split conformal with spatially weighted, EnbPI/SPCI and adaptive methods under the data regimes for which their assumptions are appropriate.
8. Include calibration-block count, number of observations per block, runtime, memory, latency, and data/field cost.

## 8. Next research slices

Not implemented:
- spatially weighted or graph-neighbourhood conformal;
- weighted conformal under a qualified covariate-shift model;
- online ACI/AgACI or sliding-window adaptation;
- block-bootstrap confidence intervals for realized block coverage;
- PIT, WIS / interval score and spatial residual Moran's I;
- variable-width or area-weighted block scores;
- empirical calibration, regional transfer or prospective coverage.

The next strongest experiment is a spatial/temporal blocking tournament with independent real blocks, once a rights-cleared dataset and a credible observation operator are admitted. The block definition must be frozen before model fitting, and all tuning must remain inside the training/calibration split.

## 9. Claim ceiling

A successful `PASS_SYNTHETIC_BLOCK_CONFORMAL` receipt means only that the finite-sample rank, pooled-vs-block radii, group separation and synthetic coverage calculations agree with the public fixture. It does not establish actual block exchangeability, independent field units, conditional spatial coverage, shift detection, ecological calibration, regional transfer or SOTA superiority.
