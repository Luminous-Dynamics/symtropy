# Maritime maintenance-effect estimation v0.1

**Owner:** [MARITIME-MAINTENANCE-EFFECT-001 #5108](https://github.com/Luminous-Dynamics/symtropy/issues/5108)  
**Parent:** [MARITIME-ENGINEERING-HEALTH-001 #4444](https://github.com/Luminous-Dynamics/symtropy/pull/4444)  
**Current increment:** P0 blocked-randomized known-truth estimator  
**Status:** source, synthetic oracle, strict schemas and adversarial manifest authored; hosted qualification pending  
**Evidence status:** synthetic-only; no marine-engine intervention dataset or physical repair outcome is represented as measured evidence.

## Why this is not the same as restoration verification

[PR #5105 / issue #5094](https://github.com/Luminous-Dynamics/symtropy/pull/5105) asks whether a declared physical repair is followed by a restored capability under matched duty, with a stable independently calibrated measurement path. That is an observed restoration outcome.

It still does not answer the counterfactual question:

> How would this same eligible asset or test case have evolved under the repair actually applied, versus another repair level, sensor-only correction, deferred intervention or an appropriate comparator?

A before/after difference cannot answer that question by itself. Load, controller changes, concurrent work, treatment selection, ambient context, missing follow-up and instrument changes can all generate an apparent repair effect.

This increment owns a narrow but defensible starting point: **known-truth blocked randomization**. It does not yet estimate causal effects from observational maintenance records.

## Composition and ownership

This increment composes with:
- #5094 / PR #5105 — measured per-capability restoration outcomes;
- #4545 — RUL and maintenance-policy utility;
- #4548 — source-grounded causal-risk factors;
- #4535 — data trust, missingness and evidence lineage;
- #4510 — engine/regime transfer;
- #5085 / PR #5091 — immutable diagnosis history and evidence-driven revisions.

It does not create another work-order lifecycle, diagnosis engine, RUL forecaster, maintenance optimizer, controller or authorization path.

## P0 implemented estimator

Rust module: `src/maintenance_effect.rs`.

### Design

- One observation per independently randomized unit (for example one engine/test asset), never one row or sliding window.
- Assignment is blocked and randomized within preregistered strata (for example load bands).
- Each stratum freezes treatment and control counts before outcomes are observed.
- Both arms must be present, with at least two units per arm to estimate within-arm variance.
- The assignment plan and each assignment carry evidence digests.
- Baseline measures must be observed and available before assignment.
- Follow-up outcomes must be observed after the declared follow-up horizon and available by the analysis as-of time.
- A trial with observational or deterministic assignment is rejected by this P0 estimator.

### Estimand

For the declared eligible randomized units:

`ATE = mean_i(Y_i(treatment) - Y_i(control))`

The estimator is the stratum-weighted difference in means. The report includes the raw effect and a benefit-oriented sign convention determined by whether higher or lower outcomes are better.

### Measurement provenance and uncertainty

Each baseline and outcome measurement binds a source ID, measurement principle, dependency domain, evidence digest, quality state, calibration-record digest and calibration-valid-until time. Only valid evidence with verified calibration that remains valid through evidence availability is admitted. The outcome measurement principle is frozen by the trial and must match across every randomization unit.

The estimator requires a deterministic absolute-error bound for each baseline/outcome measurement. Baseline uncertainty is recorded and validated but is not used for adjustment in P0. Outcome uncertainty is summarized separately:

- **Sampling uncertainty:** Neyman standard error and normal-approximation confidence interval from randomized units.
- **Measurement uncertainty:** the stratum-weighted sum of treatment-arm and control-arm mean absolute-error bounds.
- **Robust interval:** the sampling interval expanded on both sides by that deterministic measurement-error bound.

The measurement-error bound is not inserted into the Neyman sampling variance and is not described as a standard deviation. The robust interval is a deterministic error-bound expansion of the sampling interval. The module checks declared calibration/evidence metadata; it does not authenticate issuers or prove the physical validity of a stated bound.

### Uncertainty and tests

The estimator reports:
- a Neyman conservative variance estimate from within-arm sample variances;
- a normal-approximation sampling interval at a preregistered 90%, 95% or 99% level;
- a separate deterministic measurement-error bound and measurement-robust interval;
- a two-sided exact Fisher randomization p-value under the sharp null, conditional on the blocked allocation.

The P0 exact randomization enumeration is deliberately bounded to 16 independent units. It fails closed above that limit. Larger campaigns will need a separately qualified Monte Carlo randomization-inference method and reproducible random-stream contract.

The sampling confidence interval is explicitly a normal approximation. It should not be treated as an exact randomization interval or assumed to have nominal coverage at small sample sizes. The measurement-robust interval widens this interval by the declared deterministic measurement-error bound; it does not correct unobserved confounding or invalid calibration.

### Oracle separation

The synthetic potential-outcome oracle is a separate type and separate function argument. The estimator itself receives only the trial assignment and observed outcomes. The oracle is used only after estimation to score finite-population bias and whether the reference interval contains the known truth.

A test changes only unobserved potential outcomes: the known-truth score changes, while the estimator's point estimate remains unchanged. This prevents the evaluator from accidentally using information that would be unavailable in a real intervention.

## Falsification and synthetic cases

The machine-readable manifest has fourteen scenarios:
1. blocked randomized known-truth repair experiment;
2. observational assignment rejected;
3. missing treatment/control overlap rejected;
4. baseline available after assignment rejected;
5. outcome unavailable by analysis time rejected;
6. follow-up horizon not met rejected;
7. tampered oracle rejected;
8. duplicate randomization unit rejected by the Rust validator;
9. insufficient within-arm replication rejected;
10. exact enumeration beyond the frozen P0 bound rejected;
11. stale outcome measurement rejected;
12. calibration expired before evidence availability rejected;
13. outcome measurement principle differs from the frozen trial;
14. valid measurement-error bounds produce a separate robust effect interval.

The Rust unit suite exercises the estimator, exact randomization reference distribution, directionality, chronology, assignment design, oracle separation, calibration validity, measurement-principle binding, deterministic measurement-error expansion and the protocol/case manifest.

## What this does not estimate yet

This is not a universal causal inference engine. The first P0 does not handle:
- observational treatment assignment or target-trial emulation;
- time-varying confounding affected by prior maintenance;
- treatment-dependent censoring or competing events;
- interference between engines/assets or cluster-randomized campaigns;
- imperfect repair levels with changing versions and workmanship;
- transport of treatment effects across engines and operating regimes;
- heterogeneous effects or policy value;
- long-horizon recurrence, RUL or durability.

Those are explicit future tracks, not silently assumed capabilities.

## Next identification ladder

1. **Controlled known-truth randomization:** reproduce P0 and expand synthetic fault, repair-level, sensor-only and no-op negative controls.
2. **Permissioned test-bench experiments:** blocked/randomized repair level or comparator only where safe, ethical and approved; use independent measurement paths and declared capability envelopes.
3. **Observational target-trial emulation:** freeze eligibility, treatment versions, time zero, comparator, follow-up, censoring and outcomes before estimating.
4. **Doubly robust and longitudinal methods:** compare AIPW/TMLE and g-computation/marginal structural approaches only with overlap, nuisance-model, time-varying confounding and sensitivity diagnostics.
5. **Effect transport:** test whole-engine, whole-campaign and whole-regime holdouts; report uncertainty and abstain outside overlap.
6. **Policy value:** hand intervention-effect distributions to #4545. This module does not rank maintenance policies or issue work orders.

The 2026 imperfect-maintenance work by Najafi & Taghipour models stochastic repair effects and regime-dependent feasibility in a semi-Markov framework (DOI 10.1016/j.cie.2026.112109). It is a useful future comparator, not a reproduced marine-engine result. The hydraulic sensor-log repair-impact work (DOI 10.1016/j.eswa.2026.133024) provides a physics-grounded causal test-rig reproduction lead, not evidence of shipboard repair effectiveness.

## Research targets — aspirational, not achieved

- Absolute ATE bias below 5% of the declared outcome scale on adequately sampled known-truth randomized cases.
- 85–95% empirical coverage for nominal 90% intervals on adequately sampled held-out randomized cases, with interval width reported.
- Zero accepted causal effect for nonrandomized or positivity-failing P0 inputs.
- 100% of P0 reports state the estimand, outcome metric/unit, allocation, follow-up horizon and estimator.
- Zero oracle influence on the estimate.
- Zero work-order, maintenance-authorization, physical-restoration or operating-clearance side effects.

These targets are not safety thresholds and are not achieved results.

## Claim ceiling

A P0 pass establishes only that the declared blocked randomized trial satisfies this contract and that the estimator can be scored against a separate synthetic potential-outcome oracle. It does not establish repair effectiveness on a real engine, observational identification, safe remaining life, recurrence prevention, cross-engine transport, seaworthiness, class/flag approval, maintenance authorization or permission to operate.

