# EHD-PIV observation operator

Deterministic, dependency-free 2D2C planar PIV particle-image forward model.

## Model

The input is a prescribed 2D velocity field in pixel/time units and a set of subpixel particle seeds. V0:
- advects seeds using bilinear velocity interpolation and midpoint RK2;
- rejects advection outside field support (no extrapolation);
- integrates particle images at fixed midpoint samples across finite exposure;
- attenuates brightness using a Gaussian out-of-plane illumination-sheet profile;
- combines particle-image width and optical PSF width in quadrature;
- applies gain, offset, explicitly supplied additive per-pixel noise, active-pixel masking, clipping and integer quantization.

Noise values are inputs, not generated implicitly. The same frozen inputs therefore produce byte-identical image pairs.

## Units and limits

- Particle x/y, velocity and optical sigma are in pixels and pixels/time.
- z and out-of-plane drift use arbitrary declared sheet units and sheet-units/time.
- The supplied out-of-plane drift is a prescribed particle proxy. It is not a solved w-velocity field.
- The Gaussian particle/PSF rendering is a simple image approximation.
- No tracer inertia, Mie scattering, non-Gaussian PSF, optical aberration, camera nonlinearity, rolling shutter, 3D particle tracking or EHD observer perturbation is implemented.
- This is not an end-to-end PIV algorithm; it produces synthetic detector images only.

## Run

    cargo fmt --manifest-path tools/ehd-piv-observation-operator/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-observation-operator/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-observation-operator/Cargo.toml --all-targets -- -D warnings

## Manufactured test inventory

The V0 crate currently contains 11 deterministic Rust tests covering stationary frames, analytic translation, exposure smear, out-of-plane illumination attenuation, optical PSF width, sensor gain/noise/quantization/saturation, pixel masking, fail-closed domain boundaries, malformed inputs, field/image support and numeric overflow.
