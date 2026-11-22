# Benchmark Protocol — Maritime Custody Continuity v0.1

## Purpose

Test whether a finite event tape supports a unique, non-contradictory physical custody trace for a declared box and expected receiver. This complements the upstream canonical identity resolver; it does not redo identity resolution.

## Frozen experiment

- Inputs: expectations.tsv and events.tsv.
- As-of cutoff: integer tick 200.
- Time ticks are dimensionless; no timezone or UTC claim is implied.
- Event revisions are grouped by (source_id, source_event_id); future-published records are invisible.
- The latest visible revision supersedes older revisions. Exact duplicates are idempotent. Conflicting payloads at a shared latest revision reject. A latest void record suppresses the assertion.
- Planned records are counted but never mutate custody state.
- Actual records are evaluated per canonical box and ordered by event time. Equal event times are rejected rather than tie-broken.

## Oracle

Expected box outcomes:
- BOX-0001: OPEN at DGT_TERMINAL; paired RELEASE/ACCEPT at each of three handoffs; final ACCEPT by INLAND_RECEIVER carries RECEIVER_RECEIPT; COMPLETE at tick 41.
- BOX-0002: OPEN and RELEASE are actual; matching ACCEPT is planned, and another ACCEPT is future-published; INCOMPLETE with pending handoff at tick 200.

Expected aggregate counts:
- input rows 11;
- visible rows 10; future rows 1;
- planned rows 1; actual rows 9;
- complete boxes 1; incomplete boxes 1; rejected boxes 0.

## Mutation / falsification tests

Mutate one property at a time: current custodian, handoff ID, source/destination party, location, transport leg, quantity, unit, event time, classification, evidence class, revision and publication cutoff. Each negative fixture must remove the false-complete outcome or reject the trace with an explicit reason. No aggregate utility score can compensate for a false-complete chain.

## Baselines and comparison

This v0.1 finite oracle is a semantic correctness baseline, not an optimization-performance comparison. Future P1 must replay permissioned traces against a simple reference state machine and independently adjudicated handoff labels. P2 should add instrumented controlled transfers. P3 may compare custody reconstruction systems on completeness, precision, false-complete rate, latency, resource cost and uncertainty under delayed, duplicated, corrected and missing events.


## P1 integration: temporal proof is a custody-transition precondition

The API split is explicit:
- `evaluate_with_synthetic_handoff_correlations` is P0-only and accepts dimensionless event-time ticks from the frozen fixtures.
- `evaluate_with_handoff_correlations` is the proof-grade P1 path and accepts `QualifiedHandoffCorrelationReceipt` values.

For every correlated handoff, the receipt must bind an `EventTimeBinding` to the exact release/acceptance source event key and selected revision. The temporal evidence ID itself is a length-prefixed encoding of source ID, event ID and revision, preventing an evidence object from being replayed against a different event key. The evaluator checks:
1. the event refs match the correlation receipt exactly;
2. the bound revisions are the latest visible effective revisions at the selected as-of cutoff;
3. each temporal-evidence ID matches the corresponding event-key/revision tuple;
4. the interval relation is exactly `ProvenBefore`.

EstimatedBefore/After, UnresolvedOverlap, IncomparableTimeBasis and ProvenAfter all refuse the handoff. This means the general custody-evaluation entry point cannot silently treat a timestamp heuristic or exact source-local ID match as proof of release-before-acceptance.

The clock profile is still a caller-supplied claim. Calibration artifacts, source issuer, clock synchronization trace, timestamp semantics and source permissions must be independently validated by the upstream adapters/authority owners. Passing this technical gate does not authenticate them.

## Claim ceiling

Passing P0 establishes only the specified synthetic custody contract. It does not establish real-world identity, receipt authenticity, legal title, issuer authority, source permission, seal integrity, physical truth, delivery performance, or suitability for operational reliance.

## P1: cross-source temporal uncertainty

