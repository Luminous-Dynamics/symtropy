# GAUGE temporal observability contract v0.1

**Parent:** [BENCH-GAUGE-RIGID-001 #6446](https://github.com/Luminous-Dynamics/symtropy/issues/6446)  
**Issue:** [BENCH-GAUGE-RIGID-TEMPORAL-OBSERVABILITY-001 #6586](https://github.com/Luminous-Dynamics/symtropy/issues/6586)  
**Scope:** explicit sampling arithmetic and processing-provenance gate; no source data imported.

## Observed GAUGE sampling lineage

The GAUGE paper states that 16 motion-capture cameras acquire rigid-body trajectories at 180 Hz, then the valid recording interval is truncated and trajectories are downsampled to 30 FPS for JSON. Rigid-body and textile simulator states are also exported at 30 Hz from simulations running at 180 Hz.

Thus:
- 180 Hz is the reported acquisition/simulation state rate;
- 30 Hz is the released trajectory export rate;
- the source/export ratio is 6:1;
- the export interval is 33.333 ms;
- the export Nyquist frequency is 15 Hz.

The raw JSON declares `FPS=30`, correctly describing its own sample cadence. It does not preserve the original 180 Hz acquisition cadence in the inspected records.

The exact downsampling filter, sample phase/anchor, timestamp map and uncertainty propagation were not found in the inspected files/paper. This makes them explicit provenance gaps rather than assumed processing details.

## Contract

The reference keeps two outputs separate.

### 1. Temporal coverage

Given an exported sample rate and explicit, externally declared event requirements:
- event duration;
- minimum sample count across that event;
- maximum signal bandwidth that the claim needs to preserve;

derive the sample interval, Nyquist frequency and expected samples across the event.

The result is:
- `MeetsDeclaredThresholds`;
- `Undersampled`;
- `Unassessable` when event requirements are missing.

These thresholds are application-specific design inputs, not universal constants. Passing them does not prove the actual sensor response or measurement fidelity.

### 2. Processing provenance

Independently require evidence for:
- source acquisition rate;
- downsampling/resampling filter;
- sample phase/anchor;
- timestamp mapping;
- uncertainty propagation.

A trajectory can meet declared sample-count and Nyquist criteria while remaining unqualified because its processing lineage is incomplete.

## Synthetic known-answer example

For an explicitly synthetic 30 ms event with a 25 Hz signal bandwidth and a five-sample minimum:
- 30 Hz export yields 0.9 samples across the event and 15 Hz Nyquist: **undersampled**;
- 180 Hz samples yield 5.4 samples across the event and 90 Hz Nyquist: the declared arithmetic thresholds are met.

These event duration, bandwidth and sample-count values are illustrative inputs. They are **not** measured GAUGE contact-event characteristics. No conclusion about actual impact bandwidth or duration is drawn from them.

## Claim boundary

This contract does not reconstruct high-rate trajectories from 30 Hz output, perform anti-alias filtering, infer physical event bandwidth or validate a measurement system. It prevents a 30 Hz pose export from being treated as evidence of sub-frame impact timing or high-frequency peak force without a separate qualified source-rate or force/impulse measurement.
