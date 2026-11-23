# Integrity Mode Outcome Evaluation

## Purpose

This module evaluates the operating-mode stream produced by the evidence-gated controller in `integrity_mode_controller.rs`. It joins sealed mode decisions to evaluator-only truth and fault lifecycle events to measure whether the controller downgraded quickly enough and recovered appropriately.

It is an **offline/evaluator-side benchmark tool**. Evaluator truth, NLOS labels, fault onset/clear events and outcome labels must not be available to the controller when it makes the corresponding decision. This module must never be imported into a production mode-decision path.

## Event contract

Each input event represents a half-open time interval:

`[timestamp_ms, timestamp_ms + duration_ms)`

and contains:
- monotonic evaluator event sequence;
- monotonic start timestamp;
- positive duration;
- the mode chosen by the controller for the interval;
- evaluator-only truth status for that interval;
- at most one fault lifecycle boundary at the interval start.

If the next event begins after the prior interval ends, the gap is recorded as unobserved. Overlapping intervals, non-monotonic timestamps, repeated sequences, duplicate fault starts and unknown/duplicate fault clears are rejected.

The event producer must seal the controller decision and source receipt before joining evaluator truth. Ground truth is used only to label the historical interval after the fact.

## Truth labels

- **QualifiedSafe**: evaluator says the position/decision was supported and no disqualifying fault was active.
- **UnsafeOrFaulted**: evaluator says a disqualifying fault or unsupported state was active.
- **Unknown**: truth is missing, ambiguous or not independently qualified.

UNKNOWN truth is not counted as a safe result or as a false-safe result. It is separately exposed as unknown-NORMAL time.

## Exposure metrics

The report accumulates time, not just event counts:

- **Time in NORMAL / DEGRADED / UNKNOWN / UNAVAILABLE**: duration in each output mode.
- **False-NORMAL exposure**: NORMAL duration while truth is UnsafeOrFaulted.
- **Unknown-NORMAL exposure**: NORMAL duration while evaluator truth is Unknown.
- **Unnecessary non-NORMAL exposure**: DEGRADED/UNKNOWN/UNAVAILABLE duration while truth is QualifiedSafe.
- **Unobserved gap time**: elapsed time with no evaluator event. It contributes to no mode and is not assumed safe.
- **Mode transitions**: count of changes between consecutive observed event modes.

A high NORMAL availability figure is not sufficient. It must be reported alongside false-NORMAL and unknown-NORMAL exposure. Likewise, high conservative availability loss must be assessed against unnecessary non-NORMAL exposure.

## Fault latency metrics

A fault episode is introduced by a unique `Start { fault_id }` event and ended by a corresponding `Clear { fault_id }` event.

- **Detection latency**: elapsed time from fault onset to the first observed event with a mode other than NORMAL. This measures downgrade latency; it does not prove the controller correctly diagnosed that specific fault.
- **Fault cleared without detection**: fault ended before any non-NORMAL mode was observed.
- **Active undetected fault at end**: fault has not cleared and no downgrade was observed.
- **Active detected fault at end**: fault has not cleared, but the mode has left NORMAL.
- **Recovery latency**: elapsed time from a detected fault's clear timestamp to the first subsequent NORMAL event.
- **Recovery pending at end**: fault cleared after detection, but NORMAL had not yet been regained.

Mean, P95 and maximum detection/recovery latencies are reported. Undetected, active and pending episodes are reported separately and are never silently dropped from the denominator.

When a fault begins while the mode is already non-NORMAL, its measured downgrade latency is zero. This is a useful availability/integrity state, but the evaluator should not interpret it as proof the controller detected that specific new fault.

## Required benchmark scenarios

Run the same frozen controller policy against:
- nominal supported operation with no injected fault;
- each single fault family: clock, anchor survey, antenna delay, channel, firmware, synchronization, geometry, calibration and NLOS/multipath;
- common-cause combinations and shared dependency groups;
- abrupt fault onset during NORMAL;
- fault onset while already DEGRADED/UNKNOWN;
- fault clearing with clean recovery evidence;
- a fault that clears before the controller downgrades;
- stale calibration and predictor/score binding mismatch;
- source truncation, incomplete fault enumeration and unbounded hard-fault envelope;
- posterior audit failure, replay and evidence-sequence rollback;
- unknown or missing evaluator truth;
- observation gaps during fault onset and recovery.

## Scoreboards (keep separate)

**Integrity**
- false-NORMAL exposure (ms and fraction of observed time);
- unknown-NORMAL exposure;
- fault episodes cleared without downgrade;
- active undetected faults;
- detection latency mean/P95/max by fault family and severity.

**Availability and continuity**
- NORMAL, DEGRADED, UNKNOWN and UNAVAILABLE time;
- unnecessary non-NORMAL exposure;
- transition count and mode-flapping episodes;
- recovery latency mean/P95/max;
- recovery-pending count and unobserved gaps.

**Resources**
- controller update latency p50/p95/p99;
- memory/state footprint;
- receipt verification/processing cost measured separately from the controller.

No weighted aggregate should conceal false-NORMAL exposure or fault-family failures. Report all mode policy parameters, alert limit, fault prevalence, decision period, time units, source commit and evaluator truth source.

## Current implementation

Implemented:
- typed interval events and truth labels;
- explicit gap accounting;
- four-mode exposure;
- false-NORMAL, unknown-NORMAL and unnecessary non-NORMAL durations;
- fault lifecycle identities and start/clear checks;
- first downgrade latency after fault onset;
- fault cleared without detection;
- active undetected/detected faults at end;
- recovery latency after truth clear;
- pending recovery at end;
- p50/p95-style latency summaries (mean, P95 and maximum);
- tests for false-normal, unknown truth, unnecessary degradation, detection/recovery, missed fault, gaps, replay and invalid lifecycle events.

Not implemented:
- signed or authenticated event receipts;
- an automatic adapter from live controller log format;
- per-fault severity weights or multi-fault causal attribution;
- confidence intervals for empirical mode metrics;
- physical UWB measurements, hardware timing or real deployment results;
- a formal proof that the controller is a sufficient fault monitor.

This tool evaluates observed traces; it does not establish the true detection probability or a PHMI bound by itself.
