# HoloFrac Temporal Observation Fixture Protocol v0

**Status:** synthetic benchmark fixture; not measured building data  
**Parent:** HoloFrac Phase 2 protocol and temporal world-model research  
**Implementation:** \`research/holofrac-retrieval/src/temporal.rs\`  
**Executable:** \`research/holofrac-retrieval/src/bin/temporal_fixture_sweep.rs\`  
**Related issue:** [#5974 — sensor placement, virtual sensing and adaptive observation](https://github.com/Luminous-Dynamics/symtropy/issues/5974)

## 1. Purpose

Provide a deterministic event stream that can test the HoloFrac/Symthaea temporal world-model interface before any measured building dataset is integrated.

The fixture explicitly separates:
- exact source observations and their revision/event lineage;
- the time a measurement is valid in the simulated world;
- the time the event becomes available to the decision system;
- missing measurements;
- later corrections that supersede earlier source events;
- evaluator-only truth used to score forecasts and virtual-sensor estimates.

The generator creates six channels with explicit units: temperature (degC), illuminance (lux), relative humidity (%RH), moisture (fraction), airflow (m/s) and occupancy density (persons per 100m2). Coordinates use a declared building-local metric frame. The analytic process is a controlled synthetic fixture, not a calibrated building model.

## 2. Event record contract

Each \`ObservationEvent\` contains:
- event ID;
- sensor ID and spatial coordinate;
- channel, unit (through the channel schema) and coordinate-frame type;
- valid/effective timestamp in microseconds;
- recorded/observed timestamp in microseconds;
- source revision and calibration revision;
- event kind: observed, missing or correction;
- optional value (missing is \`None\`, never zero);
- exact superseded parent event ID for corrections.

A correction is a new event with the same logical sample key and a strictly higher source revision. It never mutates or deletes the original event. The source event stream is sorted by recorded time and event ID for deterministic replay.

The fixture's non-security digest is only a stable regression identifier. It is not a cryptographic receipt, integrity guarantee or authority token.

## 3. Bitemporal snapshot semantics

A decision snapshot at cutoff \`t\` includes only events with \`recorded_at <= t\`. An event whose valid time is earlier than \`t\` but whose recorded time is later is excluded from that historical decision.

For each sensor/channel/valid-time key, the snapshot selects the latest source revision available at the cutoff. A correction can supersede an earlier event only after the correction's recorded time. Evaluator-only truth is held in a separate collection and is never copied into the decision snapshot.

Required metamorphic property:

**Adding an event with recorded time after a decision cutoff cannot change the snapshot at that cutoff, even when its valid time is earlier.**

This is a fixture-level contract. Real adapters must preserve their source event IDs, trusted time quality, calibration lineage, producer authority and data rights.

## 4. Current synthetic process

The initial generator uses:
- deterministic irregular timestamp jitter around a nominal sampling interval;
- deterministic sensor coordinates distributed across the unit square;
- bounded channel-specific noise;
- configurable missing-observation probability;
- configurable delivery delay;
- configurable correction probability and correction delay;
- a deterministic analytic spatiotemporal field for each channel;
- evaluator-only truth records at every sensor/time/channel sample.

The generator is intentionally compact and synthetic. It does not model sensor physics, cross-calibration, measurement uncertainty distributions, thermal PDEs, real occupancy patterns, or a real building's control system. Those need separate profiles and measured validation.

## 5. First benchmark matrix

The initial executable sweeps 8/16/32 sensors against missing-observation rates 0/10/25%. It emits raw CSV with:
- source SHA metadata;
- seed and generator configuration;
- sensor count, time steps and channel count;
- event, observed, missing, correction and delayed counts;
- late events that are excluded at the historical cutoff;
- evaluator-truth count and decision-snapshot counts;
- fixture generation time and stable manifest digest.

Later tracks should add:
- sensor placement: regular grid, random, maximin and decision-aware;
- virtual sensing: leave-one-sensor-out and sparse network recovery;
- held-out spatial regions and full held-out temporal episodes;
- calibration drift, replacement, sensor failure and correlated failure domains;
- adaptive measurement scheduling with cost/battery/bandwidth budgets;
- cross-channel coupling and nonstationary/seasonal changes;
- forecast horizons and probabilistic calibration.

## 6. Required comparisons before an HLB claim

- persistence and seasonal baselines;
- nearest-neighbour and bilinear/trilinear interpolation;
- RBF and Gaussian-process interpolation;
- AR/VAR and Kalman/state-space estimators where assumptions apply;
- LSTM/GRU and CfC/LTC models where the task fits;
- static HDC, temporal-only models and HDC + temporal dynamics;
- random, regular-grid, maximin and sequential A-optimal sensor placement;
- uncertainty-only and expected-decision-value measurement selection.

Compare identical sensor locations, observation masks, delivery delays, correction cutoffs, training data, spatial/time holdouts, compute and memory budgets. Do not randomly split adjacent samples from the same episode across training and test.

## 7. Metrics

Report per-channel/per-horizon MAE/RMSE, skill against persistence, NLL/CRPS where distributions are emitted, interval coverage and width, calibration, virtual-sensor error, spatial coverage, false alarms, detection delay, sensor count and cost, bandwidth/energy where measured, p50/p95/p99 latency, peak RSS, build/update cost and all failures/abstentions.

A virtual-sensor estimate must remain typed as inferred and carry uncertainty and source lineage. It cannot count as an independent physical observation or replace a failed sensor's evidence without explicit domain qualification.

## 8. Claim ceiling

Passing this fixture's tests establishes deterministic generation, explicit missingness and bitemporal replay semantics over the synthetic event model only. It does not demonstrate temporal prediction quality, real sensor redundancy, virtual-sensing accuracy, building energy savings, occupant outcomes, physical sensor independence, engineering evidence or regulatory compliance.
