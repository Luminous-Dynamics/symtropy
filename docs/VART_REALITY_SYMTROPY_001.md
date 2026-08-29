# VART-REALITY-SYMTROPY-001 — Transactional world provenance separation

Status: preregistered; execution unauthorized until the compile-corrected integrated HEAD/TREE is frozen.

## Question

Can one live four-ghost artistic episode be mapped into the Reality Ledger such
that the committed baseline and all three proposal worlds retain distinct,
correct provenance through presence, observation, selection/materialization,
context exit and recall?

## Frozen study shape

Use one committed scene and exactly three proposal ghosts from a valid
`FourGhostRenderSet`.

Before the first empirical capture, open one `WorldPresenceSession` for Symthaea
in the committed Symtropy studio. Freeze:

- committed world ID and lineage;
- agent and embodiment IDs;
- sensor-suite digest;
- action-surface digest;
- entry scene-state digest and entry `StudioFrame`;
- capability surface exactly `Observe`, `Enter`, `Fork`, `Propose` for this study.

The study presence session must not request `Mutate`, `Persist`, `SpawnAgent`,
`ChangePhysics` or `Delete`, and the adapter must not fabricate an authority
receipt.

For the committed baseline, freeze an `ObjectDepthCapturePlan` before either GPU
pass runs. The resulting Reality Ledger observation must atomically require both
`ObjectId` and `Depth` planes with the same world, lineage, revision, frame,
scene-state digest, camera and fidelity identity.

Required observations and gates:

1. baseline maps to exactly one `DigitalCommitted` world;
2. each proposal maps to a unique `Counterfactual` child whose parent is the
   committed world;
3. all four candidate bundles retain the exact base revision, `StudioFrame`,
   camera and fidelity supplied by the four-ghost contract;
4. each candidate's typed state digest equals the semantic scene hash actually
   rendered for that candidate;
5. GPU artifact digest is present before Reality Ledger admission;
6. the committed object-ID/depth pair is admitted only as a complete
   two-plane transactional bundle; deleting or substituting either plane fails
   closed rather than degrading silently;
7. selected-proposal materialization requires an external authority receipt and
   exact typed source-state == committed-after-state;
8. non-selected proposal worlds remain `Counterfactual` after the committed
   world changes;
9. the presence session closes with an explicit exit state digest and exit
   frame, never a partial exit;
10. post-session memory admission for proposal observations remains
    `HypotheticalOnly` and may not claim the proposal happened in the committed
    parent world.

## Negative controls

Inject prospectively defined failures:

- wrong lineage ID;
- wrong revision;
- wrong frame;
- wrong rendered scene-state digest;
- cross-camera receipt;
- cross-fidelity receipt;
- empty fidelity identity for the paired object/depth observation;
- missing object-ID artifact digest;
- missing depth artifact digest;
- object/depth receipts from different prospective pair plans;
- equal digest bytes under a different semantic digest domain;
- counterfactual descriptor with wrong parent;
- presence session bound to a different committed world;
- presence exit before entry;
- missing exit state while an exit frame is present, or vice versa;
- missing authority receipt on selected materialization.

Every negative control must fail closed.

## Evidence to retain

- exact Symtropy HEAD/TREE;
- exact Symthaea Reality Ledger HEAD/TREE;
- Cargo.lock and Nix lock identity;
- rustc/cargo versions;
- `WorldPresenceSession` entry/exit receipt with sensor/action surface digests;
- prospective `ObjectDepthCapturePlan`;
- object-ID and depth GPU capture receipts and the combined
  `WorldObservationBundle`;
- four-ghost plan/render/decision/closure receipts;
- world descriptors for all four candidates;
- typed observation bundles;
- resulting Reality Ledger record chain and checkpoint head if enabled;
- typed materialization receipt for a selected trial;
- memory-admission receipts after the session.

## Qualification before execution

The empirical study remains unauthorized until the integrated Symtropy branch
passes, under `nix develop`:

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo test -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo clippy -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter --lib --tests
cargo test -p symthaea-bevy-brain --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter --lib --tests
cargo clippy -p symthaea-bevy-brain --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter --lib --tests -- -D warnings
```

Freeze the resulting HEAD/TREE and toolchain identities before any empirical
capture. If the source, lockfiles, toolchain or relevant build flags change,
start a new evidence lineage rather than appending to the old one.

## Interpretation boundary

A PASS establishes provenance preservation across a live simulated-world
counterfactual loop with explicit digital presence and transactionally aligned
object/depth perception. It does not establish subjective presence, aesthetic
quality, physical grounding, or unrestricted world mutation authority.
