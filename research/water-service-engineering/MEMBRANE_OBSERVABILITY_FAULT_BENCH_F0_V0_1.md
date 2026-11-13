# Membrane Observability & Fault Diagnosis F0 v0.1

**Issue:** [#5056 MEMBRANE-OBSERVABILITY-FAULT-001](https://github.com/Luminous-Dynamics/symtropy/issues/5056)  
**Parent:** [#4977](https://github.com/Luminous-Dynamics/symtropy/issues/4977)  
**Status:** synthetic transparent residual-rule baseline only. No real plant diagnosis or control authority.

## Research thesis

A process anomaly is not automatically membrane fouling. Sensor bias, gain drift, stuck-at values, clock skew, missingness, probe fouling, temperature/viscosity, feed conductivity/osmotic pressure and pump/valve changes can produce similar symptoms.

The 2026 membrane-fouling review identifies sensor drift, missing data, temporal/external validation and safe control integration among the gaps in the literature. A 2026 SWRO physics-informed spatiotemporal graph study reports strong results on its specific 1,000 m³/day plant anomaly task, but it is not a directly comparable fouling benchmark. Our contribution should be to evaluate causal discrimination and false diagnosis under controlled, independently labelled confounders—not to claim superiority from mismatched F1 figures.

## F0 synthetic suite

The first 13 cases cover:
- healthy reference;
- true normalized resistance growth;
- pressure-pair drift;
- flow-pair bias;
- clock skew;
- critical-channel dropout;
- temperature shift;
- feed salinity/conductivity shift;
- pump-speed regime change;
- fouling plus pressure-sensor drift;
- fouling plus salinity shift;
- unobservable resistance;
- excessive resistance uncertainty.

Every case uses the same explicit baseline thresholds. The oracle is separate from inputs. The resulting classifications are:
`NO_FAULT`, `FOULING_SUSPECTED`, `SENSOR_FAULT_SUSPECTED`, `OPERATING_REGIME_SHIFT`, `MULTIPLE_CAUSES`, or `INDETERMINATE`.

## Transparent baseline semantics

The baseline marks an instrument problem if independent pressure/flow channel residuals, clock skew or critical-channel missingness cross the declared thresholds. It marks a regime shift if temperature, feed conductivity or pump speed changes beyond the declared limits. It accepts a fouling suspicion only when the temperature/feed-normalized resistance proxy is observed, its relative uncertainty is within the specified limit, and the proxy exceeds its threshold.

The baseline does not infer the chemical foulant or prescribe CIP. When evidence is incomplete, it abstains. The thresholds are illustrative synthetic benchmark parameters, not plant operating limits.

## Metrics to report

Event-level and exposure-normalized metrics matter more than row accuracy:
- macro-F1 and per-class precision/recall;
- confusion between instrument, regime and physical-process causes;
- false fouling alarms under sensor-only and regime-only controls;
- detection delay and localization delay once temporal injection series exist;
- limit of detection versus injected severity;
- alarm lead time on independently labelled degradation events;
- false alarms per sensor-day/module-day/plant-week;
- calibration, coverage and selective risk with abstention.

A zero-CIP false recommendation target on sensor-fault-only cases is a safety guardrail. The diagnostic baseline itself is advisory and emits no CIP action.

## Next F1/F2 work

Extend single-window cases into time-indexed injected traces. Keep fault families and random seeds separate across training/calibration/test. Use matched pairs, overlapping causes, complete held-out regimes and hidden fault maps. Add robust parity/residual checks, EWMA/CUSUM, tree, GRU, physics-informed observer and physical-pathway graph baselines with ablations. Do not call synthetic test performance plant-generalization.

## Nonclaims

This corpus does not establish sensor-fault detection on a real plant, fouling mechanism identification, water quality, membrane integrity, cleaning benefit or a deployable digital twin.
