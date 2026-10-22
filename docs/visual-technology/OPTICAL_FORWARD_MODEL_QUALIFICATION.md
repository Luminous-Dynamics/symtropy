<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# Visual Optics Qualification Contract

**Status date:** 2026-10-17  
**Tracking issue:** [VIS-OPTICS-001 #1807](https://github.com/Luminous-Dynamics/symtropy/issues/1807)  
**Upstream program:** [Symthaea VIS-TECH-001 #7820](https://github.com/Luminous-Dynamics/symthaea/issues/7820)  
**Status:** Initial design contract; not an implemented simulator or physical qualification.

## Purpose

Define the smallest credible simulation/known-answer slice for display and optics research. Symtropy should own deterministic scenario execution and model-specific calculations where appropriate, while reusing its existing scenario/replay and SYM-EVAL-CORE experiment/evidence infrastructure.

This does **not** authorize a second generic experiment planner, receipt format, blinding framework, validity system, or vision-object model. Existing visual object-permanence work answers different questions and remains the owner of its semantics.

## Claim levels

Every output must identify its evidence class:

1. `ANALYTIC_REFERENCE`: value derived by an independently inspectable equation under declared assumptions.
2. `SYNTHETIC_MODEL`: deterministic output from a simplified model or generated fixture.
3. `NUMERICAL_SIMULATION`: output from a named numerical method/version and declared convergence/precision envelope.
4. `BENCH_MEASURED`: output from a physical device/instrument with calibration and retained raw data.
5. `INDEPENDENTLY_REPLICATED`: a separate reproduction record whose method and artifacts are inspectable.
6. `NOT_EVALUATED`: the claim requires missing inputs, hardware, calibration, rights, human evidence or specialist review.

A computed status is not physical authority. A simulated or analytically correct fixture cannot certify an actual display.

## Minimal model boundary

### A. Geometry first

Start with pinhole/rectified projection and rigid transforms because these are deterministic, tractable and sufficient to expose unit/frame errors. Each case declares:
- world/camera/projector coordinate frames, handedness and axis conventions;
- units, intrinsics, extrinsics, image origin and pixel-center convention;
- geometry and valid parameter bounds;
- expected projection/transform, tolerance and derivation;
- evidence class and model applicability.

Reject malformed transforms, unknown frames, non-finite values, zero/negative depth and unit mismatches. Keep lens distortion, non-planar surfaces, photometric response and wave effects as separately versioned extensions.

### B. Conventional display response

Represent a declared transfer function/tone curve and its assumptions. The initial implementation includes sRGB encoding/decoding and normalized linear-light relative luminance for SDR, the SMPTE ST 2084 / ITU-R BT.2100 PQ encode/decode pair for nominal 0–10,000 cd/m² signal interpretation, and BT.2100 HLG scene OETF/inverse. Verify domain/range, monotonicity where required, bounded numeric behavior and exact reference points. PQ decoding is not tone mapping; normalized relative luminance is not absolute cd/m²; HLG scene OETF output is not display luminance. Display-side HLG OOTF/EOTF requires explicit system gamma, nominal peak luminance, black-level assumptions and viewing context. No transfer-function fixture establishes HDR compliance, real panel luminance, local-dimming behavior or temporal performance. Physical EOTF tracking requires calibrated measurements on an actual device. Standards anchors: https://www.w3.org/TR/2026/CRD-css-color-4-20261008/, https://www.itu.int/rec/R-REC-BT.2100-3-202502-I/en, https://pub.smpte.org/latest/st2084/st2084-2014.pdf.

### B2. Pixel-to-ray and geometric field-of-view references

The isolated reference crate also computes an unnormalized camera-space ray from a continuous pixel coordinate and geometric horizontal/vertical field of view from focal lengths, principal point and image dimensions. These functions are intended as small consistency oracles for coordinate/angle math, not an XR optical model.

For XR specifically, keep panel resolution, per-eye render extent, nominal FOV and usable/perceived FOV distinct. Meta's developer documentation updated 25 September 2026 provides one dated profile: 70° × 66° nominal straight-ahead FOV and 58° × 54° p90 perceived target, with fit, eye position and gaze reducing usable view. Do not generalize those values to other headsets; test against each device's runtime projections and through-the-lens measurements when available. Source: https://developers.meta.com/vr/essentials/field-of-view/.

### C. Bounded optical forward models

Add optical equations one at a time: paraxial/thin-lens cases, then free-space scalar propagation where its assumptions apply, then quantization/sampling. Record whether polarization, coherence, vector diffraction, aberrations, partial coherence, temperature drift or measured component imperfections are excluded. The correct result for an input outside the declared validity envelope is abstention/rejection—not a plausible-looking image.

### D. Light-field and CGH fixture

A holography case must pin wavelength, grid size, pixel pitch, propagation distance, phase/amplitude encoding, phase quantization, boundary condition, numerical method and the output comparison metric. Report reconstruction error, speckle metric, calculation time and all declared limitations separately. Never label a simulated intensity map as a physical hologram.

#### Light-field-specific qualification

For light-field rendering, pin the view-generation method, spatial and angular sampling grids, image-depth positions, lens-array/optical parameters, viewing-angle convention and radiometric assumptions. Report angle-resolved MTF/resolution, angular crosstalk, view continuity, geometric/depth error, absolute luminance and uniformity—not only panel pixel count or nominal field of view. Test static and moving scenes separately; for scanned/mechanical systems, capture end-to-end scan latency, synchronization drift, temporal artifacts, vibration/noise, sustained brightness and input power. Compare candidates at matched usable luminance and exposure/measurement conditions.

The June 2026 *Nature Photonics* SSS-LiD paper reports 150° viewing angle and up to 8.0× horizontal / 3.2× vertical spatial-resolution improvements, but its extended data also reports an 8.6× luminance reduction for the super-Snell lens-array configuration, with 200 ms exposures in some multi-angle comparisons. These are linked trade-offs, not grounds to treat its headline FOV as a universal target. The dedicated benchmark is [VIS-LF-001 #1826](https://github.com/Luminous-Dynamics/symtropy/issues/1826), based on [the published paper](https://doi.org/10.1038/s41566-026-01938-8).

## Fixture format

Each frozen case should bind:

- `case_id`, `schema_version`, owner and status;
- model name/version and applicability envelope;
- canonical inputs, units and coordinate convention;
- expected outputs and declared tolerance/rounding policy;
- independently derived reference or provenance link;
- evidence class and negative-control role;
- exact corpus digest once a corpus manifest exists.

The expected-output oracle and its tests must be independent of candidate model-selection, optimizer and scoring code. For learned approximations, compare first with analytic/exhaustive small cases and publish an error envelope instead of using agreement with another learned model as ground truth.

## First known-answer cases

Start with 12–20 fixtures, then contribute to the 40+ cross-domain corpus defined in Symthaea #7820. Proposed examples currently include the pinhole axis, centered 1280×720 field of view, and pixel-to-ray round trip:
- `docs/visual-technology/fixtures/projector-pinhole-axis-001.json`
- `docs/visual-technology/fixtures/pinhole-fov-centered-1280x720-001.json`
- `docs/visual-technology/fixtures/pinhole-ray-roundtrip-001.json`
- `docs/visual-technology/fixtures/srgb-transfer-and-luminance-001.json`
- `docs/visual-technology/fixtures/pq-st2084-eotf-oetf-001.json`
- `docs/visual-technology/fixtures/hlg-scene-oetf-001.json`

All these files carry `UNQUALIFIED`; the Rust integration test parses the fixtures and checks initial analytic equations against the reference functions. Passing those software tests does not make them physical evidence, a qualified optical model, a device calibration, a tone mapper, or an HDR compliance result.

1. Identity rigid transform and inverse.
2. Known 90-degree axis rotation with handedness negative control.
3. Rectified pinhole projection of the optical axis.
4. Known off-axis projection at fixed positive depth.
5. Zero/negative depth rejection.
6. Invalid units and mismatched frame rejection.
7. Transfer-function domain/range and monotonicity.
8. Thin-lens relation within a declared paraxial regime.
9. Simple scalar propagation against a separately calculated reference.
10. Sampling/Nyquist boundary and undersampling negative controls.
11. Phase quantization bound for a declared phase-level count.
12. Missing wavelength/pixel-pitch/calibration rejection.
13. Stale calibration and changed-device-profile rejection.
14. Tampered expected-value/corpus-digest detection.
15. Missing raw output or provenance artifact blocks complete evidence status.

Each fixture must have explicit units and expected values; this numbered list is a work plan, not evidence that the cases already exist or pass.

## Initial acceptance gates

- 100% agreement with frozen analytic references within predeclared numerical tolerances.
- Zero false accepts for invalid frames/units, NaN/Inf, impossible bounds, stale required calibration or out-of-envelope physics.
- Deterministic canonical output for pinned inputs where deterministic behavior is promised.
- All results bind exact model/configuration, scenario/corpus digest and artifact references.
- The independent verifier must reject mutated inputs, swapped artifact roles and producer-declared PASS without required outputs.
- Report per-output error, solver runtime and known limitations. No universal aggregate quality score.
- CI queue status is not test evidence. Record exact commit/run and distinguish queued, failed, canceled, and passed.

## Integration plan

1. Audit existing `symtropy-sim`, scenario/replay and SYM-EVAL-CORE types; map required fields before creating code.
2. Commit fixture manifest and an independently written checker with adversarial tests.
3. Add only the minimal geometry/transfer-function implementation needed to execute the first fixtures.
4. Add wave optics behind an explicit feature/package boundary only after numerical references, dependency/license review and a bounded resource budget exist.
5. Connect measured artifacts only after a physical device, instrumentation and calibration workflow is available.

## Research references

- [ITU-R BT.2100-3 (2025)](https://www.itu.int/rec/R-REC-BT.2100-3-202502-I/en), specifying the PQ and HLG HDR system transfer functions.
- [SMPTE ST 2084](https://pub.smpte.org/latest/st2084/st2084-2014.pdf), defining the PQ reference transfer mapping to nominal absolute luminance 0–10,000 cd/m².
- [Khronos Data Format Specification 1.4](https://registry.khronos.org/DataFormat/specs/1.4/dataformat.1.4.html), with inspectable HLG OETF/inverse and PQ equations.
- [W3C CSS Color Module Level 4 (2026)](https://www.w3.org/TR/2026/CRD-css-color-4-20261008/), for sRGB transfer behavior and linear-light color handling.
- Follow-on owner issue: [VIS-HDR-001 #1819](https://github.com/Luminous-Dynamics/symtropy/issues/1819) keeps HLG display-side system response outside this scene OETF slice.
- [VESA DisplayHDR performance criteria](https://displayhdr.org/performance-criteria/)
- [Khronos OpenXR Registry](https://registry.khronos.org/OpenXR/)
- [Propagation-adaptive 4K computer-generated holography](https://www.nature.com/articles/s41467-025-62997-z)
- [Real-Time Rendering Framework for Holography](https://doi.org/10.1111/cgf.70058)
- [Spatial/angular resolution measurement for light-field displays](https://trepo.tuni.fi/bitstream/handle/10024/213976/LFResolutionMeasurement_JSTSP_final.pdf?sequence=-1)

## Exclusions

No eye-safety, medical, comfort, field-of-view, wavefront, manufacturing, energy-saving or production-readiness claim is established by these fixtures. Physical optical systems require calibrated measurements and specialist review.
