# Orbital Repairable-Item Power-Law Process Benchmark v0.1

**Status:** synthetic parameter-estimation prototype; exact-head hosted validation pending.  
**Owner:** [ORBITAL-RELIABILITY-POWER-LAW-001 #6621](https://github.com/Luminous-Dynamics/symtropy/issues/6621).  
**Claim ceiling:** synthetic power-law parameter estimation only.  
**Authority:** always false.

## Why this model is separate

The orbital reliability programme has separate statistical estimands:

| Profile | Question | Owner |
|---|---|---|
| Bernoulli mission success | Did an article complete one declared mission? | #6553 |
| Homogeneous Poisson intensity | What is the constant failure-event rate per operating exposure? | #6593 |
| Power-law process (PLP) | Does recurrent event intensity vary with accumulated operating age under a declared repairable-item model? | #6621 |
| First-failure Weibull life | What is the first-failure / life distribution for independent items? | #6617 model boundary |

Do not convert among these profiles without a separately versioned, validated model and its own assumptions.

## Standards alignment

- [IEC 61710:2013](https://webstore.iec.ch/en/publication/5732) is the primary comparison. It specifies power-law parameter estimation, failure-intensity confidence intervals, prediction intervals for future failure times and goodness-of-fit tests for repairable items, with data collected from one item or identical items operating under the same conditions. This crate is not a claim of IEC conformity or a replacement implementation.
- [IEC 61124:2023](https://webstore.iec.ch/en/publication/65838) remains the constant failure-rate/intensity compliance-plan comparator.
- [IEC 61649:2008](https://webstore.iec.ch/en/publication/5698) remains the independent-item Weibull analysis comparator.
- [MIL-HDBK-781 Rev B](https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=54097) remains a reliability test planning comparator.
- [NASA-STD-7009B](https://standards.nasa.gov/standard/nasa/nasa-std-7009) and [NASA-HDBK-7009B](https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009) provide model credibility and intended-use VVUQ context.

## V0.1 mathematical contract

The power-law process uses cumulative mean value:

\[
M(t)=\eta t^\beta,\qquad \lambda(t)=\eta\beta t^{\beta-1}
\]

where t is the single declared accumulated operating-exposure clock, eta is the scale parameter and beta is the trend parameter.

- beta = 1 is the constant-intensity boundary.
- beta < 1 is a declining intensity trend within the declared model.
- beta > 1 is an increasing intensity trend within the declared model.

For complete event history through a fixed exposure T with event times \(t_i\), the v0.1 MLE is:

\[
\hat{\beta}=\frac{n}{\sum_i\ln(T/t_i)},\qquad
\ln\hat{\eta}=\ln n-\hat{\beta}\ln T
\]

The evaluator reports beta, log(eta), end-of-window intensity \(\hat\beta n/T\), the model's mean cumulative count at half-window exposure, and a descriptive likelihood-ratio statistic against beta=1. The likelihood-ratio statistic is not assigned a p-value and is not an acceptance test in v0.1.

## V0.1 admissible input boundary

- One repairable-item exposure series per case; no population-level independent-article inference from its recurrent events.
- One exact subject, population, lot, configuration, operating mode, environment and failure definition.
- Ordered and complete event occurrence times in integer operating seconds, inside (0,T).
- At least two unique events are required to produce the trend parameter estimate.
- One declared minimal-repair NHPP policy is supported; other restoration policies are ModelNotSupported rather than silently treated as equivalent.
- Complete event capture, verified exposure clock and bounded clock error, data-loss counter evidence, maintenance history and exact event identity.
- Administrative end or right censoring may be declared; intervention, unknown censoring and informative withdrawal are blocked.
- Separate effective and recorded timestamps are evaluated against case cutoffs.
- Duplicate identical event records are counted as duplicate evidence and block clean estimation; conflicting event IDs are Disputed.

A passing synthetic estimate does not establish model adequacy, power-law truth, repair effectiveness, physical reliability or qualification. There is no confidence interval, goodness-of-fit p-value or inferential fit acceptance in v0.1.

## Frozen corpus and targets

The fixture currently contains 67 synthetic cases covering:
- declining / near-constant / increasing event-intensity patterns;
- event-count and exposure scaling;
- model and restoration-policy mismatch;
- missing pre-registration, sampling, maintenance, clock and capture references;
- exact population/lot/configuration/mode/environment/failure-definition scope;
- clock uncertainty, event capture, censoring and intervention;
- zero/one event, invalid times, duplicate/conflicting identities and bitemporal cutoffs;
- positive estimate controls and negative controls.

Prospective acceptance targets, not measured results:

| Metric | Target |
|---|---:|
| Analytic MLE / independent oracle mismatches | 0 |
| Exposure scaling changes beta | 0 |
| Malformed, conflicting or out-of-scope histories admitted | 0 |
| Unregistered repair policies admitted | 0 |
| Input permutation changes report | 0 |
| Inferential fit acceptances | 0 |
| Reliability claims granted | 0 |
| Operational authority grants | 0 |

Future inferential diagnostics must be separately calibrated under a pre-registered simulation plan. Report Type-I error at nominal 1%, 5% and 10%, power across beta alternatives, and uncertainty across event count, exposure, censoring and capture-loss conditions. A non-rejection must never be presented as proof that the PLP fits.

## Industry comparison protocol

Compare this profile with IEC 61710 procedures on matched event definitions, repairable-item assumptions, event histories, operating conditions, censoring and confidence criteria. Compare beta/eta estimation, confidence coverage, future-event prediction calibration, goodness-of-fit operating characteristics, audit burden and test-planning effort. Do not claim cost or time reduction unless false-accept risk is no worse and the same physical qualification obligations remain met.

## Commands

~~~bash
cargo fmt --manifest-path tools/orbital-reliability-power-law-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-reliability-power-law-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-reliability-power-law-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-reliability-power-law-bench/Cargo.toml -- \\
  tools/orbital-reliability-power-law-bench/fixtures/v0.1/power-law-cases.json \\
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-reliability-power-law-bench/tests/reference_oracle.mjs \\
  tools/orbital-reliability-power-law-bench/fixtures/v0.1/power-law-cases.json
~~~

## Ownership

- #6553 owns one-shot mission-success probability.
- #6593 owns homogeneous Poisson constant intensity.
- #6617 owns event-history admissibility and model/data-family preflight.
- #6621 owns PLP parameter estimation and a future calibrated diagnostic profile.
- #6598 consumes typed outputs but cannot promote them to physical truth or authority.
- #6256/#6303 own physical campaign design, test execution and metrology.

The crate introduces no event store, trust root, calibration engine or authority policy.