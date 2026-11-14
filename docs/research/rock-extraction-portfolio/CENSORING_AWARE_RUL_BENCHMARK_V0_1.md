# Censoring-aware competing-risk prognosis benchmark v0.1

**Status:** asset-episode data contract and evaluator for supplied probability forecasts. It does not train a prognostic model, certify a component or authorize maintenance.

## 1. Paradigm shift: RUL is time-to-event, not failed-unit regression

A maintenance dataset is not a table of failed components with measured lifetimes. Some units fail, some remain in service when observation ends, and others are inspected, repaired or replaced before failure.

Treating every removal as an observed failure creates biased RUL labels. Treating preventive replacement as harmless independent censoring can also bias evaluation if replacement was prompted by a degrading condition.

v0.1 therefore records each asset episode with an explicit exposure denominator, entry/exit exposure, event/censor status, competing failure cause, censoring reason and intervention history.

## 2. Research basis

- Dehondt et al. (online 10 October 2026), *Remaining useful life data-driven estimation: a selective evaluation and analysis*, *Signal Processing*, DOI 10.1016/j.sigpro.2026.110960, reimplement nine RUL methods in a reproducible common evaluation and do not identify a universal best model. The study highlights the gap between benchmark gains and realistic industrial requirements.
  https://doi.org/10.1016/j.sigpro.2026.110960
- Salinas-Camus, Goebel & Eleftheroglou (15 August 2025), *Mechanical Systems and Signal Processing*, article 113015, DOI 10.1016/j.ymssp.2025.113015, review PHM with emphasis on uncertainty, robustness, interpretability and feasibility.
  https://doi.org/10.1016/j.ymssp.2025.113015
- Lin, Yan & Zio (2025), *Reliability Engineering & System Safety*, article 112110, DOI 10.1016/j.ress.2025.112110, review uncertainty in RUL prediction and its calibration and decision implications.
  https://doi.org/10.1016/j.ress.2025.112110
- A 2026 *Sensors* review (26(6), 1915) surveys survival/time-to-event approaches under censoring/truncation and time-dependent Brier / integrated Brier metrics.
  https://www.mdpi.com/1424-8220/26/6/1915

These works motivate the evaluation design; none establishes a mining-specific accuracy target or qualifies Luminous' models.

## 3. Exposure and episode identity

Each benchmark case uses exactly one exposure unit:
- operating hours;
- cutting hours;
- accepted volume;
- cycles or starts.

Do not combine calendar-days, operating-hours, cutting-hours and accepted-volume in one score without an independently validated exposure transformation. Each episode records entry and exit exposure; the prediction origin in v0.1 is episode entry.

Each record binds to asset family/configuration, campaign, site, material cohort, machine configuration, source and adjudication. Repeated post-repair episodes may be represented separately, but an asset ID cannot appear in more than one data partition.

## 4. Failure, censoring and intervention semantics

- **FAILURE_EVENT:** observed failure with exactly one declared cause.
- **RIGHT_CENSORED:** failure time not observed, with an explicit censoring reason.
- **Competing causes:** cutter wear, pick fracture, bearing failure, thermal/electrical failure, hose degradation, contamination and other predeclared mechanisms remain distinct labels.
- **Interventions:** inspection, repair, preventive replacement or adjustment are separately recorded at a known exposure time.

The v0.1 censoring policy divides censor reasons:
- independent reasons (initially administrative end-of-study and loss to follow up);
- potentially informative reasons (preventive replacement, repair, transfer and unknown).

Potentially informative early censoring is excluded from the primary score, reported separately, and not treated as a failure. This does not solve informative censoring; sensitivity analysis and causal/intervention-aware models are future work.

## 5. Grouped data partitioning

The schema requires an explicit split profile and a list of held-out grouping dimensions. The synthetic profile prevents overlap across:
- asset ID;
- campaign;
- site;
- material cohort.

A stronger deployment programme should additionally evaluate leave-one-configuration-out and prospective campaign holdouts. Randomly splitting windows or adjacent inspections from one asset into train and test is not acceptable.

## 6. Competing-risk prediction outputs

A prediction row is:
- ALL_CAUSE_SURVIVAL: probability that no declared failure cause has occurred by horizon h;
- CAUSE_CUMULATIVE_INCIDENCE: probability that a specific declared cause has occurred by h before any competing cause.

