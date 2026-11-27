# Spacecraft multimodal frame synchronization contract v0.1

A Rust-first metadata preflight for synchronized RGB, depth, semantic segmentation, LiDAR and pose records. The first source profile is SpaceSense-Bench; no raw dataset archive is downloaded or committed.

## Source profile

- Dataset: SpaceSense-Bench, 136 spacecraft models.
- Pinned revision: f8c4ee9c7c0f9a4037f4977f3ddc4e0610a8fb8e.
- Pose correction: 2026-05-19, after the maintainers found frame-timing misalignment in pose_ground_truth.csv for all 136 spacecraft.
- License: CC BY-NC 4.0, non-commercial only.
- Camera: FRD X-forward/Y-right/Z-down.
- World: AirSim NED.
- Quaternion: Hamilton WXYZ.
- Pose translation: metres; depth: millimetres; LiDAR: metres in SensorLocalFrame.
- Timestamp: YYYYMMDDHHMMSSmmm.

The correction requires the regenerated pose CSVs. A syntactically valid but pre-correction pose record is OUT_OF_SCOPE.

## Contract

Each bundle binds dataset revision/license/intended use, spacecraft, trajectory, sample, frame, split, declared timestamp, corrected pose revision, transform conventions, units and five modality records.

The v0.1 profile requires:
- dataset revision exactly the corrected SpaceSense card revision;
- exact CC-BY-NC-4.0 license and NONCOMMERCIAL use declaration;
- the corrected pose CSV revision exactly equal to the pinned dataset revision;
- a valid 17-digit Gregorian timestamp in the declared format (no timezone or UTC/TAI epoch inferred);
- sample ID equals spacecraft/trajectory/timestamp;
- modality frame IDs and expected sensor IDs match the bundle;
- each modality path binds the same spacecraft, trajectory and frame timestamp;
- each modality has a well-formed SHA-256 reference and an explicit presence flag;
- exact timestamp equality across RGB, depth, segmentation, LiDAR and pose in v0.1; there is no implicit tolerance.

A missing modality remains INCOMPLETE. A timestamp disagreement is UNSYNCHRONIZED. Wrong frame/sensor/revision/license/coordinate convention is OUT_OF_SCOPE. Missing or malformed identity, time or digest is INVALID_INPUT. Duplicate sample keys and spacecraft split leakage are explicit non-success dispositions.

The naive source timestamp is used only as a dataset-local exact join key. No clock offset, epoch or timezone is invented. A future tolerance-based clock model needs its own scoped calibration receipt.

## Frozen corpus and oracle

The 36 fixtures include exact synchronized positive controls, each modality timestamp shifted, pre-correction pose revision, wrong sensor/frame/path, missing streams, malformed/invalid dates, leap-year controls, wrong convention/units, commercial intent, source revision mutation, duplicate samples and train/test spacecraft leakage.

The independent Python oracle implements the calendar check, source profile, timestamp identity, modality registration and manifest leakage checks separately. Exact Rust/Python JSON receipt equality is required.

## Reproduce

    cargo fmt --manifest-path tools/spacecraft-multimodal-frame-sync-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/spacecraft-multimodal-frame-sync-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/spacecraft-multimodal-frame-sync-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/spacecraft-multimodal-frame-sync-bench/fixtures/frame-sync-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/spacecraft-multimodal-frame-sync-bench/Cargo.toml --bin spacecraft-multimodal-frame-sync-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/spacecraft-multimodal-frame-sync-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

## Claim boundary

A SYNCHRONIZED result proves only metadata-level identity and timestamp consistency under the pinned profile. It does not prove sensor calibration, accurate physical pose truth, real-world synchronization, geometric consistency among modalities, or flight readiness. SHA-256 references are retained but source bytes are not authenticated. The receipt hard-codes source_bytes_authenticated=false, calibration_physically_validated=false and authority_granted=false.

Research parent: Symtropy #7713. Per-image projection contract: #7712. Full perception/uncertainty strategy: #7700.
