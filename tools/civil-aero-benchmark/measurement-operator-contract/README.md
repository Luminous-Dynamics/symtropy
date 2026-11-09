# AER-01 measurement-operator compatibility contract v0.1

**Tracking:** [AERO-MEASUREMENT-OPERATOR-001 / #4213](https://github.com/Luminous-Dynamics/symtropy/issues/4213)  
**Related:** #3071 unsteady evidence; #4176/#4177 virtual sensing; #4190/#4195 response-vector VVUQ; #4165 inspection observability; #4093 exact aircraft-state bridge.  
**Status:** synthetic policy/contract implementation; no JAXA/NASA test data are included or admitted.

## Purpose

A measurement is not the physical state itself. It is an instrument-specific observation with spatial support, calibration, time base, transfer function, filtering, bandwidth, uncertainty, masks and provenance.

This crate provides a conservative, read-only contract to decide whether two channels can be directly compared. It does not read real sensor archives, filter signals, align clocks, compensate transfer functions, reconstruct missing fields, compute spectra or validate an aircraft.

The key distinction is that two channels from the same test article are not automatically comparable. An ESP point mean, a Kulite point RMS/PSD, a camera-integrated uPSP surface field, a strain-derived buffet index and a balance coefficient have different observation operators.

## V0.1 contract

Each synthetic channel binds:
- source, raw-data and derived-processing SHA-256 digests;
- aircraft/configuration, geometry, loading, environment, mission phase and test-article identity;
- instrument family/ID, quantity, unit, sign convention and statistic;
- point/section/surface/integrated/derived spatial support, sensor location, coordinate frame and registration digest;
- calibration ID, artifact digest, validity envelope and validity at the test state;
- statistic, time window, sample rate, time base and synchronization receipt;
- transfer-function digest, bandwidth, low-pass flag, integration/exposure time and spatial mask;
- observation state and value, with a separate missingness reason.

Missingness states are distinct: PRESENT, MASKED, UNAVAILABLE, NOT_COLLECTED and UNKNOWN. Only PRESENT can carry a finite number. A missing/unavailable/masked value cannot be silently filled with zero.

## Compatibility rules

The direct-comparison evaluator requires matching:
- state identity, quantity, units, sign convention;
- spatial support, location, frame and registration;
- statistic, time window and measurement-run identity;
- transfer function, bandwidth, filtering, exposure/integration and mask semantics;
- spectral grid and estimator when the channel is a PSD/cross-spectrum/coherence.

Calibration must be VALID and cover the test state. Missing or expired calibration causes abstention.

For coherence/phase, both channels must have an admitted synchronization receipt, the same clock/time base and sample rate. Scalar time-mean comparison does not require synchronized clocks, but it still requires matching time-window and operator semantics.

The v0.1 evaluator does not implement any operator transform. It will not interpolate surface data to a point, deconvolve camera exposure, align clocks, convert Hz to rad/s, normalize units, or infer a sensor mapping. A later transformation must be its own versioned artifact with an uncertainty budget and independent qualification.

## Synthetic case coverage

The fixture has 12 synthetic channels and 9 pair requests:
- one compatible same-location point-pressure time-mean pair;
- Kulite point RMS vs uPSP surface RMS (incompatible);
- a phase/coherence request with different clock IDs (unsynchronized);
- PSP-simultaneous balance CD (unavailable) vs standalone BL CD (abstain);
- strain-derived buffet coefficient vs vertical tip deflection (incompatible physical quantities);
- present vs masked uPSP surface values (abstain);
- stale geometry identity (incompatible);
- expired calibration (abstain);
- official deformation not collected (abstain).

All values and calibration identities are synthetic sentinels. They are not JAXA measurement values or calibration evidence.

## External interpretation motivating the contract

The JAXA DPW-8 Test Case 3 page distinguishes:
- balance data with PSP-simultaneous vs standalone BL provenance; drag is unavailable for PSP-simultaneous measurements;
- wing-root strain-derived buffet intensity C_B, normalized by zero-lift RMS;
- ESP mean point pressure;
- Kulite point-pressure RMS and sensor-specific PSD;
- uPSP surface mean/RMS/PSD/snapshots with masks and camera-exposure low-pass effects.

The page identifies a roughly 9 kHz Kulite PSD peak as wind-tunnel noise and states that official deformation measurements are pending; its preliminary 10–12 mm tip-deflection note at alpha 4.7° is qualitative only. JAXA's page carries an all-rights-reserved notice. The catalog in #4137 records these facts as source observations but has not acquired or redistributed the linked artifacts.

These observations are therefore included as contract scenarios only. No Test Case 3 physical data are included in this repository fixture.

## Local execution

From the repository root:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/measurement-operator-contract/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civil-aero-benchmark/measurement-operator-contract/Cargo.toml
cargo run --locked --quiet --manifest-path tools/civil-aero-benchmark/measurement-operator-contract/Cargo.toml > /tmp/measurement-operator-result.json
python3 tools/civil-aero-benchmark/measurement-operator-contract/verify_independent.py --rust-output /tmp/measurement-operator-result.json
~~~

The Python standard-library oracle does not import the Rust implementation. It independently validates source/operator identity and recomputes all compatibility dispositions. The focused exact-head Actions workflow asserts the checked-out subject SHA, uses the pinned Cargo.lock with --locked, and retains the fixture, toolchain/environment, result JSON and source/result SHA-256 receipts.

## Claim ceiling

The only positive output claim is measurement_operator_compatibility_only. This is not sensor calibration, PSD qualification, physical validation, structural-health determination, damage detection, aircraft safety, airworthiness or certification.

The next advancement is to apply this contract only after the relevant Test Case 3 CSV/ZIP bytes, terms, coordinate registration, calibration, time-base and observation transfer functions have been admitted. The JAXA files remain all-rights-reserved pending review; do not redistribute them.
