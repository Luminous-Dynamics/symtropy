# World Continuation Manifest v0.1 — Status

**Date:** 2026-09-01  
**Branch:** `docs/world-continuation-manifest-v0.1`  
**Status:** design frozen; docs-only; implementation and qualification pending

## Current result

This tranche freezes the continuation/persistence semantics needed before Symtropy can honestly claim that an unloaded planet, region, catastrophe session, or living watershed can later resume as the **same causal world**.

It adds no runtime code and makes no Rust/Nix qualification claim.

## New canonical contracts

- `WORLD_CONTINUATION_MANIFEST_CONTRACT_V0_1.md`
- `Q2_CONTINUATION_EVIDENCE_CONTRACT_V0_1.md`

## Key decisions

1. `symtropy-world` remains an orchestrator, not a mega-authority.
2. World continuation uses a hierarchical content-addressed manifest root.
3. Physical-state, continuation, lineage, and snapshot-artifact identities remain distinct.
4. Same-world continuation preserves immutable world lineage identity; forks mint a distinct lineage identity while retaining ancestry.
5. Inactive-time behavior is explicit and simulation-time based; wall-clock time is never authoritative gameplay time.
6. Hidden continuation state such as Hydrology/Thermal active frontiers, ActiveLava `next_step`, nested eruption continuation, and landscape integration residuals must be preserved before Q2 can pass.
7. Derived indexes/caches may be omitted only with deterministic rebuild/equivalence evidence.
8. Representation residency semantics must survive suspend/resume or be provably reconstructed.
9. Snapshot bytes are content-addressed independently from semantic state identities.
10. A causal journal head becomes canonical world-continuation evidence only under an explicitly canonical serializer-independent event identity.
11. Q2 requires machine-checkable evidence tied to the exact promoted Git tree.
12. Q3 native CUF adapters remain blocked until Q2 produces a qualified parent.

## Current dependency graph

```text
#74 Q0/Q1 exact v4.8 replay/build truth
        |
        v
#76 / #79 continuation identity repairs
#81 landscape residual continuation
        |
        +---- #82 canonical event identity when journal head required
        |
        v
#83 world continuation manifest
  + #87 snapshot/artifact semantics
  + #89 same-world vs fork semantics
  + #90 child scope/conflict rules
  + #85 inactive-world time policy
  + #86 representation residency lifecycle
  + #88 golden vectors
        |
        v
#84 Q2 machine-checkable evidence capsule
        |
        v
Q2-qualified exact tree
        |
        v
#72 CUF v0.11 native adapters / Q3
        |
        v
Living Watershed / Q4
```

Issue #91 is the sequencing umbrella for this program.

## Public CI status

The dependency-light CUF GitHub Actions job associated with PR #74 is still queued at the time of this status record. GitHub reports no assigned runner and no executed steps, so no PASS/FAIL inference is made from that queue.

Public Tier A CI remains supplementary to the local/private full Q0-Q3 evidence gates.

## Implementation start conditions

The manifest Rust implementation may be prototyped after the docs contract is accepted, but it must not be promoted as Q2-complete until:

- exact Universal Matter v4.8 Q0/Q1 replay result is preserved;
- continuation-significant authority/state repairs are present;
- required snapshot/restore APIs expose complete identities;
- the Q2 evidence runner can execute the hidden-state and suspend/resume fixtures.

## Preferred implementation placement

The portable canonical identity primitives should live in a dependency-light layer where practical.

`symtropy-world` may own orchestration/assembly/validation of the hierarchical manifest, but domain-specific snapshot creation and authority continuation identity remain owned by their domains.

`symtropy-game-state` may provide canonical causal journal identity after #82, but the world manifest should consume that identity rather than hashing generic event payload serialization itself.

## Non-goals

This status does not claim:

- Universal Matter v4.8 has been replayed locally;
- Q1 is green;
- Q2 repairs are implemented;
- the world manifest Rust types exist;
- native v0.11 adapters are implemented;
- Living Watershed is Q4-qualified;
- planetary/stellar Q5 scale has been demonstrated.

## Next decisive work

The next real executable milestone remains the guarded local Universal Matter v4.8 replay and Q0/Q1 evidence capture.

After that result is preserved, the Q2 hardening sequence should be implemented against the exact replay lineage rather than modifying the retained historical artifact.
