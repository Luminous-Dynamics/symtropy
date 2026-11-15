# AFEX plot-paired ratio estimand and uncertainty benchmark v0.1

**Issue:** [#5443 — AFEX-RATIO-ESTIMAND-001](https://github.com/Luminous-Dynamics/symtropy/issues/5443)  
**Composes:** [#5431 root stock–turnover](https://github.com/Luminous-Dynamics/symtropy/issues/5431), [#5340 process trajectories](https://github.com/Luminous-Dynamics/symtropy/issues/5340), [#5360 practical identifiability](https://github.com/Luminous-Dynamics/symtropy/issues/5360), [#5408 AFEX intervention](https://github.com/Luminous-Dynamics/symtropy/issues/5408)  
**Date:** 15 November 2026  
**Status:** synthetic known-truth; no AFEX raw plot pairs ingested

## 1. The estimand is part of the model

The 2026 AFEX root paper defines turnover at plot level as annual fine-root productivity divided by fine-root standing stock for 0–30 cm. It uses a specific year/depth pair: year 1 productivity from August 2017–September 2018 divided by August 2017 stock; year 2 productivity from September 2018–September 2019 divided by September 2019 stock. The analysis uses 32 plots and fits mixed-effects models to plot-level turnover. Source: https://doi.org/10.1007/s11104-026-08303-2

For plot (i):
`tau_i = P_i / S_i`

There are at least four different aggregate quantities:
- **equal-plot mean turnover:** `mean(tau_i)`;
- **pooled ratio:** `sum(P_i)/sum(S_i)`, which weights plots by stock;
- **mean plot lifespan:** `mean(365.25/tau_i)`;
- **inverse mean turnover:** `365.25/mean(tau_i)`.

They are generally not equal. Replacing one with another changes the estimand. In particular, `mean(1/tau_i)` is not `1/mean(tau_i)`. A benchmark must expose the chosen estimand and never silently substitute it.

## 2. Pairing and covariance matter

The same marginal stock and productivity distributions can produce different plot-level turnover when the within-plot pairing changes. The fixture contains two four-plot synthetic groups:
- stock `[1,2,3,4]`, productivity `[2,4,6,8]`;
- same stock, same productivity marginal values, but productivity assigned `[8,6,4,2]`.

Both have mean stock 2.5, mean productivity 5, and pooled ratio 2. The first has mean plot turnover 2; the second has mean plot turnover 3.20833. Their stock–productivity sample covariance has opposite signs. Therefore, a data pipeline that loses plot identity or pairs the wrong stock and productivity rows can change the reported ecological conclusion even when the marginal distributions are unchanged.

## 3. Uncertainty is estimator-specific

For the ratio of sample means `R = mean(P)/mean(S)`, a first-order delta-method approximation is:

`Var(R) ≈ [Var(P)/μS² + μP² Var(S)/μS⁴ − 2 μP Cov(P,S)/μS³] / n`

The covariance term is not optional. It depends on paired observations. This is an approximation for the ratio-of-means estimator, not a Fieller confidence set, not a finite-sample coverage guarantee, and not the uncertainty for the equal-plot mean of ratios.

The benchmark separately reports:
- the sample SE of plot-level turnover ratios;
- the delta-method SE of the pooled ratio-of-means;
- sample covariance of plot-level stock and productivity;
- mean plot lifespan and inverse mean turnover.

Do not treat these SEs as interchangeable. With small plot counts, block-aware inference and raw measurement uncertainty remain necessary.

## 4. Executable v0.1

The corpus has two same-marginal/different-pairing synthetic groups and one numerator/denominator plot-ID mismatch that must be refused. The validator requires:
- exact plot ID match for stock and productivity;
- exact year-specific stock date and productivity interval;
- exact 0–30 cm depth and units;
- positive stock and non-negative productivity;
- one row per plot and unique plot IDs;
- plot as independent unit and block retained.

The receipt binds the exact fixture bytes and reports all estimands independently. It does not ingest AFEX data or fit a treatment model.

Run from repository root:

```bash
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/afex-ratio-estimand-synthetic-v0.1.json

cargo test --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml
cargo fmt --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- --check
cargo clippy --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml --all-targets -- -D warnings
```

## 5. Proposed empirical acceptance targets

- Reproduce the paper's plot-level turnover definition and year/depth support exactly.
- Report the equal-plot mean turnover as the published response; report pooled ratio only as a distinct secondary estimand.
- Preserve within-plot stock/productivity covariance and block structure.
- Estimate measurement and sampling uncertainty separately; compare delta, Fieller or paired bootstrap methods on admitted raw data.
- Verify lifespan calculation at plot level before group aggregation; distinguish an inverse-rate proxy from actual cohort survival.
- Perform matched plot/block holdout and show the consequences of support/identity corruption with adversarial controls.

## 6. Claim ceiling

Synthetic PASS establishes only ratio arithmetic, estimator distinctions, covariance handling and source support pairing. The 2026 paper's published means/SEs/p-values remain literature-transcribed. Its fine-root stock denominator data are available from the corresponding author on reasonable request; the public EIDC productivity record alone does not permit raw turnover reproduction. No raw-data reanalysis, empirical uncertainty calibration, causal effect estimate or SOTA superiority is claimed.
