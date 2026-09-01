# World Continuation Manifest v0.1 — Status

**Date:** 2026-09-01  
**Branch:** `docs/world-continuation-manifest-v0.1`  
**Status:** design frozen; docs-only; implementation and qualification pending

## Current result

This tranche freezes the continuation/persistence semantics needed before Symtropy can honestly claim that an unloaded planet, region, catastrophe session, living watershed, or migrated network authority can later resume as the **same causal world**.

It adds no runtime code and makes no Rust/Nix qualification claim.

## New canonical contracts

- `WORLD_CONTINUATION_MANIFEST_CONTRACT_V0_1.md`
- `Q2_CONTINUATION_EVIDENCE_CONTRACT_V0_1.md`
- `DISTRIBUTED_AUTHORITY_CONTINUATION_CONTRACT_V0_1.md`
- `SIMULATION_TIMEBASE_CONTINUATION_CONTRACT_V0_1.md`

## Key decisions

1. `symtropy-world` remains an orchestrator, not a mega-authority.
2. World continuation uses a hierarchical content-addressed manifest root.
3. Physical-state, continuation, lineage, snapshot-artifact, and runtime/cache identities remain distinct.
4. Same-world continuation preserves immutable world lineage identity; forks mint a distinct lineage identity while retaining ancestry.
5. Inactive-time behavior is explicit and simulation-time based; wall-clock time is never authoritative gameplay time.
6. Hidden continuation state such as Hydrology/Thermal active frontiers, ActiveLava `next_step`, nested eruption continuation, and landscape integration residuals must be preserved before Q2 can pass.
7. Derived indexes/caches may be omitted only with deterministic rebuild/equivalence evidence.
8. Representation residency semantics must survive suspend/resume or be provably reconstructed.
9. Snapshot bytes are content-addressed independently from semantic state identities.
10. A causal journal head becomes canonical world-continuation evidence only under an explicitly canonical serializer-independent event identity with valid causal-parent/schema semantics.
11. Tick-based gameplay time must map through an explicitly identified timebase to the shared `SimInstant`; a bare tick is not globally meaningful.
12. Distributed/server-migrated authority must bind an owner generation/epoch and the exact continuation state accepted at handoff; offline worlds omit this protocol state.
13. Q2 requires machine-checkable evidence tied to the exact promoted Git tree.
14. Q3 native CUF adapters remain blocked until Q2 produces a qualified parent.

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
  + #95 tick <-> SimInstant timebase identity
  + #94 distributed authority ownership generation (networked profile only)
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
        |
        v
Planetary / interplanetary / stellar scale / Q5
```

Issue #91 is the sequencing umbrella for this program.

## Public CI status

The dependency-light CUF GitHub Actions job associated with PR #74 remains queued at the time of this status record. GitHub reports `runner_id: 0`, an empty runner name, and no executed steps, so the queue is infrastructure availability rather than evidence of a test failure.

Public Tier A CI remains supplementary to the local/private full Q0-Q3 evidence gates.

## Timebase audit result

`symtropy-game-state::SimulationClock` and `symtropy-sim-contracts::SimInstant` are currently separate useful primitives with no explicit shared conversion contract. The game-state crate does not currently depend on sim-contracts.

The new timebase contract therefore requires:

- an identified fixed-step timebase;
- exact checked mapping from `(timebase, tick)` to `SimInstant`;
- exact reverse conversion only for aligned instants;
- no silent rounding or implicit wall-clock conversion;
- explicit migration if a world's numerical step changes.

The current `SimulationClock::from_hz` integer-nanosecond divisibility rule remains a useful explicit invariant rather than something to weaken through hidden truncation.

## Distributed authority audit result

Current `symtropy-net-core::SpatialAuthority` is runtime routing state over body handles and peer IDs. It currently has no canonical epoch/lease/handoff continuation identity.

For networked/migratable profiles, the new companion contract requires:

- stable world/authority/scope identity;
- current owner identity;
- monotonically ordered authority epoch/generation;
- effective simulation instant;
- exact continuation digest accepted at handoff;
- parent handoff/policy identity;
- stale epoch/replayed claim/split-brain rejection.

Offline/single-player manifests do not include this requirement.

## Implementation start conditions

The manifest Rust implementation may be prototyped after the docs contract is accepted, but it must not be promoted as Q2-complete until:

- exact Universal Matter v4.8 Q0/Q1 replay result is preserved;
- continuation-significant authority/state repairs are present;
- required snapshot/restore APIs expose complete identities;
- timebase mapping is explicit for tick-based scopes;
- networked profiles have accepted ownership-generation semantics before authoritative resume;
- the Q2 evidence runner can execute the hidden-state and suspend/resume fixtures.

## Preferred implementation placement

The portable canonical identity primitives should live in a dependency-light layer where practical.

`symtropy-world` may own orchestration/assembly/validation of the hierarchical manifest, but domain-specific snapshot creation and authority continuation identity remain owned by their domains.

`symtropy-game-state` may provide canonical causal journal identity after #82, but the world manifest should consume that identity rather than hashing generic event payload serialization itself.

A portable timebase identity/bridge should live close to the shared simulation contracts or in another dependency-light core rather than being reimplemented differently by each domain.

Network ownership/handoff semantics should derive runtime routing state rather than persist unstable `BodyHandle`/`HashMap` state as world truth.

## Non-goals

This status does not claim:

- Universal Matter v4.8 has been replayed locally;
- Q1 is green;
- Q2 repairs are implemented;
- the world manifest Rust types exist;
- timebase/network ownership contracts are implemented;
- native v0.11 adapters are implemented;
- Living Watershed is Q4-qualified;
- planetary/stellar Q5 scale has been demonstrated.

## Next decisive work

The next real executable milestone remains the guarded local Universal Matter v4.8 replay and Q0/Q1 evidence capture.

After that result is preserved, the Q2 hardening sequence should be implemented against the exact replay lineage rather than modifying the retained historical artifact.
