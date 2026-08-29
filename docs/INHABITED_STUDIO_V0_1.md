# INHABITED-STUDIO-v0.1 — provenance-preserving world episodes

Status: architecture contract for the next integrated tranche.

## Purpose

`INHABITED-STUDIO-v0.1` turns the already-integrated Reality Ledger +
ARTIST-EYE-v1E substrate into one explicit world episode:

1. one reproducible `WorldGenesisManifest` for the committed Symtropy world;
2. one authority-poor `WorldPresenceSession` for Symthaea;
3. one persistent `WorldGraph` containing the committed world and any registered
   counterfactual children;
4. one append-only `RealityLedger` whose sequence and previous-head digest are
   assigned by the episode runtime rather than by callers;
5. transactionally aligned color/depth/object-ID or other observation bundles;
6. an explicit exit state and final verified ledger head.

The episode runtime is orchestration and provenance. It does not grant scene
mutation authority, decide artistic value, or imply subjective presence.

## Frozen distinctions

The following remain distinct:

- world existence != agent presence;
- presence != subjective experience;
- observation != derived inference;
- counterfactual creation != committed materialization;
- creator provenance != mutation authority;
- object/depth evidence != aesthetic judgment;
- ledger membership != physical truth.

## Episode lifecycle

```text
WorldGenesisManifest
        |
        v
DigitalCommitted world ----> WorldGraph
        |
        v
WorldPresenceSession
  Observe / Enter / Fork / Propose only
        |
        +----> transactional observations ----> RealityLedger
        |
        +----> three four-ghost children -----> WorldGraph
        |
        v
explicit exit state/frame
        |
        v
verified final ledger head
```

## Fail-closed invariants

- Genesis initial-state digest and presence entry-state digest must be the same
  typed state.
- The committed world must be `DigitalCommitted`.
- An observation bundle is rejected if its world/lineage is absent from the
  episode `WorldGraph`.
- Ledger sequence and previous-record digest are generated from the current
  ledger state, never supplied as free caller metadata.
- Multi-plane observations remain one bundle; the episode commits one digest
  over the complete aligned bundle, not an arbitrary first plane.
- Counterfactual children must retain their parent identity and cannot be
  silently relabeled as committed worlds.
- Exit state and exit frame are written together.
- No authority-bearing presence capability is minted by this runtime.

## Next empirical study

After local Nix qualification, execute `VART-INHABIT-001` before adding deeper
nested worlds or autonomous world mutation.
