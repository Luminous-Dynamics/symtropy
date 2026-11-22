# Orbital Multi-Statistic Conditional PLP Goodness-of-Fit Suite v0.2

**Status:** synthetic reference implementation; exact-head hosted validation pending.  
**Owner:** [#6636 — PLP diagnostic calibration](https://github.com/Luminous-Dynamics/symtropy/issues/6636).  
**CvM-only baseline:** [#6683](https://github.com/Luminous-Dynamics/symtropy/pull/6683).  
**Claim ceiling:** synthetic conditional five-statistic PLP diagnostic suite only.  
**Fit acceptance / reliability / physical qualification / authority claims:** always false.

## Why a diagnostic portfolio

A single goodness-of-fit statistic is not a universal detector of every departure from a power-law process. The statistic determines which aspects of the event-time distribution are visible.

Lindqvist & Rannestad (2011), Section 5.3 and Table 1, compare Laplace, Greenwood, modified Cramér–von Mises, modified Anderson–Darling and modified Kolmogorov–Smirnov statistics. Their finite-sample simulations show substantial power differences by alternative. For n = 10 and nominal 5% size under a Weibull renewal alternative:
- shape 1.5: Greenwood power 0.1708 versus CvM 0.0081;
- shape 2.0: Greenwood 0.4636 versus CvM 0.0013.

Under their Musa–Okumoto NHPP alternative, Greenwood is weak while CvM / AD / KS perform better. These are results for the paper's particular designs, not universal power guarantees for this implementation.

The v0.2 shift is from a single CvM diagnostic to a predeclared five-statistic portfolio that discloses component signals and controls family-wise false rejection.

Published foundations:
- Lindqvist & Rannestad (2011), *Monte Carlo exact goodness-of-fit tests for nonhomogeneous Poisson processes*: [DOI 10.1002/asmb.841](https://doi.org/10.1002/asmb.841); [author manuscript](https://bo.folk.ntnu.no/forskning/LindqvistRannestadASMB.R2.pdf).
- Baker (1994), *Some New Tests of the Power Law Process*: [paper](https://www.stat.cmu.edu/technometrics/90-00/vol-38-03/v3803256.pdf).

## Distinct questions

- #6637 tests the constant-intensity boundary beta = 1.
- #6683 implements the conditional CvM PLP-family diagnostic.
- This v0.2 suite asks whether any member of a predeclared family identifies a PLP discrepancy, with family-wise multiplicity controlled.
- Future temporal validation asks whether a frozen model predicts unseen later events on a sealed holdout.

A genuine PLP with beta below or above one is not automatically a bad PLP.

## Conditional PLP reference distribution

For a complete event-time history through fixed administrative endpoint T, conditional on event count n:

\[
X_i=\log(T/t_i),\qquad S=\sum_{i=1}^{n}X_i.
\]

Under the PLP, the X_i are exponential with rate beta. S is sufficient for beta; conditional on S, P_i = X_i/S have a nuisance-free Dirichlet(1,...,1) distribution. The complete-window MLE is:

\[
\hat\beta=n/S.
\]

The fitted log transforms satisfy:

\[
\log(\hat U_i)=\hat\beta\log(t_i/T)=-nP_i.
\]

Each conditional reference replicate is generated from normalized iid unit-exponential draws, then all five statistics are computed from the same ordered conditional sample. This preserves their dependence and allows reproducible cross-statistic comparison. Tail-weighted calculations remain in the log domain to avoid underflow.

## Five predeclared statistics

For ordered fitted transforms U_(i), with endpoints U_(0)=0 and U_(n+1)=1:

1. **Laplace:** signed early/late imbalance, tested two-sided using the absolute value:

\[
L=\sqrt{12/n}\sum_{i=1}^{n}(\hat U_{(i)}-\tfrac12).
\]

2. **Greenwood:** event-spacing regularity / clustering, tested in both tails:

\[
G=\sum_{i=1}^{n+1}(\hat U_{(i)}-\hat U_{(i-1)})^2.
\]

Small G indicates overly regular spacing; large G indicates clustering / large gaps. The component p-value is twice the smaller one-sided tail probability, capped at one.

3. **Cramér–von Mises:** integrated empirical-distribution discrepancy:

\[
W^2=\frac{1}{12n}+\sum_{i=1}^{n}
(\hat U_{(i)}-\frac{2i-1}{2n})^2.
\]

4. **Anderson–Darling:** tail-weighted discrepancy:

\[
A^2=-n-\frac{1}{n}\sum_{i=1}^{n}(2i-1)
[\log(\hat U_{(i)})+\log(1-\hat U_{(n+1-i)})].
\]

5. **Kolmogorov–Smirnov:** maximum absolute empirical-CDF discrepancy, max(D+,D-).

Tail directions are predeclared in the suite and receipt. They are not chosen after seeing the data.

## Monte Carlo p-values and family-wise control

For B shared conditional reference replicates, each component uses a plus-one Monte Carlo p-value:

\[
\hat p_i=(1+b_i)/(B+1).
\]

The suite applies Bonferroni over m = 5 components:

\[
p_{\mathrm{FWER}}=\min(1,\;5\min_i\hat p_i).
\]

This controls family-wise false rejection without requiring the five dependent statistics to be independent. It is conservative but transparent. The receipt preserves each component's observed value, tail method, lower/upper exceedance counts, raw p-value, Bonferroni component p-value, Monte Carlo standard error and Wilson interval, alongside the suite-level adjusted minimum p-value.

FIT_REJECTED means the adjusted p-value is at or below preregistered alpha. FIT_NOT_REJECTED_AT_DECLARED_LEVEL is not a fit acceptance, proof of PLP truth or predictive adequacy.

The resolution gate uses the family-wise minimum attainable p-value 5/(B+1), not the CvM-only resolution. The profile requires B between 999 and 100,000, supports alpha from 1% to 10%, and rejects requests above a 10,000,000 event-draw budget rather than silently reducing B.

## Strict v0.2 scope

Supported only for minimal-repair histories, complete from exposure time zero, one fixed administrative time-terminated window, one exposure clock/configuration/environment and one declared failure definition, with upstream admissibility and capture receipt pins.

Delayed entry, interval censoring, failure-terminated windows, gapped observation, mixed configurations/environments, non-minimal repair, incomplete histories and unsupported model families remain unsupported or indeterminate. The fixed-window conditional endpoint must not be reused for failure-terminated plans.

The evaluator checks only the syntax of the declared upstream receipt hashes and source commits. It does not authenticate their producers or establish physical event completeness.

## Frozen synthetic corpus

The v0.2 corpus contains 24 cases:
- stochastic complete-window PLP samples with true beta values 0.6, 0.85, 1.0, 1.6 and 2.0;
- piecewise / burst, heterogeneous and irregular-gap alternatives;
- a deterministic over-regular-spacing control, explicitly labelled as an alternative rather than a stochastic PLP null sample;
- Weibull profile, failure-terminated, delayed-entry and imperfect-repair exclusions;
- missing receipt pins, incomplete history, insufficient event count, invalid chronology, unsupported alpha, unsafe seed, insufficient family-wise Monte Carlo resolution and excessive draw-budget controls.

In the frozen synthetic run, the over-regular control is detected specifically by Greenwood; CvM, AD and KS do not flag it. Some weak alternatives remain not rejected. This is intentional: limited power must be disclosed rather than forcing every seeded alternative into a rejection.

These fixtures demonstrate software/statistical-contract behavior; they do not estimate field Type-I error or power.

## Outer family-wise null-calibration smoke study

The exact-head workflow runs tests/outer_suite_null_calibration.mjs. It generates 250 stochastic complete-window histories at each true beta in {0.6, 1.0, 1.6}, with 10 events per history and 999 inner conditional replicates. Outer-history and inner-reference seeds are deterministically domain-separated.

It reports family-wise rejection rates at alpha 1%, 5% and 10%, with exact two-sided 95% Clopper–Pearson intervals, plus unadjusted component rejection rates. This first smoke study is a deterministic n=10 cross-beta check with wide intervals, not full Type-I certification. A second stratified smoke grid now runs true beta in {0.6, 1.0, 1.6} across event counts {5, 10, 20, 40}, with 50 independent outer histories per cell, 999 inner conditional replicates and exact two-sided 95% Clopper–Pearson intervals for family-wise and component rejection rates. It is a wider invariance check, still not full Type-I certification. Full calibration requires preregistered outer counts in the thousands/tens of thousands where needed, independent seed sets, event-count strata and alternative-specific power curves.

## Industry / standards comparison

Compare on identical event histories, event definitions, exposure clocks, entry/exit, censoring and repair semantics:
1. **IEC 61710:2013** — PLP estimation, confidence / prediction intervals and GOF. Its PLP CvM procedure and event-count critical values remain a standards baseline. [IEC publication](https://webstore.iec.ch/en/publication/5732).
2. **Lindqvist & Rannestad (2011)** — published conditional exact NHPP GOF methods and five-statistic power comparison. [DOI](https://doi.org/10.1002/asmb.841).
3. **JMP 19.2 Crow-AMSAA** — commercial PLP implementation with exact-event-time CvM and interval-censored chi-square procedures. [JMP documentation](https://www.jmp.com/support/help/en/19.2/jmp/crow-amsaa.shtml).
4. **SurPyval 0.14.0** — open-source recurrent-event portfolio with MCF / Lawless–Nadeau variance, NHPP and imperfect-repair renewal models. [Documentation](https://surpyval.readthedocs.io/en/latest/Recurrent%20Event%20Analysis.html).

This implementation does not claim superiority over standards or commercial software. It creates a more explicit sensitivity portfolio and a fair basis for matched Type-I / power comparisons at the same family-wise size.

## Observation-channel limitation

Constant independent capture thinning cannot be identified from timestamp shape alone. If the physical intensity is lambda(t) and each event is independently captured with constant probability p, the observed intensity is p lambda(t). For a PLP this rescales eta but leaves beta and the conditional time-shape distribution unchanged. Time-varying capture can create or conceal trend.

This suite therefore consumes a separate capture-evidence pin and cannot establish event completeness. #6659 owns capture observability; #6617/#6618 own admissibility; #6637 owns the beta = 1 trend diagnostic.

## Ownership and next work

- #6636 owns diagnostic calibration, operating-characteristic benchmarking and sealed prediction.
- #6637 remains the beta = 1 trend test.
- #6617/#6618 own model admissibility and capture-evidence consumption.
- #6659 owns observation-channel completeness.
- #6598 composes typed claims without promoting synthetic diagnostics into physical truth or authority.
- #6683 preserves the CvM-only v0.1 baseline; this suite is its v0.2 multi-statistic follow-on.

Next: full Type-I / FWER calibration across event-count strata; power curves under Weibull renewal, piecewise and Musa–Okumoto NHPP, clustering, inhibition and heterogeneity; matched IEC/JMP/SurPyval comparisons; calibrated max-T only if Bonferroni's conservatism justifies it; sealed temporal prediction; then physical validation.

## Commands

~~~bash
cargo fmt --manifest-path tools/orbital-reliability-plp-gof-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-reliability-plp-gof-bench/Cargo.toml --all-targets
cargo clippy --locked --manifest-path tools/orbital-reliability-plp-gof-bench/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/orbital-reliability-plp-gof-bench/Cargo.toml -- \
  tools/orbital-reliability-plp-gof-bench/fixtures/v0.2/plp-gof-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-reliability-plp-gof-bench/tests/reference_oracle.mjs \
  tools/orbital-reliability-plp-gof-bench/fixtures/v0.2/plp-gof-cases.json
node tools/orbital-reliability-plp-gof-bench/tests/outer_suite_null_calibration.mjs
~~~
