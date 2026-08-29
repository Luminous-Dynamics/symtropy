# VART-INHABIT-001 — explicit digital-world presence and provenance continuity

Status: preregistered; execution unauthorized until the final integrated HEAD/TREE passes the mechanical gates below.

## Question

Can Symthaea enter one committed Symtropy world under an explicit
`WorldPresenceSession`, observe it through provenance-bound evidence, register
three counterfactual sibling worlds, leave the world, and retain a verified
history in which counterfactual events never become committed history?

## Frozen episode shape

Use one `InhabitedWorldEpisode` rooted in one `DigitalCommitted` Symtropy world.
Before the first empirical observation freeze:

- episode ID;
- committed world ID and lineage;
- `WorldGenesisManifest` and genesis digest;
- simulation-kernel digest;
- physics-profile digest;
- asset-manifest digest;
- determinism class and seed policy;
- timebase identity;
- agent and embodiment IDs;
- sensor-suite digest;
- action-surface digest;
- initial scene-state digest;
- entry `StudioFrame`;
- capability surface exactly `Observe`, `Enter`, `Fork`, `Propose`.

The genesis initial-state digest must equal the presence entry-state digest as a
typed value, including domain and algorithm.

## Required episode

1. Open one inhabited-world episode.
2. Verify the episode ledger contains genesis and presence-entry records with a
   valid append-only chain.
3. Capture one committed observation. For the ARTIST-EYE-v1E trial, require a
   prospectively paired object-ID + metric-depth bundle.
4. Register exactly three four-ghost proposal worlds in the episode `WorldGraph`.
5. Admit at least one observation from each proposal world.
6. Verify every admitted observation belongs to a world already present in the
   graph.
7. Select or abstain using the existing four-ghost decision boundary. Selection
   must not itself grant mutation authority.
8. If a proposal is materialized, require the existing external-authority and
   typed source-state == committed-after-state gates.
9. Close the presence interval with both exit-state digest and exit frame.
10. Verify the final `RealityLedger` and `WorldGraph`.
11. Perform post-episode memory admission and verify proposal-world records
    remain `HypotheticalOnly`.

## Negative controls

Every negative control must fail closed:

- empty episode identity;
- root world not `DigitalCommitted`;
- genesis state and entry state differ by value;
- equal-looking state bytes under a different digest domain;
- seeded-stochastic genesis without a seed;
- duplicate/cyclic/missing-parent world graph insertion;
- attempt to admit an observation for an unregistered world;
- object-ID/depth planes from different revision, frame, scene state, camera or
  fidelity;
- missing required object-ID or depth plane;
- same bundle receipts reordered must produce the same canonical episode
  observation digest;
- changing any artifact digest must change the episode observation digest;
- caller-supplied stale sequence/previous-head state must be impossible because
  episode append derives them from the live ledger;
- presence exit before entry;
- partial presence exit;
- direct Dream -> committed mutation;
- selected proposal without external authority evidence.

## Mechanical qualification before execution

Run under the project `nix develop` shell and retain the exact toolchain and
lockfile identities:

```bash
cargo fmt --all -- --check

cargo check -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo test -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests
cargo clippy -p symthaea-bevy-brain --features reality-ledger-adapter --lib --tests -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter --lib --tests
cargo test -p symthaea-bevy-brain --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter --lib --tests
cargo clippy -p symthaea-bevy-brain --features realtime-art-render,realtime-art-object-id,reality-ledger-adapter --lib --tests -- -D warnings
```

If any compile, test, clippy, lockfile or source correction is required, freeze a
new HEAD/TREE and restart the empirical lineage from that corrected root.

## Evidence to retain

- Symtropy HEAD/TREE;
- pinned Symthaea Reality Ledger HEAD/TREE;
- Cargo.lock, flake.lock, rustc/cargo and architecture;
- genesis manifest + typed digest;
- open and closed presence receipts;
- final world graph;
- prospective object/depth capture plan;
- all GPU capture receipts;
- all transactional observation bundles;
- four-ghost plan/render/decision/closure receipts;
- any typed materialization receipt;
- complete Reality Ledger and verified final head;
- post-episode memory-admission receipts.

## Interpretation boundary

A PASS establishes a provenance-preserving digital-world episode with explicit
functional presence, persistent world ancestry, aligned perception and clean
counterfactual memory separation. It does not establish subjective experience,
physical grounding, aesthetic quality, general causal understanding, or
unrestricted world mutation authority.
