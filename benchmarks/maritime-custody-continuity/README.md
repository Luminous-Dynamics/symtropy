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

## Frozen synthetic result

The fixtures contain 11 rows: 10 visible at tick 200 and one future-published row. They include one three-handoff chain with a synthetic receiver receipt and one chain that remains pending after release. The correct result is one complete chain and one incomplete chain. The data contain no real DGT IDs, documents or cargo.

## Claim ceiling

This verifies a synthetic state-machine and event-revision contract only. Evidence IDs are opaque; the verifier does not dereference receipts, authenticate issuers, validate authority or source rights, prove sensor truth, inspect seals, prove legal custody, or verify real physical movement. It does not authorize cargo release, dispatch, booking, payment, TOS mutation or operational decisions. Real event time must retain provider timezone, clock source, precision, publication/observation time and semantic mapping before cross-source ordering.
