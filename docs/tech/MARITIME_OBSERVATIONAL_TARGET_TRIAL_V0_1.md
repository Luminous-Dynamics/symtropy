# Maritime observational target-trial contract v0.1

**Owner:** [MARITIME-OBSERVATIONAL-TARGET-TRIAL-001 #5113](https://github.com/Luminous-Dynamics/symtropy/issues/5113)  
**Parent:** [MARITIME-MAINTENANCE-EFFECT-001 #5108](https://github.com/Luminous-Dynamics/symtropy/issues/5108)  
**Randomized reference:** [PR #5111](https://github.com/Luminous-Dynamics/symtropy/pull/5111)  
**Current increment:** protocol/diagnostic gate only; no observational effect estimator is implemented here  
**Evidence status:** synthetic contract tests only; no shipboard maintenance logs or field intervention data are represented.

## Why this must be a separate path

The blocked-randomized estimator in #5111 starts from a declared randomized assignment. Real maintenance logs are observational: a more deteriorated engine may be more likely to be repaired, monitored, or removed from service. Treatment, follow-up, instrumentation and operating duty can all change together.

A before/after repair comparison or a predictive maintenance score does not solve confounding by indication. Observational causal analysis must first specify the randomized trial that would answer the question, then show how its components map to the available records.

This matches current target-trial guidance:
- Fu et al. (2025), [align eligibility, treatment assignment and follow-up at time zero](https://www.bmj.com/content/392/bmj-2025-084909.full), describes how misalignment creates immortal-time and selection bias.
- Hernán et al. (2022), [Target trial emulation: applying principles of randomised trials to observational studies](https://www.bmj.com/content/378/bmj-2022-071108.full), specifies eligibility, strategies, assignment, outcomes, follow-up, contrasts and analysis plan as core protocol elements.
- The 2025 [TARGET reporting statement](https://www.bmj.com/content/390/bmj-2025-087179) provides 21 reporting items, including identifying assumptions, data mapping and sensitivity analyses.
- A 2026 [operational target-trial framework](https://www.nature.com/articles/s41746-026-02563-z) emphasizes that observational emulation is a data-constrained design problem: when essential components are not observed, the intended estimand is not identifiable.

## Implemented contract

Rust module: `src/target_trial_contract.rs`.

It accepts a target-trial specification and an observational diagnostic snapshot, and returns exactly one of:

- `protocol_incomplete`: required definitions, digests or temporal rules are missing/invalid;
- `not_identified`: the protocol is specified but data/assumption/overlap diagnostics fail;
- `design_qualified_for_estimator`: the declared design and diagnostics pass this gate and may be considered by a separately qualified estimator.

The report **never estimates a causal effect**. Even the third disposition is only design/diagnostic qualification; it is not proof that unmeasured confounding is absent.

## Required trial protocol

The protocol binds:
- target population and eligibility rule;
- two or more intervention strategies, each with version and definition digests;
- strategy grace period, if any;
- assignment rule and causal graph;
- causal contrast (assignment, per-protocol or treatment-policy);
- primary outcomes with metric, unit and definition digest;
- baseline covariates and their temporal role;
- analysis-plan digest;
- cluster key, grouping-rule digest and explicit cluster-robust uncertainty declaration;
- follow-up calendar horizon and exposure unit;
- preregistered overlap, effective-sample-size, balance, weight and follow-up thresholds.

Strategies remain distinct: physical component repair, alternative repair, sensor-only correction, deferred/usual care and dynamic maintenance strategy must not be silently collapsed.

A non-zero grace period requires clone-censor-weight emulation. The module does not implement cloning or weighting; it verifies that the necessary design is declared before a later estimator is considered.

## Required observational diagnostics

The diagnostic snapshot binds:
- source data snapshot digest;
- emulation-entry count and independent asset-cluster count;
- counts for every declared strategy arm;
- common-support coverage;
- minimum arm effective sample size;
- maximum absolute standardized mean difference;
- maximum stabilized weight;
- follow-up completeness;
- evidence-bound assessments of consistency, conditional exchangeability, positivity, no interference, censoring exchangeability and measurement validity;
- a negative-control disposition and an explicit sensitivity-analysis status;
- unit-level eligibility, assignment, follow-up, treatment-start and follow-up-end timelines;
- source digests and availability timestamps for every declared baseline covariate.

**Time zero is non-negotiable:** eligibility, strategy assignment and follow-up start must coincide for every emulation entry. Baseline covariates must have been observed and available no later than time zero. Post-treatment variables cannot be relabelled as baseline confounders.

Repeated telemetry windows and repeated emulation entries are not independent assets. The protocol requires an explicit cluster key and cluster-robust uncertainty plan. Parallel-group designs cannot put the same asset in multiple strategy arms; sequential-trial or clone-censor-weight designs may reuse the asset only as distinct, correctly clustered entries.

Administrative follow-up cannot silently end before the target horizon. Early event, censoring and competing-event exits must be represented explicitly.

## Identification abstention

The module returns `not_identified` when:
- any of the six identifying assumptions is unknown, violated or missing;
- common support or effective sample size is below its preregistered threshold;
- covariate balance or weight diagnostics exceed thresholds;
- follow-up completeness is insufficient;
- time zero or baseline temporal ordering is wrong;
- the negative control is unsupported;
- cluster IDs or strategy-arm counts do not reconcile;
- a treatment strategy is inconsistent with its recorded treatment start.

Threshold defaults in the synthetic fixture are not fleet-wide safety limits. Every real analysis must justify and freeze its own thresholds before estimating an effect.

## Falsification corpus

The machine-readable manifest has eighteen cases:
1. complete protocol and supported diagnostics;
2. time-zero misalignment;
3. post-time-zero baseline covariate;
4. post-treatment covariate declared as baseline;
5. unknown exchangeability/positivity;
6. poor common support and effective sample size;
7. grace period without clone-censor-weight design;
8. negative-control failure/unknown state;
9. inadequate follow-up completeness;
10. repeated asset in sequential emulation with distinct entry and cluster identity;
11. one asset assigned to both arms in parallel-group emulation;
12. premature administrative follow-up end;
13. missing cluster-robust uncertainty plan;
14. deferred strategy contains a physical treatment start;
15. duplicate strategy-arm count metadata;
16. missing, duplicate or invalid baseline evidence.

The 18-test Rust suite is synthetic software validation. It does not validate a real propensity model or causal estimate.

## Next estimators — explicitly not implemented

The next stage should compare, against known-truth synthetic trials and then controlled test-bench records:
- unadjusted and regression association baselines (labelled noncausal unless assumptions support otherwise);
- standardization / g-computation for static baseline strategies;
- inverse-probability weighting and marginal structural models for time-varying treatment/confounding;
- AIPW and TMLE with overlap and nuisance-model diagnostics;
- clone-censor-weight for strategies with grace periods or dynamic adherence;
- longitudinal g-formula for time-varying confounders affected by prior maintenance;
- partial identification and quantitative sensitivity analysis when exchangeability is doubtful.

"Doubly robust" does not mean robust to unmeasured confounding, lack of positivity, inconsistent treatment versions or incorrect time zero.

## Benchmarks and aspirations

Measure:
- time-zero and temporal-leakage rejection rates;
- unknown-assumption and positivity abstention rates;
- common-support, effective sample size and standardized-balance distributions;
- weight tails and truncation sensitivity;
- follow-up and censoring completeness;
- negative-control violations;
- sensitivity to alternate causal graphs and unmeasured-confounding strength;
- effect bias/RMSE and interval coverage on known-truth simulations only after an estimator is implemented;
- transport across held-out engines, campaigns and load regimes.

Aspirational targets, not achieved:
- zero causal claims when time zero, baseline timing or positivity is invalid;
- zero positive plant-repair effects for sensor-only correction controls;
- every supported design records all six assumptions and the full protocol;
- cluster-aware standard errors for every repeated-asset/sequential design;
- at least 90–95% empirical coverage for nominal 90% effect intervals on adequately sampled known-truth synthetic cases, with interval width reported;
- explicit NOT_IDENTIFIED output instead of unsupported extrapolation.

## Claim ceiling

A `design_qualified_for_estimator` result means only that the target-trial protocol and supplied diagnostic snapshot passed this structural gate. It is not a causal effect estimate, evidence that unmeasured confounding is absent, validation on a marine engine, repair-effectiveness claim, safe-life statement, seaworthiness opinion, class/flag approval, maintenance authorization or permission to operate.
