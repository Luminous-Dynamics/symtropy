# Benchmark protocol v0.1 — Maritime Cargo Lineage

## Research question

Can independently authored identity crosswalks reconcile source-local equipment identifiers to one physical container identity without false merges, while preserving source revisions, time validity, event publication cutoffs and delivery evidence quality?

## Standards position

Use existing interoperability vocabularies rather than inventing another one:

- DCSA Track & Trace covers shipment, transport, equipment, IoT and reefer event domains.
- GS1 EPCIS 2.0 describes event dimensions through What, When, Where, Why and How and supports sharing visibility events across organizations.
- DCSA Port Call / JIT and IAPH-IHMA PCO describe port-call and service-time semantics; those records do not become container identity assertions merely because they share a vessel or voyage reference.

A standard-conformant message is not by itself proof that two local identifiers refer to the same physical box. The crosswalk still needs an issuer, evidence reference, time window, revision, authority and conflict policy.

## Identity rules

1. Physical container, vessel visit, shipment/booking and source event are separate entity types.
2. Alias resolution requires an exact key: entity kind + source ID + local ID.
3. Each mapping ID has revisions. Only the latest revision published by the as-of cutoff is visible.
4. A mapping is valid on the half-open interval [valid_from, valid_until). This prevents two adjacent mappings from both owning the boundary instant.
5. Overlapping active mappings to different canonical identities are ambiguous; do not pick the newest download or closest timestamp.
6. Revoked, unmatched, ambiguous, conflicting and out-of-window aliases are not resolved.
7. If a mapping revision is published after an historical cutoff, it cannot change the earlier replay.

## Event rules

- Source event identity is scoped by source ID and source event ID.
- Exact duplicate rows for the same revision are idempotent.
- Conflicting rows reusing the same source event ID and revision reject.
- A later visible revision supersedes the earlier one; a voided latest revision removes that source event from the effective set.
- Future-published revisions are invisible at the replay cutoff.
- Planned milestones are retained in the audit counts but cannot mutate a physical-delivery count.
- A counted receipt-backed delivery assertion requires a resolved physical CONTAINER_BOX identity, a DELIVERED_TO_RECEIVER milestone, RECEIVER_RECEIPT evidence kind and a non-empty evidence_id. Provider-only delivery claims are reported separately. Evidence IDs are opaque references in v0.1; the referenced artifact, issuer identity and receiving-party authority are not authenticated here.

## Frozen synthetic oracle

The finite corpus contains one physical box represented in four source systems, a second distinct box, a provider-only delivery assertion, a planned delivery, an unmatched receiver alias, an exact duplicated source event and a future-published receiver event.

At as-of tick 200, the hand-computed golden outcome is:

| Measure | Expected |
|---|---:|
| Input event rows | 11 |
| Visible event rows | 10 |
| Future event rows | 1 |
| Exact replay rows | 1 |
| Planned event rows | 1 |
| Actual event rows | 8 |
| Resolved actual event rows | 7 |
| Unresolved actual event rows | 1 |
| Unqualified delivery claims | 1 |
| Receiver-receipted delivery assertions | 1 |
| Unique receipt-backed physical-box IDs | 1 |

These are synthetic ticks and records, not minutes, containers handled by DGT, or a real cargo-delivery result. The receipt label is not a source-authentication result.

## Adversarial test families

- ambiguous or conflicting aliases;
- alias reuse over non-overlapping time windows;
- mapping correction, late publication and revocation;
- future event revisions and same-revision conflicts;
- exact replay versus event-ID collision;
- provider assertion versus receiver receipt;
- planned versus actual event class;
- unresolved container identity at a receiver;
- one box referenced by several systems and several receipts;
- two boxes belonging to one hypothetical booking;
- box identity versus vessel-visit identity;
- event time at validity interval boundaries.

## Promotion gates

P0 synthetic conformance:
- 100% agreement with independent golden outcomes;
- zero false merges;
- zero duplicate canonical-box delivery counts;
- zero planned-to-actual promotions;
- zero future-publication leakage;
- zero delivery counts for unresolved, ambiguous, revoked or out-of-window aliases;
- deterministic offline replay from pinned fixtures.

P1 source admission:
- documented source data rights;
- source-specific ID dictionary and issuer authority;
- stable physical-container identifiers or independently supported crosswalks;
- source revisions, valid/event times, publication/observation/receipt times;
- explicit treatment of transshipment, split/merge, rebooking and container replacement.

P2 observed lineage:
- match receiver receipts against source-local terminal, feeder and rail aliases;
- independently sample crosswalk accuracy against physical/documentary evidence;
- report false-merge and false-split rates with confidence intervals;
- retain ambiguous/unmatched share and sensitivity to crosswalk coverage.

P3 network benchmark:
- compose identity-qualified events with #6496 stock accounting and #6578 intervention lifecycles;
- compare local silo counts with unique-box delivered counts;
- measure displaced inventory, extra lifts, feeder legs, rail service and delivery tails;
- do not make a causal claim without a defendable comparison design.

No promotion from P0 synthetic correctness to real-world delivery or causal impact is permitted.
