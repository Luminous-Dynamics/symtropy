# display-stimulus-reference

Standalone Rust 2024 / Rust 1.96 generator for the four initial ODRH-STIM-001 raster primitives, mapped to existing case IDs in Symthaea's frozen 32-case display workload corpus.

From the repository root:

    cargo fmt --manifest-path tools/display-stimulus-reference/Cargo.toml -- --check
    cargo clippy --manifest-path tools/display-stimulus-reference/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/display-stimulus-reference/Cargo.toml
    cargo run --manifest-path tools/display-stimulus-reference/Cargo.toml -- --case ODRH-STIM-GREY-RAMP-001 --out /tmp/grey-ramp.ppm

Use `--report PATH` to write a JSON report beside the generated raster. The default recipe/schema paths are pinned relative to the repository root; pass `--manifest PATH` or `--schema PATH` to test alternate versions.

## Implemented primitives

- `neutral_ramp`: monotonic horizontal code ramp replicated vertically.
- `near_black_steps`: even-width grayscale bars for declared numeric RGB code values.
- `center_patch`: exact-size centered foreground RGB code patch over a declared background code.
- `rgb_gradient`: red and green code ramps with blue defined as the rounded arithmetic mean of the two channel values.

P6 PPM stores RGB samples without embedded ICC data. For maxval <=255, each channel uses one byte. For maxval >255 in this v0.1 profile, each channel uses two big-endian bytes. The 10-bit sample values occupy the range 0..1023; they are stored in 16-bit sample containers. The report describes these as explicit code values only.

## Evidence boundary

A generated digest proves which source raster was emitted. It does not prove those bytes survived host/GPU/compositor/color-management/video-signal processing, or that a physical display produced a particular luminance, chromaticity, HDR response, cadence, or perceived quality. Each report is marked `SYNTHETIC_STIMULUS`, `UNQUALIFIED`, and `physical_output_validation: NOT_PERFORMED`.

The recipes point to existing source case IDs and pin the Git blob SHA-1 for the source corpus. They do not fork or alter that corpus. To add cases, update the allowlist/schema/tests only after the workload owner defines exact generator semantics and claims.
