# WPT first-failure lifecycle evidence v0

**Status:** typed, single-cohort right-censored first-failure ledger and descriptive Kaplan–Meier evaluator. It is not a charger design, accelerated-life model, field-life predictor, maintenance scheduler, safety assessor or certification.

## Why this is a programme differentiator

The 25 October 2026 UAV charging review notes that actual UAV-specific service-life data are missing across the charging methods it surveys. Performance demonstrations therefore do not yet establish unattended lifetime.

This v0 creates the minimum reproducible evidence contract to collect those missing data without conflating:
- contact mating cycles with IPT receiver operating hours;
- battery-swap actuator cycles with power-line gripper cycles;
- accelerated bench exposure with field exposure;
- repeated cycles on one asset with independent component units.

## Input contract

One TSV row per physical component serial. Exact header:

    test_group_id evidence_class bearer component_family hardware_sha256 site_id environment_stratum test_regime exposure_unit measurement_sha256 failure_criterion_sha256 asset_serial exposure_value outcome failure_mode valid_until_us

A row outcome is:
- FAILED: first failure occurred at exposure_value; failure_mode is a predeclared identifier.
- RIGHT_CENSORED: observation ended at exposure_value without failure; failure_mode must be NONE.

Allowed test regimes: BENCH, FIELD, ACCELERATED. Allowed exposure units remain separate: MATING_CYCLES, THERMAL_CYCLES, POWER_ON_HOURS, FLIGHT_HOURS, CALENDAR_HOURS, ENERGY_THROUGHPUT_MWH.

All records in one file must share a test group, evidence class, bearer/component family, exact hardware revision, site/environment, test regime, exposure unit, measurement digest and failure-criterion digest. Asset serials must be unique. This prevents accidental pooling across physical revisions or stress profiles.

## Kaplan–Meier semantics

For each distinct failure exposure t:
- at-risk count includes all units whose observed failure or censoring exposure is >= t;
- d(t) counts units that failed at t;
- the survival estimate is multiplied by (1 - d(t)/at_risk(t)).

When a failure and a right-censor occur at the same exposure, the censored unit is included in that exposure's risk set before removal. This tie convention is fixed and deterministic.

The estimator assumes independent component units and non-informative right-censoring conditional on the declared test group. The tool records those as assumptions; it cannot prove them. v0 reports the descriptive curve only, not confidence intervals or an acceleration model.

Accelerated, bench and field groups are never pooled. No Arrhenius, inverse-power or other stress acceleration extrapolation is performed. Accelerated exposure is not converted to field years or flight hours.

## Hardware-specific measurement examples

- Conductive: four-wire contact resistance, contact dropout and thermal rise over mating cycles.
- IPT/CPT: coil Q, receiver temperature, capacitor ESR/capacitance and delivered-power derating over power-on/thermal cycles.
- Battery exchange: jam, latch failure, actuator degradation and mis-seat over mechanical cycles.
- Power-line perch: gripper degradation, cable-interface effects, weather and EMI exposure.
- RF/optical: tracker/gimbal faults, window transmission/fouling and emitter/receiver operating hours.

The actual failure criterion and measurement channels must be calibrated and frozen by the physical test owner. This software only binds their digests.

## Run

    cargo test --locked --manifest-path tools/wpt-lifecycle-evidence/Cargo.toml
    cargo run --locked --manifest-path tools/wpt-lifecycle-evidence/Cargo.toml -- research/wpt-lifecycle-evidence-v0/synthetic-contact-cohort.tsv

## Claim ceiling

A pass establishes only parser semantics and the described Kaplan–Meier arithmetic for synthetic/declared first-failure observations. It does not establish actual service life, field reliability, maintenance intervals, censoring assumptions, test-unit independence, accelerated-to-field mapping, hardware safety or operating permission.
