# Symthaea Art Studio v1 — Bevy Embodiment Plan

Status: proposal-first typed port implemented; scene mutation intentionally deferred.

## Why Bevy

Bevy should be Symthaea's **living/embodied studio**, not merely another renderer. Symtropy already carries the useful substrate: Bevy ECS, physics bridges, scene abstractions, Symthaea cognition, embodiment adapters, terrain, audio, and capture tooling. The art integration should therefore teach one persistent artistic system to perceive and act through a simulated body/tool environment.

The first implementation is deliberately conservative: an entity may carry both `CognitiveBrain` and `ArtPort`, but `ArtPort` does not mutate the Bevy world. It receives an exact-revision art perception frame and stores semantic proposals until a separate host adapter explicitly previews or applies them.

```rust,ignore
commands.spawn((
    CognitiveBrain::new(128, "atelier-study-01"),
    ArtPort::new(ArtAuthorityMode::Propose),
));
```

## Contract

The local compatibility vocabulary mirrors `symthaea.art-world.v1` from the full Symthaea workspace:

- `ArtPerceptionFrame`: read-only scene/artifact evidence;
- `ArtOperation`: host-neutral semantic intervention;
- `ArtActionProposal`: revision-bound proposed action;
- `ArtAuthorityMode`: Observe / Propose / Author;
- `ArtPortEvent`: append-only local proposal/decision trace;
- first-class `Abstain`.

The standalone Symtropy repository intentionally keeps a small mirror of the semantic contract because its public `symthaea` dependency is a stub. In the private/full workspace, the next integration step should replace this mirror with the real `symthaea-art-world` dependency and add a schema-conformance test so drift is impossible.

## Invariants

1. **Observation is not authority.** Merely attaching a `CognitiveBrain` or providing a render never grants mutation rights.
2. **Proposal is not mutation.** An `ArtActionProposal` remains inert until a host adapter acts on it.
3. **Every proposal is revision-bound.** If the art world advances from R1 to R2, an R1 proposal becomes stale rather than silently applying to a changed scene.
4. **Abstention is valid.** The agent can explicitly choose to preserve the current work.
5. **Accepted is not applied.** Acceptance records a decision; only the host adapter can mark application after a real scene mutation succeeds.
6. **No scalar art score is introduced by this bridge.** Artistic cognition remains upstream.

## Next Bevy patch tranche

### B1 — deterministic art-world snapshot

Create an adapter that extracts a stable semantic snapshot from explicitly tagged artistic entities. Include stable IDs, transforms, parentage, material identifiers, camera/light state, selected entities, and a deterministic revision hash.

Do **not** hash transient ECS entity numbers alone. Persistent artistic identity needs stable host IDs.

### B2 — render observation

Capture a bounded-resolution camera render associated with the same revision as the semantic snapshot. Semantic and pixel observations must share one revision receipt so Vision Manifold cannot accidentally reason over mismatched states.

### B3 — proposal ghosts

Translate proposals into temporary preview entities/components in a dedicated preview layer/world. Preview state must be disposable and must not advance the committed art revision.

### B4 — explicit commit adapter

Only accepted proposals may advance the committed scene in `Propose` mode. `Author` mode requires an explicit autonomous-workspace policy. Every successful mutation produces a new revision hash and an application receipt.

### B5 — whole-scene counterfactual vision

Render baseline + proposal branches and feed each through the same visual-perception contract. Preserve consequence vectors rather than selecting by one beauty score.

### B6 — embodied tools

Introduce actual simulated artistic tools and media: brush, palette knife, charcoal, clay/sculpting tool, light/camera rig, procedural materials. The artistic system should learn intention -> motor trajectory -> material consequence rather than relying only on perfect API operations.

## Research comparison

Eventually preregister three matched conditions:

- API artist: direct Blender-style semantic operations;
- embodied artist: Bevy physical tools only;
- hybrid artist: learns techniques in Bevy and transfers concepts to high-level digital operations.

Primary questions should test consequence prediction, technique acquisition, revision quality, restraint, transfer between media, and persistence of artistic questions — not maximum acceptance or a universal aesthetic scalar.

## Qualification for current patch

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --all-targets
cargo test -p symthaea-bevy-brain
cargo clippy -p symthaea-bevy-brain --all-targets -- -D warnings
```

Specific acceptance checks:

- existing `CognitiveBrain` behavior remains unchanged when no `ArtPort` is attached;
- Observe mode cannot enqueue proposals;
- stale-revision proposals are rejected;
- accepting a proposal does not mutate the observed revision;
- `Abstain` survives the typed port unchanged;
- autonomous commit capability is explicit and only true in Author mode.
