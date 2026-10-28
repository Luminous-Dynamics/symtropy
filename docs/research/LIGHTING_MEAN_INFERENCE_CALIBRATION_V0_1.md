# Lighting mean-inference calibration bake-off v0.1

**Status:** completed local synthetic calibration run; **no method passes** the frozen confirmatory gates.  
**Related work:** Symtropy PR #2831 (fitted MAR / MNAR interval sensitivity), issue #2832 (calibration acceptance gates).  
**Scope:** inference calibration for arithmetic mean percentage-point energy effects across independent scene families. This is not an optical or energy-performance experiment.

## 1. Aim and decision estimand

For independent scene-family effects (Y_i), compare one-sided lower-confidence-bound procedures for the arithmetic mean. Superiority is declared only if the lower bound exceeds the predeclared margin (m=5) percentage points. The confirmation DGP uses a family-effect standard deviation of 3 percentage points. The target alternative is fixed at (m+1.5sigma=9.5) percentage points before the methods are evaluated.

Repeated seeds, lux-grid points, and repeated solver samples inside one scene family are not additional independent families. Missingness and MAR/MNAR are addressed by the adjacent #2831 workstream; this benchmark holds outcome values complete to isolate finite-sample mean-inference behavior. The methods tested here do not therefore establish robustness to outcome-dependent missingness.

## 2. Methods and frozen confirmation grid

Three procedures target the same arithmetic mean:

1. One-sample Student-t lower bound.
2. Percentile-bootstrap lower bound for the arithmetic mean.
3. Studentized bootstrap-t lower bound for the arithmetic mean.

The grid was fixed at 13 cells with seed `20261029`: five null cells at n=12, three additional null cells at n=24 (normal, left-skew two-point, and Student-t(3)), and five target cells at n=24. Null cells use 5,000 outer Monte Carlo replications each. Target cells use 1,000 replications each. Each dataset uses 199 bootstrap resamples. Distributions are normal, left-skew two-point, right-skew two-point, symmetric rare-extreme, and Student-t with 3 degrees of freedom standardized to unit variance.

A faster vectorized implementation is used when NumPy is available. The bootstrap statistics and quantile definitions mirror the repository comparator; a unit test compares the standard-library-optimized procedure to the source comparator on identical data and RNG streams. Critical t quantiles are cached by degrees of freedom and alpha, because the quantile does not change within a scenario. The report records both the benchmark source digest and imported comparator digest. The downloadable standalone package includes a local source mirror to run outside the repository; repository execution will record the real repository file digest.

## 3. Monte Carlo uncertainty and gates

A one-sided family-wise Monte Carlo error budget of 0.05 is Bonferroni-adjusted over 24 method/null-scenario comparisons for false-superiority and 39 method/scenario coverage comparisons. The Wilson score limits use those adjusted one-sided critical values.

Frozen gates:

- The simultaneous upper confidence bound on null false-superiority probability must be at most 0.06 in every null scenario.
- For the one-sided lower confidence bounds used by the superiority decision, the simultaneous Monte Carlo lower confidence bound on the finite-bound coverage probability P(L <= true mean) must be at least 0.94 in every scenario. This is one-sided lower-bound coverage, not two-sided interval coverage.
- Unresolved lower bounds and nonfinite/failed analyses must each remain at or below 1% per method/scenario.
- Report target power, but never use power to override error calibration or coverage failures.

A negative-infinite lower bound is an explicit unresolved result produced by conservative handling of a degenerate sample or bootstrap-t distribution. It counts as no rejection for false-superiority and power denominators, but is not called a finite confidence bound. Its rate is reported separately and is part of the acceptance gate; it is never silently excluded.

## 4. Result summary

| Procedure | Maximum simultaneous null false-superiority upper bound | Minimum simultaneous one-sided lower-bound coverage lower bound | Maximum unresolved-bound rate | Mean target power | Minimum target power | Gate |
|---|---:|---:|---:|---:|---:|---|
| Student-t | 6.089% | 92.604% | 28.82% | 94.96% | 91.5% | **FAIL** |
| Percentile bootstrap | 10.080% | 89.852% | 28.82% | 94.96% | 91.5% | **FAIL** |
| Studentized bootstrap-t | 8.617% | 77.611% | 89.86% | 62.14% | 36.9% | **FAIL** |

