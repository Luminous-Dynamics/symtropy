# AeroWeft — Heterogeneous Sensor Temporal Evidence

**Tracking issue:** #5398  
**Composes:** Track M #5275, Track L #5270, Track Q #5351, Track P #5337, Mycelix #435/#5899/#5904  
**Status:** synthetic-first temporal-evidence admission oracle  
**Claim ceiling:** structural qualification of declared time relations only.

## Research question

When multiple sensing modalities contribute observations to one aviation assessment, what evidence justifies treating those observations as temporally comparable? Which claims are valid when only an acquisition-cycle index or file order is available?

The central distinction is:

- sequence index identifies or orders records;
- acquisition-cycle identity associates records with a software-controlled batch;
- bounded capture-time evidence supports a bounded temporal relation;
- authenticated, qualified timing evidence is still separate from physical sensor calibration, target truth, surveillance coverage and operational DAA performance.

## Public research anchor: TSMS-Drone

Han & Jung, "A Time-Synchronized Multi-Sensor drone dataset acquired from multiple radars and RF receiver", Scientific Data 13, 407 (published 10 February 2026), https://doi.org/10.1038/s41597-026-06802-6.

Dataset: TSMS-Drone, Figshare+ DOI 10.25452/figshare.plus.30027313.v1.

The paper reports:
- CW radar, FMCW radar and RF receiver data for four commercial drone types and one non-drone target;
- controlled distances from 2 m to 30 m, in 2 m intervals, with repeated measurements (500 repetitions per condition);
- raw complex I/Q samples for CW and RF, but chiefly preprocessed FMCW range-Doppler frames (161 × 4096); one representative raw FMCW sample and reconstruction example are supplied;
- one software acquisition controller/master loop, common sequential acquisition-cycle/file indexes, and no internal timestamps in the stored files;
- capture and processing durations that differ across sensors, reported at approximately 243 ms for FMCW, 252 ms for CW and 58 ms for RF;
- a dataset designed for multimodal detection/classification and distance-aware fusion in controlled short-range conditions.

The paper's "time-synchronized" acquisition architecture is a useful dataset-level association method. Without independently measured per-sensor clock error, it is not interchangeable with a hardware-PPS/PTP traceable common timebase for aviation-grade real-time fusion. The sequence index is a join key, not a timestamp.

The data availability statement identifies the dataset as CC BY 4.0. The paper itself is under a separate CC BY-NC-ND 4.0 publication licence. Check the data release's exact licence and attribution at retrieval; do not assume the article licence and dataset licence are identical.

## Ownership and non-goals

Track X owns:
- dataset release and object-manifest admission fields for temporal use;
- sensor capture-window and clock-relation semantics;
- a bounded time-alignment oracle;
- leakage-resistant trial/session split requirements;
- synthetic controls for file-order and software-cycle overclaim.

It consumes:
- Track M #5275 for observability/coverage claim semantics;
- Track L #5270 and Mycelix #435 for time-scale/clock-correlation evidence;
- Mycelix #5899/#5895 for authenticated sensor provenance and policy;
- Mycelix #5904 for common-mode failure domains;
- Track P #5337 for causal source/parent receipts;
- Track Q #5351 for aviation-facing trust custody;
- Track T/U for dataset identity and change/requalification.

Track X does not create a clock service, sensor driver, detector, tracking engine, fusion model, source registry, provenance verifier or aircraft authorization service.

## Temporal evidence classes

1. **INDEPENDENT_HARDWARE_BOUNDED** — each sensor's capture window is measured against the same qualified independent hardware timing reference, with bounded uncertainty.
2. **SHARED_CLOCK_BOUNDED** — sensors use one qualified clock-domain/profile, with declared offset/drift bounds and a named time scale.
3. **SOFTWARE_CYCLE_ASSOCIATED** — files belong to one software acquisition cycle, but no independent per-sensor timestamp/uncertainty is available. Supports per-cycle association only.
4. **ORDER_ONLY_OR_UNKNOWN** — file ordering or sequence exists without defensible temporal relation. Modality-specific analysis only.

The Rust oracle's positive time disposition means only that the supplied capture-window intervals, clock/reference identities and declared skew/drift policy satisfy a structural bound. It does not authenticate the reference, prove calibration, or establish physical simultaneity.

## Benchmark implementation

The dependency-free crate is tools/aeroweift-sensor-time-evidence/.

Each sensor capture carries sensor ID, modality, acquisition-cycle ID, file sequence, raw/processed status, preprocessing digest, calibration-profile digest, time scale, clock identity, reference identity, drift bound and optional capture window.

The policy binds the expected time-evidence class, policy digest, time scale, shared clock profile or independent timing reference, maximum pairwise skew and drift allowance.

The output distinguishes:
- BOUNDED_FUSION_TIME_ELIGIBLE
- PER_CYCLE_ASSOCIATION_ONLY
- MODALITY_SPECIFIC_ONLY
- INDETERMINATE

Calibration and preprocessing gaps are returned as explicit evidence gaps. They do not invalidate a valid clock relation, but they do prevent a stronger calibrated sensor-measurement claim. A missing clock relation cannot be repaired by a valid calibration record, and a clock pass does not prove sensor coverage.

## Frozen synthetic benchmark

The Rust suite currently contains 21 tests:
- independently hardware-bounded positive control;
- shared-clock positive control with bounded drift;
- software-cycle-only result remains association-only;
- file-order-only result remains modality-specific;
- file-sequence changes cannot change temporal qualification;
- wrong hardware reference and wrong time scale;
- missing capture window;
- excessive pairwise skew;
- wrong shared clock domain and over-budget drift;
- calibration absence remains visible without falsifying clock qualification;
- processed/raw preprocessing identity consistency;
- malformed capture window;
- mismatched acquisition-cycle ID;
- duplicate sensor ID;
- invalid policy;
- one-sensor input cannot claim cross-modal fusion;
- input permutation invariance.

The fixtures are fictional. The crate does not ingest TSMS-Drone files or recalculate dataset metrics.

## Hard targets

Prospective:
- 100% exact agreement with an independently authored finite temporal oracle;
- zero file-order-only records promoted to a timestamp;
- zero software-cycle-only records promoted to bounded sub-cycle timing;
- zero fusion timing passes without common time scale and the required reference/clock identity;
- zero pairwise skew or clock drift above the frozen limit admitted;
- zero processed records without exact preprocessing identity;
- zero repeated measurement rows treated as independent flight encounters;
- single-modality results preserved when joint temporal evidence is insufficient;
- deterministic output under sensor and file-record permutation;
- independent reproduction of dataset object hashes and split membership.

## SOTA comparison

Compare on identical sensor records and trial/session splits:
- modality-specific CW/FMCW/RF results;
- naive index-aligned early fusion (explicit negative control);
- per-cycle software association;
- late/decision-level fusion;
- calibrated shared-clock fusion;
- independently hardware-bounded fusion where available.

Report paired fusion benefit and harm, timing error, calibration, distance/target strata, missingness and outage behavior. Do not rank by pooled accuracy alone.

## Claim ceiling

This is a temporal evidence and dataset-admission benchmark only. It does not qualify an operational sensor, establish airborne coverage, identify or track aircraft, set DAA performance, prove separation or authorize flight.
