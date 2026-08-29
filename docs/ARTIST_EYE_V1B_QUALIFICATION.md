# ARTIST-EYE-v1B Qualification Contract

This contract closes the depth/occlusion tranche without conflating a typed depth-analysis substrate with proof that the live Bevy host already exposes a trustworthy linear-depth plane.

## Required code gates

Run under the project Nix development shell:

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --all-targets
cargo test -p symthaea-bevy-brain
cargo clippy -p symthaea-bevy-brain --all-targets -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render --all-targets
cargo test -p symthaea-bevy-brain --features realtime-art-render
cargo clippy -p symthaea-bevy-brain --features realtime-art-render --all-targets -- -D warnings
```

The final receipt must record exact HEAD/TREE, rustc/cargo, Nix/devShell identity, target, GPU/driver/backend for live runs, and enabled feature set.

## Unit-level semantic gates

At minimum demonstrate:

- constant linear-depth plane -> zero occlusion-boundary fraction;
- near/far split -> detected depth discontinuity in the correct neighbor orientation;
- reverse-linear encoding maps known endpoints to known metric depths;
- a depth observation is rejected when the capture did not declare `ArtRenderChannel::Depth`;
- invalid/non-finite encoding or configuration is rejected;
- row padding is not interpreted as depth samples;
- missing/invalid samples reduce `valid_fraction` rather than silently becoming zero distance.

## Host depth acquisition gate

The live host depth path is a separate qualification item.

Before calling the host adapter qualified, save a receipt containing:

- exact camera identity and projection type;
- near/far parameters where applicable;
- whether reverse-Z is used;
- source texture/buffer format;
- source render pass identity;
- linearization method;
- a deterministic test scene with analytically known depths;
- observed error against those known depths;
- evidence that the depth pass is bound to the same revision/frame/scene/camera plane as its color capture.

Do not pass perspective hardware depth directly as `Linear01` unless the host first reconstructs linear distance.

## VART-DEPTH-001 decision surface

Freeze numerical tolerances before confirmatory scene outcomes are inspected. Use independent scene seeds as independent experimental units.

Decision categories:

- `Confirmed`: all preregistered direction and tolerance gates pass;
- `NotConfirmed`: a preregistered effect is reproducibly in a materially wrong direction or beyond tolerance;
- `Inconclusive`: evidence is insufficient for either claim;
- `InvalidRun`: provenance, alignment, missingness, or preregistration integrity failed.

Do not convert `Inconclusive` into `NotConfirmed` merely because a confirmation threshold was missed.

## VART-DEPTH-002 four-ghost gate

A confirmatory four-ghost depth episode requires:

1. one validated `FourGhostRenderSet`;
2. exactly four depth capture receipts;
3. exactly four `ArtistDepthObservation` values;
4. exact candidate coverage;
5. revision/frame/semantic-scene-hash/camera/resolution alignment for every depth pass;
6. no capture or readback loss;
7. baseline with no consequence delta;
8. three proposals with candidate-minus-baseline depth consequences;
9. existing color-path preview/commit hash equality unchanged;
10. no depth-derived scalar beauty, reward, utility, fitness, or mutation authority.

## Interpretation boundary

Passing v1B establishes that Symthaea can receive trustworthy descriptive evidence about spatial layering and occlusion. It does not establish that she understands artistic depth, prefers particular depth structures, or experiences them phenomenologically.
