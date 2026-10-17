<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->
# Visual optics reference

This is a dependency-free, isolated Rust package containing small analytical references for the visual-technology benchmark program.

## Scope

The initial function implements ideal rectified pinhole projection only. It does not model lens distortion, surface projection, photometry, HDR response, wave optics, spatial light modulators, display electronics or physical calibration.

## Run locally

From the repository root:

```sh
cargo test --manifest-path tools/visual-optics-reference/Cargo.toml
cargo clippy --manifest-path tools/visual-optics-reference/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tools/visual-optics-reference/Cargo.toml -- --check
```

This crate defines its own small Cargo workspace so it does not enlarge or change the dependency graph of the main Bevy workspace. It has no third-party dependencies.

## Current tests

- optical-axis point maps to the principal point;
- off-axis point matches hand-calculated coordinates;
- common positive scaling of point coordinates preserves projection within a declared floating-point tolerance;
- zero/negative depth is rejected;
- non-finite coordinates and invalid intrinsics are rejected;
- finite inputs whose arithmetic overflows to a non-finite output are rejected.

These are source-level test cases added in the current research PR. They have **not** been executed in the authoring environment, and a green result is not claimed.

## Evidence boundary

The equations and test constants are known-answer references for an idealized mathematical model, not evidence of physical projector/camera calibration or actual display performance. Any later model must separately declare its assumptions and be compared to analytic or measured reference data.