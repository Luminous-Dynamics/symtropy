# Maritime AIS Observability

A dependency-free Rust reference state observer for already-qualified AIS zone labels. It tests the semantics needed to turn position messages into bounded service-state inferences without pretending that an AIS point is a port-call event.

This is the observation layer for [MARITIME-AIS-OBSERVABILITY-001 #6562](https://github.com/Luminous-Dynamics/symtropy/issues/6562), composed with the queue/event measurement stack:
- [conserved backlog ledger #6493 / PR #6496](https://github.com/Luminous-Dynamics/symtropy/pull/6496);
- [censored wait cohort #6525 / PR #6526](https://github.com/Luminous-Dynamics/symtropy/pull/6526);
- [censoring robustness #6557 / PR #6560](https://github.com/Luminous-Dynamics/symtropy/pull/6560);
- [DGT source qualification #4003](https://github.com/Luminous-Dynamics/symtropy/issues/4003).

## Run

From this directory:

```bash
cargo fmt --all --check
cargo test --offline --locked
cargo run --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
```

The crate has no third-party dependencies. Its tests are synthetic; it does not ingest AIS messages from a live receiver or public data portal.

## What this module does

- Accepts an upstream-qualified zone label (OUTSIDE, ANCHORAGE, APPROACH, BERTH); it does not implement geospatial polygons or CRS transformations.
- Requires stable MMSI, source, receiver, geofence digest and spatial-reference identity for one trajectory.
- Preserves sender event tick separately from receiver `received_tick`; it never assumes the two clock domains are synchronized.
- Breaks dwell if either the source-event gap or receiver-observation gap exceeds its own configured threshold. Inferred events carry both confirmation clocks and gap evidence.
- Makes message identity explicit: exact message replay is idempotent; conflicting reuse rejects.
- Enforces monotonic message time.
- Requires dwell confirmation before a zone transition is emitted.
- Breaks candidate dwell across any observation gap larger than the declared maximum.
- Emits an observability-gap event and changes the state to INDETERMINATE when continuity is lost.
- Invalid position or implausible speed inside a stable zone invalidates the state claim.
- Accepts raw AIS navigation-status lexemes as input observations; a conflict counter is diagnostic only and never overrides zone/motion evidence. The upstream feed owner must retain the original raw messages; this reference verifier is not a raw-message archive.
- Requires an evidence reference when a caller supplies a non-INDETERMINATE initial state. Without one, a first observed berth dwell is reported as zone reacquisition—not a fabricated berth arrival.
- Carries the entire dwell profile, profile version, evidence-referenced initial state, source-local call reference, receiver, source, geofence digest and spatial reference in the call report.
- Scores inferred transition intervals against independent ground-truth event labels using maximum-cardinality, minimum-total-interval-distance one-to-one matching by call reference and event kind.

## Event semantics and claim ceiling

The output events are explicitly AIS-inferred geofence events:
- AIS-inferred anchorage-zone entry;
- AIS-inferred approach-zone entry;
- AIS-inferred berth-zone entry;
- AIS-inferred berth-zone exit / departure;
- AIS-inferred port-geofence exit;
- zone reacquisition after uncertainty;
- observability gap;
- invalid position or zone-speed conflict.

They are **not** DCSA actual port-call timestamps, pilot boarding, first/last line tied, lines-fast/lines-untied, cargo start/end, berth booking or navigation instructions. A berth polygon does not prove that the ship is physically all-fast.

## Synthetic profile

The frozen default profile uses integer ticks:
- maximum sender-event gap: 2 ticks;
- maximum receiver-observation gap: 2 ticks;
- anchorage dwell: 2 ticks, speed at most 1.5 knots;
- approach dwell: 1 tick, speed at most 12 knots;
- berth dwell: 3 ticks, speed at most 0.5 knots;
- berth-exit/departure dwell: 2 ticks;
- outside dwell: 2 ticks.

These are synthetic fixture parameters, not universal thresholds or a claimed reproduction of the 2026 edge-AIS field study. The dwell-sensitivity test uses the same observations while varying berth dwell from one to four ticks.

## Event-detection scorecard

The module provides `predictions_from_report` and `score_events` for independent, source-local ground-truth labels. It reports:
- true positives, false positives and false negatives by event type and overall;
- precision and recall;
- mean distance from the true event tick to the predicted interval;
- mean width of matched event intervals.

A prediction matches only one truth label, and a truth label matches only one prediction. The solver maximizes match count first, then minimizes total interval distance; it is not a greedy nearest-pair shortcut. Exact duplicates therefore count as additional false positives rather than additional detections. Matching is restricted to the same source-local call reference and event type. Predictions outside the declared interval-distance tolerance are false positives; unmatched truths are false negatives. Reacquisition, observability gaps, invalid positions and kinematic conflicts are not scored as berth or port-call events.

The ground-truth call reference must be independently adjudicated; MMSI alone is not a visit identity. The tolerance and clock domain are part of the benchmark protocol and are not automatically converted to hours.

## Claim ceiling

Passing the suite demonstrates only the declared synthetic state-machine, replay, time-ordering, identity, dwell and gap semantics. It does not establish real receiver coverage, DGT AIS access rights, South African event-detection accuracy, CPPI equivalence or actual vessel arrival/departure. Real validation requires raw AIS rights, receiver logs, surveyed zone geometry, source/publication timestamps and independently adjudicated call/berth references.
