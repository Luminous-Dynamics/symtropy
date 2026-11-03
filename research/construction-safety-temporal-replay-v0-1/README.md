# Construction Safety Temporal Replay V1

A dependency-free Rust reference reducer for event-time safety evidence in
construction robotics. This package is the temporal follow-on to
[CONSTRUCTION-ROBOT-SAFETY-001 / #3317](https://github.com/Luminous-Dynamics/symtropy/issues/3317)
and is tracked in
[CONSTRUCTION-ROBOT-SAFETY-002 / #3499](https://github.com/Luminous-Dynamics/symtropy/issues/3499).

It adds no dependency to the production workspace and no network transport,
event store, scenario runtime, dispatcher, safety PLC, actuator or emergency
stop interface.

## Research question

Can safety evidence be evaluated reproducibly at a declared event-time cutoff
when sensors deliver stale, delayed, reordered, duplicated, conflicting or
replayed observations?

A one-frame snapshot is not enough. The reducer binds each event to:

- scenario/run identity;
- exact policy and coverage-grid revisions;
- sensor identity, modality and declared fault domain;
- source epoch and monotonic source sequence;
- source timestamp, receiver timestamp and clock uncertainty;
- calibration epoch;
- hazard-zone cell identity;
- observed state, confidence/coverage and source digest reference.

## Paradigm shift

Move from:

`latest received snapshot -> classify`

to:

`revision-bound source events -> deterministic event-time reduction -> per-cell temporal evidence -> bounded disposition`

Receive order is not source-time truth. An older clear event that arrives late
cannot overwrite a newer source frame. A duplicate event ID is idempotent only
when its payload matches. A conflicting payload under the same identity is
rejected. Missing, unknown, stale, contradictory or low-confidence evidence is
not converted into a clear zone.

The key output is no longer just “person present/absent”. It is a statement
about which required hazard-zone cells have sufficiently fresh clear evidence,
which sensors contributed, which modalities and fault domains cover each cell,
what the source times were, and which event receipts support the decision.

## Temporal contract

### Two clocks and one cutoff

Every event carries:
- `source_time_ms`: when the sensor/source says it observed the state;
- `received_at_ms`: when the event reached the evaluation boundary;
- `clock_uncertainty_ms`: the declared uncertainty used to check whether a
  source timestamp is implausibly ahead of receipt.

Each policy fixes `evaluated_at_ms`. Events received after that cutoff are
unavailable to that decision and are excluded. A later evaluation is a new
decision with a new cutoff; it cannot mutate the earlier result.

Source age is computed from source time, not receive time. Recently received
stale measurements remain stale.

### Optional continuous evidence window

A point-in-time decision can set `required_evidence_window_ms = 0`. A stronger
qualification profile sets a positive evidence window and a maximum
inter-sample gap. For every required hazard-zone cell and each contributing
sensor, the reducer then requires:

- a first observation no later than the declared maximum gap after the window
  starts;
- subsequent source-time gaps no larger than the configured maximum;
- a latest observation still within the normal freshness bound at the cutoff;
- clear, sufficiently confident and sufficiently covered observations
  throughout the window.

A human-present observation anywhere inside the window requires a stop. Unknown,
conflicting, stale or low-confidence samples cannot be erased by a later clear
sample. The decision receipt reports the window start, maximum observed gap
(including leading/trailing gaps) and the number of contributing temporal
samples for each cell.

This is a bounded event-cadence contract, not proof that a sensor physically
observed every instant between events. Actual exposure, occlusion, update
cadence, clock error and detection performance require independent
instrumentation and physical validation.

### Epochs, sequences and replay

For each source:
- the source epoch must match the policy-pinned source profile;
- source sequence must be positive;
- source time must not regress as sequence increases;
- all cell records from one sequence/frame must share one source timestamp;
- same event ID + same payload is idempotent, with earliest receive time
  retained for deterministic replay;
- same event ID + different payload is an identity conflict;
- a conflicting second record for the same source/epoch/sequence/cell is
  rejected rather than silently chosen.

A source reset is not inferred from a counter rollback. V1 pins one source epoch
and calibration epoch in the policy. A changed source epoch requires a new
explicit policy/profile revision and fresh evidence admission in a later run.

### Per-cell reduction

For each exact policy-required hazard-zone cell, V1:

1. Selects the highest admissible source sequence for each sensor/cell.
2. Does not fall back to an older clear event when the latest state is unknown,
   stale, low-confidence or missing.
3. Stops on a sufficiently well-formed `HumanPresent` event.
4. Rejects contradictory or malformed evidence.
5. Requires the minimum modality and independent fault-domain diversity on
   every cell.
6. Requires the minimum number of modalities and fault domains to cover the
   entire required zone.
7. Checks inter-sensor source-time skew at each cell.
8. Optionally requires a complete source-time evidence window per cell, with a
   bounded leading gap, bounded inter-sample gaps, a fresh final sample and no
   unknown/low-confidence/human-present sample within the window.
9. Emits cell-level source IDs, modality/fault-domain coverage, source-time
   bounds, maximum age, maximum observation gap and temporal sample count.

A clear claim for one part of a work zone cannot be averaged into coverage of a
blind cell elsewhere.

## Frozen trace corpus

V1 defines **69 named synthetic traces** and **17 authored Rust unit tests**.
The trace corpus includes:

- synchronized clear baseline and arrival-order permutations;
- identical duplicate event retransmission;
- human-present and unknown observations across sensor/cell combinations;
- missing, stale and low-confidence sensor/cell evidence;
- grid-revision, source-epoch and calibration-epoch mismatch;
- malformed source digest and confidence overflow;
- clock uncertainty and source timestamp/receive-window violations;
- future source observation at the decision cutoff;
- run, source, sequence and hazard-zone binding failures;
- source-time regression and conflicting timestamps for one acquisition frame;
- a newer source event followed by a delayed older event;
- a late clear event that cannot rewrite a prior cutoff;
- a new cutoff that can consume a later, complete clear frame;
- per-cell inter-sensor skew;
- full clear evidence windows, leading blind intervals, excessive inter-sample
  gaps, missing middle frames and stale final samples;
- human-present, unknown and low-confidence observations that occur inside a
  window and are followed by later clear snapshots;
- exact maximum-gap boundary behaviour;
- single-modality and common-mode-only profiles;
- empty event batch and duplicate policy sensor identities.

Expected dispositions are pinned in the fixtures and are not generated by the
reducer under test. Hazardous/unknown/stale outcomes are paired with benign
controls where the bounded scenario permits it. This is a public synthetic
corpus, not a secret or independently held blind set.

Run locally, independently of hosted CI:

```sh
cargo fmt --manifest-path research/construction-safety-temporal-replay-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-safety-temporal-replay-v0-1/Cargo.toml --locked
cargo run --manifest-path research/construction-safety-temporal-replay-v0-1/Cargo.toml --locked --bin temporal-safety-benchmark
cargo clippy --manifest-path research/construction-safety-temporal-replay-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

The dedicated workflow pins Rust 1.96.0 and runs formatting, tests, corpus CLI
and strict Clippy on Linux, macOS and Windows.

## Benchmark plan

Baselines are defined for future matched evaluation but are not implemented or
measured by this first reducer:

| Baseline | Reduction policy |
|---|---|
| T0 | Latest-arrival snapshot (intentionally naive comparator). |
| T1 | Source-timestamp-only ordering, without explicit epoch/sequence semantics. |
| T2 | Source-epoch + monotonic-sequence + event-time point reduction. |
| T3 | T2 plus per-cell continuous evidence windows and bounded inter-sample gaps (this V1). |
| T4 | Separately implemented independent trace oracle / mutation evaluator. |

Report separately:

- false-clear / false-within-envelope outcomes (prospective target: zero on
  critical planted controls);
- per-cell temporal coverage recall and maximum unobserved-cell interval;
- stale, missing, duplicated, conflicting, replayed, out-of-order and misbound
  event detection;
- source-time vs receive-time lag distribution (p50/p95/p99);
- time from source hazard event and from receipt to stop disposition;
- source-order monotonicity and duplicate idempotence;
- arrival-order permutation invariance at the same cutoff;
- per-cell modality/fault-domain completeness and maximum inter-sensor skew;
- maximum per-cell observation gap and temporal sample depth;
- false stop/hold rate on benign paired controls;
- results stratified by sensor modality, fault domain, source reset, zone cell
  and ODD.

No aggregate score may hide a false-clear. A lower nuisance-stop rate or faster
task time cannot compensate for a critical safety escape. The exact reference
corpus is a software-contract benchmark, not a physical detection-performance
benchmark.

## Beyond current practice

The next research steps are:

1. **Causal event campaigns:** compose with the existing scenario fault
   schedule rather than inventing another scenario engine.
2. **Temporal cell coverage:** the positive evidence-window profile now rejects
   excessive leading/inter-sample/trailing gaps and reports the maximum observed
   gap and sample depth per cell. Next add explicit variable-cadence models,
   source dropout-duration campaigns and window-level cross-modal disagreement.
3. **Cross-modal conflict:** preserve the raw disagreement and reason codes;
   do not average contradictory sensors into a reassuring confidence value.
4. **Independent evaluation:** implement T3 in a structurally different reducer
   and compare normalized receipts, including malformed-input counterexamples.
5. **Mission timeline integration:** pass immutable per-cutoff decisions to
   ROBOT-QUAL-ORACLE-001 rather than declaring a full mission complete here.
6. **Physical qualification:** compare these synthetic assumptions against
   instrumented sensor/source data, known clock error, measured coverage,
   occlusion, calibration and validated response times before making any field
   claims.

## Industry and standards context

This reducer is not a standards implementation or conformity assessment.
Machine family, jurisdiction, task and installed safeguards determine the
applicable framework.

- [ISO 19014:2026](https://www.iso.org/ics/53.100/x/) addresses functional
  safety for in-scope earth-moving machinery. Its five parts address required
  machine performance levels (MPLr), achieved performance (MPLa), environmental
  testing, safety-related software/data transmission and required-level tables.
- [ISO 17757:2019](https://www.iso.org/standard/76126.html) addresses
  autonomous/semi-autonomous earth-moving and mobile-mining machine systems.
- [ISO 10218-1:2025](https://www.iso.org/standard/73933.html) and
  [ISO 10218-2:2025](https://www.iso.org/standard/73934.html) concern industrial
  robots and industrial robot applications/cells.
- [ISO 3691-4:2023](https://www.iso.org/standard/83545.html) is the published
  driverless industrial-truck standard; its third edition remains a draft at
  stage 40.60 as of 3 November 2026.
- [NIST Mobility Performance of Robotic Systems](https://www.nist.gov/programs-projects/mobility-performance-robotic-systems)
  develops measurement methods and artifacts for mobile-robot performance.

MPLr/MPLa are not interchangeable with ISO 13849 PLr/PL or IEC 61508 SIL. This
package computes none of these, does not interpret standard clauses, and
cannot establish compliance or physical safety.

## Ownership / composition boundaries

- [#3317 — snapshot safety-envelope sub-oracle](https://github.com/Luminous-Dynamics/symtropy/issues/3317)
  owns machine/site/policy bindings and per-observation spatial hazard-zone
  evaluation.
- [#1595 — shared robotics scenario pack](https://github.com/Luminous-Dynamics/symtropy/issues/1595)
  owns scenario identity and deterministic fault schedules.
- [#1172 — ENG-SYNTH](https://github.com/Luminous-Dynamics/symtropy/issues/1172)
  owns deterministic engineering execution and replay identity.
- [#2672 — ROBOT-QUAL-ORACLE-001](https://github.com/Luminous-Dynamics/symtropy/issues/2672)
  owns mission-level fault-to-disposition qualification.
- [#3499 — CONSTRUCTION-ROBOT-SAFETY-002](https://github.com/Luminous-Dynamics/symtropy/issues/3499)
  tracks temporal evidence replay and space-time hazard coverage.
- Symthaea #9870 owns platform design, capability qualification and field
  evidence. Mycelix retains identity/authority/provenance where integrated;
  Sol-Atlas supplies spatial/site projections, not safety authority.

## Claim ceiling and limitations

All V1 records and values are synthetic. The reducer validates event identity,
time/sequence relationships, per-cell evidence completeness and deterministic
disposition for the declared bounded traces. It does not:

- authenticate or hash source bytes, verify signatures or trust sensor issuers;
- establish that a source timestamp or sequence is truthful;
- prove that the site grid matches physical sensor fields of view;
- ray-cast occlusion, validate sensor calibration or prove actual detection;
- derive physically valid stopping distances or risk thresholds;
- implement a safety controller, safety PLC, protective stop or emergency-stop
  reset;
- calculate MPLr/MPLa, PLr/PL or SIL, assess a machine's conformity, authorize
  work or declare a physical mission safe.

SHA-256 references are syntax-checked identifiers, not authentication. Physical
deployment requires machine-specific risk assessment, qualified safety
systems, competent engineering, independent instrumentation, operator
procedures and applicable regulatory approval.
