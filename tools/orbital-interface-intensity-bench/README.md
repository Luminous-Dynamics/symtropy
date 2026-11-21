# Orbital Interface Constant Failure-Intensity Benchmark v0.1

**Status:** synthetic reference implementation; exact-head hosted validation pending.  
**Owner:** [ORBITAL-INTERFACE-INTENSITY-001 #6589](https://github.com/Luminous-Dynamics/symtropy/issues/6589).  
**Claim ceiling:** synthetic constant-failure-intensity arithmetic and evidence-scope semantics only.

## Why this is a separate profile

The existing mission-success benchmark (#6553) estimates a one-shot probability: did one article complete one declared mission profile successfully? This benchmark estimates a different quantity: the rate of a declared failure event per unit of accumulated operating exposure.

V0.1 uses a homogeneous Poisson-process / constant failure-intensity model. Under the model, the event count over accumulated exposure has a Poisson distribution with mean \(\lambda T\). The one-sided upper confidence limit for the mean, \(\mu_U\), is obtained by solving:

\[
P(X \le k\mid X\sim\operatorname{Poisson}(\mu_U))=1-C
\]

The upper failure intensity is \(\lambda_U=\mu_U/T\), reported as events per million operating hours. A rate claim is demonstrated only when \(\lambda_U\) is no greater than the predeclared target.

For up to \(k\) observed events, the required exposure for target rate \(\lambda_0\) is \(T_{\min}=\mu_U(k,C)/\lambda_0\), where \(\mu_U\) is the exact Poisson upper mean bound. The zero-event special case simplifies because \(\mu_U=-\ln(1-C)\):

\[
T_{\min}=\frac{-\ln(1-C)}{\lambda_0}
\]

When the target is expressed per million operating hours, multiply the result by 1,000,000 before reporting hours. For a target of 100 events per million hours at 90% one-sided confidence, this is 23,025.85 operating hours. Zero events in 1,000 hours is therefore not a passing demonstration: its upper rate bound is approximately 2,302.6 events per million hours.

## V0.1 input and evidence contract

- One aggregated exposure record per physical article.
- Exposure is integer operating seconds, not calendar time or an unverified cycle count.
- Failure events are counted against one frozen all-cause functional-failure definition for one operating mode.
- The population, lot, configuration, operating mode, restoration policy and failure definition must match the case.
- At least two distinct article IDs and two distinct declared sampling-cluster IDs are required for this population-level profile.
- A same-ID, same-typed-record duplicate is idempotent; a conflicting same-ID record is Disputed.
- A second aggregate row for the same article is Indeterminate, or Disputed when exposure, event count or artifact digest differs. Aggregate article periods before evaluation.
- A repeated sampling cluster is Indeterminate.
- Administrative end-of-test and failure-terminated exposure may contribute when exposure and event capture are complete.
- Unknown censoring, informative withdrawal, intervention, incomplete event capture, unverified operating time, missing evidence or zero exposure is Indeterminate.
- Population, lot, configuration, mode, restoration-policy or failure-definition mismatch is Rejected.
- Convenience samples, wear-out/Weibull, accelerated-life, competing-risk, degradation-path, frailty/heterogeneity, Bayesian, reliability-growth, availability and maintainability profiles are ModelNotSupported.

Reference IDs and booleans are declarations. This evaluator checks their presence and typed values; it does not authenticate preregistration, validate a sampling frame, prove independent sampling, verify source bytes, establish repair policy correctness, or prove a constant hazard. The minimum of two articles is a structural anti-pseudoreplication guard, not proof of representativeness or independence.

## Industry and standards comparison

- **[IEC 61124:2023](https://webstore.iec.ch/en/publication/65838)** specifies optimized compliance-test plans for constant failure rate and constant failure intensity, including operating-time-to-failure / inter-failure data. Its default assumption is independent, identically exponentially distributed operating times, implying a constant failure rate/intensity. This crate is not a replacement for IEC test-plan selection or implementation.
- **[MIL-HDBK-781 Rev B status](https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=54097)** is listed active by DLA (validation notice dated 29 July 2026) and covers reliability test methods, plans and environments for engineering, development, qualification and production. It remains a programme-tailoring comparator.
- **[NASA-STD-5017B](https://standards.nasa.gov/standard/NASA/NASA-STD-5017)** addresses physical mechanism design/development and qualification, including service-life environments, all functions and energy extremes. A statistical rate demonstration cannot replace mechanism qualification.
- **[NASA-STD-7009B](https://standards.nasa.gov/standard/nasa/nasa-std-7009)** and the **[NIST 2026 UQ/metrology harmonization report](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing)** reinforce intended-use model credibility, uncertainty, and decision-risk-scaled VVUQ. A numerical model result must remain bounded by its validation domain.

This is a narrow complementary profile, not a claim to outperform or supersede any standard. Comparisons must use matched estimands, sampling assumptions, censoring, confidence, event definitions and exposure units.

## Frozen adversarial corpus

The v0.1 corpus contains 34 synthetic cases, including:

- zero-event exposure threshold and underpowered zero-event false assurance;
- one- and two-event upper confidence limits;
- confidence and exposure scaling;
- recurrent events versus independent article count;
- exact duplicate replay, conflicting records, repeated article IDs and shared clusters;
- failure-terminated exposure;
- incomplete event capture, unverified operating time, interventions and informative/unknown censoring;
- population/configuration/restoration-policy mismatches;
- missing references, invalid target, invalid digest, empty cohorts and Weibull model-boundary rejection.

The Rust implementation evaluates in the log domain using log-sum-exp and bisection. It exposes sample-exposure sizing for a predeclared maximum event count (with the zero-failure function retained as a special case). The JavaScript reference oracle uses a probability-domain recurrence and independent bisection for this bounded fixture, checks one-event exposure sizing, and checks article-order permutation invariance.

## Benchmark targets and SOTA comparison protocol

These are acceptance targets, not measured results:

| Dimension | Target |
|---|---:|
| Frozen synthetic expected-result mismatches | 0 / 34 |
| Independent arithmetic oracle mismatches | 0 |
| Known-answer numerical relative error | ≤ 1e-10 |
| False conformances in declared adversarial cases | 0 |
| Unsupported-model profile leakage | 0 |
| Authority grants from this evaluator | 0 |
| Reproducible receipt for identical bytes and commit | byte-identical JSON |
| Matched plan comparison against IEC 61124 / MIL-HDBK-781 | same estimand, failure definition, confidence, exposure and censoring before comparing effort or decision risk |

To claim a real advantage over established practice, benchmark on independently curated, blinded historical datasets and pre-registered physical campaigns. Report decision-risk (false-accept and false-reject operating characteristics), calendar time, total article-hours, test-facility occupancy, instrumentation burden, audit effort and uncertainty. Do not claim test-time reduction unless the accepted risk profile is unchanged.

## Commands

\`\`\`bash
cargo fmt --manifest-path tools/orbital-interface-intensity-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-interface-intensity-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-interface-intensity-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-interface-intensity-bench/Cargo.toml -- \\
  tools/orbital-interface-intensity-bench/fixtures/v0.1/intensity-cases.json \\
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-interface-intensity-bench/tests/reference_oracle.mjs \\
  tools/orbital-interface-intensity-bench/fixtures/v0.1/intensity-cases.json
\`\`\`

## Composition and limits

This profile complements, but does not merge, the distinct claims in:

- #6484/#6485 lifecycle qualification state;
- #6515/#6516 exact-byte lifecycle event replay;
- #6540/#6542 test-matrix coverage adequacy;
- #6548/#6553 independent one-shot mission-success reliability;
- #6519 signed authority evidence adapter;
- #6256/#6303 physical campaign, metrology and acceptance.

It does not establish physical hardware performance, actual constant intensity, independent sampling, representative populations, calibrated measurements, qualification, mission readiness or operational authority.
