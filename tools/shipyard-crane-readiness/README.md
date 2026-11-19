# Shipyard bridge/gantry crane operational readiness V0.1

This isolated Rust crate implements a synthetic-first, fail-closed **evidence readiness evaluator** for a land-based bridge/gantry crane operation. It composes a lift-preflight receipt with source evidence for crane inspections, supporting-structure inspections, limiting/indicating devices, crane duty, personnel credentials, task briefing and exclusion-zone controls.

It is a child of [SHIPYARD-CRANE-READINESS-001 #6163](https://github.com/Luminous-Dynamics/symtropy/issues/6163) and composes [SHIPYARD-LIFT-ENVELOPE-001 #6077](https://github.com/Luminous-Dynamics/symtropy/issues/6077).

## What it verifies

- Exact lift-preflight receipt identity, digests, disposition, crane/configuration/site/clock binding, operation window, invalidation state and validity.
- Source-owned inspection and duty records for the exact crane/supporting-structure subjects and revisions.
- Required limiter/indicator device state and exact device-verification evidence.
- Required operator, slinger, signaller, supervisor and appointed-person role assignments, each backed by role-scoped credential evidence.
- The exact method-statement revision and individual source-bound acknowledgement records for each assigned required role.
- Site-epoch-bound exclusion-zone evidence and explicit enforcement state.
- Evidence-record identity, owner, document revision, subject/revision, SHA-256 lexical shape, clock domain, validity interval and source status.
- Deterministic report values under input collection permutation.

## Status semantics

- `Current`: record is accepted only if all identity, digest, clock and validity checks pass.
- `Restricted`, `Revoked`, `Faulted`, `Overdue`: source-owned disqualifying state; produces `Rejected`.
- `Unknown`, missing, ambiguous, stale, wrong-subject, wrong-revision or cross-clock evidence: produces `Indeterminate`.

For protective devices:
- `Operational` requires an exact, current limiter-verification evidence record.
- `Faulted` and `Bypassed` reject.
- `Unknown` abstains as indeterminate.

For personnel:
- required role names are supplied by the responsible operating profile; this crate does not infer required roles from a title or crane type;
- credentials use pseudonymous person references, exact role scope and source validity;
- each briefing acknowledgement binds the assigned pseudonymous person, exact role, exact method-statement revision, acknowledgement time and source record;
- no unnecessary personal name/email/identity data is required.

## Four independent evidence domains

1. **Inspection and equipment condition:** source-authorised current inspection/disposition and scope; the crate does not calculate inspection intervals.
2. **Protective devices:** exact device revision, verification evidence, current fault/bypass/operational state. No live hardware connection or command is implemented.
3. **Crane duty:** source-owned duty/load-spectrum status; the crate does not calculate fatigue life or remaining design life.
4. **People and work controls:** required role credentials, briefing acknowledgements and exclusion-zone state; the crate is not a workforce directory or permit system.

The readiness layer consumes the exact #6077 lift-preflight receipt. If the underlying receipt is rejected, indeterminate, invalidated, mismatched or outside its operation window, readiness cannot be positive.

## Standards applicability

The public scope is a declared land-based bridge/gantry crane. Applicability is a source-owner decision, not inferred here.

- ISO 12480-1:2024: general crane safe-system-of-work practices.
- ISO 12480-5:2026: safe use of bridge and gantry cranes.
- ISO 9927-5:2017: inspections for bridge/gantry cranes and supporting structures (confirmed current in 2022).
- ISO 10245-5:2026: bridge/gantry limiting and indicating devices.
- ISO 4301-5:2025: bridge/gantry crane and mechanism classification by duty conditions.
- ISO/DIS 12482: entered enquiry on 5 November 2026; draft intended to replace ISO 12482:2014, not yet a published replacement.

These standards are reference points for applicability. This crate makes **no standards conformity claim**.

## Claim ceiling

The positive disposition is `ReadyForHumanReview`. It means only that supplied readiness evidence is internally consistent under this synthetic contract. It does not establish crane physical condition, that protective devices actually function, competence in fact, fatigue/duty safety, regulatory/class compliance, permit status or permission to lift.

There is no `SafeToLift`, `Approved`, `PermitIssued`, `StartHoist` or actuator command state. Responsible engineers, appointed persons, employers and site authorities retain authority.

## Event replay and invalidation

The replay model accepts source-owned event envelopes with a source ID, monotonic per-source sequence and event ID. It:
- uses the highest event sequence per source independent of input arrival order;
- treats exact duplicate event IDs/payloads as idempotent;
- ignores stale superseded events;
- invalidates only when the latest event changes a dependency pinned in the readiness plan;
- returns indeterminate for source-identity mismatch or conflicting payloads at the same source sequence.

This is deterministic batch replay over supplied envelopes, not a live event transport or source-authentication system. The source owner must guarantee monotonic sequence semantics; event receipts and source signatures remain external. The frozen vector suite includes changed-state, no-op/unrelated, event-order, same-sequence conflict, duplicate and source-spoof controls.

## Machine-readable contracts

- `schemas/readiness-plan-v0.1.schema.json`: canonical input shape.
- `schemas/readiness-report-v0.1.schema.json`: report/disposition shape.
- `schemas/conformance-vectors-v0.1.schema.json` and `fixtures/conformance-vectors-v0.1.json`: 22 frozen synthetic vectors mapped to named Rust tests.

The Rust suite checks that every vector maps to a test; CI checks unique IDs/test names, fixture status and the corresponding schema contract fields. Schema shape validation does not authenticate source records, credentials, inspection reports or physical truth.

## Tests

The 23 unit tests (22 frozen conformance vectors plus one fixture-to-test mapping check) cover a current baseline, missing/revoked inspections, bypassed/unknown devices, stale duty evidence, missing personnel, briefing revision/acknowledgement controls, exclusion-zone enforcement, preflight receipt invalidation and context binding, duplicate evidence IDs and input-order invariance.

Run with pinned Rust 1.96:

```sh
cargo fmt --manifest-path tools/shipyard-crane-readiness/Cargo.toml -- --check
cargo test --locked --offline --manifest-path tools/shipyard-crane-readiness/Cargo.toml
cargo check --locked --offline --manifest-path tools/shipyard-crane-readiness/Cargo.toml --all-targets
cargo clippy --locked --offline --manifest-path tools/shipyard-crane-readiness/Cargo.toml --all-targets -- -D warnings
```

The local authoring environment has no Rust/Cargo toolchain. These tests are authored but have not been compiled or run locally; the exact-head workflow is the next gate.

## Next qualification gates

1. Add a machine-readable input/report schema and frozen conformance vector map.
2. Event-sourced invalidation and event-order convergence now have a bounded first implementation in the Rust crate; expand source contracts and independent model checking next.
3. Add an independent oracle for dependency closure and false-ready negative controls.
4. Bind source-owned ISO/OEM/site applicability profiles without copying licensed standard text or proprietary inspection/duty data into public fixtures.
5. Only after rights and engineering review, replay permissioned historical yard evidence in shadow mode.
