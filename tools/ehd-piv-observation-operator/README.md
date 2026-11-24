# EHD-PIV observation operator

Deterministic, dependency-free 2D2C planar PIV particle-image reference forward model.

## Model

The input is a prescribed 2D velocity field in pixel/time units and a set of subpixel particle seeds. V0:
- advects seeds using bilinear velocity interpolation and midpoint RK2;
- rejects advection outside field support (no extrapolation);
- integrates particle images at fixed midpoint samples across finite exposure;
- attenuates brightness using a Gaussian out-of-plane illumination-sheet profile;
- combines particle-image width and optical PSF width in quadrature;
- applies gain, offset, explicitly supplied additive per-pixel noise, active-pixel masking, clipping and integer quantization.

Noise values are inputs, not generated implicitly. The same frozen inputs therefore produce byte-identical image pairs.

The returned pair also exposes floating-point analog detector values after gain/offset/noise/masking but before clipping/quantization. `central_difference_image_sensitivity(plus, minus, step)` builds a row-major two-frame image sensitivity column. Keep seeds, masks and explicit noise identical between parameter perturbations; assemble the resulting columns into an identifiability Jacobian only after separately qualified covariance whitening. The analog path preserves sub-LSB sensitivity that would disappear in the uint16 image.

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

The V0 crate currently contains 17 deterministic Rust tests covering stationary frames, analytic translation, exposure smear, out-of-plane illumination attenuation, optical PSF width, sensor gain/noise/quantization/saturation, pixel masking, fail-closed domain boundaries, malformed inputs, field/image support and numeric overflow.

## Related high-throughput generator: SynthPix

Use [SynthPix](https://github.com/antonioterpin/synthpix) (MIT, current public repository v0.4.0 at commit `0effe6e70b77d0d5c97348d8220e74d31bfaee2e`) for high-throughput batched JAX image generation, large nuisance sweeps and training-data production. This crate is not intended to compete on GPU throughput. Its role is a small deterministic reference/negative-control implementation for explicit exposure integration, RK2 particle paths, Gaussian sheet attenuation, PSF, fixed sensor-noise arrays and fail-closed numerical/domain behavior. Compare semantic coverage first and throughput only under matched settings/hardware. See the research note for the pinned paper and limitations.

## Manufactured tests

The current operator has 17 deterministic tests, including sub-LSB analog finite differences, gain/brightness confounding, and malformed sensitivity input rejection.

The sensitivity suite includes four single-view confounding controls: particle brightness/camera gain, particle diameter/optical PSF, out-of-plane particle position/light-sheet thickness, and uniform flow velocity/frame separation. These are manufactured local ambiguities, not evidence that all experiments are unidentifiable; additional independent calibration views or measurement actions may break them.
