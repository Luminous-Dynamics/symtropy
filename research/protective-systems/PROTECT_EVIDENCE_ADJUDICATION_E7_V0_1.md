# Protective Evidence E7 — Conflict sets, independence and bitemporal replay

**Snapshot:** 17 November 2026  
**Owner:** PROTECT-EVIDENCE-ADJUDICATION-001 #5741  
**Branch:** research/protect-evidence-adjudication-e7-v0-1  
**Parent:** E6 schema lifecycle / PR #5737, with E1–E4 evidence semantics retained  
**Status:** synthetic corpus, Draft 2020-12 schema, Rust adjudication oracle, CI integration and 34 tests written; exact-head hosted execution pending

## 1. Paradigm shift: evidence is adjudicated, not averaged

Evidence count is not evidence independence. Five reports may be copies of one sensor observation. Two vendors may consume the same upstream feed. Several nominally separate sensors may share a calibration chain, model or operator. A source with unknown dependence is not evidence of independence.

Likewise, contradictory readings are not made more trustworthy by taking their mean. Before comparing two claims, the system must establish that they concern the same subject, configuration, quantity, unit, method and overlapping valid-time interval. If any of those are unknown or incompatible, the observations remain incomparable; the engine does not convert units, normalize scope or guess.

E7 separates:
- evidence artifact identity from the assertion made using it;
- source observation time from the time the system learned about it;
- valid/effective time from recorded/transaction time;
- data duplication from independent measurement;
- a known dependency from unknown dependence;
- detecting conflict from resolving conflict;
- source identity or signature validity from scientific measurement validity;
- historical as-of replay from the current view.

The output is a structured UNRESOLVED, ABSTAIN, or NO_CONFLICT_DETECTED_NOT_QUALIFIED result. E7 has no scalar evidence-confidence score and emits no aggregate numeric value.

## 2. Typed evidence and assertion model

The frozen synthetic corpus contains:
- 12 EvidenceArtifact records with source identity, SHA-256, lifecycle state, recorded time and rights/use ceiling;
- 12 assertions with subject, exact configuration digest, quantity, value/unit, method, valid-time interval, observed time and recorded time;
- six typed dependency edges;
- one claim- and time-bound independence attestation;
- one back-dated revocation event recorded later than its effective time.

### 2.1 Claim comparability

An assertion can enter a claim's comparable set only when:
1. it refers to the same claim and subject;
2. the configuration digest matches exactly;
3. quantity, unit and named measurement method match;
4. its valid-time interval contains the requested valid_at;
5. the assertion and evidence artifact were known by the decision_time;
6. the evidence state is VALID, with no applicable lifecycle event known by that decision time.

E7 deliberately does not perform implicit unit conversion, CRS transformation, method equivalence or configuration substitution. A future explicit, versioned and reversible conversion rule may be introduced as a separate adapter contract; it is not inferred here.

### 2.2 Dependence and independence

Dependency relations include:
- DERIVED_FROM;
- COPIED_FROM;
- SAME_UPSTREAM_OBSERVATION;
- SHARED_SENSOR;
- SHARED_CALIBRATION;
- COMMON_OPERATOR;
- COMMON_MODEL;
- UNKNOWN_DEPENDENCE.

For independence checking, every dependency edge is treated conservatively as an undirected connection. An explicit VERIFIED_INDEPENDENT attestation is accepted only when:
- it is known by the decision time;
- its claim, configuration, quantity, unit, method and valid-time scope match;
- all referenced evidence is admissible at that decision time;
- no dependency path connects any attested pair.

An UNKNOWN_DEPENDENCE edge blocks independence exactly like a known common cause. Absence of a dependency edge does not itself prove independence; the explicit scope-bound attestation is still required. A copy, shared sensor feed or shared calibration therefore cannot increase the independent-source count.

## 3. Conflict detection and resolution boundary

Within one compatible claim scope, E7 compares the source observations pairwise against the claim's declared absolute tolerance. It preserves each assertion and emits a VALUE_DISAGREEMENT ConflictSet when any compatible pair exceeds the tolerance.

The engine does not:
- average incompatible readings;
- use majority vote;
- weight sources by reputation;
- treat a signature as scientific corroboration;
- resolve a conflict merely because multiple dependent copies agree.

When conflict remains, E7 emits an INDEPENDENT_REPEAT_MEASUREMENT action with explicit acceptance criteria: same asset/configuration, same quantity/unit/method, overlapping valid time, retained original artefacts, and demonstrated independence from the conflicting source set. This is a resolution request, not a qualification or procurement authorization.

## 4. Bitemporal replay

E7 distinguishes two clocks:
- **valid/effective time:** when the assertion or lifecycle change applies to the asset or world;
- **recorded time:** when the evidence system learned about it.

The evaluation is a pure function of the frozen corpus, rule version, decision_time and valid_at.

