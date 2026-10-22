<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->
# Visual optics reference

This isolated Rust package contains small analytical references for the visual-technology benchmark program. It is deliberately not a full optical simulator.

## Scope

The current implementation provides:

- Ideal rectified pinhole projection.
- Pixel-to-ray back-projection.
- Geometric horizontal and vertical field-of-view calculations.
- Bounded sRGB channel transfer and normalized linear-light relative luminance.
- SMPTE ST 2084 / ITU-R BT.2100 PQ signal-to-nits mapping and its inverse.
- BT.2100 HLG **scene-referred OETF and inverse**.

It does not model lens distortion, non-planar projection, physical panel response, HDR tone mapping, HLG display-side OOTF/EOTF, spatial light modulators, wave optics, eye fit, vignetting, or physical calibration. In particular, sRGB relative luminance is normalized Y, not cd/m²; PQ is a transfer mapping, not tone mapping or a panel measurement; and HLG OETF output is a nonlinear signal, not display luminance.

## Run locally

From the repository root:

```sh
cargo fmt --manifest-path tools/visual-optics-reference/Cargo.toml -- --check
cargo clippy --manifest-path tools/visual-optics-reference/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/visual-optics-reference/Cargo.toml
```

This crate defines its own small Cargo workspace, so it does not enlarge the main Bevy workspace. The library has no third-party runtime dependencies. Integration tests use `serde_json` as a dev-only dependency to parse the checked-in analytic fixtures. A path-filtered workflow runs formatting, Clippy and tests; its exact-head result must pass before we claim qualification.

## Test coverage

The source-level tests cover:

- Pinhole axis and off-axis projection against hand-derived values.
- Positive scale invariance and projection/back-projection round trips.
- Geometric FOV, off-center principal point handling and zero-dimension rejection.
- Invalid intrinsics, non-finite input, non-positive depth and overflow handling.
- sRGB transfer anchors, encode/decode round trips, and normalized linear-light luminance.
- PQ EOTF/OETF references, 100-, 203-, 1,000- and 10,000-nit points, domain rejection and round trips.
- HLG OETF/inverse references on both piecewise branches, domain rejection, round trips and a deterministic monotonicity grid.
- Integration tests that parse all six JSON fixtures, require `UNQUALIFIED` status, and compare outputs against separately evaluated equations and the reference functions.

The first focused workflow failed at rustfmt on the initial implementation; that specific formatting diff was corrected. Several mathematical slices and their fixtures have since been added, so only a successful workflow on the exact current head can qualify the current source. At last inspection the focused run was queued; no passing status is claimed.

## Research anchors

- [W3C CSS Color Module Level 4 (2026)](https://www.w3.org/TR/2026/CRD-css-color-4-20261008/) — sRGB transfer behavior.
- [ITU-R BT.2100-3 (2025)](https://www.itu.int/rec/R-REC-BT.2100-3-202502-I/en) — PQ and HLG HDR system parameters.
- [SMPTE ST 2084](https://pub.smpte.org/latest/st2084/st2084-2014.pdf) — PQ reference transfer mapping.
- [Khronos Data Format Specification 1.4](https://registry.khronos.org/DataFormat/specs/1.4/dataformat.1.4.html) — independently inspectable HLG/PQ equations.

## Evidence boundary

These are analytical software references. Passing tests would establish agreement with the checked mathematical formulas and test fixtures, not device calibration, VESA certification, measured luminance, HDR rendering quality, usable AR/VR field of view, physical holographic reconstruction, or display-manufacturing performance.
