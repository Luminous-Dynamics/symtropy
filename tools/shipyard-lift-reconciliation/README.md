# Shipyard lift execution reconciliation V0.1

This dependency-light Rust crate reconciles a frozen lift plan and preflight receipt with source-bound execution telemetry. It is a child of [SHIPYARD-LIFT-RECONCILIATION-001 #6116](https://github.com/Luminous-Dynamics/symtropy/issues/6116), and consumes the preflight contract in [#6077 / PR #6107](https://github.com/Luminous-Dynamics/symtropy/pull/6107).

It is an evidence evaluator only. It is **not** a crane controller, safety PLC, load limiter, emergency stop, operator interface, lift supervisor, or permission to continue/stop a lift. It emits no actuator/control state.

## Paradigm shift

Move from a lift-plan document plus separate crane logs to a continuous, immutable evidence thread:

authorised plan -> exact preflight receipt -> external lift authorisation -> source telemetry -> motion/occupancy evidence -> deterministic reconciliation -> post-lift record.

Keep distinct:
- preflight arithmetic consistency and actual owner/operator authorisation;
- machine controller telemetry and independently observed physical truth;
- source event time and ingestion time;
- "no alarm event received" and an independently attested complete alarm-event stream;
- a measured value and its uncertainty/conservative upper bound;
- a complete session and a session with missing/unobserved intervals.

The evaluator never treats missing events as normal operation.

## V0.1 input contract

### Execution binding

The session binds:
- plan ID/revision and canonical plan-input digest;
- preflight-bound input digest, preflight result digest and its disposition;
- externally issued authorisation ID/revision/evidence covering the lift interval;
- exact block/crane/configuration, site epoch, frame and clock domain;
- half-open execution interval;
- source-owned hook-load and gust limits, each with a distinct evidence reference;
- exact motion and occupancy report IDs/digests;
- caller-computed SHA-256 digests for the canonical event ledger and channel-coverage bundle.

The preflight disposition must be ReadyForHumanReview, but this is not authorisation. A separate source-owner authorisation record is mandatory. Evidence signatures, authority trust and physical truth are verified by their respective owner systems, not by this crate.

### Telemetry events

V0.1 channels:
- hook-load upper bound in milli-newtons;
- gust upper bound in millimetres per second;
- crane ID and configuration revision;
- independent motion-envelope report ID/digest and within-envelope assertion;
- independent occupancy projection ID/digest and coverage assertion;
- overload-alarm active/inactive state;
- hoist state.

Each event binds a source ID/revision, sequence, source clock domain, exact source-to-yard mapping ID/revision, closed mapped event-time uncertainty interval, target-clock receipt tick, calibration/source evidence and clock-mapping evidence. A point timestamp is not inferred from an interval.

### Coverage statements

Every required channel has exactly one source-owned coverage statement with:
- exact source ID/revision;
- first sequence and exclusive next sequence;
- target clock-domain ID;
- covered start/end ticks;
- a complete/incomplete assertion;
- source evidence valid over the complete execution interval.

For HookLoad and WindGust, the maximum sample gap is checked at the leading, internal and trailing intervals. For event-only streams, the source coverage assertion and contiguous sequence window are required; an empty overload-alarm stream is admissible only when its source coverage is complete.

The coverage assertion is evidence supplied by a source owner. This crate checks its declared structure and consistency with admitted events; it does not authenticate the source or prove there were no undisclosed events.

## Deterministic dispositions

- `ConformingToDeclaredEnvelope`: required event streams are complete under the declared coverage policy and no admitted measurement or independent envelope assertion exceeds the supplied limits.
- `DeviationObserved`: at least one explicit limit or independent envelope assertion reports a deviation. This disposition takes precedence over Indeterminate so that an observed deviation is not hidden by an unrelated coverage gap; all unknown intervals and diagnostics remain in the report.
- `Indeterminate`: required evidence, telemetry, sequence continuity, source identity, clock binding, calibration validity, sample coverage or authority binding is missing or inconsistent.

A complete execution with no alarms is not established by the absence of alarm records. It requires a complete source coverage statement for the alarm channel.

## Canonical receipts

- `canonical_event_ledger_json()` sorts events by channel/source/sequence/ID, preserving retransmissions.
- `canonical_coverage_bundle_json()` sorts channel coverage attestations.
- `ReconciliationPolicy::canonical_json()` pins required channels and timing limits.
- `ReconciliationReport::canonical_json()` emits deterministic result bytes and echoes the plan, preflight, authorisation, policy, event-ledger and coverage-bundle digests.

The caller computes SHA-256 over the canonical policy payload, event ledger and coverage bundle, supplies those digests in the execution binding, and hashes the canonical report bytes after reconciliation. The mapped-time interval bounds are retained in canonical event bytes, so a changed clock map or uncertainty interval changes the evidence digest. The crate checks SHA-256 lexical shape only; it does not perform signature verification or cryptographic source authentication.

## Fail-closed cases covered

The synthetic suite covers:
- plan/preflight digest mismatch and preflight rejection;
- separate, expired and wrong-subject authorisation evidence;
- distinct hook-load and gust limit provenance;
- missing/incomplete channel coverage, sequence gaps and duplicate coverage statements;
- exact event retransmission vs conflicting event identity/sequence;
- wrong source stream trying to fill another stream's coverage;
- stale/revoked/mismatched sensor calibration and clock-domain mismatch;
- source/target clock-map mismatch, mapping-revision mismatch, overlapping mapped-time uncertainty, out-of-window intervals and conservative ingestion lateness;
- leading/inter-sample/trailing sample gaps and exact gap-boundary behavior;
- load/gust equality vs one-unit-over deviations;
- crane configuration, motion and occupancy deviations;
- active overload alarm and incomplete/aborted sessions;
- permitted transport reordering with stable canonical event, coverage and result bytes.

## Machine-readable contracts and conformance

The public JSON contracts live in `schemas/`:
- `telemetry-event-v0.1.schema.json` — event IDs, source revisions, mapped-time uncertainty, clock mapping, calibration evidence and tagged observation channel;
- `stream-coverage-v0.1.schema.json` — channel/source identity, sequence interval, target clock domain and coverage attestation;
- `reconciliation-policy-v0.1.schema.json` — required channels and timing policies;
- `reconciliation-report-v0.1.schema.json` — final disposition, full execution/authority/configuration context, raw input digests, unknown intervals and diagnostics;
- `conformance-vectors-v0.1.schema.json` — frozen vector envelope.

The 42 synthetic vectors are in `fixtures/conformance-vectors-v0.1.json`; each maps to a named Rust test. A Rust test verifies all vector IDs/test names are present, and the exact-head workflow checks unique IDs, unique test names, schema anchors and that each mapped test exists in source. Schema shape validation does not authenticate evidence or establish that sensors are physically true.

## Build and test

The crate depends only on the local `shipyard-lift-envelope` package; it adds no third-party Rust dependencies.

```sh
cargo fmt --manifest-path tools/shipyard-lift-reconciliation/Cargo.toml -- --check
cargo test --locked --offline --manifest-path tools/shipyard-lift-reconciliation/Cargo.toml
cargo check --locked --offline --manifest-path tools/shipyard-lift-reconciliation/Cargo.toml --all-targets
cargo clippy --locked --offline --manifest-path tools/shipyard-lift-reconciliation/Cargo.toml --all-targets -- -D warnings
cargo run --locked --offline --manifest-path tools/shipyard-lift-reconciliation/Cargo.toml --example synthetic_reconciliation -- --emit-inputs
```

The current unit suite contains 42 deterministic tests (plus a fixture-to-test mapping test). The current authoring environment has no Rust/Cargo toolchain; formatting, compilation, tests and Clippy are not claimed as passing. The dedicated exact-head workflow is the execution gate.

## Industry and standards comparison

Commercial systems already provide crane telemetry, alarms, overload/event histories, load spectrum and maintenance data. ABRA Crane Tracking describes machine-controller data such as running hours, cycles, load events, overloads, brake operations and faults, while explicitly distinguishing machine data from physical crane/load position. That is why this crate consumes both source telemetry and separately qualified motion/occupancy evidence:
https://www.abracrane.com/products/crane-tracking

3D Lift Plan already provides lift visualisation, crane selection, capacity monitoring, ground-bearing calculations and rigging planning/calculations. This crate does not replace those capabilities; it is a vendor-neutral evidence reconciliation and provenance layer:
https://www.3dliftplan.com/info/Features.aspx

For bridge/gantry crane safe use, ISO 12480-5:2026 (published June 2026) is the specific Part 5 and is intended to be used with ISO 12480-1:2024. The applicable crane-type/jurisdiction profile and site procedure remain owner decisions. This crate makes no standards-conformance claim:
https://www.iso.org/standard/90196.html
https://www.iso.org/standard/83637.html

## Claim ceiling

A conforming result means only that admitted source events are complete enough under the declared coverage contract and do not report an excursion against the supplied limits and independent envelope assertions. It does not prove sensor calibration authenticity, unobserved physical state, legal authorisation, lift safety or whether an operator should continue/stop. Only responsible people and authorities retain those decisions.

No live operational deployment, control-plane integration or productivity/safety claim is justified by synthetic tests.
