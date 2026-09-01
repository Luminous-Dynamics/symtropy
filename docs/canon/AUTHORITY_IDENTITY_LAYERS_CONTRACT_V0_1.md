# Authority Identity Layers Contract v0.1

**Status:** canonical architecture contract  
**Date:** 2026-09-01  
**Scope:** deterministic mutable authorities, persistence, replay, networking, CUF provenance, world lifecycle

## Governing principle

An authority can have more than one valid identity.

Do not force every identity question into one `digest()` and do not assume that hashing every private Rust field is correct.

The identity used by a consumer must match the claim that consumer is making.

## Identity layers

### 1. Physical state identity

Answers:

> What canonical physical/domain state exists at this instant?

Examples:

- sparse matter state;
- current surface-water cells;
- current disturbed thermal deltas;
- current active-lava cells;
- current ecosystem recovery cells.

A physical-state digest may intentionally omit:

- scheduling frontiers;
- replay sequence numbers;
- commit-parent metadata;
- reconstructible indexes;
- caches.

If so, its name and documentation must not imply a stronger claim.

### 2. Continuation identity

Answers:

> Given equal external inputs and the same deterministic implementation, is the authority in the same state for determining what happens next?

Continuation identity must bind every piece of authority-owned state that can alter the next authoritative transition.

Examples include:

- active work/frontier sets;
- deterministic tie-breaking step counters;
- PRNG state when a stateful PRNG is legitimately authority-owned;
- pending queues that influence the next transition;
- internal phase/state-machine position;
- allocator counters when generated identities alter later semantics.

A physical-state digest is not automatically a continuation digest.

### 3. Lineage / replay identity

Answers:

> Is this authority at the same point in its evidence/replay history?

Examples include:

- next commit sequence;
- previous commit digest;
- branch/revision lineage metadata;
- replay cursor;
- signed/transcript parent identity.

Lineage metadata may change receipt identity without changing the next physical mutation produced by an externally supplied command.

Do not automatically fold lineage metadata into physical-state identity.

### 4. Derived index / cache identity

Derived indexes, accelerators, presentation caches, and reconstructed lookup tables are not independent truth when they can be deterministically rebuilt from canonical authority state.

Examples:

- cell-to-node lookup rebuilt from canonical structural nodes;
- render chunks derived from Matter;
- acceleration structures;
- sorted search indexes that are regenerated from canonical entries.

Such structures may have cache-integrity hashes for diagnostics, but their presence must not create a second persistence authority.

## Consumer rule

Every API, receipt, snapshot, network packet, or CUF view that embeds a digest must state which layer it represents.

Examples:

- a renderer invalidation key usually needs physical-state or region identity;
- deterministic replay resumption may need continuation + lineage identity;
- a CUF observation of present water may use physical-state identity;
- a CUF claim that two authorities will continue identically under equal inputs needs continuation identity;
- a signed commit chain needs lineage identity.

Do not label a physical-state digest as `authority_digest` unless the authority contract explicitly defines it as complete for the intended claim.

## Canonical continuation digest construction

Prefer additive, domain-separated continuation identities rather than silently widening existing stable digest semantics.

A continuation digest should bind:

1. a unique domain separator and schema version;
2. the authority's physical-state digest or canonical physical state;
3. every non-derived authority-owned continuation field;
4. explicit counts for collections;
5. deterministic canonical collection order;
6. stable typed/enum codes;
7. fixed integer endianness;
8. any upstream authority identities whose state is part of the continuation contract, if the authority explicitly owns a coupled continuation rather than merely reading an external input.

For native cell sets use canonical address ordering and hash x/y/z explicitly.

## Existing digest compatibility

Do not retroactively broaden an existing digest merely because a stronger identity becomes necessary.

If `FooStateDigest` has already meant "physical field state," prefer adding:

- `FooContinuationDigest`;
- `FooLineageDigest`;
- or an explicit composite receipt

rather than silently changing `FooStateDigest` bytes or semantics.

Schema/version changes are required if an existing digest's semantic contract truly must change.

## Checkpoint rule

A checkpoint must preserve every state layer required to resume the authority correctly.

If a checkpoint persists a field that is absent from physical-state digest identity, reviewers must classify it:

- continuation state;
- lineage state;
- derived/rebuildable state;
- incidental diagnostic metadata.

Persisted-but-unclassified hidden state is a qualification failure.

## Determinism test pattern

For every mutable deterministic authority, qualification should include two complementary properties.

### State sensitivity

Construct two authorities that share the same physical-state digest but differ in one continuation field. If that field can alter future behavior, the continuation digests must differ.

### Continuation sufficiency

Construct two authorities with equal continuation identities and equal external inputs. Their next bounded transition must produce equal:

- reports/receipts where deterministic;
- resulting physical-state digest;
- resulting continuation identity.

This is stronger than snapshot round-trip alone.

## Derived-index test pattern

For a field classified as derived/rebuildable:

1. construct canonical authority state;
2. snapshot it without the index;
3. restore/rebuild the index;
4. prove public semantics and canonical digest are unchanged;
5. reject malformed canonical inputs such as duplicate keys rather than allowing ambiguous reconstruction.

## Scheduling/frontier rule

A worklist, active set, event queue, or frontier is continuation state when membership or ordering can change which subset of a bounded simulation is evaluated next.

This remains true even if the frontier contains no physical quantity itself.

If the algorithm is proven to converge to the same next observable state regardless of frontier membership/order, that proof may justify treating it as derived. Absent such a proof, fail closed and bind it to continuation identity.

## Counter rule

A sequence/counter is continuation state when its value participates in physical selection, randomization, tie-breaking, or transition equations.

A counter is lineage-only when it changes receipt/commit identity but cannot change the physical transition under equal externally supplied commands.

This distinction must be demonstrated from code/tests rather than inferred from the field name.

## Cross-authority derived values

A resolved value may depend on more than one authority.

Do not solve this by inventing a fake composite authority name.

Instead bind the exact source identities in a domain-specific derived-view or receipt contract.

Example:

- a resolved groundwater voxel may depend on Matter geometry/procedural hydrogeology and sparse Hydrology overrides;
- its provenance therefore has multiple source authorities even though the returned value is one water sample.

## Review checklist for a new Authority

Before qualification, reviewers should answer:

1. What fields does the authority own?
2. Which are canonical physical state?
3. Which affect the next bounded transition?
4. Which affect only replay/evidence lineage?
5. Which are deterministic indexes/caches?
6. What does `digest()` actually claim?
7. What does the checkpoint persist that the digest omits?
8. Does collection ordering affect behavior?
9. Do counters affect physical tie-breaking or only receipts?
10. Can two equal digests currently produce different next physical state?
11. Can every omitted index be deterministically rebuilt and validated?
12. Are the necessary identities tested independently?

## Fail-closed rule

If reviewers cannot classify a persisted authority field, do not call the existing digest a complete authority identity.

A narrower, accurately named digest is preferable to a stronger but false claim.