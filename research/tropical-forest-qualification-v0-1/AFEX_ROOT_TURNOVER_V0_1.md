# AFEX root stock–productivity–turnover qualification v0.1

**Issue:** [#5431 — AFEX-ROOT-TURNOVER-001](https://github.com/Luminous-Dynamics/symtropy/issues/5431)  
**AFEX design:** [#5408](https://github.com/Luminous-Dynamics/symtropy/issues/5408)  
**Process trajectory:** [#5340](https://github.com/Luminous-Dynamics/symtropy/issues/5340)  
**Date:** 15 November 2026  
**Status:** synthetic known-truth arithmetic plus literature-summary crosscheck; no raw plot-level reanalysis

## 1. New empirical challenge

Schmeisk-Rosa et al. (2026), *Phosphorus and base cations drive contrasting root dynamics in a central Amazon forest*, Plant and Soil 520:957–972, DOI [10.1007/s11104-026-08303-2](https://doi.org/10.1007/s11104-026-08303-2), analyzed the first two years of AFEX fine-root dynamics.

Its central result is highly relevant to #5340: fine-root standing stock did not show significant nutrient treatment effects, but process rates changed:
- phosphorus increased fine-root turnover by 23% in year 1 and 48% in year 2;
- base cation additions decreased turnover by 24% in year 2;
- in year 2 the P-associated increase was counteracted by cation addition;
- N alone showed no statistically significant difference in the two-year period.

The paper defines turnover as annual fine-root productivity divided by fine-root stock. It computes year 1 productivity over August 2017–September 2018 divided by August 2017 stock; year 2 productivity over September 2018–September 2019 divided by September 2019 stock.

**This is a published result, not our reproduction.** The paper's data-availability statement says productivity data are in EIDC DOI 10.5285/b3a55011-bf46-40f5-8850-86dc8bc4c85d, while the fine-root stock data used in its analysis are available from the corresponding author on reasonable request. No raw plot-level stock/flow pairs are available in this current benchmark fixture.

## 2. Paradigm shift: state is not process

A root standing stock is a state variable. Productivity and mortality/turnover are process quantities. A stable stock can arise from low production and low replacement, or high production and high replacement.

For a declared annual period:

`turnover = annual fine-root productivity / fine-root stock`

`nominal lifespan_days = 365.25 / turnover`

The lifespan expression is the paper's inverse-turnover interpretation, not a universal individual-root survival distribution. A turnover ratio does not by itself identify a cohort survival curve, age distribution or cause-specific mortality process.

Do not let a stock-only score stand in for process fidelity. Report stock error, productivity error, turnover error and lifespan separately.

## 3. Measurement and support contract

- Fine roots: diameter <2 mm.
- Soil depth in the published main analysis: 0–30 cm; component data are collected at 0–10 and 10–30 cm and may be combined only with correct support/weighting.
- Root stock: August 2017 for year 1 and September 2019 for year 2.
- Productivity: in-growth-core collections over August 2017–September 2018 and September 2018–September 2019 respectively.
- The paper describes five cores per plot, split by depth and bulked into plot×depth composites; the ingrowth collections occur at approximately three-month intervals.
- A depth-mismatched numerator and denominator do not define the intended turnover estimand.
- A stock date outside the specified anchor, a mismatched plot, incompatible units or unknown missingness must be refused, not coerced to zero or silently interpolated.

The EIDC data record describes fine-root (<2 mm) mass at two depths and repeated ingrowth-core campaigns, with five cores bulked into one composite per plot and depth. Do not count those cores as independent treatment replicates.

## 3.1 Published design and analysis to reproduce later

The 2026 article describes 32 50 m × 50 m plots in four blocks, with eight treatments per block, at least 50 m between plots and at least 200 m between blocks. Nutrient applications were split into three equal wet-season applications per year, beginning May–June 2017. Its reported rates are preserved source-verbatim in the dataset candidate register; the original field protocol must resolve nutrient-element versus fertilizer-product mass before any dose normalization.

The published mixed-effects analysis used:
- a across-year full-factorial model with P × cations × N × Year and random terms for Year and Block/Plot;
- within-year factorial models with P × cations × N and Block random effect;
- backward elimination for fixed terms while retaining random effects;
- post-hoc pairwise estimated marginal means with Tukey adjustment where interactions remained.

A future empirical reproduction must have two tracks:
1. **As-published reproduction:** reconstruct the article's exact observation pairing, response definitions, model formula, term simplification and post-hoc contrasts.
2. **Preregistered robustness analysis:** keep the full factorial terms, report all seven nutrient contrasts, assess block-level/randomization-based sensitivity and document missingness/interference assumptions.

Do not substitute the second analysis for the first and call it a reproduction. The stock data used in the 2026 turnover denominator are request-only according to the article, so the public EIDC productivity record by itself cannot currently reproduce the full root-turnover analysis.

## 4. Executable v0.1

The synthetic reference uses four cases:
1. process and stock match;
2. exact stock match but candidate productivity/turnover are wrong;
3. exact turnover but stock state is wrong;
4. support/depth mismatch that must be refused.

It checks:
- finite positive stocks and non-negative productivity;
- exact unit and date/period pairing;
- same plot identity and declared depth support;
- derived turnover and lifespan;
- independent stock, productivity and turnover comparisons;
- exact fixture digest in the receipt.

The fixture also transcribes six published summary comparisons (means, SEs, p-values and reported rounded percent changes). The verifier checks only the percentage arithmetic against the reported means. It does not recompute the paper's p-values, standard errors, confidence intervals, root stocks or treatment effects.

Run from repository root:

```bash
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/afex-root-turnover-synthetic-v0.1.json

cargo test --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml
cargo fmt --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- --check
cargo clippy --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml --all-targets -- -D warnings
```

## 5. Research tournament and proposed targets

Once corresponding-author stock values and plot-level productivity files are admitted, compare:
- stock-only model;
- productivity-only model;
- turnover derived from paired stock/productivity;
- process model predicting productivity and stock separately;
- process model predicting turnover/lifespan jointly or through a declared observation model.

Keep the published factorial N/P/cation structure and block-level assignment. Preserve plot as the treatment unit. A future empirical model must reproduce P, cation and P×cation responses on held-out plots/blocks and report uncertainty; it must not regard the paper's means as hidden raw training data.

Proposed targets (not achieved):
- 100% rejection of incompatible period/depth/plot pairing in a frozen adversarial corpus;
- 100% detection of same-stock/different-turnover synthetic cases;
- zero missing-to-zero conversions;
- independent scores for stock, productivity, turnover and lifespan;
- calibrated uncertainty propagated through numerator/denominator covariance once raw measurement uncertainty is admitted;
- replicated treatment response on a separate site or experiment before transfer claims.

## 6. Claim ceiling

Synthetic PASS means only arithmetic and support-pairing logic passed. The published summary effect sizes are source-transcribed literature facts with arithmetic crosschecks, not independent raw-data reproduction. Root stock data used by the 2026 paper are not in the public EIDC dataset according to its data-availability statement. No raw-data reanalysis, causal re-estimation, forest calibration, cross-site transfer or SOTA superiority is claimed.
