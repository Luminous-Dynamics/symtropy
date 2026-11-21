# Orbital PLP Trend Diagnostic Benchmark v0.1

**Status:** synthetic exact-conditional trend diagnostic; exact-head hosted validation pending.  
**Owner:** [ORBITAL-RELIABILITY-PLP-DIAGNOSTIC-CALIBRATION-001 #6636](https://github.com/Luminous-Dynamics/symtropy/issues/6636).  
**Claim ceiling:** synthetic constant-intensity-vs-PLP trend diagnostic only.  
**Authority / fit acceptance / reliability claims:** always false.

## The hypothesis being tested

This profile answers one narrow question:

**Under the declared complete-window repairable-item PLP assumptions, is the constant-intensity boundary beta = 1 inconsistent with the observed event-time history?**

It is not a goodness-of-fit test for the full PLP family. A genuine PLP with beta = 0.6 is a trending process, not automatically a poor PLP fit. Three questions must remain separate:

1. **Trend:** is beta = 1 inconsistent with the event history?
2. **PLP adequacy:** does some PLP model describe the history, rather than a piecewise, heterogeneous, dependent or otherwise misspecified process?
3. **Prediction:** does a model fitted to an earlier history predict future events on a pre-registered holdout?

V0.1 implements only the first question. PLP adequacy tests and holdout prediction are separate subsequent profiles under #6636.

## Exact conditional null distribution

For a complete time-terminated NHPP history under the homogeneous constant-intensity null (beta = 1), and conditional on observing n events in exposure window T, the event locations are iid Uniform(0, T).

Define:

S = sum_i log(T / t_i)

Then under the null:

S ~ Gamma(n, 1),    beta_hat = n / S

The profile likelihood-ratio statistic comparing the fitted PLP against the constant-intensity boundary is:

D = 2n * (r - 1 - log(r)),    r = S/n = 1/beta_hat

For a two-sided trend test, solve for the lower and upper roots rL < 1 < rU such that:

rL - 1 - log(rL) = rU - 1 - log(rU) = r - 1 - log(r)

The exact conditional p-value is:

p = P(Gamma(n,1) <= n*rL) + P(Gamma(n,1) >= n*rU)

The Rust implementation evaluates the regularized incomplete gamma function using a series or continued fraction. The independent JavaScript oracle uses the integer-shape gamma/Poisson-tail identity with a probability-domain recurrence. This gives two numerically distinct implementations of the same finite-sample null calculation.

## Interpretation and strict limits

- trend_signal_detected: the exact conditional p-value is at or below the predeclared significance level. This is evidence against the constant-intensity boundary under the declared PLP assumptions.
- no_trend_signal_detected: the test did not detect a trend at the predeclared level. This is **not** proof of constant intensity, adequate power, a valid PLP model, or reliability.
- Too few events: indeterminate.
- Wrong model profile: model_not_supported.
- Invalid chronology, exposure, event times or significance: rejected.

This test does not confirm PLP goodness-of-fit, estimate a confidence interval for beta, test competing risks, account for heterogeneity or shared frailty, or validate the physical repair policy. It is conditional on the admitted event count and assumes the selected repairable-item model and admissibility evidence are valid. It does not replace #6617/#6618.

## Frozen corpus

The v0.1 corpus contains 24 synthetic cases:
- beta-one / no-trend controls;
- declining and increasing trend signals;
- near-constant, small-sample and high-event-count cases;
- 1%, 5% and 10% significance levels;
- exposure-unit scaling invariance;
- malformed/missing admissibility receipt pins;
- unsupported Weibull profile;
- invalid significance level, zero exposure, endpoint events, tied/unsorted events and insufficient event counts.

Prospective targets (not measured achievements):
- 100% agreement with the independent gamma-tail oracle;
- zero profile substitutions between recurrent PLP and first-failure Weibull;
- zero trend signals without an exact p-value;
- zero model-fit acceptance, reliability claim or authority grants;
- time-unit scaling invariance;
- deterministic receipt pinned to source commit and fixture digest.

## Industry / SOTA context

- [IEC 61710:2013](https://tc56.iec.ch/standards-navigator/) is the PLP comparator: it provides parameter estimation, confidence and prediction intervals, and goodness-of-fit methods for repairable-item event histories from one item or identical items under the same operating conditions.
- [IEC 61124:2023](https://webstore.iec.ch/en/publication/65838) is the separate constant-failure-rate/intensity compliance-test comparator under exponential/constant-intensity assumptions.
- [IEC 61649:2008](https://webstore.iec.ch/en/publication/5698) is the distinct first-failure Weibull analysis comparator.
- [MIL-HDBK-781 Rev B](https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=54097) is a programme-level reliability test planning comparator.
- [NASA-STD-7009B](https://standards.nasa.gov/standard/nasa/nasa-std-7009) and [NASA-HDBK-7009B](https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009) provide model credibility, VVUQ and intended-use context.

The claimed advance is not a new reliability theorem or a replacement for IEC methods. It is an exact conditional trend diagnostic with an independently implemented finite-sample p-value and a hard separation between trend detection, PLP adequacy, prediction, physical qualification and mission authority.

## Related ownership

- #6553 — one-shot mission-success Bernoulli profile.
- #6593 — homogeneous Poisson constant-intensity estimator.
- #6617/#6618 — event-history model admissibility.
- #6621/#6622 — PLP estimand and parameter estimation.
- #6636 — finite-sample diagnostic calibration and prediction programme.
- #6595/#6598 — typed assurance composition.
- #6256/#6303 — physical test programme and metrology.

This evaluator consumes a declared admissibility receipt and does not implement another event store, repair-history engine, calibration service or authority policy.

## Commands

~~~bash
cargo fmt --manifest-path tools/orbital-reliability-plp-trend-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-reliability-plp-trend-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-reliability-plp-trend-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-reliability-plp-trend-bench/Cargo.toml -- \
  tools/orbital-reliability-plp-trend-bench/fixtures/v0.1/trend-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-reliability-plp-trend-bench/tests/reference_oracle.mjs \
  tools/orbital-reliability-plp-trend-bench/fixtures/v0.1/trend-cases.json
~~~
