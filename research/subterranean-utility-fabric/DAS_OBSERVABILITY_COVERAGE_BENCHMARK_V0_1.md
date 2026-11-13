# DAS Observability Coverage Benchmark v0.1

**Owner:** Symtropy #4972  
**Status:** proposed qualification protocol; not a measured benchmark  
**Date:** 2026-11-13

## 1. Why coverage needs a new definition

A single percentage such as "95% of the fibre is monitored" hides whether the system can detect the event that matters. DAS observability varies with event class, minimum event magnitude, coupling, installation, route environment, interrogator configuration, timebase and system health.

A segment may be observable for a large impact but not a small leak-related vibration; observable for traffic but not excavation; observable during dry conditions but degraded during rain; or observable at the start of a route but not after a splice or interrogator configuration change.

The benchmark therefore qualifies a conditional detection surface, not a binary monitored/unmonitored label.

## 2. Qualification unit

The smallest reportable qualification cell is:

**route segment × event class × declared minimum magnitude × environmental profile × fibre/coupling profile × interrogator configuration × time interval.**

The unit must also bind the exact fibre identity and revision, route revision, optical path, channel/gauge-length configuration, timebase and raw-data artifact.

A result does not transfer automatically to another cell. Aggregation is permitted only when the denominator, profile and weighting are explicit.

## 3. Required metrics

### 3.1 Detection performance

For every class and declared magnitude band:

- probability of detection (POD), with numerator, denominator and confidence interval;
- critical-event miss rate, with the event definition and consequence class;
- false alarms per observable fibre-km-day and per 1,000 healthy channel-hours;
- detection latency P50/P95/max from independently timestamped event onset;
- localization error along fibre chainage P50/P95/max;
- SNR distribution and minimum validated signal margin;
- repeatability across events, sessions and relevant environmental conditions.

Do not calculate a false-alarm rate per fibre-km-day by dividing by nominal route length when portions were blind, degraded, unavailable or not assessed. Report both nominal and actually observable exposure.

### 3.2 Spatial coverage

For each segment and time interval, report separate length and duration fractions for:

- qualified observable;
- degraded / conditional;
- unobserved;
- unusable / decoupled;
- unavailable because of interrogator or communication outage;
- not assessed;
- unknown.

A coverage map is a product of the calibration and observability models. It must carry route revision, valid time, method revision, uncertainty and the reasons for all gaps.

### 3.3 Calibration and uncertainty

- 90% and 95% interval empirical coverage on held-out profiles;
- mean and P95 interval width;
- calibration error by event class and installation profile;
- risk-versus-coverage curves when the system abstains;
- false high-confidence assertions in invalid, ambiguous or out-of-domain cells.

Calibration is not established by a model reporting a confidence score. It requires independent reference truth and held-out evaluation.

## 4. Benchmark populations and splits

Keep distinct populations for:

1. operational communications fibre adjacent to a medium-voltage cable;
2. directly buried sensing fibre;
3. fibre installed in conduit;
4. shared-trench fibre offset from the target asset;
5. fibre with slack, microbends, coils, splices or branch points;
6. traffic-induced route geolocalization;
7. laboratory audio-replay or other laboratory-generated fibre vibration;
8. subsea observatory DAS;
9. exploratory controlled mechanical disturbance.

These populations support different claims. Laboratory audio replay can test ingestion and signal-processing; subsea data can test long-duration continuity and degraded states; neither by itself qualifies terrestrial cable-fault detection.

Use grouped splits at the highest independent level available: corridor/site, campaign/day, event session, fibre span, physical route and interrogator/configuration. Overlapping windows, repeated captures and augmentations from one acquisition remain in one split. A random-window split is not a valid site-generalization result.

## 5. Baselines and comparators

The same frozen data and acceptance rules must be used for:

- the interrogator/vendor event classifier where outputs and terms permit;
- simple energy/threshold and band-energy detectors;
- matched-filter baseline where a reference waveform exists;
- deterministic one-class/background anomaly detector;
- candidate Luminous classifier;
- candidate classifier with calibrated uncertainty and abstention;
- human adjudication as a separate comparator, with reviewer time and agreement recorded.

Compare alerting workflows as well as classifiers. A higher recall system that sends many more false dispatches may have worse operational value.

Vendor specifications for maximum range, channel pitch, gauge length or nominal resolution are not substitutes for measured POD, false alarms, localization error or uptime under a matched fibre profile.

## 6. Pre-registration and reporting

Before running an evaluation, freeze:

- population, site and independent split groups;
- event taxonomy and severity mapping;
- minimum relevant event magnitude per class;
- truth-generation or adjudication method;
- fibre length and time actually at risk;
- interrogator and optical configuration;
- route calibration and uncertainty;
- all thresholds and decision rules;
- confidence-interval method and treatment of correlated events;
- acquisition, compute, storage, analyst and field-verification costs.

Preserve all scenario failures, including incomplete, rejected and indeterminate runs. Report class-specific confusion matrices, calibration curves, false-alarm exposure and uncertainty. Do not collapse them into one scalar SOTA rank.

## 7. Proposed research gates (not industry standards or achieved results)

**Integrity gates**

- Zero high-confidence event coordinates when the fibre identity, timebase, optical configuration or required route transform is invalid.
- 100% correct rejection or indeterminate handling of seeded stale-calibration, endpoint reversal, clock-offset, channel-dropout and route-revision cases in the frozen finite corpus.
- 100% of in-scope fibre length assigned an explicit coverage state for each reported interval.
- Zero conversion of unobserved or unavailable intervals into a "no event" claim.

**Statistical gates**

- 95% empirical coverage for nominal 95% uncertainty intervals on held-out, in-domain profiles, with confidence intervals and interval width published.
- Event-detection targets set per class and magnitude after baseline reproduction and operator consequence analysis; no global 99% target.
- False-alarm budget set in units of events per observable fibre-km-day, with the tolerated burden agreed before test evaluation.
- Report P50/P95 detection latency and location error separately.

**Research ambition**

- Demonstrate materially lower false-dispatch cost at equal or lower critical-event miss rate than a vendor baseline on matched data, or demonstrate equivalent performance with a materially lower total lifecycle cost.
- Demonstrate increased useful observability per dollar through adaptive calibration, fibre selection and targeted independent measurements.
- Demonstrate cross-site transfer without hidden fine-tuning on held-out site labels. A transfer failure is reported as a domain boundary, not patched away.

## 8. Executable reference evaluator

The standalone Rust package implements the finite-cell arithmetic in `tools/suf-fibre-calibration/src/coverage.rs`. It requires every in-scope segment length to be allocated exactly once across observable, degraded, unobserved, unusable, unavailable, not-assessed and unknown states. It reports POD only when independent truth and eligible event trials exist, emits a 95% Wilson interval, and computes false alarms only per fully observable fibre-km-day. Degraded and blind exposure remain separate. These are calculation/contract tests, not field measurements.

## 9. Claim ceiling

This is a proposed benchmark contract. It contains no observed POD, field coverage, false-alarm, localization or cost result. Passing a synthetic integrity corpus does not establish physical DAS performance. Vendor claims and literature-specific metrics remain separate from Luminous measurements.
