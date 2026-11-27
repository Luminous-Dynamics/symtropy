# Spacecraft translation geometry audit v0.1

A dependency-free Rust evaluator for independent source/translated geometry annotations. It tests whether appearance translation preserves the labels needed by downstream spacecraft pose estimation and calibration.

It composes with #7785, #7786, #7800/#7801, #7723, #7715 and #7782. It does not duplicate those tools.

## What is audited

The tool consumes normalized TSV records describing independent annotations on each source and translated frame. Every frame binds:
- target/session/sequence/unit identity and fixed frame index / budget;
- translator build digest;
- source and translated image SHA-256;
- source and translated independent annotation artifact SHA-256;
- image width/height and coordinate convention;
- source and translated 2D landmark coordinates in integer milli-pixels;
- visibility (VISIBLE, OCCLUDED, OUT_OF_FRAME);
- crop state (FULL, CROPPED, TRUNCATED);
- source and translated binary masks for SILHOUETTE and declared semantic parts.

This v0.1 audit accepts CALIBRATION rows only. Translation of test rows into a calibration corpus is not allowed.

## Numeric checks

### Landmarks
For each visible landmark, Euclidean source-to-translated displacement is computed using integer squared distances and an outward-rounded integer square root. Maximum and p95 displacement are reported per frame. A displacement greater than the declared tolerance blocks that frame.

Visibility and crop state must match exactly. A change blocks the frame regardless of keypoint average error. Visible coordinates must remain inside the declared image dimensions.

### Masks
Masks use a packed hexadecimal representation: each nibble encodes four row-major pixels, with pixel 0 in bit 0 of the first nibble. Image dimensions and exact mask length are checked. The evaluator computes integer IoU in basis points, rounded down, and four-connected component counts in source and translated masks.

Every declared mask must meet its individual IoU floor (which cannot be lower than the scope floor) and preserve component count. A high aggregate IoU cannot hide a missing or disconnected part.

### Temporal geometry
For each landmark and consecutive frame pair, the auditor compares the source motion vector with the translated motion vector and computes the Euclidean residual. This catches jitter that remains within the static per-frame displacement tolerance but changes temporal geometry.

A unit is blocked if any frame fails or the maximum temporal motion residual exceeds the declared threshold. For a multi-frame unit, absence of any visible landmark trajectory blocks the unit rather than silently skipping temporal validation.

## Input schema

The TSV is normalized: one LANDMARK row per keypoint and one MASK row per semantic mask per frame. Metadata must be identical across all records for one sample ID. LANDMARK rows use NONE in mask fields; MASK rows use NONE in keypoint/visibility/crop fields.

Every unit must contain exactly its declared frame budget and frame indices 0..budget-1. Every frame within a unit must contain the same landmark ID set and mask region ID set. Sample IDs and record IDs must be unique.

Mask dimensions are bounded to 16,384 pixels in v0.1 to keep the pure-Rust implementation predictable. Larger images should use a separately versioned tiled or run-length representation, not silently bypass the cap.

## Frozen benchmark

Seven synthetic manifests cover:
1. Positive photometric-only translation: exact landmark trajectories, visibility, crops and semantic masks pass.
2. Keypoint shift: one translated keypoint exceeds the static tolerance.
3. Visibility change: source-visible landmark becomes occluded.
4. Crop change: a landmark changes from full-frame to truncated.
5. Mask IoU failure: a semantic region disappears.
6. Topology change: a disconnected stray component is introduced while aggregate IoU remains above the declared 99% threshold.
7. Temporal jitter: every frame remains within the static keypoint threshold, but the source/translated motion-vector residual exceeds the sequence tolerance.

An incomplete-sequence manifest is a malformed negative control and must be rejected by both implementations.

## Reproduce

    cargo fmt --manifest-path tools/spacecraft-translation-geometry-audit-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/spacecraft-translation-geometry-audit-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/spacecraft-translation-geometry-audit-bench/Cargo.toml --all-targets -- -D warnings

    MANIFEST=tools/spacecraft-translation-geometry-audit-bench/fixtures/positive-photometric-only-v0_1.tsv
    DIGEST=$(sha256sum "$MANIFEST" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/spacecraft-translation-geometry-audit-bench/Cargo.toml -- "$MANIFEST" "$GIT_COMMIT" "$DIGEST"
    python3 tools/spacecraft-translation-geometry-audit-bench/tests/reference_oracle.py "$MANIFEST" "$GIT_COMMIT" "$DIGEST"

The independent Python implementation recomputes geometry metrics and verifies raw input bytes with SHA-256. The hosted workflow compares complete JSON receipts.

## Research context

The 2026 sequence-conditioned ConvLSTM + conditional-GAN spacecraft translation paper reports improved SSIM/PSNR/FID and downstream HIL pose error. Its focus on temporal consistency motivates our sequence motion-residual test, but image/perception metrics do not themselves establish label preservation:
https://doi.org/10.1016/j.ast.2026.112871

The Nov 2026 conformal calibration study relies on translated synthetic images retaining valid geometry labels:
https://arxiv.org/abs/2611.26065

The Nov 2026 UE-pose event/RGB fusion paper uses per-keypoint evidential uncertainty and rigid-body topology validation, an important complementary pose-estimation baseline:
https://www.sciencedirect.com/science/article/pii/S0273117726012202

## Claim ceiling

A PASS establishes only that supplied independent annotations satisfy the declared numeric geometry contract. The tool does not inspect image pixels, authenticate images/annotations, validate annotation independence, establish physical truth, prove the translator preserves geometry outside the supplied cases, establish exchangeability, guarantee conformal coverage or qualify a flight system. Those upstream evidence properties must be separately verified. No result authorizes a spacecraft maneuver.
