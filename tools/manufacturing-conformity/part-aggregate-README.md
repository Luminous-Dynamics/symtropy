# Part-level manufacturing conformity aggregation

This standalone Rust utility aggregates scalar manufacturing conformity receipts under one immutable part/serial, lot, product revision, operation and required-characteristics manifest. It does not recompute metrology and does not authorize final product release.

## Disposition semantics

- **REJECT_NONCONFORMING** when at least one valid, current, mandatory scalar receipt is REJECT_NONCONFORMING.
- **INDETERMINATE** when no valid mandatory rejection exists but any mandatory characteristic is absent, unresolved, evidence-incomplete, stale, duplicated, supersession-invalid or scope-mismatched.
- **CHARACTERISTICS_ACCEPTED** only when every mandatory characteristic has exactly one current, verified scalar ACCEPT receipt and the manifest is valid.
- **INVALID_MANIFEST** for malformed manifest/receipt identity, digest format or impossible receipt identity structure.

The positive status is deliberately named CHARACTERISTICS_ACCEPTED, not PRODUCT_RELEASED. The existing authorized quality function retains final release authority.

Optional characteristics affect the aggregate only when the manifest marks them mandatory. Their absence is recorded. Extra or undeclared characteristic records are retained as extra evidence but cannot substitute for a missing required characteristic.

## Supersession

Remeasurement creates a new child receipt. The new receipt names the prior receipt it supersedes; the previous receipt remains in the chain as historical. The manifest names the exact current receipt. The chain must have one root, one current terminal node, no branches/cycles and monotonic issued timestamps. Disconnected or ambiguous histories block acceptance.

## Scope and provenance

Every required current receipt must also be available as of the aggregate decision timestamp: both measurement time and receipt issue time must be at or before that timestamp. A later remeasurement or later-issued result cannot be used to rewrite an earlier historical disposition; it creates a new current decision at a later time.

Every required current receipt must match the manifest's:
- part and serial;
- lot and product revision;
- operation;
- characteristic, measurand and unit;
- specification identifier and revision;
- effective interval / decision time.

Each child report digest, source ID, scalar outcome/reason, evidence state, issue timestamp, current/superseded flag and supersession parent are preserved in separate source-receipt rows. Historical and undeclared extra receipts remain in the replayable report rather than being reduced to counts. This v0.1 reference validates the supplied declaration and digest format; it does not cryptographically authenticate signatures or independently revalidate child measurement systems.

## Complexity

Receipts and requirements are sorted / grouped by identity. The intended aggregation is O(n log n) in the number of child receipts, with no probabilistic independence assumptions and no averaging of scalar dispositions.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-conformity/part_aggregate.rs -o /tmp/part-aggregate-tests
    /tmp/part-aggregate-tests

    rustc --edition=2024 -D warnings tools/manufacturing-conformity/part_aggregate.rs -o /tmp/part-aggregate
    /tmp/part-aggregate --verify
    /tmp/part-aggregate --csv

## Claim and authority ceiling

The aggregate classifies the required scalar evidence set. It does not establish process capability, validate upstream uncertainty, certify a part/assembly, establish production-lot risk or authorize shipment/release/actuation. CHARACTERISTICS_ACCEPTED is not PRODUCT_RELEASED.
