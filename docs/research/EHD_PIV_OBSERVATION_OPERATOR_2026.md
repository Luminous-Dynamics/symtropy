# EHD-PIV-OBS-001 — Measurement-operator-aware particle-image model

**Status:** V0 deterministic synthetic planar 2D2C forward operator; no real optical calibration or EHD validation.

**Issue:** [#7256](https://github.com/Luminous-Dynamics/symtropy/issues/7256)

## The measurement chain is the observation

PIV does not directly observe a velocity vector. The measured image depends on flow advection, tracer distribution, illumination, out-of-plane particle loss, exposure, particle image formation, optical PSF, camera gain/offset/noise, clipping, sensor mask, calibration and subsequent image-correlation processing.

A reconstruction method conditioned only on velocity vectors bypasses many of these causes. The more complete inverse target is:

`p(flow, nuisance | raw image pairs, acquisition metadata)`

with nuisance variables explicitly represented and independently qualified. The objective is not to let a flexible posterior explain away sensor mismatch as a confident change in the physical flow.

## V0 executable kernel

Crate: `tools/ehd-piv-observation-operator/`.

Inputs:
- regular 2D2C velocity field in pixel/time units;
- subpixel particle seeds with brightness, intrinsic image sigma, sheet position and prescribed out-of-plane drift;
- Gaussian illumination-sheet width;
- frame separation, exposure duration, midpoint quadrature count and max RK2 step;
- camera dimensions, bit depth, gain, offset, optical PSF sigma and active-pixel mask;
- explicit additive per-frame noise arrays.

Outputs: two quantized `u16` detector frames and a deterministic receipt with particle count, timing, effective PSF sigma range, mask count and saturation counts.

The model uses bilinear field sampling, midpoint RK2 advection and fixed midpoint samples across each exposure. Particle-image and optical PSF widths are combined in quadrature. Out-of-plane illumination weighting is Gaussian. Sensor noise is supplied as data rather than generated from a hidden RNG. Particle trajectories leaving the declared field support fail closed.

## Manufactured tests

The eleven V0 tests cover:
1. identical stationary frames;
2. analytic centroid translation under uniform advection;
3. increased blur for a longer exposure;
4. out-of-plane attenuation through the Gaussian light sheet;
5. optical PSF widening;
6. deterministic gain/offset/noise/quantization/saturation;
7. masked pixels forced to zero;
8. no silent velocity extrapolation;
9. malformed noise and acquisition metadata;
10. field support outside the sensor bounds;
11. non-finite sensor signal overflow.

These are analytic/constructed controls, not comparisons to measured hardware.

## V1 research agenda

The next physically meaningful additions require source-pinned device metadata and independent measurements:
- particle-size/intensity distribution and tracer response (including Stokes-number regime);
- 3D out-of-plane motion and sheet-crossing probability;
- calibrated laser-sheet intensity and thickness;
- measured optical PSF/aberration, camera response/nonlinearity, exposure timing, rolling shutter and read/shot noise;
- calibration target, lens distortion, mapping and registration covariance;
- observed raw-image likelihood / reprojection consistency with a separately calibrated PIV pipeline;
- controlled negative tests where a nuisance-only perturbation must not be interpreted as a physical flow change.

Potential inverse inference can combine a conventional PIV likelihood, sparse sensor likelihood and a conditional generative prior, but every prior/measurement privilege and uncertainty source must be explicit. Use the external PIVtools raw-image capsule (#7248) only with its exact CC-BY-4.0 attribution and pinned run configs.

## Benchmark interface

Keep these scoreboards separate:
- detector image-domain residual and particle-centroid/diameter recovery;
- vector reconstruction and valid-vector retention;
- weak-structure / multi-scale fidelity;
- posterior consistency, calibration, coverage, sharpness and physical admissibility;
- latency, memory and acquisition/inference cost.

This connects to UQ-010 (#6963), external raw images (#7248), structure preservation (#7209), observer effects (#6937), OED (#7036) and adaptive acquisition (#7097).

## Claim boundary

This is a deterministic synthetic 2D2C observation operator. It does not model tracer inertia, Mie scattering, non-Gaussian PSFs, optical aberration, camera nonlinearity, rolling shutter, solved 3D particle dynamics, Shake-the-Box, or physical EHD observer perturbation. No raw-image algorithm or measured calibration has been implemented.