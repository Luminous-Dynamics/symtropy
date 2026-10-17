<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# Light-field display workload corpus v0.1

**Owner:** [VIS-LF-001 #1826](https://github.com/Luminous-Dynamics/symtropy/issues/1826)  
**Corpus ID:** `VIS-LF-CORPUS-001`  
**Status:** `UNQUALIFIED` — case definitions only, not measured results.

## Why this benchmark differs from nominal FOV tests

The June 2026 SSS-LiD paper reports 150° viewing angle and up to 8.0× horizontal / 3.2× vertical resolution improvements, but an 8.6× luminance reduction for the super-Snell lens-array configuration. Some multi-angle figures use 200 ms exposure, which cannot independently establish dynamic-scene fidelity. The target, therefore, is not to repeat one headline number. It is to characterize the joint Pareto surface of **usable angular range × resolution × depth × luminance × temporal fidelity × power × mechanical stability**. [Source](https://doi.org/10.1038/s41566-026-01938-8).

## Files

- `light-field-workload-corpus-v0.1.json`: 24 proposed cases, all explicitly `UNQUALIFIED`.
- `light-field-workload-corpus.schema.json`: JSON Schema Draft 2020-12 contract.

## The 24-case matrix

The manifest defines reproducible case IDs for:
1. usable angular range;
2. center and off-axis MTF;
3. 20 mm and 40 mm depth-plane fidelity;
4. known-depth order/error;
5. adjacent-view crosstalk;
6. view continuity/aliasing;
7. absolute luminance by angle/depth;
8. brightness-conditioned usable FOV;
9. angular color/white-point shift;
10. static reconstruction fidelity;
11. dynamic-scene scan artifacts;
12. long-exposure negative control;
13. sustained scan synchronization drift;
14. power/thermal response;
15. ambient-light robustness;
16. mechanical noise/vibration and alignment sensitivity;
17. calibration repeatability across sessions/operators;
18. stale-calibration negative control;
19. missing/corrupt raw evidence negative control;
20. device/profile mismatch negative control;
21. matched-task baselines; and
22. upstream source identity/reuse-rights provenance.

The remaining test IDs split several of those dimensions across dedicated depth/geometry conditions so one aggregate score cannot hide where the optical system fails.

## Protocol invariants

- Pin device model/firmware/optical configuration, image-depth position, angular convention, camera geometry, exposure, lens/focus/gain, ambient illuminance, color processing, instrument calibration and all relevant units.
- Sample angle and depth as a grid; never infer a continuous usable viewing zone from only center samples.
- Measure spatial resolution/MTF, angular sampling, crosstalk, depth error, luminance, color and temporal behavior separately.
- Define luminance/MTF/crosstalk floors before seeing candidate results. Report each failed angle/depth instead of averaging it away.
- Compare configurations at matched content and an explicit brightness floor. Keep input-power and thermal costs, including any scanning mechanism.
- A still image captured with a long exposure is not dynamic evidence. Retain frame-synchronized or high-speed capture when temporal behavior is claimed.
- Split calibration and holdout by scene, angle, surface and session, not random pixels from the same capture.
- The camera/instrument must itself be calibrated; uncalibrated RGB is not absolute luminance or spectral truth.
- If raw captures, calibration records, uncertainties or reuse rights are missing, the relevant claim stays `NOT_EVALUATED`.
- Every corpus case stays `UNQUALIFIED` until a later evidence-bearing evaluation binds input/model/device/artifact digests and qualified review.

## Proposed gates (not achieved results)

- 100% schema and semantic integrity for the frozen 24 cases.
- Zero false accepts on injected stale calibration, missing raw data, wrong device configuration, long-exposure-only dynamic claim, and schema/corpus mutation negatives.
- Every published angular/MTF improvement includes per-depth, per-angle values, raw measurement references, calibration uncertainty and a preregistered usable-quality threshold.
- A candidate improvement claim must be measured against a named baseline under matched conditions. Do not state a percentage or benchmark win before the baseline is reproduced.
- Physical claims require repeated measurements and independent review; human comfort or vision-health claims are outside this corpus.

## Research references

- Chen et al. (2026), [Super-Snell scanning light-field display](https://doi.org/10.1038/s41566-026-01938-8).
- [Quantifying spatial and angular resolution of light-field 3D displays](https://trepo.tuni.fi/bitstream/handle/10024/213976/LFResolutionMeasurement_JSTSP_final.pdf?sequence=-1).
- [Camera-in-the-loop optical calibration, Optica 2026](https://opg.optica.org/abstract.cfm?uri=DH-2026-Tu1A.3).
- Symtropy [visual optics reference package](https://github.com/Luminous-Dynamics/symtropy/tree/main/tools/visual-optics-reference).

## Limits

This corpus does not implement an optical simulator or establish a physical viewing angle, usable resolution, luminance, crosstalk, power, or lifetime. It does not claim to reconstruct the SSS-LiD result; source code and dataset rights/identity must be checked before any reproduction effort. It distinguishes a light-field display from a true holographic wavefront reconstruction.
