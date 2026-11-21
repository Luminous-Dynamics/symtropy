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
