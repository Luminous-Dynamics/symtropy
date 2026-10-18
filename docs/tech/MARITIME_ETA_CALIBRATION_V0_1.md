# MARITIME-ETA-CAL-001: calibrated arrival uncertainty envelopes

**Status:** executable split-conformal reference and synthetic unit suite authored; exact-head Rust CI pending.  
**Implementation:** crates/domains/symtropy-maritime-evidence/src/arrival_calibration.rs  
**Purpose:** estimate uncertainty around an ETA and report coverage, interval width, and abstention separately.  
**Tracking:** [MARITIME-ETA-CAL-001 #1973](https://github.com/Luminous-Dynamics/symtropy/issues/1973)

## Paradigm shift

Do not treat source disagreement as the only measure of uncertainty. Two sources may agree and both be wrong; a single source may have an error distribution that can be quantified from previously observed forecasts. The proposed next layer is a calibrated ETA envelope whose width is learned from residuals that were genuinely available before a frozen cutoff.

This module deliberately implements only the finite-sample split-conformal baseline. It does not silently claim that time-series forecast errors are exchangeable, nor does it adapt the radius automatically after drift.

## Method

For each historical calibration example, store:
- forecast identity and the time the forecast was issued;
- predicted ETA;
- actual event time;
- time the actual event became available to the calibration system.

A calibration cutoff is explicit. The function rejects duplicated or empty IDs, forecasts issued after the cutoff, actuals recorded after the cutoff, forecasts issued after their own actual observation/event, and impossible event/recording chronology.

For a target miscoverage level alpha and n eligible calibration residuals, use the one-indexed absolute-residual order statistic at:

ceil((n + 1) * (1 - alpha))

If that rank exceeds n, the mathematically conservative quantile is +infinity. The module represents this as no finite radius, not as a clipped maximum or a guessed interval. Any scheduling caller must choose an explicit fallback or abstain.

The fitted radius is symmetric around a point prediction. This is a reference baseline: production work may need asymmetric residuals, lead-time-specific calibration, vessel/source/port strata, seasonal regime modeling, or multi-stage forecasts.

## Why this research matters

- Split-conformal methods can give finite-sample marginal coverage under exchangeability; they do not promise conditional coverage in every port, season, vessel class, or lead-time bucket.
- Covariate shift and online distribution shift are separate problems. See the 2024 paper on doubly robust prediction-set calibration under covariate shift: https://academic.oup.com/jrsssb/article/86/4/943/7618755
- For distribution shift over time, see Gibbs and Candès, *Conformal Inference for Online Prediction with Arbitrary Distribution Shifts*, JMLR 25(162), 2024: https://www.jmlr.org/papers/v25/22-1218.html
- DCSA’s Port Call Standard emphasizes common operational event semantics and ongoing updates from Estimated to Requested to Planned and Actual. Calibration must condition on the correct event meaning rather than pooling unlike event types: https://dcsa.org/standards/port-call/documentation-port-call

These references motivate future work; no paper implementation, maritime forecast dataset, or live API is included in this PR.

## Current executable slice

Nine Rust tests cover:
1. exact finite-sample rank for nine synthetic residuals at 10% nominal miscoverage;
2. insufficient sample size producing an infinite/unavailable radius;
3. invalid target levels and empty calibration;
4. cutoff leakage and invalid forecast/actual chronology;
5. duplicate IDs and temporal ordering;
6. symmetric interval bounds and overflow rejection;
7. held-out coverage, mean interval width, and issuance rate as separate metrics;
8. explicit abstention accounting when the radius is unbounded;
9. calibration output invariance under input permutation.

All timestamps and residuals in the tests are synthetic. The test timestamps are deliberately constructed so forecasts precede event times and actuals are observed before the calibration cutoff unless a negative control intentionally violates that rule.

## Proposed benchmark gates (not measured)

- Exact order-statistic agreement with an independent integer-arithmetic oracle across alpha values and sample counts, including ranks that require an infinite interval.
- Zero calibration leakage across randomized cutoffs and chronological holdouts.
- Report empirical coverage, interval width, issuance rate and abstention separately at 90%, 95%, and 99% nominal coverage.
- Report the same metrics stratified by port, source, vessel class, forecast lead-time, day/night, season and regime—only where sample sizes justify it.
- Detect and report coverage decay after distribution shifts. Do not silently re-fit using evaluation labels before they are observed.
- Benchmark against point ETA, fixed-width intervals, historical quantiles and split-conformal intervals on the same chronological holdout.
- No cross-port generalization claim without a held-out-port experiment; no operational claim without rights-cleared historical or shadow-mode data.

## Authority boundary

The module outputs uncertainty intervals and evaluation metrics only. It does not select berths, negotiate requested/planned timestamps, change speeds, dispatch tug or pilot services, create bookings or settle flexibility payments. Any robust scheduling integration must keep the forecast interval, source-conflict status, and operational authority as separate dimensions.

The next stage after synthetic qualification is an offline rolling-origin evaluation with an immutable calibration cutoff and a separately sealed evaluation period. Only after that should adaptive online calibration or decision-focused scheduling be considered.
