# Maritime observational target-trial design v0.1

**Owner:** [MARITIME-OBSERVATIONAL-TARGET-TRIAL-001 #5113](https://github.com/Luminous-Dynamics/symtropy/issues/5113)  
**Parent programme:** [MARITIME-MAINTENANCE-EFFECT-001 #5108](https://github.com/Luminous-Dynamics/symtropy/issues/5108)  
**Randomized reference:** [PR #5111 — blocked-randomized known-truth estimator](https://github.com/Luminous-Dynamics/symtropy/pull/5111)  
**Restoration contract:** [PR #5105 — matched-duty repair restoration](https://github.com/Luminous-Dynamics/symtropy/pull/5105)  
**Status:** structural validator, synthetic confounding negative control, closed schemas, protocol, cases and exact-head workflow authored; hosted qualification pending  
**Evidence class:** synthetic design validation only; no shipboard maintenance records or field repair effects are represented.

## Why a separate observational track is required

The randomized P0 estimator in #5111 accepts only blocked randomized assignment. Real maintenance logs are observational: assets are repaired because their condition, operating duty, inspections or perceived risk differ. A post-repair improvement can be driven by the intervention, lower load, concurrent work, sensor replacement, asset selection or changing observation probability.

The target-trial framework requires the causal question to be defined as a hypothetical experiment before the observational analysis is interpreted. Its core protocol includes eligibility, strategies, assignment, follow-up, outcomes, causal contrast and analysis plan. This explicit design prevents many avoidable time-related and treatment-definition errors, but it does not magically establish exchangeability or remove unmeasured confounding:
- Hernán & Robins (2016), [Using Big Data to Emulate a Target Trial When a Randomized Trial Is Not Available](https://doi.org/10.1093/aje/kwv254).
- Syed & Doi (2026), [Target trial emulation: a primer for clinical research](https://doi.org/10.1097/pxh.0000000000000079), emphasizes aligning time zero, eligibility and treatment assignment at the design stage.
- [2026 review: Target Trial Emulation in Observational Research—Strengths, Limitations, and Methodological Considerations](https://doi.org/10.1111/jre.70095) discusses confounding and time-related biases.

## P0 implemented design contract

Rust module: `src/observational_target_trial.rs`.

The module validates the structure of an observational target-trial cohort and returns either:
- `structurally_eligible_under_declared_assumptions`; or
- `not_identified`, with one or more explicit reasons.

**Neither status is a causal estimate.** Even a structurally eligible report says only that the declared protocol and supplied data satisfy this version's structural gates. It does not prove the causal graph is correct, that all confounders are measured, that treatment assignment is exchangeable, or that an effect transports to a different engine fleet.

The Rust report always sets:
- `causal_effect_estimated = false`;
- maintenance authorization = false;
- physical-restoration authorization = false;
- operating-restriction clearance = false.

### Per-unit time zero

A crucial correction is built into the validator: **time zero is per asset, not one shared wall-clock timestamp for the entire cohort.** Different engines can become eligible on different dates. For each unit, eligibility, strategy assignment and follow-up start must align within the protocol's tolerance. Baseline covariates must be observed and available before that unit's own time zero.

This prevents immortal-time and guarantee-time errors without incorrectly requiring an entire fleet to enter an emulation on the same date.

### Treatment versions and baseline-only features

Treatment and comparator strategies have distinct version IDs. Every unit's assigned arm must match its recorded strategy version, and that version is bound to a source-record reference and digest. A mismatch is preserved as a NOT_IDENTIFIED reason. This v0.1 does not collapse partial, temporary, failed and complete repair into one generic treatment.

The propensity model's fit-covariate IDs must exactly equal the declared baseline covariate set. Propensity scores may be estimated retrospectively from those baseline covariates; the score itself need not have existed at treatment time. What is prohibited is using post-treatment telemetry, post-repair diagnosis or future work-order information as a baseline feature. Model and fit-data digests are bound in the protocol, but their issuers and provenance are not authenticated by this module.

### Overlap and effective sample size

Each stratum declares minimum repair/comparator counts, a minimum intersection width for the observed propensity ranges, and a minimum effective sample size per arm. The report emits these diagnostics separately.

This is only a **coarsened stratum-level support check**. It does not prove full multivariate positivity. A narrow overlap interval, unstable weights or poor covariate balance can still make an observational effect non-identifiable. A large row count cannot compensate for very few independent engines or for no credible comparator in a fault/load regime.

### Follow-up, censoring and outcome evidence

Preventive replacement, lay-up, competing events, loss to follow-up and administrative censoring are not silently treated as successful maintenance outcomes. A complete-follow-up unit cannot simultaneously carry a censoring timestamp; a censored/competing unit must preserve an in-window censoring time. Any incomplete follow-up is returned as `not_identified` in this P0 because the censoring model has not yet been implemented.

Complete follow-up must include an outcome measured after the declared horizon and available by the analysis time. The outcome binds:
- metric and unit;
- source ID and evidence digest;
- measurement principle and dependency domain;
- measurement quality and uncertainty;
- calibration-record digest and validity interval.

The calibration must cover observation through evidence availability. This is structural metadata validation, not issuer authentication or proof that the declared uncertainty bound is physically correct.

### Confounding-by-indication negative control

The synthetic cohort contains eight independent units in two coarse load strata. Within each stratum, more severe assets are preferentially repaired. The hidden potential-outcome oracle assigns a **true positive treatment effect of +5 normalized capability units** to every unit.

Yet the unadjusted observational difference is negative:

<grid columns={2} gap={2}>
  <grid-item>
    <box border radius="lg" padding={3} gap={1}>
      <text color="secondary" size="xs">True finite-population ATE</text>
      <title color="success" size="2xl" tabularNums>+5.0</title>
      <caption>normalized capability</caption>
    </box>
  </grid-item>
  <grid-item>
    <box border radius="lg" padding={3} gap={1}>
      <text color="secondary" size="xs">Observed crude difference</text>
      <title color="danger" size="2xl" tabularNums>−12.5</title>
      <caption>repair minus comparator</caption>
    </box>
  </grid-item>
</grid>

This is a deliberate sign-reversal demonstration of confounding by indication. The scorer receives the hidden oracle only after calculating the crude association; it marks the association as noncausal and the oracle as unused by the association calculation. It does **not** implement g-computation, AIPW, TMLE or a causal effect estimator.

## Frozen falsification suite

The 15 machine-readable scenarios cover:
1. structurally eligible synthetic cohort with valid per-unit time zero;
2. crude association sign reversal under confounding by indication;
3. within-unit time-zero misalignment;
4. post-treatment baseline leakage;
5. no comparator arm in a stratum;
6. preventive replacement / incomplete follow-up;
7. concurrent controller change and repair;
8. expired calibration before evidence availability;
9. different wall-clock time zeros across valid asset entries;
10. propensity outside the preregistered support interval;
11. missing baseline covariate;
12. strategy-version mismatch;
13. propensity model includes a post-treatment covariate;
14. treatment strategy label lacks its bound work-order/strategy record digest;
15. an independent exchangeability review reference is missing.

The Rust suite currently contains 17 tests. It exercises the structural validator, the synthetic oracle/association separation, sign reversal, per-unit time-zero logic, missing baseline covariates, post-treatment leakage in both outcome and propensity features, overlap/support, treatment-version and strategy-record binding, censoring, calibration, concurrent actions, missing review references and protocol/corpus binding.

## What this P0 does not identify

It does not implement:
- target-trial emulation estimators for an ATE;
- standardized outcome regression or g-computation;
- inverse-probability weighting or marginal structural models;
- AIPW or TMLE;
- clone–censor–weight strategies for dynamic treatment regimes;
- longitudinal g-formula for time-varying confounding affected by prior treatment;
- competing-risk estimation or treatment-dependent censoring models;
- unmeasured-confounding sensitivity bounds;
- heterogeneous effects or cross-engine transport.

The next estimator layer must be compared on known-truth synthetic data and controlled test-bench trials before any observational result is described as a marine-engine causal estimate.

## Standards and industry comparison

This work sits beside—not in place of—industrial monitoring and classification services:

- **Kongsberg Maritime K-CMS and Health Management** combine class-approved condition monitoring, sensor/data acquisition, vibration/oil-quality evidence, certified specialist analysis and maintenance recommendations. Kongsberg also describes a thruster RUL service using digital-twin technology. This is an end-to-end asset monitoring and expert-service capability, broader than an observational design validator:
  - [K-CMS / Condition Monitoring](https://www.kongsbergmaritime.com/products/engines-engine-room-and-automation-systems/condition-monitoring/)
  - [Health Management](https://www.kongsbergmaritime.com/services/kongsberg-remote-services/health-management/)
- **DNV Condition Based Maintenance** is a survey arrangement combining scheduled maintenance, predictive condition evidence and an approved service supplier. This is a class/survey-governance comparator:
  - [DNV Condition Based Maintenance](https://www.dnv.com/services/offshore-classification/fleet-in-service/condition-based-maintenance/)
- **Wärtsilä Expert Insight** combines vessel data, anomaly/rule-based diagnostics and expert investigation to support crew action. The Aurora Spirit case demonstrates why sensor faults, physical faults, expert review and operational response must remain distinct:
  - [Aurora Spirit case](https://www.wartsila.com/insights/case-study/aurora-spirit)
  - [Expert Insight service](https://www.wartsila.com/marine/services/lifecycle-agreements/expert-insight-service)
- **ABB Ability Digital Powertrain** offers expert-led predictive intelligence and self-service asset-health analytics. It is a relevant fleet monitoring and predictive-maintenance comparator:
  - [Digital Powertrain portfolio](https://library.e.abb.com/public/ad6138cb14d5433d85a2e4fe43932009/4MWA000447_ABB_Ability_Digital_Powertrain_Drives_Enabling%20Hardware_RevA_Nov2026.pdf)
  - [Digital Powertrain Insights](https://www.abb.com/global/en/areas/motion/services/asset-health-and-monitoring/abb-ability-digital-powertrain-insights)

These providers' public descriptions do not publish the matched de-identified repair histories, assignment propensities, censoring mechanisms or known counterfactuals required to score target-trial ATE bias. That absence of public benchmark data is **not** evidence that their internal systems lack causal methods.

### Fair benchmark strategy

Do not rank this validator against an OEM's anomaly detector, digital twin or class-approved monitoring service. Compare comparable layers:
1. monitoring products on fault/event detection, calibration and lead time;
2. restoration verification on independent matched-duty capability outcomes;
3. target-trial emulation on shared de-identified intervention histories with the same eligibility, strategies, time zero, outcomes and follow-up;
4. causal estimators on hidden known-truth synthetic trials and controlled test benches;
5. downstream policy value through #4545, with downtime, cost and engineering constraints exposed.

The P0 differentiator is a strict design contract that makes temporal leakage, treatment-version mixing, missing overlap, censoring and residual confounding visible before an estimator is allowed to produce an effect.

## Next research ladder

- **P0 — current:** structural target-trial validator and confounding-by-indication negative control.
- **P1:** synthetic known-truth datasets with zero, positive, negative and heterogeneous effects; treatment selection based on evolving severity; treatment-dependent censoring; competing events; measurement-path changes; whole-engine/whole-campaign splits.
- **P2:** baseline-only g-computation and propensity diagnostics, then AIPW/TMLE against the hidden oracle; longitudinal MSM/g-formula for time-varying confounding.
- **P3:** permissioned, de-identified repair histories with independent engineering adjudication, overlap and negative-control reports.
- **P4:** field-shadow estimates reviewed by independent causal/statistical and marine-engineering experts; only then consider policy handoff to #4545.

A positive causal claim is never issued solely because a model is well calibrated or its predictive score is high. If exchangeability, positivity, consistency, censoring or interference assumptions are not supportable, return `NOT_IDENTIFIED`.

## Claim ceiling

A pass establishes only that the exact cohort and protocol satisfy this version's declared structural rules. It does not prove conditional exchangeability, the correctness of the causal graph, full multivariate positivity, censoring exchangeability, absence of unmeasured confounding, treatment-effect transport, real-engine repair effectiveness, safe remaining life, vessel seaworthiness, class/flag approval, maintenance authorization or permission to operate.