The fixture demonstrates both directions of the distinction:
- A report observed earlier but recorded at 14:30 is absent from a decision replay at 12:00. It appears in a later replay, but cannot rewrite the historical 12:00 result.
- A revocation effective at 10:00 but recorded at 16:00 does not alter what was knowable at 12:00. It invalidates the affected evidence in a later as-of view.

Lifecycle events are append-only. The report preserves event ID, reason code, effective time and recorded time separately. A historical result can therefore be reconstructed without silently applying future knowledge to a past decision.

## 5. Rust gate and report

Binary: protect-evidence-adjudication-e7

The executable:
- validates the frozen corpus against a pinned Draft 2020-12 JSON Schema;
- rejects duplicate artifact, assertion, attestation, lifecycle-event and dependency identities;
- checks evidence references and supported dependency relation types;
- rejects noncanonical timestamp forms and observations recorded before they were observed;
- computes conservative dependency reachability;
- admits only explicitly attested, scope-bound independent sources with no dependency path;
- creates conflict sets only from compatible, admissible same-scope assertions;
- reports incomparable configuration, unit, method and valid-time findings rather than coercing them;
- evaluates lifecycle events as-of the decision-time cutoff;
- retains source identities/digests/rights ceilings, independence-attestation basis/attester and lifecycle event receipts;
- reports an explicit resolution action without emitting a numeric aggregate or safety pass.

The initial synthetic replay is expected to return UNRESOLVED_CONFLICT. It identifies the sensor-A and independent-lab-E pair as the only explicitly attested independent sources; copied/vendor/shared-calibration/unknown-dependence sources do not increase the independent-source count. The late report is excluded at the 12:00 decision cutoff, and the back-dated revocation is not applied until its 16:00 recording time is within the decision horizon.

## 6. Benchmark protocol and targets

The benchmark corpus must be split into known-truth dependency graphs, compatible/incompatible scope cases, value disagreements, and bitemporal event sequences. The scorer should report per-class and worst-case outcomes, not only average performance.

| Metric | Prospective target |
|---|---:|
| Copied/derived/shared-source evidence counted as independently corroborating | 0 |
| Unknown dependence treated as proven independence | 0 |
| Independence accepted without explicit scope-bound attestation | 0 |
| Dependency path contradicting an independence attestation accepted | 0 |
| Same-scope conflict silently averaged or majority-resolved | 0 |
| Configuration/quantity/unit/method/time mismatch silently compared | 0 |
| Late evidence altering a historical as-of result | 0 |
| Back-dated revocation applied before it was recorded | 0 |
| Revoked/expired/unknown evidence silently admitted | 0 |
| Unresolved conflicts emitted with assertion/evidence IDs and reason code | 100% |
| Adjudication replay deterministic on frozen corpus | 100% |
| Resolution actions with explicit scope and acceptance criteria | 100% |
| Human investigation time and resolution latency | p50/p95 measured |

For dependency clustering, report precision, recall and false-independent rate against the synthetic known-truth graph. For conflict detection, report pairwise precision/recall and false-clear rate. For bitemporal replay, compare the complete report bytes for identical inputs and compare historical snapshots before/after late events. Preserve per-scenario results and the worst-class score.

### Industry baselines

E7 complements rather than replaces:
- W3C PROV constraints, which define provenance validity, uniqueness and equivalence rules: https://www.w3.org/TR/prov-constraints/
- NIST SP 800-160 Vol. 2 Rev. 1, which frames cyber-resiliency engineering around anticipation, withstanding, recovery and adaptation: https://csrc.nist.gov/pubs/sp/800/160/v2/r1/final
- ISO 22301:2019 and Amendment 1:2024 for business-continuity management requirements and climate-context changes: https://www.iso.org/standard/75106.html

The competitive claim to test is narrower: fewer false independent confirmations, fewer false clears, safer historical replay and less human adjudication effort than document repositories, manual spreadsheets, naive majority voting or ad-hoc ETL. A synthetic test result is not field validation or a certification against any of these standards.

## 7. Tests and status

**34 Rust tests are written**, covering:
- valid synthetic replay and unresolved conflict;
- duplicate/copy, shared sensor, shared calibration and unknown-dependence controls;
- explicit independence attestation, scope/time checks and dependency-path rejection;
- configuration, unit and method mismatch;
- no averaging or majority-based conflict resolution;
- late evidence and bitemporal historical replay;
- back-dated revocation recorded after the decision cutoff;
- unknown evidence state and noncanonical timestamps;
- duplicate identities and dangling dependencies;
- deterministic output under reordered input records;
- source, independence and lifecycle receipt preservation.

Exact-head formatting, Clippy, compilation, test execution and executable output still need hosted CI evidence. A test count is not a pass.

All E7 records are static synthetic fixtures. No live sensor, facility, external provider, credential, procurement process, or physical action is used or authorized. The result is limited to evidence conflict/replay logic and remains non-qualifying.
