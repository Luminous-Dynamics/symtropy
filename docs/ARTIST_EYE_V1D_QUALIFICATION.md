# ARTIST-EYE-v1D Qualification Contract

ARTIST-EYE-v1D is qualified only when both the mechanical Rust gates and the
identity/motion semantics below pass against one exact HEAD/TREE.

## Mechanical gates

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --all-targets
cargo test -p symthaea-bevy-brain
cargo clippy -p symthaea-bevy-brain --all-targets -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render --all-targets
cargo test -p symthaea-bevy-brain --features realtime-art-render
cargo clippy -p symthaea-bevy-brain --features realtime-art-render --all-targets -- -D warnings
```

## Required semantic regression gates

The following must pass:

1. object registry construction is deterministic under input-order changes;
2. stable IDs are unique and empty IDs are rejected;
3. object-ID row padding is ignored;
4. unknown non-zero raster IDs fail closed;
5. object absence in a raster is not semantic destruction;
6. semantic scene hashes are recomputed and forged hashes are rejected;
7. semantic and raster frames must match revision/frame/scene identity;
8. registry digest changes invalidate a confirmatory transition;
9. camera identity may not change inside a transition/window;
10. frames strictly increase and respect the frozen maximum frame gap;
11. semantic-transform delta and screen-centroid delta remain separate;
12. camera motion and semantic-transform motion remain separate;
13. raster visibility loss/gain is not labeled concealment/reveal;
14. object-window aggregation retains separate event/motion channels;
15. no beauty, utility, reward, fitness, preference or mutation-authority scalar is introduced.

## VART-OBJ-001 live object-ID gate

A live GPU object-ID adapter is not implicitly qualified by the renderer-neutral
analyzer. Before reporting live object tracking, freeze and record:

- object-ID render target format;
- exact stable-ID -> raster-ID registry/digest;
- scene seed and expected object set;
- camera stable ID and projection;
- resolution and row-stride decoding;
- exact revision/frame/scene hash for every capture;
- GPU / driver / backend;
- capture/readback queue capacities and zero-drop requirement;
- expected object pixel ownership for an analytically simple scene.

The first live scene should contain non-overlapping known shapes whose expected
pixel ownership can be checked exactly or within a prospectively frozen raster
edge tolerance. Then add a controlled overlap/occlusion condition.

Any missing capture, unknown raster ID, registry mismatch or ambiguous decode
invalidates the confirmatory unit.

## VART-MOT-001 motion discrimination gate

Freeze all epsilons in `MotionAttributionConfig` before outcomes exist.
Prospectively exercise:

- static/static;
- semantic-transform-only change;
- camera-only change;
- mixed camera + semantic-transform change;
- semantic change with negligible screen-centroid change;
- unexplained screen-centroid change;
- visibility loss/gain.

The observed `ObjectMotionAttribution` must match the preregistered qualitative
category in each condition.

## Transform-space boundary

`ArtSceneRecord` currently serializes the host-provided `Transform`; it is not
proven to be a global transform for parented objects. Therefore qualification
must report `SemanticTransformDelta` as semantic-transform-space evidence.
Do not relabel it global/world displacement without a separate host contract
using qualified `GlobalTransform` or equivalent world-space records.

## Claims allowed after PASS

A clean v1D PASS supports:

- persistent stable identity across synchronized semantic/raster frames;
- per-object raster visibility, centroid, bounding box and visible fraction;
- separate semantic-transform, camera and screen-trajectory evidence;
- conservative motion attribution categories;
- bounded persistent track summaries;
- fail-closed registry/hash/frame/camera integrity.

It does not establish causal occluder identity, physical rigid-body motion,
qualified GPU motion vectors, long-gap object permanence, aesthetic quality,
subjective experience, or active-policy authority.
