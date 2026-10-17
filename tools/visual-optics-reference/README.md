<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->
# Visual optics reference

This is a dependency-free, isolated Rust package containing small analytical references for the visual-technology benchmark program.

## Scope

The current reference implements ideal rectified pinhole projection, pixel-to-ray back-projection, geometric horizontal/vertical field-of-view calculations, bounded sRGB transfer/relative-luminance functions, and the SMPTE ST 2084 / ITU-R BT.2100 PQ transfer mapping. The sRGB/PQ functions are mathematical transfer references, not tone mapping or panel calibration. This crate does not model lens distortion, surface projection, absolute photometry measurement, HLG, wave optics, spatial light modulators, display electronics, eye fit, vignetting or physical calibration.

## Run locally

From the repository root:

```sh
cargo test --manifest-path tools/visual-optics-reference/Cargo.toml
cargo clippy --manifest-path tools/visual-optics-reference/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tools/visual-optics-reference/Cargo.toml -- --check
```

This crate defines its own small Cargo workspace so it does not enlarge or change the dependency graph of the main Bevy workspace. The reference library has no third-party runtime dependencies; integration tests use serde_json as a dev-only dependency to parse the checked-in analytic fixtures. A dedicated path-filtered workflow runs these three checks on PRs/pushes that touch this crate; its exact-head result still must be observed before claiming qualification.

## Current tests

- optical-axis point maps to the principal point;
- off-axis point matches hand-calculated coordinates;
- common positive scaling of point coordinates preserves projection within a declared floating-point tolerance;
- zero/negative depth is rejected;
- non-finite coordinates and invalid intrinsics are rejected;
- finite inputs whose arithmetic overflows to a non-finite output are rejected;\n- sRGB transfer functions are checked at known reference points, including linear-light relative luminance;\n- PQ EOTF/OETF is checked at 100-nit, 203-nit, 1,000-nit and 10,000-nit references with encode/decode round trips;\n- integration tests parse the checked-in JSON fixtures, require UNQUALIFIED status and compare values against independent equations plus the reference functions.

The current source includes unit and integration tests, and a path-filtered workflow runs formatting, Clippy and tests. An earlier focused run failed at rustfmt; that exact formatting diff was corrected before subsequent features were added. The latest workflow has been queued/pending while the fixture-contract tests are added, so no passing status is claimed here.

## Research anchors\n\n- [W3C CSS Color Module Level 4 (2026)](https://www.w3.org/TR/2026/CRD-css-color-4-20261008/) for sRGB transfer behavior.\n- [ITU-R BT.2100-3 (2025)](https://www.itu.int/rec/R-REC-BT.2100-3-202502-I/en) and [SMPTE ST 2084](https://pub.smpte.org/latest/st2084/st2084-2014.pdf) for PQ HDR transfer mapping.\n\n## Evidence boundary

The equations and test constants are known-answer references for an idealized mathematical model, not evidence of physical projector/camera calibration or actual display performance. Any later model must separately declare its assumptions and be compared to analytic or measured reference data.