P0 uses exact synthetic ticks only. A real cross-source trace must not impose a total order from point timestamps whose clocks, precision or offsets are not qualified. Each event must carry source-specific event-time semantics and, where the source clock is bounded rather than exact, a conservative event-time interval [earliest, latest]. A release is provably before an acceptance only when release.latest is strictly less than accept.earliest. Overlapping or touching intervals are `TemporalOrderIndeterminate`; they are not ordered by ingestion time, source preference, or arbitrary tie-break. Preserve event time, source publication/record time, ingestion/receipt time, and decision as-of cutoff separately. Compose #1930/#1955 and Mycelix #437 rather than creating another universal timestamp profile.

## P1: identity and handoff-correlation receipts

A canonical box ID must come from the qualified crosswalk layer (#6631). If two source systems lack a common handoff ID, do not fuzzy-match by time/location/container text. Require a separate, reviewed correlation receipt binding the exact release event and acceptance event, box identity, parties, location, transport leg, semantic mapping profile and evidence commitments. Preserve unmapped fields and declared translation loss. A correlation receipt proves the mapping decision exists, not that the physical handoff was true.

## P1/P2: independent authentication and authority

The v0.1 RECEIVER_RECEIPT enum is only an evidence-class gate. A later implementation must separately validate the referenced artifact, signature/provenance, issuer identity, currentness, receiver identity and exact receiving authority under an explicit purpose and policy. Compose Mycelix #437/#1073/#1109/#1110/#1121. Authentication must not be confused with physical observation, legal title, customs clearance, cargo release or operational authority.

## P1/P2: matched industry comparison

Use identical adjudicated, permissioned event tapes, as-of cutoffs, crosswalks and receiver declarations for:
- source-native TOS / interchange history;
- publicly available DCSA Track & Trace profile/version supported by the test;
- GS1 EPCIS 2.0.1 / CBV 2.0.0;
- relational reconciliation baseline;
- simple independent custody state machine;
- identity-plus-custody composition.

Publish event/chain precision and recall, false-complete and false-incomplete rates, abstention/unresolved fraction, correction/void handling, interval-order abstention, mismatch-reason coverage, runtime, memory and sensitivity to reporting delay. Keep physical truth adjudication independent of every tested system. No superiority claim until this matched experiment is complete.

## P1 time evidence: partial order, not timestamp sorting

The standalone time-order primitive now included with the benchmark accepts event-time intervals in one explicitly named normalized time basis. Its relation is:
- `ProvenBefore/After` only when the intervals are strictly disjoint and both inputs are upstream-qualified enclosures;
- `EstimatedBefore/After` when intervals are disjoint but at least one bound is estimate-only;
- `UnresolvedOverlap` for intersecting intervals, including touching endpoints and identical singleton times;
- `IncomparableTimeBasis` when event times have not been mapped to the same declared basis.

A pair of overlapping intervals is not a simultaneity proof. This module returns pairwise relations and never inserts an arbitrary total-order tie-break. Its P1 acceptance corpus covers interval overlap, touching, identical singleton times, clock-basis mismatch, proof-grade vs estimate-only bounds, positive time-unit scaling and input permutation.

The module does not itself normalize RFC 3339 timestamps, verify clock synchronization, establish a trustworthy error bound, authenticate the source, or validate event-time semantics. These remain owned by #1930/#1955 and the source/forecast applicability work in #1650. In the DCSA Track & Trace profile, preserve eventDateTime separately from eventCreatedDateTime; the DCSA Information Model defines the former as when the event occurred or will occur and the latter as when the event entry was created. For EPCIS, preserve eventTime separately from recordTime; the latter is repository bookkeeping, not real-world event time. Neither schema conformance nor a timezone offset alone certifies clock accuracy. References: https://reference.dcsa.org/files/content/standards/guidelines/information-model-2-0/dcsa-information-model-2-0.pdf and https://ref.gs1.org/epcis/ObjectEvent

The existing P0 custody report still uses exact synthetic ticks. The next integration increment must require time-basis mapping receipts and qualified clock-error enclosures for each source event, then use the partial-order result as a precondition for custody-state mutation. An unresolved or incomparable release/acceptance pair must remain indeterminate and cannot complete a chain.

## P1 exact cross-source handoff correlation

The verifier now supports an optional explicit correlation receipt when release and acceptance systems use different source-local handoff IDs. The receipt binds the exact source event keys on both sides, canonical box ID from the lineage layer, both local handoff IDs, canonical handoff ID, parties, location, transport leg, quantity/unit, versioned mapping profile and opaque evidence reference.

Acceptance conditions:
- source event references resolve to the latest visible, non-void, ACTUAL records at the chosen as-of cutoff;
- the referenced records are respectively RELEASE and ACCEPT;
- canonical box identity matches on both sides and the receipt;
- the receipt's local handoff IDs exactly equal the raw source event fields;
- parties, location, transport leg, quantity and unit match on both events and the receipt;
- each release event and each acceptance event can be mapped at most once;
- all unreferenced records retain their source-local values;
- correlation is applied to a cloned event tape; raw source records are never rewritten.

No fuzzy matching by vessel name, timestamp proximity, route, location text or identifier edit-distance is allowed. A receipt that fails one exact binding does not repair the underlying event records. Without a receipt, mismatched local handoff IDs remain a hard handoff mismatch.

The correlation receipt's evidence ID is opaque. P1 does not dereference or authenticate it, identify an issuer or validate representative authority. It records an explicit mapping claim; it is not physical proof of the handoff. Source identity resolution (#6631), semantic translation (#1955/#437) and authority admission (Mycelix) remain independent prerequisites/owners.

The new finite corpus includes different source-local IDs that can be joined only by an exact receipt, no-receipt rejection, wrong field binding, wrong event reference, voided release, planned acceptance, duplicate release mapping, missing evidence reference and source-record immutability.

## P1 integration: whole-chain temporal proof before custody replay

The general `evaluate_with_handoff_correlations` API requires a `EventTimeBinding` for every effective ACTUAL event visible at the selected publication cutoff. Each binding must name the exact `(source_id, source_event_id, revision)` and carry a clock-profile-derived `QualifiedEnclosure` with a common normalized time basis for that box's event chain.

The evaluator:
- selects the latest visible revision for every source event key and rejects same-revision payload conflicts;
- ignores planned, latest-voided and future-published events for custody mutation;
- rejects missing, duplicate, stale-revision, unexpected or event-ID-substituted time bindings;
- requires every pair of effective actual events for the same canonical box to be strictly ordered by `ProvenBefore` or `ProvenAfter`; overlap, touching, estimated order and incomparable time bases refuse the chain;
- only after that proof sorts the events and assigns ordinal times on a cloned event tape, so the existing deterministic custody reducer never relies on unrelated source-local clock ticks;
- applies exact handoff-correlation receipts and then evaluates the custody state machine.

The input events and their raw source-local timestamps are never modified. Ordinal ticks exist only inside the cloned evaluator tape and are not exported as physical timestamps.

`evaluate_with_synthetic_handoff_correlations` remains a separately named P0 path for dimensionless frozen fixtures. It uses strictly increasing fixture ticks and must not be used to claim real chronology. The pair-level `evaluate_with_paired_handoff_time_evidence` helper validates only an individual release/acceptance pair; it is not a replacement for the whole-chain P1 API.

Production adapters must pass only time bindings for effective visible actual revisions, after source event-time semantics and clock bounds have been qualified by the existing #1930/#1955/Mycelix adapter owners. Calibration references remain unauthenticated by this crate.

## API boundary clarification

The low-level `apply_handoff_correlations` utility validates exact event identity and physical handoff fields only; it deliberately does not compare raw event timestamps. The `apply_qualified_handoff_correlations` helper is a pair-level diagnostic over one exact release/acceptance pair and is not a complete custody evaluator.

Only these evaluator entry points make a chain-level claim:
- `evaluate_with_synthetic_handoff_correlations`: P0 frozen synthetic ticks, with a strict release-tick-before-acceptance-tick check;
- `evaluate_with_handoff_correlations`: P1 whole-chain path; requires qualified time bindings for every effective actual event on each box, proves pairwise strict order, assigns private ordinal ticks on a clone, applies exact correlation receipts, and then runs custody replay.

Do not use the low-level correlation utility or pair-level diagnostic as evidence that a complete real custody chain is temporally qualified.
