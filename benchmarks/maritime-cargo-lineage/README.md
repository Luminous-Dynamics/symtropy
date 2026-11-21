# Maritime Cargo Lineage

A dependency-free Rust v0.1 verifier for time-bounded, source-local container identity crosswalks and evidence-qualified unique-box delivery counting.

This is the first executable slice of [MARITIME-CARGO-LINEAGE-001 #6631](https://github.com/Luminous-Dynamics/symtropy/issues/6631). It composes with, and does not replace:

- [DGT source admission #4003](https://github.com/Luminous-Dynamics/symtropy/issues/4003);
- [conserved backlog ledger #6493 / PR #6496](https://github.com/Luminous-Dynamics/symtropy/pull/6496);
- [DGT intervention evaluation #6578](https://github.com/Luminous-Dynamics/symtropy/pull/6578);
- [end-to-end service-chain benchmark #3683](https://github.com/Luminous-Dynamics/symtropy/issues/3683);
- [PCO evidence envelope #1930](https://github.com/Luminous-Dynamics/symtropy/issues/1930) and [DCSA cross-standard semantics #1955](https://github.com/Luminous-Dynamics/symtropy/issues/1955).

## Run

From this directory:

```sh
sha256sum --check data/SHA256SUMS
cargo fmt --all --check
cargo test --offline --locked
cargo run --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
```

## What is actually verified

- Alias identity is scoped by entity kind, source system and local identifier.
- A mapping has a stable mapping ID, revision, publication time and half-open valid-time interval [valid_from, valid_until).
- A historical replay sees only mappings and events published by its as-of cutoff.
- A higher visible mapping/event revision supersedes the earlier revision.
- Conflicting payloads at the same mapping or event revision fail closed.
- Explicit revocation blocks identity resolution.
- An identifier reused in non-overlapping validity intervals can resolve to different canonical boxes at different event times.
- Exact replay of one source event is idempotent.
- Planned records do not count as physical realization.
- A delivery count requires a resolved CONTAINER_BOX identity, a DELIVERED_TO_RECEIVER milestone and RECEIVER_RECEIPT evidence.
- Multiple source aliases and receipt rows for one canonical box contribute one unique delivered box.

## What it deliberately does not establish

This v0.1 is a crosswalk and counting contract—not a complete cargo-custody chain verifier. It does not infer that a box physically moved through every intermediate handoff, authenticate issuers, validate source rights, resolve booking/consignment relationships, verify seals, establish a DGT visit-to-container crosswalk, or estimate real port performance.

The event fixture and crosswalks are synthetic. They contain no DGT records and no real container IDs. A receiver receipt in this finite test fixture is an example of the required evidence class, not a claim about a real shipment.

The benchmark counts physical boxes, not TEUs, container moves, bookings, consignments, or tonnes. Those are different measures and require their own explicit mappings and units.
