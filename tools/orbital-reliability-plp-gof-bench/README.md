# Orbital Conditional PLP Goodness-of-Fit Benchmark v0.1

**Status:** synthetic reference implementation; exact-head hosted validation pending.  
**Owner:** [ORBITAL-RELIABILITY-PLP-DIAGNOSTIC-CALIBRATION-001 #6636](https://github.com/Luminous-Dynamics/symtropy/issues/6636).  
**Claim ceiling:** synthetic conditional PLP goodness-of-fit diagnostic only.  
**Fit acceptance / reliability / physical qualification / authority claims:** always false.

## Purpose and hypothesis

This benchmark answers a different question from the exact trend test in #6637.

- **Trend test (#6637):** is the constant-intensity boundary \(\beta=1\) inconsistent with the event history?
- **PLP goodness-of-fit (this crate):** under the declared complete-window minimal-repair PLP profile, are the fitted event-time locations inconsistent with the PLP family?
- **Prediction:** does a model fitted to an earlier event history predict later events on a sealed temporal holdout? This remains a separate future phase.

A genuine PLP with \(\beta=0.6\) is a trending PLP, not automatically a bad PLP. The goodness-of-fit statistic here does not test \(\beta=1\).

## Conditional exact method

This is an implementation of an established conditional goodness-of-fit method for the power-law NHPP, not a new statistical theorem.

For a complete event-time history through a fixed administrative endpoint \(T\), conditional on the observed event count \(n\), define:

\[
X_i=\log(T/t_i).
\]

Under the PLP, the \(X_i\) have exponential distribution with rate \(\beta\). Their sum is sufficient for \(\beta\). Conditional on:

\[
S=\sum_{i=1}^{n}X_i,
\]

the proportions:

\[
P_i=X_i/S
\]

have a nuisance-parameter-free \(\operatorname{Dirichlet}(1,\ldots,1)\) distribution. The complete-window MLE is:

\[
\hat\beta=n/S.
\]

The fitted transformations are:

\[
\hat U_i=\exp(-nP_i)=(t_i/T)^{\hat\beta}.
\]

After ordering the fitted transforms, the implementation uses a one-sample Cramér–von Mises discrepancy:

\[
W^2=\frac{1}{12n}+
\sum_{i=1}^{n}\left(\hat U_{(i)}-\frac{2i-1}{2n}\right)^2.
\]

The reference distribution is simulated from normalized iid unit-exponential draws, which produce the Dirichlet proportions. It is **not** correct to apply ordinary known-parameter Uniform critical values to fitted transformed values.

The Monte Carlo p-value uses:

\[
\hat p=\frac{1+b}{B+1},
\]

where \(b\) is the number of conditional reference statistics at least as extreme as the observed statistic and \(B\) is the frozen replicate count. The receipt preserves the seed, PRNG identifier, statistic identifier, replicate count, exceedance count, minimum attainable p-value, Monte Carlo standard error and Wilson 95% interval for the tail-probability estimate. The PRNG is SplitMix64 mapped to an explicitly open 52-bit uniform grid (k/(2^{52}+1)), for integer (k\in\{1,\ldots,2^{52}\}), before the inverse-exponential transform. Its version identifier is `splitmix64-top52-open-uniform-exp-normalize-v1`.

Published foundation: Lindqvist & Rannestad (2011), *Monte Carlo exact goodness-of-fit tests for nonhomogeneous Poisson processes*, which develops conditional tests given sufficient statistics for power-law and log-linear NHPPs. [DOI: 10.1002/asmb.841](https://doi.org/10.1002/asmb.841); [author manuscript](https://bo.folk.ntnu.no/forskning/LindqvistRannestadASMB.R2.pdf).

## Strict v0.1 admissibility boundary

Supported only for:
- one repairable item or explicitly identical items operating under one declared condition;
- minimal repair;
- full observation beginning at exposure time zero;
- complete event-time history;
- a single fixed administrative time-terminated observation window;
- a single exposure clock, configuration, environment and declared failure definition;
- upstream model-admissibility and event-capture receipt pins;
- predeclared analysis protocol, multiplicity policy, alpha, Monte Carlo replicate count and seed.

The v0.1 profile rejects or leaves indeterminate delayed entry, interval censoring, failure-terminated plans, mixed environments/configurations, gapped windows, non-minimal repair, missing capture evidence, incomplete histories and unsupported model families. Do not transplant the fixed-window conditional distribution to failure-terminated designs: their conditional endpoint and terminal-event conventions differ.

The supported significance range is 1%–10%. The Monte Carlo replicate count is 999–100,000 and must provide at least the declared p-value resolution. A hard 10,000,000 total conditional-draw budget bounds work per case; the evaluator rejects rather than silently reducing B. Exact direct dependency versions and a minimal committed `Cargo.lock` pin the Rust build graph; CI uses `--locked`. Multiple diagnostic testing requires an externally declared multiplicity policy.

## Result semantics

- FIT_REJECTED — the conditional Monte Carlo p-value is at or below the predeclared significance level.
- FIT_NOT_REJECTED_AT_DECLARED_LEVEL — the test did not reject at the declared level. This is **not** proof that the PLP is true, adequate, predictive or appropriate for physical qualification.
- INDETERMINATE — insufficient event count, incomplete history or missing admissibility/capture receipt evidence.
- MODEL_NOT_SUPPORTED — wrong model family, termination plan, delayed entry or repair policy.
- REJECTED — malformed identity/protocol, unsupported alpha/resolution/seed, invalid chronology or invalid exposure window.

No result grants fit acceptance, reliability, physical qualification or authority. FIT_NOT_REJECTED_AT_DECLARED_LEVEL must never be renamed “fit accepted”.

## Frozen synthetic corpus

The v0.1 corpus contains 23 cases:
- complete-history PLP controls with \(\beta\) near 0.6, 0.85, 1.0, 1.6 and 2.0;
- seeded piecewise/burst and irregular-gap alternatives expected to reject;
- Weibull profile and failure-terminated / delayed-entry plans outside scope;
- incomplete history and missing admissibility/capture pins;
- insufficient event count, invalid chronology, unsupported alpha, insufficient Monte Carlo resolution, an excessive total-draw request, imperfect repair, malformed configuration and unsafe random seed.

The separately implemented JavaScript reference oracle reproduces the conditional reference simulation and case dispositions. Agreement on this finite corpus is software/statistical-contract validation only; it is not an empirical estimate of field error rates or proof of diagnostic operating characteristics.

## Outer null-calibration smoke study

The exact-head workflow also runs `tests/outer_null_calibration.mjs`. It generates 250 independent complete-window PLP histories at each true beta in {0.6, 1.0, 1.6}, with 10 events per history, and uses 999 independent conditional reference draws per test. It reports rejection rates at alpha = 1%, 5% and 10%, each with an exact two-sided 95% Clopper–Pearson interval. The report is synthetic and explicitly capped at `finite_smoke_calibration_only`; it is a smoke test, not sufficient evidence of nominal size.

The full calibration target must use a pre-registered outer replication count and independent holdout seeds. As rough planning values only, a normal-approximation calculation suggests about 38,000 independent outer null replications for a 95% half-width near 0.1 percentage points at a 1% rejection rate; exact binomial intervals must be used for the final report. Calibration should be repeated across event-count strata, with the conditional beta-invariance claim tested using independently generated PLP histories.

## Calibration and SOTA comparison

Keep a separate calibration tournament for:
- Type-I error at nominal 1%, 5% and 10% with exact-binomial / Monte Carlo confidence intervals;
- power against piecewise intensity, heterogeneous-item mixtures, shared-dependence and time-varying alternatives;
- sensitivity to event count and test termination design;
- Monte Carlo p-value resolution and computational cost;
- sealed temporal predictive coverage and proper scoring as separate future outputs.

Compare on matched event histories with the same event definition, time base, entry/exit, censoring, restoration semantics and model scope:

1. **IEC 61710:2013** — normative PLP estimation, confidence/prediction intervals and GOF methods. Its PLP CvM procedure and event-count critical values are the primary standard comparator. [IEC publication](https://webstore.iec.ch/en/publication/5732).
2. **Lindqvist & Rannestad (2011)** — published conditional exact NHPP goodness-of-fit algorithms, including power-law profiles. [DOI](https://doi.org/10.1002/asmb.841).
3. **JMP 19.2 Crow-AMSAA** — commercial implementation; documented exact-event-time CvM and interval-censored chi-square reporting, with p-values capped at 0.25. [JMP documentation](https://www.jmp.com/support/help/en/19.2/jmp/crow-amsaa.shtml).
4. **SurPyval 0.14.0 (19 July 2026)** — open-source recurrent-event benchmark including non-parametric MCF / Lawless–Nadeau robust variance, NHPP alternatives and imperfect-repair models. [Recurrent event docs](https://surpyval.readthedocs.io/en/latest/Recurrent%20Event%20Analysis.html); [changelog](https://surpyval.readthedocs.io/en/latest/changelog.html).

Do not compare models across mismatched estimands, termination conventions or repair regimes. A model family that is outside the current input contract is a boundary control, not a failed competitor.

## Observation-channel limitation

A timestamp-only GOF test cannot identify constant event-capture thinning. If the physical process intensity is \(\lambda(t)\) and each event is independently captured with constant probability \(p\), the observed intensity is \(p\lambda(t)\). For a PLP this rescales \(\eta\) but leaves \(\beta\) and the conditional time-shape distribution unchanged. Time-varying capture can create or conceal a trend.

Therefore this crate consumes a separate capture-evidence receipt pin. It cannot establish event completeness. [#6659](https://github.com/Luminous-Dynamics/symtropy/issues/6659) owns observation-channel qualification; #6617/#6618 own admissibility; #6637 owns the separate constant-boundary trend test.

## Ownership

- #6553 — one-shot Bernoulli mission-success profile.
- #6593 — homogeneous Poisson constant-intensity estimation.
- #6617/#6618 — model-input admissibility and capture-evidence consumption.
- #6621/#6622 — minimal-repair PLP estimand and parameter estimation.
- #6636 — PLP diagnostic calibration and held-out predictive-validation programme.
- #6637 — exact conditional trend test against \(\beta=1\), not PLP-family adequacy.
- #6659 — event-capture observability and completeness evidence.
- #6598 — typed claim composition; cannot upgrade a synthetic GOF outcome into physical truth or authority.

## Commands

~~~bash
cargo fmt --manifest-path tools/orbital-reliability-plp-gof-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-reliability-plp-gof-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-reliability-plp-gof-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-reliability-plp-gof-bench/Cargo.toml -- \
  tools/orbital-reliability-plp-gof-bench/fixtures/v0.1/plp-gof-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-reliability-plp-gof-bench/tests/reference_oracle.mjs \
  tools/orbital-reliability-plp-gof-bench/fixtures/v0.1/plp-gof-cases.json
~~~

## Claim ceiling and next phase

A passing v0.1 shows that the declared conditional-CvM algorithm and oracle agree on the frozen synthetic fixtures. It does not establish calibrated Type-I error or power across all sample sizes, PLP truth for a physical mechanism, prediction accuracy, standards conformity, hardware reliability, qualification, risk acceptance or mission authority.

Next phases must add a Monte Carlo calibration study with confidence intervals, an independent public corpus, direct method comparisons to IEC/JMP/SurPyval, sealed temporal prediction and only then physical-domain validation. Physical event-capture completeness remains separately owned by #6659.
