# Symtropy Sensor Observability Bench

Standalone, read-only evaluator for **fault-pair diagnosability** of a declared excavation / pneumatic-conveying sensor layout.

Implements [issue #3993](https://github.com/Luminous-Dynamics/symtropy/issues/3993), and composes with #3877/#3913 event truth, #3927 stage-resolved material flow, and #3987/#3991 metrology evidence.

## Run

    cargo test --manifest-path crates/domains/symtropy-sensor-observability-bench/Cargo.toml

## v0.1 method

For every required fault pair:
- all channel/state response cells must exist;
- every response binds to an exact channel unit, material/path/geometry domain, synchronized state window, source and adjudication;
- a channel discriminates the pair only if both responses are VALID, independently evidenced and strictly non-overlapping;
- channels assigned to the same independent-principle group count once, regardless of serial number or channel count;
- DEGRADED/MISSING channels never count as healthy signatures;
- a state-class SENSOR_FAULT must explicitly identify its faulted channel(s), and those channels must be DEGRADED/MISSING in that state;
- known unavailability caused by a declared sensor fault is not confused with unexpected telemetry dropout;
- unexpected missing/degraded/unknown channels or inadmissible model-only responses make a below-threshold pair INSUFFICIENT_EVIDENCE;
- a complete response matrix with too few discriminating independent principles is NOT_IDENTIFIABLE;
- the pair-specific minimum principle count cannot be lower than the versioned critical-pair layout threshold.

This v0.1 crate scores an explicitly proposed layout. It does not optimize or approve placement, learn response signatures, or set safety-critical operating actions.

## Synthetic fixture

- 7 selected channels;
- 6 independent-principle groups;
- 8 state hypotheses;
- 8 required fault pairs;
- 56 state/channel response records;
- two pressure-differential channels share one principle group;
- pressure and airflow sensor fault states explicitly mark the affected channel unavailable;
- OOD state deliberately has unknown response envelopes.

The fixture is fabricated. Its IDENTIFIABLE outcomes exercise interval and grouping logic only; they are not physical fault-coverage claims.

## Research motivation

- Borg, Refalo & Francalanza (2026), *Measurement* 258A, 118992 (DOI 10.1016/j.measurement.2025.118992), evaluate pressure-drop/cycle-time fault monitoring across three compressed-air system-complexity levels and seven fault-profile levels. This is adjacent pneumatic evidence, not coarse hard-rock qualification.
- Zhang et al. (2026), *Measurement* 272, 121117 (DOI 10.1016/j.measurement.2026.121117), report >96.4% current-only recognition for very dilute-phase conveying including blockage and water accumulation. Reproduce as a baseline only in its declared regime; do not transfer the score to coarse rock.
- A 2026 OOD process monitoring review (DOI 10.1016/j.dche.2026.100345; online 28 September 2026) distinguishes adaptive mechanisms from robust cross-domain representations and emphasizes the IID failure under aging and changing process conditions.
- Kościelny & Sztyber-Betley (2025), *Mechanical Systems and Signal Processing* 236, 112979, motivate explicit pairwise fault isolability.
- An October 2026 multimodal sensor-fusion review (DOI 10.3390/s26216741) highlights gaps in RUL estimation, degradation forecasting and principled fusion in adjacent conveyor-belt fault research.

## Non-claims

Response intervals in the fixture are synthetic. A pair reported as structurally identifiable under a synthetic fixture is not physically diagnosable. Real response envelopes must come from independent calibrated measurements on the declared material/path domain; common-mode dependencies must be reviewed, and prospective unseen campaigns must be evaluated.
