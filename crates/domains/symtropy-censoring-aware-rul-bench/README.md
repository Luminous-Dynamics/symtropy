# Symtropy Censoring-Aware RUL Bench

An evaluation contract for asset-episode time-to-event prognosis with right censoring, competing failure causes, maintenance interventions, exposure-normalized time and inverse-probability-of-censoring-weighted (IPCW) Brier scoring.

Implements [issue #4028](https://github.com/Luminous-Dynamics/symtropy/issues/4028).

## Run

    cargo test --manifest-path crates/domains/symtropy-censoring-aware-rul-bench/Cargo.toml

## v0.1 scope

- One exposure unit per benchmark case (for example cutting-hours, operating-hours, accepted-volume or cycles). Calendar time and cutting time are not silently mixed.
- Each asset episode has entry and exit exposure, failure or right-censor status, failure cause if observed, censoring reason if censored, site/campaign/material/configuration identifiers and intervention history.
- Preventive replacement, repair, asset transfer and unknown censoring reasons are treated as potentially informative. They are not silently treated as independent administrative censoring.
- A separate CALIBRATION partition estimates the censoring survival function G(t) using only declared independent censoring reasons.
- VALIDATION / PROSPECTIVE_HOLDOUT predictions are scored with IPCW Brier loss for all-cause survival and cause-specific cumulative-incidence probabilities.
- Predictions are anchored at episode entry in v0.1. Delayed entry is represented in exposure time and enters the calibration risk set.
- A zero censoring-survival support value at the prediction horizon is a hard support violation, not an infinite weight or silent clipped score.

This is an evaluator, not an RUL model trainer, Aalen–Johansen implementation, maintenance authority or production deployment.

## IPCW Brier definition

For an episode with follow-up exposure T and horizon h:
- failure/event observed by h: target is 0 for all-cause survival; for cause-specific incidence it is 1 only for the matching cause and 0 for competing causes. Weight is 1/G(T-).
- observed event-free through h (event after h or censoring at/after h): target is 1 for survival and 0 for each cause-specific incidence. Weight is 1/G(h-).
- ordinary independent censoring before h: outcome is unknown; contribution is zero under the standard IPCW estimator.
- preventive intervention / repair / transfer / unknown censoring before h: excluded from the primary score denominator and reported separately, because independent censoring is not assumed.

The Brier score is the IPCW weighted squared-error sum divided by the number of eligible evaluation predictions. It is not ordinary MAE on failed-only units.

## Limits and non-claims

The synthetic fixture uses abstract cutting-hour episodes and fabricated forecasts. It does not demonstrate a physical RUL model or maintenance improvement. A real use requires validated failure adjudication, censoring mechanism review, exposure definitions, episode-level leakage controls, calibrated predictions and prospective asset-cohort validation.
