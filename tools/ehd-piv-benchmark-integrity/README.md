# EHD PIV benchmark integrity

A small Rust reference crate for auditing whether train, validation and test cohorts obey a predeclared geometry/session/flow-family/acquisition split policy before anyone reports PIV accuracy or uncertainty calibration.

## What it prevents

- Complete sessions or geometries accidentally appearing in both train and test when those dimensions were declared as holdouts.
- Direct raw image-pair or multi-frame sequence reuse across splits.
- Constituent frame/image/view reuse across train/validation/test. Overlapping windows may share constituent assets inside one split, but not across split boundaries.
- Silent omission of required grouping metadata.
- Reporting sample counts as if they were independent acquisition counts.

The benchmark owner explicitly chooses the dimensions to hold out. For example, a session-transfer test can hold out `Session` and `CameraCalibrationBatch`; a new-flow-family test can additionally hold out `FlowFamily` and `Geometry`.

## Run

```sh
cargo test --manifest-path Cargo.toml
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path Cargo.toml -- --check
```

## Limitations

This is metadata-only. It does not hash the files or inspect raw images; the curator supplies SHA-256 values computed over the correct byte streams. Passing the audit does not prove statistical independence or representative coverage.
