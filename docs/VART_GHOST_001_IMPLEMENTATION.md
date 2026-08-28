# VART-GHOST-001 — Bevy Live Host Integration

Status: contract + deterministic visual observer + fail-closed session implemented; live host smoke remains to qualify.

## Runtime shape

```text
committed Bevy scene R / frame F / camera C
          |
          +--> baseline capture (do nothing)
          |
          +--> isolated preview A -> capture A
          +--> isolated preview B -> capture B
          +--> isolated preview C -> capture C
          |
          v
exact four-capture barrier
          |
          v
pixel consequence observation
          |
          v
Select / Abstain / Revise / Inconclusive
          |
          v
separate art-world authority gate
          |
          v
optional real commit + semantic hash check
          |
          v
dispose all preview branches
```

## Important hash distinction

All four candidates are causally bound to the same committed **base scene hash**.

The proposal previews should normally have different **rendered semantic scene hashes** because their scenes have actually changed. A proposal capture must not claim that its pixels came from the unchanged committed scene.

The selected deterministic real commit must reproduce the selected preview's semantic scene hash. Abstain/Revise/Inconclusive must leave the committed hash equal to the original base hash.

## Current modules

- `art_visual`: inexpensive deterministic whole-frame perception channels;
- `art_ghost_loop`: exactly-one-baseline + exactly-three-proposal evidence, consequence vectors, decision and causal closure receipts;
- `art_ghost_session`: fail-closed Planned -> Rendering -> Perceived -> Decided -> Closed lifecycle;
- `art_offscreen`: optional real GPU render/readback substrate behind `realtime-art-render`;
- `art_preview_scene`: isolated canonical proposal-scene mutation substrate.

None of these grant scene-mutation authority.

## First live scene

Keep the first live qualification intentionally simple:

- one tagged form;
- one tagged light;
- one tagged camera;
- deterministic semantic revision/hash;
- cognitive-resolution color captures.

Use three distinct bounded proposals, for example:

1. translate the form;
2. move/intensify the light;
3. alter camera/form composition.

The goal is to prove causal traceability, not artistic sophistication.

## Hard gates

A run is not confirmatory unless:

- all four expected captures complete with non-empty bytes;
- no capture or completed-readback eviction occurs;
- revision, frame, stable camera, base scene and fidelity bindings validate;
- baseline rendered scene hash equals the committed base hash;
- each proposal capture is bound to its own rendered preview-scene hash;
- all four pixel observations complete;
- each proposal receives a separate candidate-minus-baseline consequence vector;
- no aggregate beauty/utility/reward score is introduced;
- a decision is emitted only after the four-capture barrier;
- no preview changes committed scene state before authority;
- selection does not itself grant commit authority;
- a selected authorized commit reproduces the selected preview semantic hash;
- non-selection preserves the base semantic hash;
- all three preview branches are disposed before closure.

## Qualification

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --all-targets
cargo test -p symthaea-bevy-brain
cargo clippy -p symthaea-bevy-brain --all-targets -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render --all-targets
cargo test -p symthaea-bevy-brain --features realtime-art-render
cargo clippy -p symthaea-bevy-brain --features realtime-art-render --all-targets -- -D warnings
```

Then execute the live Bevy smoke described above. Unit tests alone do not establish that the GPU/render-world integration is functioning on the target machine.
