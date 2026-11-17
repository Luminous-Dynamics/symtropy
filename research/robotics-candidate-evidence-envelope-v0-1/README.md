# Robotics Candidate Evidence Envelope v0.1

Tracking: Symtropy #5888  
Parent portfolio: Symtropy #5700  
Status: standalone Rust reference contract; synthetic-only.

## Purpose

A geometry or sensor evaluator should not hand a downstream planner an unbound boolean such as quality=true. The candidate must be pinned to the exact mission, robot configuration, module manifest, site, coordinate frame, map, timebase, sensor, mount, calibration bundle, observation bundle, target set and cost model that were actually evaluated.

This package defines that cross-platform candidate evidence envelope. It validates structural consistency of claims and receipts, not their physical truth or authenticity.

## Composition

- Symthaea #12046 owns generic decision-sufficient evidence-request selection and upstream requirement priorities.
- PV #5810, AUV #5798 and manipulator #5765 own domain-specific geometry, sensor and functional qualification.
- This contract carries their already-issued receipts and exact revision bindings.
- Symtropy #5857 owns downstream physical candidate ordering under declared resource/independence constraints.
- Symtropy #5657 owns mission command/observation/postcondition/acceptance transitions.
- Symtropy #2836 owns controller-boundary dispatch conformance.
- Mycelix #5149 owns provenance, custody, authorization, correction/revocation and disputes.
- Symtropy #5016 owns the claim-to-standard scope crosswalk.

No subsystem gets authority merely because its output is structurally valid.

## Candidate identity

The v0.1 binding pins:
- mission/scenario identity and revision;
- robot identity, configuration revision and configuration digest;
- module manifest digest;
- site revision, frame revision and map revision;
- source timebase identity/revision and an acquisition interval in that clock domain;
- sensor identity/revision, mount identity/revision and calibration bundle digest;
- observation bundle digest and exact target set;
- frame-relative candidate position and view angle;
- acquisition/energy/risk cost model identity, revision, digest and estimates. Travel cost is deliberately excluded from the candidate record because it depends on the previous route state; #5857 must compute transition cost as it builds the route.

The contract does not assume that a source timestamp is UTC. Clock mapping, knowledge-time and temporal supersession remain with the relevant time/evidence owners.

## Three separate receipts

A candidate requires exactly one receipt per domain:

1. **GeometryCoverage** — binds the exact candidate context and must cover precisely the target IDs declared by the candidate.
2. **SensorQuality** — binds the same context and observation bundle.
3. **Calibration** — binds the sensor calibration bundle and must be valid across the full acquisition interval.

Each receipt has distinct identity, issuer/profile revision, status, binding digest, result digest, source-bundle digest, timebase identity and issue time. All receipt times must use the candidate's declared timebase revision. Geometry/sensor receipts must be issued after acquisition completion; calibration is checked by validity interval.

Eligibility is derived conservatively:
- any InvalidEvidence receipt → InvalidEvidence;
- otherwise any NotQualified receipt → NotQualified;
- otherwise any Unknown receipt → Unknown;
- only all Qualified receipts → Qualified.

Malformed/missing/mismatched identities, digests, receipt domains, timebases, time windows, costs or coverage return structural errors rather than an eligible candidate. Successful validation returns a non-forgeable-by-field-construction ValidatedCandidateEnvelope token; its planner projection exists only when all three domains are Qualified.

## Canonical binding

CandidateBinding::canonical_binding_bytes() produces deterministic, length-prefixed preimage bytes. Target IDs are sorted before encoding; floating-point negative zero is normalized. The caller must compute SHA-256 over these exact bytes, place that commitment on the envelope and receipts, and pass the independently computed digest into validate_candidate_envelope. The validator rejects a computed digest that does not match the envelope commitment.

**Cryptographic boundary:** this crate does not implement SHA-256, authenticate issuer identity, validate signatures, fetch source records or verify custody. It requires the caller to supply the computed binding digest, but cannot independently prove that the caller used a trusted cryptographic implementation. Receipt/result/source digests are checked for syntax and binding consistency, not checked against fetched content or signatures. Those responsibilities remain with the independent digest/provenance/authentication infrastructure (including Mycelix).

## Implementation status

The reference crate includes 26 deterministic tests for eligibility precedence, externally computed binding digest matching, exact receipt/source binding, timebase binding, target coverage, calibration validity, duplicate receipts, numeric bounds and canonical target-order invariance. A private-field validation token exposes a QualifiedCandidateProjection only for structurally consistent, fully Qualified candidates.

## Claim boundary

This crate is not a geometry/sensor evaluator, an independent referee, a general value-of-information planner, evidence custodian, temporal event store, mission-state authority, motion/trajectory planner, collision checker, safety controller or controller dispatcher. A Qualified result means only that three structurally consistent upstream receipts report Qualified for one exact declared candidate context. It does not establish physical truth, authenticity, permission to move or task acceptance.

## Future integration

1. Add exact source adapters from PV/AUV/manipulator evaluator reports.
2. Add planner adapter that only converts envelopes with CandidateEligibility::Qualified into #5857 candidate inputs.
3. Retain reason codes for NotQualified / Unknown / InvalidEvidence in planning diagnostics.
4. Route execution through existing #2836 dispatch gate and #5657 mission-stage evidence contract.
5. Benchmark stale-revision rejection, exact replay, target-coverage equality, calibration validity and adapter latency across frozen cross-platform fixtures.

The machine-readable Draft 2020-12 schema is in schema/candidate-evidence-envelope-v0.1.schema.json. fixtures/qualified-candidate-v0.1.json is a structurally valid synthetic example; its digest strings are placeholders, not cryptographic proof.

## Standards / evaluation context

- NIST's Performance Assessment Framework for Robotic Systems decomposes robot performance into perception, mobility, dexterity and safety test methods; it is a measurement-science reference, not a certificate: https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems
- ISO 18646-5:2026 supplies legged-robot locomotion performance test methods but explicitly is not safety verification/validation: https://www.iso.org/standard/86850.html
- ISO 21423:2026 covers communications/interoperability for industrial mobile robots in restricted-access industrial environments; it explicitly excludes navigation and safety requirements: https://www.iso.org/standard/86749.html
- LADDER reports a 145-task, 1–10-subgoal benchmark and a large gap between short-horizon skills and extended tasks; it motivates our stage/progress and recovery evaluation, not direct construction-task score comparison: https://ladderrobotics.ai/research

## Run

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo run

No third-party dependencies are required.
