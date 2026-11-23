# Orbital Reliability Model-Admissibility Benchmark v0.1

**Status:** synthetic preflight prototype; exact-head hosted validation pending.  
**Owner:** [ORBITAL-RELIABILITY-MODEL-ADMISSIBILITY-001 #6617](https://github.com/Luminous-Dynamics/symtropy/issues/6617).  
**Claim ceiling:** synthetic model-input admissibility only.  
**Authority:** always false.

## Why this layer exists

The constant-intensity estimator in #6593 computes a confidence bound under a homogeneous Poisson / constant-intensity model. It cannot establish that the model assumptions hold. A good-looking bound under a mismatched model is not a valid reliability claim.

The industry standards separate these model families:

- [IEC 61124:2023](https://webstore.iec.ch/en/publication/65838) supplies optimized compliance-test plans for constant failure rate/intensity, assuming independent, identically exponentially distributed operating times or inter-failure times by default.
- [IEC 61649:2008](https://webstore.iec.ch/en/publication/5698) supplies distinct Weibull-analysis methods for time, cycles or stress-to-failure data, including graphical/numerical approaches and confidence limits.
- [MIL-HDBK-781 Rev B](https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=54097) is active (DLA record updated 19 November 2026) and provides reliability-test methods, plans and environments for development, qualification and production.
- [ISO 14224:2016](https://www.iso.org/standard/64076.html) standardizes equipment, failure and maintenance data taxonomy in oil, gas and petrochemical industries; it does not prescribe analysis methods and excludes laboratory/manufacturing accelerated-life data from its scope.
- [NASA-STD-7009B](https://standards.nasa.gov/standard/nasa/nasa-std-7009) and [NASA-HDBK-7009B](https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009) provide model verification, validation, credibility and intended-use guidance.

This crate is not a replacement for any of those standards, test plans or specialist reliability engineering.

## Architectural shift: model contract before estimator

The preflight separates:
1. Data-integrity checks.
2. Sampling and dependence checks.
3. Model/input-family compatibility.
4. Assumption-diagnostic readiness.
5. Statistical estimation (owned by a distinct model-specific estimator).

A passing preflight says only that required data declarations and record-quality checks are sufficiently complete to attempt the selected analysis. It does not prove constant hazard, Weibull fit, independent sampling, representativeness, calibration or physical truth.

## V0.1 profiles

- homogeneous_poisson_constant_intensity: candidate for #6593. Aggregate count/exposure can be used for rate-only analysis when declared assumptions and scope references are present. To challenge time-constant assumptions, event-level histories and predeclared diagnostic controls are required.
- power_law_recurrent_minimal_repair: event-level preflight for the one-repairable-series minimal-repair PLP profile in #6622. It requires one series, ordered events and minimal-repair semantics. It does not establish PLP fit or accept a statistical test.
- weibull_first_failure_life: a separate first-failure life input family, using event-level/suspension histories. Recurrent-event histories are explicitly not treated as first-failure Weibull data.
- nonhomogeneous_poisson_recurrent_events: deferred.
- accelerated_life: unsupported until the acceleration model, stress equivalence and failure-mechanism assumptions have their own evidence contract.
- degradation_path: unsupported until the measurement-error and trajectory model have their own contract.
- unknown_or_unregistered: unsupported.

V0.1 does not run inferential goodness-of-fit tests or produce fitted hazard curves. It audits declared inputs and descriptive evidence coverage only. A later diagnostic profile must pre-register the tests, thresholds, multiplicity handling, held-out split and operating-characteristic calibration before implementation.

## Data contract

Each article record carries immutable record/article/cluster IDs, population/lot/configuration/mode/environment identity, integer operating exposure seconds, reported failure count, exact event rows when event-level resolution is requested, failure definition, maintenance/restoration evidence, censoring, raw-artifact digest shape, clock verification and error bound, event-capture completeness and DAQ data-loss-counter verification.

Every event carries event ID, event time within the article's exposure window, failure-definition ID, restoration-state declaration and separate effective/recorded times.

Preregistration, sampling frame, dependence, assumption, maintenance, event-capture, clock, diagnostic protocol, multiplicity and holdout references are declarations only. They are not authenticated by this crate.

## Result states

- supported_for_analysis — preflight input requirements pass; model assumptions remain unproven.
- indeterminate — missing or unresolved data prevent admissibility assessment.
- model_not_supported — selected model/input family is not implemented.
- rejected — malformed, out-of-scope or internally inconsistent records.
- disputed — conflicting article/event identity or lineage.

The output fields constant_intensity_confirmed, reliability_claim_granted and authority_granted are permanently false. There is no state called "constant hazard confirmed".

## Frozen corpus

The current fixture contains 63 synthetic cases, including:
- rate-only aggregate inputs vs event-level assumption challenge;
- event history completeness, count consistency, time ordering, observation-window boundaries and bitemporal cutoff;
- shared sampling clusters, duplicate/conflicting article/event identities;
- clocks, calibration references, data-loss counters and event-capture completeness;
- maintenance history and recurrent-event restoration declarations;
- censoring, intervention and informative withdrawal;
- exact population/lot/configuration/mode/environment/failure-definition matching;
- Weibull first-failure versus recurrent-event separation;
- deferred NHPP, accelerated-life and degradation-profile boundaries;
- predeclared diagnostics, frozen thresholds, multiplicity and holdout references;
- positive controls, including zero-event event-level histories and aggregate-only rate analysis.

## Prospective benchmark targets

These are targets, not measured achievements:

| Metric | Target |
|---|---:|
| Malformed / conflicting event histories accepted | 0 |
| Wrong profile/input-family admissions | 0 |
| Missing required event-time, clock, repair or censoring evidence accepted | 0 |
| Scope/cutoff mismatches accepted | 0 |
| Input permutation changes report | 0 |
| Independent finite oracle mismatches | 0 |
| Constant-hazard confirmations | 0 |
| Reliability claims granted by preflight | 0 |
| Operational-authority grants | 0 |

Future inferential diagnostics must be evaluated for type-I error calibration at nominal 1%, 5% and 10%; power under Weibull shape values below/above one and NHPP trend alternatives; detection latency; and false alarms under censoring, heterogeneity, cluster dependence and data loss. Report confidence intervals and calibration curves, not only a pooled accuracy number.

## Industry comparison and how to improve

The claim is not "we replace IEC 61124/61649/MIL-HDBK-781". The proposed improvement is a machine-readable preflight before estimators run, preventing aggregate count/exposure from being mistaken for evidence that a hazard is constant and preventing Weibull first-failure methods from consuming recurrent-event data as though those were the same estimand.

A defensible matched comparison should use identical populations, failure definitions, censoring, exposure, confidence and acceptance risk. Measure false-accept / false-reject probability, operating-characteristic curves, article-hours, test-facility occupancy, calendar duration, instrumentation burden, audit/rework effort and uncertainty. Only claim cost reduction when false-accept risk is no worse and the same physical qualification obligations remain met.

## Commands

~~~bash
cargo fmt --manifest-path tools/orbital-reliability-model-admissibility-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-reliability-model-admissibility-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-reliability-model-admissibility-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-reliability-model-admissibility-bench/Cargo.toml -- \
  tools/orbital-reliability-model-admissibility-bench/fixtures/v0.1/admissibility-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-reliability-model-admissibility-bench/tests/reference_oracle.mjs \
  tools/orbital-reliability-model-admissibility-bench/fixtures/v0.1/admissibility-cases.json
~~~

## Ownership

- #6593 owns constant-intensity estimation.
- #6553 owns one-shot Bernoulli mission-success estimation.
- #6595/#6598 own typed assurance composition.
- #6516 owns exact-byte event replay.
- #6542 owns campaign test-matrix coverage.
- #6256/#6303 own physical test and metrology.
- #6519 owns evidence custody and authority integration.

This crate owns only model-input admissibility and diagnostic-readiness declarations. It does not duplicate estimators, trust roots, calibration algorithms, event stores or repair semantics.
