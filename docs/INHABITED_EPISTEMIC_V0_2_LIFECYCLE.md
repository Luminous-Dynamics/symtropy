# Inhabited Epistemic Worlds v0.2 — Persistent World Lifecycle

Status: **implementation / fresh qualification required**.

v0.2 extends the bounded inhabited-world episode into a persistent world that
can be left, snapshotted, suspended, restored, revisited, forked, and archived
without collapsing world identity or epistemic provenance.

## Design target

The required continuity chain is:

```text
presence A closes at typed state S
        |
        v
persist exact host bytes -> BLAKE3 artifact digest A
        |
        v
WorldSnapshotManifest(S, A, ledger head, genesis, world)
        |
        v
Suspend (external authority)
        |
        v
host stops evolving the world
        |
        v
restore exact persisted artifact
        |
        v
independently recompute semantic state S'
        |
        +-- require typed S' == typed S
        |
        v
Resume (external authority)
        |
        v
presence B opens at S
        |
        v
WorldRevisitReceipt proves
presence-A exit == snapshot == presence-B entry
```

This is **digital world-state continuity**, not a claim of subjective continuity.

## Semantic state and persisted artifact are different evidence planes

The committed Symtropy semantic scene identity remains the frozen FNV-1a64
protocol identity in `symtropy.scene-state.v1`. It answers whether canonical
semantic scene records are the same under that protocol.

The persisted snapshot artifact is separately hashed with BLAKE3 under
`symtropy.world-snapshot-artifact.v1`. It answers which persisted bytes are
being restored.

Neither digest substitutes for the other.

## Ordered lifecycle state machine

Every snapshot receives an ordered `WorldLifecycleTimeline`:

```text
Active --Suspend--> Suspended --Resume--> Active
                           |
                           +--Archive--> Archived
```

Suspend, Resume and Archive all require externally supplied authority evidence.
An archived timeline cannot accept Resume. A suspended or archived world cannot
be reopened as a presence session.

## Snapshot succession

A later snapshot can link to the exact prior `WorldSnapshotManifest` digest.
The successor must retain the exact same world descriptor and may not regress
its host frame when both frames are known.

This separates:

- **same world, later snapshot** — same world/lineage descriptor with a chained
  previous-snapshot digest;
- **new world fork** — a new child world/lineage with explicit parent relation.

## Forks

Two fork classes are currently admitted:

- ephemeral `Counterfactual` + `CounterfactualOf` child: no persistence
  authority is required;
- persisted `DigitalCommitted` + `SpawnedFrom` child: an external persistence
  authority receipt is required.

Both begin from the exact typed source snapshot state, use new child
world/lineage identity, and retain an explicit child genesis digest.

A fork is not a snapshot successor. A snapshot successor continues one world;
a fork creates a distinct world.

## Authority boundary

The Reality Ledger and Symtropy adapter do not:

- mint lifecycle authority;
- serialize or deserialize a Bevy world;
- start or stop a runtime;
- claim restored GPU/physics state is correct merely because metadata matches;
- grant mutation, persistence, spawn, physics-change or delete authority to the
  inhabiting agent.

Actual host persistence/restoration must be independently qualified.

## Scientific boundary

A clean v0.2 qualification may support the narrow statement that a
provenance-identified digital world can be persisted and revisited with
verifiable state-lineage continuity.

It does not establish consciousness, uninterrupted subjective experience,
physical-world identity, perfect deterministic replay, metaphysical persistence,
or autonomous world-management authority.