All predictions identify episode, horizon (in the case's exposure unit), partition, model reference/digest and immutable prediction artifact.

Cause-specific cumulative incidences for an episode/model/horizon must sum to at most one. v0.1 evaluates supplied forecasts; it does not fit a Kaplan–Meier, Cox, random survival forest or Aalen–Johansen outcome model. Nonparametric baselines and calibration analysis belong in the next model-tournament increment.

## 7. Censoring-adjusted Brier score

Fit the censoring survival function G(t) using CALIBRATION episodes only, with delayed-entry risk sets:

at risk at t iff entry_exposure <= t <= exit_exposure

Only declared independent censor reasons reduce G(t). Failure events and informative interventions exit the censoring risk set without being counted as independent censor events.

For a prediction at horizon h:
- Failure event by h: known target, weighted by 1/G(T-).
- Observed event-free through h (event after h or censor at/after h): survival target 1 / cause-incidence target 0, weighted by 1/G(h-).
- Independent censor before h: unknown outcome; zero numerator contribution but retained in the ordinary IPCW denominator.
- Informative censor before h: excluded from the primary score denominator and counted separately.

For forecast p and observed IPCW target y, weighted loss is:

weighted loss = w * (y - p)^2

The horizon Brier score is the sum of weighted losses divided by the eligible prediction denominator. The evaluator reports denominator counts, observed-event counts, ordinary early censors and informative exclusions.

At event time T use the left-limit G(T-), not G(T); the convention is explicit when event/censor times tie. A horizon or event time with G equal to zero is a support violation: the evaluator fails closed rather than clipping or extrapolating the weight.

The Integrated Brier Score is calculated by trapezoidal integration across at least two declared horizons and divided by the horizon range. It is not computed for a single horizon.

## 8. v0.1 executable contract

Standalone Rust crate: symtropy-censoring-aware-rul-bench.

It validates:
- unique asset episode and prediction identities;
- event cause vs censor reason consistency;
- intervention time within episode exposure;
- exact exposure unit across all episodes;
- cause-specific probability sums <= 1;
- predictions only for validation/prospective episodes;
- exact partition binding between episode and prediction;
- no leakage across declared asset/campaign/site/material grouping dimensions;
- calibration-only censoring reference;
- no overlap between independent and informative censoring reason classes;
- censoring support G(t-) > 0 for every scored target.

It produces a calibration-only censoring KM curve, horizon-specific all-cause/cause-specific IPCW Brier scores and integrated Brier summaries.

## 9. Synthetic conformance replay

Fixture:
- 14 asset episodes (8 CALIBRATION, 6 VALIDATION);
- three competing failure causes;
- administrative/loss-to-follow-up censoring plus preventive replacement and repair;
- 48 forecasts at 5 and 8 cutting-hour horizons.

Calibration censoring curve:
- at 7 cutting-hours: risk set 5, one independent censor, G=0.8;
- at 9 cutting-hours: risk set 3, one independent censor, G≈0.5333;
- at 11 cutting-hours: risk set 1, one independent censor, G=0.

At the validation horizons, structural replay gives:
- all-cause survival Brier = 0.0656 at 5 hours;
- all-cause survival Brier = 0.07696 at 8 hours;
- trapezoidal two-horizon integrated Brier = 0.07128.

One ordinary administrative censor before the 8-hour horizon contributes zero loss but remains in the IPCW denominator; one early repair censor is excluded as potentially informative. All predictions and observations are fabricated conformance values, not model performance.

## 10. Model tournament and next extensions

Required baselines:
- Kaplan–Meier / Nelson–Aalen for all-cause reliability;
- Aalen–Johansen cause-specific cumulative incidence;
- parametric Weibull/log-normal survival;
- Cox proportional hazards with time-varying covariates;
- random survival forest;
- discrete-time hazard;
- degradation-trajectory and physics-informed wear;
- hybrid calibrated model with OOD/abstention.

Future extensions:
- interval-censored inspections;
- delayed-entry/left-truncation sensitivity tests;
- time-varying covariates and landmark predictions;
- informative censoring/IPCW sensitivity and intervention-aware causal models;
- uncertainty calibration, confidence bands and bootstrap uncertainty;
- maintenance decision utility using actual replacement/inspection costs.

## 11. Claim boundary and integration

This benchmark evaluates probability forecasts; a Brier score does not itself establish economic value, remaining-life accuracy or safe maintenance intervals. Prospective validation on unseen asset cohorts is required.

Composes with #3863/#3910 cutter health/morphology, #3991 metrology, #3927 material-flow/PSD exposure context, #3917/#4031 experiment selection and #3853 maintenance Pareto decisions. Maintenance actions remain subject to OEM limits and the asset owner's approved procedures.
