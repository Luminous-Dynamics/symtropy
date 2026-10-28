# Lighting MAR adjustment and MNAR tipping-point benchmark v0.1

**Status:** synthetic method diagnostics only. No lighting optimizer, solver or product is evaluated here.

## Objective and estimand

This follow-on adds assumption-aware MAR methods and an explicit MNAR tipping-point diagnostic to the informative-missingness audit. The target remains the arithmetic mean latent percentage energy saving across all independently sampled scene families attempted. Each independent family contributes one outcome; repeat seeds and workplane grid points are not extra independent units.

The experiment distinguishes:
- **technical non-observation** of an otherwise defined latent energy-effect outcome;
- **physical/service non-compliance**, which is a separate observed feasibility endpoint; and
- **unknown / unvalidated** evidence, which cannot be converted to zero energy or a pass.

## Methods

### Complete-case reference

A one-sided Student-t lower bound applied to observed effects only. Its estimand is the observed-case mean. Under MAR or MNAR it is not generally an all-family mean bound.

### Covariate-adjusted regression for MAR

The synthetic data include a fully observed, pre-outcome difficulty covariate \(D\), with a linear conditional mean for the effect: \(\mu + \beta(0.5-D)\). The procedure regresses observed outcomes on centered difficulty and estimates the intercept at mean difficulty \(0.5\). This can restore the all-family mean under the declared linear conditional-mean model when missingness depends on observed difficulty (MAR). It is not generally valid under MNAR, model misspecification, or unrecorded confounding.

### Oracle inverse-probability weighting for MCAR/MAR

Uses the exact response probability from the synthetic data-generating mechanism, which makes this an *oracle benchmark*. It uses a Horvitz-Thompson mean across all attempted families, assigning zero to missing contribution only as part of the estimator \(R_iY_i/\pi_i\), not as an imputed energy observation. In actual work, propensity scores have to be estimated from pre-outcome observed variables; their uncertainty, model misspecification, overlap/positivity and weight instability need to be handled. This oracle procedure is not run under MNAR because the response probability depends on the unobserved effect.

### Bounded worst-case reference

Reuses the predeclared \([L,U]\) outcome interval, replaces missing values by \(L\), then calculates a one-sided Hoeffding lower bound over all attempted independent families. It is robust to the missingness mechanism under valid finite bounds and independence, but can be very conservative and uninformative.

### MNAR tipping point

For each replicate with at least one observed and one missing outcome, the script reports the shift \(\delta\) such that the hypothetical missing-outcome mean equals the observed-case mean plus \(\delta\), and the all-family *point estimate* equals the frozen margin:

\[
\widehat{\mu}(\delta) = \bar{Y}_{obs} + p_{miss}\delta,\qquad
\delta_{tip} = \frac{margin-\bar{Y}_{obs}}{p_{miss}}.
\]

Negative values mean that missing outcomes would need to be lower than observed outcomes by that many percentage points to move the full-sample point estimate down to the margin. This is a sensitivity diagnostic, **not** a confidence interval or significance test. A point-estimate crossing must not be reported as statistically significant.

## Design

The standard run has 96 scenarios: 10, 20 and 40 independent families; four bounded outcome distributions; complete, MCAR, MAR-difficulty and MNAR-low-outcome mechanisms; and null/target mean locations. The default local CLI uses 1,000 repetitions; the included evidence run uses 500 repetitions per scenario. Each rejection rate includes an individual 95% Wilson Monte Carlo interval; intervals are not simultaneous across the scenario family.

## Synthetic findings from the local 500-replicate run

- The maximum null rejection rate across the declared scenario grid was **68.4%** for complete-case Student-t, **56.6%** for the MAR regression procedure, and **34.0%** for oracle IPW within the MCAR/MAR scopes. The bounded Hoeffding reference had zero observed null rejections.
- These maxima are not directly ranked method claims: the scenarios include strongly skewed outcomes for which the Student-t critical-value approximation is itself anti-conservative. The MAR regression and IPW are assumption-dependent estimators, not universal inferential repairs.
- At \(n=20\), clipped-normal-like null effects with MAR difficulty selection, the observed rejection fractions were 8.8% (complete-case), 5.6% (regression), 3.0% (oracle IPW), and 0% (bounded Hoeffding). Mean estimate biases were +0.221, -0.018, -0.061 and -7.522 percentage points, respectively.
- Under the \(n=20\), clipped-normal-like MNAR null, complete-case rejection was 38.2%, and its observed-case mean was biased upwards by 1.075 percentage points. MAR regression also failed under this MNAR stress, with 33.4% rejection and +1.017-point bias. Oracle IPW is deliberately not applicable to MNAR. The median tipping delta was -3.00 percentage points.
- The bound reference prevented observed false positive rejections in this grid but paid for that with very low power. No method is declared a winner.

Every number above is generated from constructed data. The experiment does not estimate real lighting failure rates or establish physical savings.

## Limitations and next work

- The regression and IPW methods use oracle-known functional form / propensity to establish a lower-complexity benchmark. They do not represent a production fitted model.
- The skew stress cases show that fixing missingness bias does not automatically fix type-I error; the inferential procedure requires its own held-out stress validation.
- The tipping delta is point-estimate sensitivity only. Add interval-aware pattern-mixture sensitivity separately; do not quietly impute missing outcomes to manufacture significance.
- A future method-choice study must freeze a confirmatory scenario set before choosing a procedure, and report type-I error, coverage, bias, power and Monte Carlo uncertainty.
- Physical non-compliance, illuminance/service quality, temporal-light-modulation risk and optical calibration remain separate gates.

## References

- Morris, White & Crowther (2019), *Using simulation studies to evaluate statistical methods*, Statistics in Medicine. https://pmc.ncbi.nlm.nih.gov/articles/PMC6492164/
- Campbell et al. (2023), *Key considerations for designing, conducting and analysing a cluster randomised trial*, International Journal of Epidemiology. https://academic.oup.com/ije/article/52/5/1648/7172835
- Little & Rubin, *Statistical Analysis with Missing Data*, 3rd ed. Missingness assumptions determine whether an all-family estimand is identified by observed data.
