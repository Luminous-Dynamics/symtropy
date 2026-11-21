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
