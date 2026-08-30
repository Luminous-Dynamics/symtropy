# VART-LIFECYCLE-001 — Persistent World Revisit Continuity

Status: **preregistered; execution is not authorized by this document**.

## Question

Can one committed Symtropy world be left, persisted, suspended, restored, and
re-entered while preserving exact typed state identity, world provenance,
ordered lifecycle state, and distinct presence-session identity?

## Narrow hypothesis

For a frozen world, host build and persistence format, restoring the exact
snapshot artifact will reproduce the preregistered semantic state identity and
permit a new presence session whose entry state equals the prior session exit
state through the exact snapshot manifest.

This is a digital-state continuity claim only.

## Frozen sequence

1. Complete one bounded passive inhabited episode in a `DigitalCommitted` world.
2. Close presence A at frame F and typed semantic state S.
3. Serialize the host world using the frozen persistence adapter.
4. Compute BLAKE3 over the exact persisted artifact bytes.
5. Create snapshot manifest Q binding world, genesis, S, ledger head, artifact
   digest and F.
6. Open a lifecycle timeline for Q.
7. Supply external test authority and append `Suspend`.
8. Stop host evolution for the committed world.
9. Destroy or unload the live host representation according to the frozen test
   procedure; retaining the live state as the restoration source is forbidden.
10. Restore from the exact artifact whose digest is bound into Q.
11. Independently recompute the canonical semantic scene state S'.
12. Require typed S' == typed S before any new presence opens.
13. Supply external test authority and append `Resume`.
14. Open distinct presence B on the restored world.
15. Construct `WorldRevisitReceipt`; require presence-A exit == Q state ==
    presence-B entry.
16. Verify the ordered lifecycle timeline.
17. Re-observe the frozen sentinel objects and compare their preregistered
    semantic identities and selected sensor evidence.
18. Close presence B and seal the evidence lineage.

## Primary gates

A confirmatory PASS requires all of the following:

- exact world + lineage descriptor retained;
- exact genesis digest retained;
- snapshot artifact digest matches bytes used for restoration;
- semantic state digest after restoration is typed-equal to snapshot state;
- no frame regression under the frozen timebase policy;
- Suspend occurs from Active;
- Resume occurs from Suspended;
- both lifecycle operations contain external authority receipts;
- presence A is closed before snapshot/revisit proof;
- presence B has a distinct session ID and is open at revisit proof time;
- agent identity is unchanged;
- presence-A exit state == snapshot state == presence-B entry state;
- lifecycle timeline replay verifies;
- no ghost/dream/replay memory is promoted to committed history by lifecycle
  restoration;
- all queue/drop/error counters named in the frozen sensor protocol remain in
  their preregistered acceptable range.

## Required negative controls

Run before confirmatory interpretation:

- alter one byte of the persisted artifact;
- substitute a snapshot from another world;
- substitute the same state value under a different digest domain;
- substitute the same state value under a different digest algorithm;
- attempt Resume before Suspend;
- attempt Revisit while Suspended;
- perform Suspend -> Archive -> Resume;
- attempt Revisit after Archive;
- reuse presence A's session ID for presence B;
- change agent identity;
- regress the frame/timebase coordinate;
- remove lifecycle authority receipt;
- change the committed world descriptor while retaining world/lineage IDs.

Every negative control must fail closed at the intended contract boundary.

## Exploratory fork sub-study

After the primary revisit test is frozen and interpreted, one snapshot may be
used for an exploratory fork check:

- create one ephemeral `CounterfactualOf` child from the exact snapshot state;
- confirm it has distinct world/lineage identity and no persistence authority;
- separately construct one prospective persisted `DigitalCommitted`
  `SpawnedFrom` child and confirm that omitting persist authority fails.

The fork sub-study is not part of the primary confirmatory PASS and must not be
used to rescue a failed revisit study.

## Evidence capsule

Freeze before execution:

- Symtropy HEAD/TREE;
- exact pinned `symthaea-reality-ledger` commit;
- `Cargo.lock`, `flake.lock`, `rust-toolchain.toml` hashes;
- rustc/cargo/Nix identity and build flags;
- Bevy version;
- persistence adapter/version and serialization schema;
- relevant asset manifest;
- world genesis digest;
- initial and exit semantic state contract;
- timebase policy;
- GPU/backend/driver when sensor evidence participates;
- persistence artifact path/name and final artifact digest;
- external test-authority identity/digest policy.

Any relevant source, lockfile, serializer, asset, timebase, semantic hash,
physics profile, or evidence-policy change starts a new evidence lineage.

## Interpretation boundary

PASS supports:

> A declared Symtropy digital world can be restored from a frozen persisted
> artifact and re-entered under a new explicit presence session while preserving
> the preregistered world/state provenance chain.

PASS does not support claims of subjective continuity, consciousness across
suspension, physical identity, perfect arbitrary-host restoration, or autonomous
persistence authority.
