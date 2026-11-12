# TLS Precision Health Monitor

Standalone Rust 1.96 monitor for longitudinal changes in the repeatability of one pinned physical TLS unit under one pinned check-field configuration.

## Statistical unit and method

The independent unit is an independently acquired **capture/check-target session**. A session contains one within-session standard-deviation summary per stable reference target. The repeated measurements used to produce that summary are not counted as independent sessions.

For each target, the monitor compares the median session standard deviation in the baseline period with the median in the current period. The test statistic is the maximum absolute log standard-deviation ratio across all targets. It performs a seeded, deterministic permutation test by shuffling baseline/current labels across whole independent sessions while preserving period sizes. The max statistic provides a family-wise control across the monitored targets under the session-label exchangeability null.

The reported ratio is:

`current median within-session SD / baseline median within-session SD`

A ratio above 1 indicates degraded repeatability; below 1 indicates improved repeatability. A significant change in either direction is reported.

## Scope and safeguards

- The sensor unit, firmware, profile digest, acquisition configuration, control artifact/reference digest, coordinate frame, range/incidence and environment strata are bound at input level and therefore apply to both periods.
- Every session must have the exact same target set, unique session identity, and at least six repeated measurements per target.
- Baseline and current periods each require at least five independent sessions.
- Session and target ordering is canonicalized before the seeded permutation procedure.
- Input is bounded to 4 MiB; at most 100 sessions per period, 128 targets and 100,000 permutations.
- The exact serialized input is SHA-256 bound into the report.

## Claim ceiling

This is a repeatability/precision-health monitor only. It does **not** establish absolute accuracy, calibration validity, registration validity, instrument acceptance, survey quality, physical building change or fitness for a task. The workflow deliberately emits `accuracy_disposition = NOT_EVALUATED` and `task_fitness_disposition = NOT_EVALUATED`. A significant precision change should trigger investigation and re-verification, not an automatic survey correction or asset-state mutation.

The session-label permutation test assumes independent sessions and exchangeability under the no-change null. Correlated sessions, configuration changes, different control artifacts, temperature/material strata or changes in the measurement/reference procedure invalidate direct comparison. P1 requires real unit-specific check-field data and a pre-registered sampling/false-alarm protocol.

## Run

```bash
cargo test --locked --manifest-path tools/scan-bim-tls-health-monitor/Cargo.toml
cargo run --locked --manifest-path tools/scan-bim-tls-health-monitor/Cargo.toml -- \
  --input tools/scan-bim-tls-health-monitor/fixtures/no-change.json
```


## Method identity: not the NIST/OSAC IPA Part II reproduction

This crate implements a **lightweight session-level sentinel**, not the published Gregg–Muralikrishnan–Shilling / NIST–OSAC IPA Part II statistical procedure. It compares per-target median within-session standard deviations with a max-statistic session-label permutation test.

The 2026 IPA Part II method instead:
- estimates a full 3×3 covariance matrix for spherical residuals `(azimuth, elevation, range)`;
- retains all six unique variance/covariance components;
- applies O'Brien's robust covariance-equality transformation to median-centred residual products;
- tests the resulting six-dimensional transformed vectors with one-way MANOVA, which for two timepoints reduces to Hotelling's T²;
- pairs the test with covariance/data-ellipse interpretation.

Use this sentinel for a cheap, explicit health trigger. Use the separate IPA statistical consumer in `tools/scan-bim-tls-ipa-statistic/` for the published robust multivariate method. Neither should be called an absolute-accuracy or task-fitness test, and the IPA consumer still depends on independently qualified centralized-frame residual production.
