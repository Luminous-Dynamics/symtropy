# Canonical Strata Ingestion V0

## Status

Normative Living World authority contract. This document constrains future persistence, network, import, and migration adapters that construct sparse canonical population strata. It does not claim the current Rust API already implements every boundary described here.

## Problem

`StratifiedPopulationState` is represented internally by a keyed sparse map. A map guarantees one value per key *after* construction, but a naive decoder can accidentally do this:

1. decode a sequence of stratum records;
2. collect directly into a map;
3. silently overwrite an earlier record when the same key appears twice;
4. pass the already-collapsed map to canonical validation.

At that point duplicate source authority has been lost before validation can observe it.

For canonical ecological state this is unacceptable. Duplicate records may indicate corrupted persistence, ambiguous merge semantics, replay bugs, malicious input, or incompatible producer behavior. They must never be interpreted through ordinary last-write-wins map semantics.

## Canonical ingestion rule

Any external or serialized sequence that can contain more than one record MUST cross a duplicate-detecting ingestion boundary before it becomes `StratifiedPopulationState` authority.

Conceptually:

```text
untrusted / serialized stratum records
            |
            v
      decode structural fields
            |
            v
  duplicate-detecting builder
            |
   +--------+---------+
   |                  |
unique                 duplicate
   |                  |
   v                  v
validate exact      REJECT
count/biomass
   |
   v
canonical sparse state
```

## V0 uniqueness key

For the current schema, uniqueness is over the complete joint key:

```text
(age band, condition band, occupancy cell)
```

When the structural schema grows, uniqueness applies to the complete canonical key for that schema version. A producer must not rely on omitted/defaulted axes to distinguish entries.

## Duplicate semantics

Duplicate keys are rejected even when both records carry identical count and biomass values.

The ingest layer MUST NOT silently:

- keep the first record;
- keep the last record;
- sum duplicate counts;
- sum duplicate biomass;
- average values;
- deduplicate identical records;
- resolve duplicates by input order.

Those are all ecological policy choices masquerading as parsing behavior.

If an explicit future merge operation wants to combine two authorities, it must do so through a typed ecological settlement/merge boundary with provenance and conservation checks. Canonical decoding is not that boundary.

## Ordering

Input ordering is not ecological authority.

A duplicate-free sequence may be accepted in any order if the decoder's schema permits it. Canonical in-memory and encoded forms should normalize ordering independently, for example through canonical key order.

Therefore:

```text
same unique records + different input order
    -> same canonical state
```

but:

```text
same key repeated twice
    -> reject
```

## Atomicity

Ingestion is all-or-nothing.

Before canonical state is published, the builder must validate at least:

- schema/version understood;
- every key structurally valid;
- no duplicate complete key;
- no zero-count occupied stratum;
- count aggregation does not overflow;
- exact biomass aggregation does not overflow;
- any schema-specific extensive-state constraints.

Failure leaves no partially authoritative population.

## Error identity

The public error contract should distinguish duplicate-key rejection from numerical or structural failure.

A future Rust shape may resemble:

```text
DuplicateStratumKey { key }
```

or a schema-neutral equivalent.

The error must identify enough information for diagnostics without relying on input-order side effects.

## Trusted-map constructor boundary

A constructor that accepts an already-built `BTreeMap` can remain useful as an internal/trusted structural constructor, because duplicate keys are already impossible at its type boundary.

However, persistence/network/import code MUST NOT prove input uniqueness merely by collecting directly into that map first.

Preferred layering:

```text
from_records(iterable)  -- duplicate detecting canonical ingress
        |
        v
validated unique map
        |
        v
from_canonical_map(...) -- trusted/internal structural constructor
```

The exact Rust visibility can be chosen later, but the semantic distinction is normative.

## Persistence / wire implication

Serialized strata should be represented as an explicit sequence with schema versioning rather than relying on language-map decoding behavior to define duplicate semantics.

A decoder must reject ambiguous duplicate records before producing canonical authority.

If a canonical wire format later hashes/signs strata, duplicate rejection occurs before the decoded object is admitted as current semantic authority.

## Qualification fixtures

Minimum executable evidence should include:

1. two distinct unique records ingest successfully;
2. the same records in reversed order produce the same canonical state;
3. duplicate key with identical extensive state is rejected;
4. duplicate key with different extensive state is rejected;
5. a duplicate cannot disappear through intermediate map collection in the tested public ingress path;
6. zero-count strata still fail closed;
7. count overflow still fails closed;
8. biomass overflow still fails closed;
9. failure publishes no partial state;
10. canonical normalized iteration order is independent of source record order.

## Non-goals

This contract does not define:

- ecological merging of two populations;
- migration settlement;
- conflict resolution between authorities;
- persistence encoding bytes;
- CRK event identity;
- arbitrary dynamic stratum schemas.

It freezes only the rule that **canonical sparse ecological authority must observe and reject duplicate source keys before ordinary map semantics can erase the ambiguity**.
