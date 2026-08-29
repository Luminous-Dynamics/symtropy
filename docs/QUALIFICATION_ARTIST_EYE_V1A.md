# Qualification Contract — ARTIST-EYE-v1A

This document is normative for the ARTIST-EYE-v1A tranche.

## Expected patch-source parent

```text
Symtropy parent:
e8794485c905e358d5b014048b8f5adae0c032b5
```

The public patch-source branch and any path-relocated outer integration history are distinct Git histories. Record the final live HEAD/TREE after qualification; do not substitute this branch's construction identifiers for the integrated receipt.

## Rust gates

Run under the repository's normal Nix dev shell:

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --all-targets
cargo test -p symthaea-bevy-brain
cargo clippy -p symthaea-bevy-brain --all-targets -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render --all-targets
cargo test -p symthaea-bevy-brain --features realtime-art-render
cargo clippy -p symthaea-bevy-brain --features realtime-art-render --all-targets -- -D warnings
```

## Required semantic regressions

At minimum the unit suite must demonstrate:

- a uniform field yields no spurious silhouette component and no dominant focal separation;
- a centered foreground form is separated from a uniform border background;
- a resolved vertical boundary produces vertical edge-family evidence rather than horizontal dominance;
- left-right asymmetry produces reflection-mismatch evidence;
- identical bytes/configuration produce exactly identical evidence on the same qualified binary/environment;
- row padding is not interpreted as pixels;
- mismatched pyramid shapes cannot be compared as a candidate consequence;
- four-ghost ARTIST-EYE evidence requires exact candidate coverage and capture/hash alignment.

## Policy / scalarization fence

Inspect added ARTIST-EYE implementation lines for active mutation authority and aesthetic scalarization.

Forbidden concepts include an aggregate beauty, preference, reward, fitness, utility, or weighted aesthetic objective used to choose an artistic action.

A bounded scalar used internally to rank *where to report perceptual focal regions* is permitted only if its contributing evidence dimensions remain separately recorded and the scalar is not exported as artistic value or policy authority.

## Live VART-EYE-001 gate

Do not claim live spatial-vision qualification from synthetic unit tests alone.

Execute the preregistered scene family in `docs/ARTIST_EYE_V1A.md` using real GPU readbacks produced by the `realtime-art-render` path. Record:

- qualified HEAD/TREE;
- Rust/Cargo versions;
- GPU/driver identity;
- render backend and feature set;
- scene seed and exact revision/frame/camera/fidelity;
- capture receipts and rendered semantic hashes;
- ARTIST-EYE configuration;
- evidence for every pyramid level;
- preregistered expected direction and observed result for each intervention;
- any capture/readback failure or exclusion.

A missing required readback invalidates the confirmatory episode. It must not silently reduce the scene family.

## Live four-ghost integration gate

After VART-EYE-001 passes, execute VART-EYE-002 with one abstention baseline and exactly three proposal ghosts. Require:

1. all four GPU readbacks;
2. a valid `FourGhostRenderSet`;
3. four aligned `ArtistEyeObservation`s;
4. a valid `FourGhostArtistEyeEvidenceSet`;
5. proposal-minus-baseline evidence at every retained scale;
6. no ARTIST-EYE field directly selecting or committing a proposal;
7. the existing four-ghost commit/abstention hash-equality fence still passing.

## Claim language

Before the live gate, preferred language is:

> ARTIST-EYE-v1A deterministic spatial evidence implementation compiled/tested/qualified.

After VART-EYE-001 and VART-EYE-002 pass, preferred language is:

> ARTIST-EYE-v1A is live-GPU qualified for deterministic multi-scale spatial evidence and four-ghost consequence attribution.

Neither claim establishes aesthetic quality, human-level vision, semantic object understanding, or subjective experience.
