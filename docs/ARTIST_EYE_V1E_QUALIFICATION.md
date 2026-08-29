# ARTIST-EYE-v1E Qualification Contract

Status: prospective

This document defines the minimum evidence required before ARTIST-EYE-v1E may
be described as qualified.

## A. Mechanical Rust gates

Run all three feature surfaces because v1D's successful default-feature gates
do not establish the GPU paths.

```bash
cargo fmt --all -- --check

cargo check -p symthaea-bevy-brain --lib --tests
cargo test -p symthaea-bevy-brain --lib --tests
cargo clippy -p symthaea-bevy-brain --lib --tests -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render --lib --tests
cargo test -p symthaea-bevy-brain --features realtime-art-render --lib --tests
cargo clippy -p symthaea-bevy-brain --features realtime-art-render --lib --tests -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-object-id --lib --tests
cargo test -p symthaea-bevy-brain --features realtime-art-object-id --lib --tests
cargo clippy -p symthaea-bevy-brain --features realtime-art-object-id --lib --tests -- -D warnings
```

Any compile or clippy correction to the Bevy 0.19 adapter creates a new
qualification HEAD/TREE. Record that final lineage rather than quoting this
unqualified construction head.

## B. Regression semantics

At minimum verify:

1. `u32 -> RGBA8 -> u32` is exact across zero, byte boundaries and `u32::MAX`.
2. Object-ID decoder ignores padded row bytes.
3. Unknown non-zero object IDs remain fail-closed.
4. Object/depth fusion rejects revision/frame/scene/camera/resolution mismatch.
5. Object/depth fusion ignores row padding in both planes.
6. Per-object depth summaries never mix stable identities.
7. Authored hide/show is not upgraded to occlusion/reveal.
8. Semantic creation/destruction is not upgraded to occlusion/reveal.
9. Target/camera instability blocks the static depth-takeover mechanism.
10. A synthetic closer-object takeover can pass only under explicitly supplied
    regression thresholds.
11. Completed GPU evidence queues reject rather than silently evict evidence.
12. No perception type exports mutation authority or an aggregate artistic
    beauty/reward/utility/fitness value.

## C. VART-OBJ-GPU-001 live GPU gate

Freeze before execution:

- final HEAD/TREE;
- `Cargo.lock` and feature set;
- Rust/Cargo/Nix identity;
- GPU model, driver, WGPU backend and adapter information;
- output format (`Rgba8Unorm` for this adapter);
- render resolution;
- scene seed and deterministic scene hash;
- stable camera ID and projection;
- complete object-ID registry and digest;
- exact `ObjectIdRenderPlan`;
- static-repeat equality/tolerance policy;
- centroid/area tolerances for analytically simple geometry.

Run a deliberately simple scene with several stable objects, including overlap,
background and at least one object touching the image boundary.

A clean live unit requires:

- exactly the planned IDs plus background zero;
- zero unknown labels;
- zero readback drops;
- complete planned source coverage;
- correct registry digest;
- non-empty readback;
- decoded object areas/centroids within the prospectively frozen simple-scene
  tolerances;
- committed semantic scene hash unchanged before/after the proxy evidence pass;
- repeated static captures meet the frozen repeatability policy.

Adversarial cases must include registry mismatch, omitted proxy source, extra
unplanned source, and backpressure.

## D. Color / object / depth alignment gate

For one frame, obtain color, object-ID and metric-depth evidence using the same
qualified camera/revision/frame/scene/resolution plane.

Before fusion, independently verify both the object-ID and depth receipts. Then
require exact identity equality on:

```text
revision
studio frame
semantic scene hash
stable camera ID
width
height
```

Any disagreement invalidates the fusion unit. Timing proximity is not a
substitute for identity equality.

## E. VART-OCC-001 threshold freeze

Do not execute the confirmatory occlusion study while any required field in
`VART_OCC_001_THRESHOLDS.template.json` remains null or while
`execution_authorized` remains false.

Thresholds must be chosen without looking at confirmatory outcomes and bound to
the final qualified HEAD/TREE and acquisition profile.

## F. VART-OCC-001 mechanism study

Use independent runs from a preregistered scene family covering the controls in
`ARTIST_EYE_V1E.md`.

The primary result must report confusion counts/rates separately for:

- true front-object takeover;
- behind-object control;
- authored visibility changes;
- semantic destruction/creation;
- target self-motion;
- camera-motion protocol violation;
- insufficient overlap;
- insufficient metric depth margin.

Do not tune thresholds on those confirmatory results. If the frozen criteria are
not met, report NotConfirmed or Inconclusive according to the preregistered
rules; do not silently weaken the gate.

## G. Claim boundary

A v1E PASS authorizes the narrow statement:

> On the qualified Bevy acquisition path, persistent object-ID evidence can be
> fused with aligned metric depth, and the preregistered static target/camera
> depth-takeover mechanism met its frozen validation criteria.

It does **not** establish general physical causality, unrestricted object
permanence, arbitrary conceal/reveal reasoning, aesthetic competence,
subjective experience, or active mutation authority.