No method passed. Student-t was closest on false-superiority calibration but missed the predeclared 6% upper-bound criterion and the 94% one-sided lower-bound coverage criterion. Percentile bootstrap was anti-conservative in several null scenarios. Studentized bootstrap-t was less usable on discrete/rare-extreme samples because degenerate bootstrap resamples often forced an infinite upper t critical value, which gives an unresolved `-infinity` bound. The two-point distributions also create frequent zero-variance original samples at n=12. These are genuine limitations of the tested procedures/finite sample, not a reason to weaken the gate.

Representative cell results:

- n=12 normal null: false-superiority was 4.90% (Student-t), 7.54% (percentile bootstrap), and 5.50% (studentized bootstrap-t).
- n=12 Student-t(3) null: 4.74% (Student-t), 8.86% (percentile bootstrap), 7.48% (studentized bootstrap-t).
- n=12 left-skew two-point: 27.7% of Student-t and percentile-bootstrap attempts yielded an unresolved `-infinity` bound; the studentized bootstrap-t unresolved rate was 89.68%. The zero rejection rate is not evidence of a good test without considering this non-conclusion rate.
- n=24 normal null: all three had observed false-superiority near 5%, but simultaneous worst-case bounds and coverage gates over the full scenario family still failed.

Monte Carlo rates here are simulation estimates. The report includes per-cell simultaneous Wilson bounds, all attempted denominators, finite one-sided lower-bound coverage, target power, nonfinite/failure counts, unresolved-bound counts, deterministic seed schedule and SHA-256 source receipts. No method winner is selected from the same grid used for the stress test.

## 5. Why this is a paradigm shift

The evaluator treats evidence reliability as an optimization constraint rather than a reporting afterthought. A method cannot win on raw energy-saving estimate or statistical power alone; it must show calibrated false-superiority rates, one-sided lower-bound coverage, and an acceptable rate of no-decision outcomes. A failed or unresolved inference is not transformed into a positive result, and a conservative method is not called best merely because it never rejects.

This layer complements the preceding informative-missingness work rather than replacing it. Even a well-calibrated inference method under complete data would still need sensitivity to MCAR/MAR/MNAR selection, independent scene-family generalization, and separate photometric feasibility checks.

## 6. Industry and SOTA comparison plan

The current study does not compare lighting systems or software. The following stages remain mandatory before a SOTA claim:

1. Pin the Radiance version, scene geometry, material/spectral properties, ray-tracing settings, grid coordinates, and candidate/measurement receipt hashes.
2. Compare Symtropy against conventional search/engineering baselines and commercial lighting workflows (including DIALux evo) under a matched scene corpus and compute budget.
3. Report energy and optical quality separately: illuminance average/minimum, uniformity, glare, colour quality, temporal light modulation, service hours, and power/energy. The exact metrics and limits must be specified for the intended use case, not assumed universal.
4. Evaluate held-out building/room families and daylight/luminaire conditions; do not split near-duplicate rooms across development and holdout.
5. Validate solver outputs against independently parsed raw grids and calibrated lux/power measurements. A statistically valid comparison cannot repair an incorrect optical engine or uncalibrated sensor.
6. Only report superiority if the method clears the statistical gates, the candidate passes the predeclared optical constraints, and measured end-use energy confirms the improvement.

## References

- Morris, White & Crowther (2019), “Using simulation studies to evaluate statistical methods”, *Statistics in Medicine*, 38:2074–2102. The paper recommends defining aims, data-generating mechanisms, estimands, methods and performance measures (ADEMP), and reporting Monte Carlo uncertainty: https://pmc.ncbi.nlm.nih.gov/articles/PMC6492164/.
- Radiance `rtrace` reference manual: https://radsite.lbl.gov/radiance/man_html/rtrace.1.html.
- DIALux official downloads and release information: https://www.dialux.com/en-GB/download.

## Reproduction

From the repository root:

```sh
cd tools/lighting-study-plan
python3 -m unittest -v test_calibrate_mean_inference_confirmation.py
cd ../..
python3 tools/lighting-study-plan/calibrate_mean_inference_confirmation.py \
  --null-replicates 5000 --target-replicates 1000 \
  --bootstrap-replicates 199 --seed 20261029 \
  --backend numpy --output results/mean-inference-calibration-confirmation.json
python3 -m json.tool results/mean-inference-calibration-confirmation.json >/dev/null
```

The full simulation is an explicit manual benchmark, not a CI pass. The new workflow runs only the unit tests. Queued or missing actions are not evidence that the full benchmark ran.
