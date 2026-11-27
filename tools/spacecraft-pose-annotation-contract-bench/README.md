# Spacecraft pose annotation contract v0.1

A small Rust-first geometric preflight for monocular spacecraft 6-DoF pose annotations. The first fixture lane is based on the published SDKP camera/annotation conventions and contains no downloaded image corpus.

## Supported contract

Each test record explicitly declares:
- sample ID and train/validation/test split;
- annotation release identity and presence of corrected 3D bounding-box annotation;
- camera-to-world pose semantics;
- quaternion component order (XYZW or WXYZ); the adapter never guesses this from values;
- Blender camera axes: +X right, +Y up, -Z forward;
- OpenCV projection axes: +X right, +Y down, +Z forward;
- metre units for camera translation and 3D keypoints;
- image dimensions, focal lengths and principal point;
- unit quaternion, camera position, one world/model-frame keypoint and observed pixel.

For camera-to-world quaternion q and camera position t, the point is transformed to the camera frame using q inverse and (p_world - t). Blender camera coordinates are converted to OpenCV by (x, y, z) -> (x, -y, -z), then projected with the declared pinhole intrinsics.

The validator distinguishes geometrically consistent, incomplete annotation, inconsistent reprojection, unsupported conventions, invalid inputs, split leakage and duplicate keypoint IDs. It does not require all labelled points to fall inside the image: off-image coordinates are not automatically truncated or discarded.

## SDKP source notes

Primary paper: Wang et al. (2026-11-03), SDKP, Data 11(11), 294, https://doi.org/10.3390/data11110294.

Dataset DOI: https://doi.org/10.57967/hf/9797

The paper states that the camera extrinsic stores a quaternion and translation describing the camera pose in Blender world coordinates; translations and point coordinates in this adapter are explicitly metres. The paper's camera sampling uses a pinhole model, camera optical axis directed at the spacecraft centre, 100–600 m ranges and zero camera roll, with 0.5-degree one-sigma pointing perturbations. The Fermi-GLAST target is the single published model. Earth images are alpha-composited after spacecraft rendering; they do not illuminate the mesh or receive its shadows. SDKP is therefore an appropriate synchronized-annotation / geometry baseline, not the only domain-generalization benchmark.

The Hugging Face card confirms 24,000 images (20k/2k/2k), 2D and 3D boxes, 2D keypoints, camera quaternion/translation and camera intrinsics. It explicitly warns that the initial train/validation/test JSON files omitted bbox_3d; corrected annotation JSONs are in the Correction folder. Use those files for future experiments. The card labels CC BY 4.0 for final composited images only; raw Earth-background sources are not released and their underlying licenses are not confirmed.

The exact source quaternion order is not hard-coded. A real-file adapter must record the annotation's observed schema and an independent known-pose sample before it can select XYZW or WXYZ. No file SHA-256 or byte count is claimed until the exact files are fetched.

## 41-case synthetic corpus

The corpus covers:
- identity and rotated camera poses;
- XYZW and WXYZ ordering as explicit inputs;
- q and -q equivalence;
- camera-to-world inverse transform and Blender-to-OpenCV axis conversion;
- in-front, behind-camera and near-plane cases;
- unit quaternion tolerance and malformed/zero quaternions;
- corrected bbox_3d required vs initial incomplete release;
- unit, coordinate and pose-semantic mismatches;
- camera focal length/image dimensions and non-finite values;
- reprojection mismatch and within-tolerance residual;
- split leakage and duplicate keypoint identifiers.

The independent standard-library Python oracle computes the transform and pinhole projection separately and compares the complete receipt with Rust output. The synthetic oracle fixtures are not a pose-estimation benchmark and do not establish physical truth.

## Reproduce

    cargo fmt --manifest-path tools/spacecraft-pose-annotation-contract-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/spacecraft-pose-annotation-contract-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/spacecraft-pose-annotation-contract-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/spacecraft-pose-annotation-contract-bench/fixtures/pose-annotation-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/spacecraft-pose-annotation-contract-bench/Cargo.toml --bin spacecraft-pose-annotation-contract-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/spacecraft-pose-annotation-contract-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

## Claim boundary

This package checks only synthetic annotation geometry under the declared camera/pose conventions. It does not authenticate source files, prove the underlying pose labels are physically accurate, validate the Fermi model, test a neural network, estimate uncertainty, establish on-orbit navigation performance, or authorize maneuvers. The receipt hard-codes source_bytes_authenticated=false, pose_ground_truth_physically_validated=false and authority_granted=false.

Research parent: Symtropy #7700.
External comparators to keep as separate, non-poolable lanes:
- SpaceSense-Bench (136 models, RGB/depth/LiDAR, dense part semantics): https://arxiv.org/abs/2603.09320
- SPEED+ HIL and synthetic domain gap: https://techfinder.stanford.edu/technology/next-generation-spacecraft-pose-estimation-dataset-speed
- SPADES real/synthetic event-camera trajectories: https://ai4space.space/challenge.html
