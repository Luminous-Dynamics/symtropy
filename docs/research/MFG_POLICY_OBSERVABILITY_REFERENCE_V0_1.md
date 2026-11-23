# Manufacturing Policy Observability Reference v0.1

**Owner:** Symtropy issue [#7190](https://github.com/Luminous-Dynamics/symtropy/issues/7190)  
**Implementation:** draft branch `research/mfg-policy-observability-v0-1`  
**Evidence class:** synthetic, offline  
**Claim ceiling:** event-receipt cutoff and canonical policy-history projection only; no source-system reconciliation or physical-state claim.

## Purpose

The v0.1 multi-period optimizer #7174 uses period snapshots. This separate reference isolates a temporal correctness question: what was actually visible and admissible to a decision at the instant it was made?

An event's effective/occurrence time is not its observation/receipt time. A late-arriving message with an old effective time is invisible to earlier decision cuts. A correction never rewrites history; it appears at the time its own receipt is observed and may supersede a prior receipt only after the correction itself is admitted and definitely effective by the cut.

This crate does not implement an event ledger, reconcile source records, authorize a correction or attest that any source event is true. It consumes synthetic source-receipt-shaped records.

## Contract

Each event includes:
- stable event ID and source ID/owner;
- source sequence;
- exact subject scope and event type;
- effective-time interval `[effective_min_ms, effective_max_ms]`;
- observation/admission time `observed_at_ms`;
- owner-native disposition: ADMITTED, RECONCILED, UNKNOWN or CONFLICTED;
- optional supersession reference;
- SHA-256 evidence digest string.

Each decision cut includes a unique cut ID and cutoff time.

The projection distinguishes:
- **visible receipts:** every receipt observed by the decision cutoff, including Unknown and Conflicted receipts;
- **active/admissible events:** visible ADMITTED/RECONCILED events whose effective-time upper bound is no later than the cut, after applying only visible, admitted supersession relationships;
- **unresolved receipts:** Unknown/Conflicted or not-definitely-effective events, except a prior receipt explicitly superseded by an admitted correction;
- **superseded receipt IDs:** preserved historical IDs whose correction is both visible and admissible.

Clock intervals that overlap the decision cutoff remain unresolved. No midpoint or guessed ordering is used.

Duplicate delivery of the exact same event ID/payload is idempotent. Reuse of an event ID with a different payload, reuse of a source sequence for another event, cross-subject supersession, missing supersession target or correction cycle fails closed.

The history SHA-256 hashes the canonical decision-cut identity/time and every visible source event in a deterministic encoding. It is a digest of this local projection input, not a signature or proof of source truth.

## Frozen 20-timeline corpus

The fixture includes:
1. on-time observation;
2. late arrival with old effective time;
3. exact replay duplicate;
4. admitted correction;
5. Unknown correction;
6. uncertain effective-time interval crossing cutoff;
7. future-effective event;
8. conflicted versus admitted sources;
9. unresolved execution receipt;
10. late measurement receipt;
11. reordered source sequence delivery;
12. product-revision scope isolation;
13. duplicate plus conflict;
14. correction chain;
15. two-source disagreement;
16. Unknown then reconciled execution;
17. bounded event time known to be before the cut;
18. exact cutoff boundary;
19. product-revision-specific evidence;
20. mixed MES/metrology/provider receipts.

Golden projection answers are frozen separately from the event corpus. The current 20 cases are a synthetic conformance corpus, not a statistical campaign.

## Validation

- strict Draft 2020-12 schemas for corpus and golden answers;
- exact comparison of visible, active, unresolved and superseded event IDs at each decision cut;
- event identity and source-sequence collision negatives;
- no-future-receipt and late-arrival non-retroactivity;
- correction visibility and unresolved-correction tests;
- clock interval cutoff behavior;
- exact duplicate replay idempotence;
- serialization permutation invariance;
- post-cutoff payload mutation non-interference for earlier projection and digest;
- SHA-256 history digest generated for every decision cut.

## Relationship to existing owners

- #5758 remains the source-native intended/recorded/observed execution reconciliation owner.
- #7053 owns unresolved-execution freezes and recovery semantics.
- #7101 owns canonical information partitions.
- #7168 owns multi-job policy and benchmark composition.
- This crate owns only the small benchmark-side visibility/projection oracle.

## Claim ceiling

A passing test establishes that this Rust projection implementation matches the frozen synthetic cutoff cases. It does not prove a real MES, sensor, metrology or provider event is true, complete, timely or authorized. It does not change source records, production execution, capacity, route admission, physical machine state or product acceptance.
