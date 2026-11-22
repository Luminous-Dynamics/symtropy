# Maritime Custody Continuity

A dependency-free Rust v0.1 verifier for source-event custody continuity and paired physical handoffs. It consumes canonical container-box IDs from the separate cargo-lineage layer; this package does not resolve identity aliases.

This implements [MARITIME-CUSTODY-CONTINUITY-001 #6662](https://github.com/Luminous-Dynamics/symtropy/issues/6662).

## Composition

- [Cargo identity crosswalk / #6631 and draft PR #6632](https://github.com/Luminous-Dynamics/symtropy/pull/6632)
- [Conserved queue and cargo stock / #6493 and draft PR #6496](https://github.com/Luminous-Dynamics/symtropy/pull/6496)
- [DGT intervention evaluation / #6578](https://github.com/Luminous-Dynamics/symtropy/pull/6578)
- [Coastal-to-inland service chain / #3683](https://github.com/Luminous-Dynamics/symtropy/issues/3683)
- [DGT source admission / #4003](https://github.com/Luminous-Dynamics/symtropy/issues/4003)

## Run

From this directory:

```sh
sha256sum --check data/SHA256SUMS
cargo fmt --all --check
cargo test --offline --locked
cargo run --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
```

## Contract

- One declared opening custodian per physical box, supported by an `INDEPENDENT_OBSERVATION` event class in the synthetic contract.
- A release is valid only from the current custodian.
- One open handoff is allowed at a time.
- Release and acceptance must agree on handoff ID, parties, location, transport leg, quantity and unit.
- Acceptance must occur strictly after release; same-tick chronology is ambiguous and rejected.
- Planned events never advance custody.
- Latest source-event revision at the selected as-of cutoff supersedes older revisions; exact replay is idempotent, same-revision conflicts reject, and a latest void revision removes the assertion.
- Only an acceptance by the declared receiver with a RECEIVER_RECEIPT evidence class qualifies as a receiver-receipted arrival.
- Every box is an indivisible quantity of one BOX. This is not a TEU, tonnes, move-count or consignment ledger.

## Temporal partial-order primitive

The package also exposes a standalone, conservative event-time comparison primitive:

- each event has a closed interval in a declared normalized time basis;
- disjoint intervals produce a proof-grade order only when both bounds were derived from explicit clock profiles with calibration provenance, active validity windows and complete error budgets;
- disjoint estimate-only or mixed-quality bounds produce an estimated order, never a proof-grade order;
- touching or overlapping intervals remain unresolved; identical singleton times are not a simultaneity certificate;
- differing time-basis identifiers are incomparable;
- pairwise reports are deterministic under input permutation and do not manufacture a total order.

The clock-profile constructor requires clock/device ID, source profile/version, time-basis ID, calibration-evidence reference, synchronization method/state, calibration validity window, and explicit maximum offset, holdover-drift, capture-path and rounding errors. Unknown error components, alarmed/unknown clock states, out-of-window timestamps and arithmetic overflow fail closed. It sums the declared worst-case components rather than assuming statistical independence. The calibration reference is still opaque: this code does not authenticate the issuer or independently verify the calibration artifact, and the caller remains responsible for timestamp normalization and the soundness of each bound. The P0 custody state-machine evaluator still uses exact synthetic ticks. The P1 `evaluate_with_handoff_correlations` path now requires a proof-grade temporal binding for each exact release/acceptance event pair; `evaluate_with_synthetic_handoff_correlations` is explicitly limited to dimensionless P0 fixtures.

Reuse the existing maritime timestamp owners (#1930/#1955/#1650) for source parsing, profile semantics and decision-time applicability. The interval-order concept is also related to the generic hybrid event-order work in [#3410](https://github.com/Luminous-Dynamics/symtropy/issues/3410) and the exact-rational interval reference in [#4113](https://github.com/Luminous-Dynamics/symtropy/issues/4113); this crate does not duplicate their continuous-simulation or exact-rational arithmetic scope.

## Frozen synthetic result

The fixtures contain 11 rows: 10 visible at tick 200 and one future-published row. They include one three-handoff chain with a synthetic receiver receipt and one chain that remains pending after release. The correct result is one complete chain and one incomplete chain. The data contain no real DGT IDs, documents or cargo.

## Claim ceiling

This verifies a synthetic state-machine and event-revision contract only. Evidence IDs are opaque; the verifier does not dereference receipts, authenticate issuers, validate authority or source rights, prove sensor truth, inspect seals, prove legal custody, or verify real physical movement. It does not authorize cargo release, dispatch, booking, payment, TOS mutation or operational decisions. Real event time must retain provider timezone, clock source, precision, publication/observation time and semantic mapping before cross-source ordering.

## Exact cross-source handoff correlation receipts

When two systems use different local handoff identifiers, the verifier does not fuzzy-match them. The optional handoff-correlation API requires a receipt that binds:
- exact release source ID + event ID;
- exact acceptance source ID + event ID;
- canonical box ID already resolved by the lineage layer;
- both source-local handoff IDs;
- canonical handoff ID;
- transferring/receiving parties, location, transport leg, quantity and unit;
- versioned mapping profile and an opaque evidence reference.

The verifier checks that the referenced records are the latest visible, non-void, actual release/acceptance events; that the local handoff IDs match the receipt; and that every bound physical field agrees. Correlation is one-to-one for each release and acceptance event. It evaluates a cloned event tape using the canonical handoff ID; original source events are never mutated.

The general `evaluate_with_handoff_correlations` API requires additional `EventTimeBinding` values for the exact source-event revisions. It rejects any order other than `ProvenBefore`: both event-time intervals must be qualified, strictly disjoint, and use the same normalized time basis. Overlap, touching, estimate-only or incomparable time bases refuse the handoff. The separate `evaluate_with_synthetic_handoff_correlations` API is for dimensionless P0 fixtures only; it checks increasing fixture ticks but does not make a real-world chronology claim.

The correlation evidence reference and clock calibration references remain opaque and are not authenticated here. The receipt proves only that an explicit mapping and time-qualification claim was supplied and that its declared fields satisfy the technical contract. It does not prove issuer authority, calibration truth, or physical transfer truth.
