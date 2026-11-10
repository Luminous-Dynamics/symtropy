# Perceptual metric bundle requirements — V0

This file specifies what the upstream benchmark/capture owner should freeze into the `metric_bundle_sha256` consumed by the task-conditioned policy. It does not implement image metrics or replace the metric/capture authorities in #4486, #1999, #2008 and #2009.

## Design rule

A metric is evidence only for the perceptual question and viewing condition for which it has been validated. The bundle must distinguish:
- **diagnostic metric:** useful for explaining image differences, but not calibrated sufficiently for an admission gate;
- **candidate gate:** tested against human judgments for the relevant artifact/task stratum, with uncertainty reported;
- **qualified gate:** independently reproduced, calibrated on held-out samples and accepted for that declared task/stratum.

A metric cannot be promoted to a gate merely because it is widely used or produces a scalar score. No universal “visual realism” score is defined here.

## Recommended metric families

| Metric | Primary role | Important limitations |
|---|---|---|
| FLIP / HDR-FLIP | Full-reference difference diagnostics for fixed-display rendered images; pixels-per-degree couples resolution to display size and viewing distance | Not a universal engineering-legibility or temporal-motion metric; freeze viewing geometry |
| HDR-VDP-3 | HDR visibility, quality and contrast-distortion diagnostics | Record peak luminance, transfer function and viewing condition; validate on the target artifact class |
| LPIPS / DISTS | Feature/perceptual similarity diagnostics | Model/version and domain matter; do not assume feature similarity equals scientific or engineering correctness |
| SSIM / MS-SSIM / PSNR | Structural and signal-level diagnostics | Sensitive to alignment and display encoding; often miss task semantics |
| VMAF v1 / HDR VMAF | Secondary quality diagnostics for encoded temporal/video delivery, e.g. remote streaming | Streaming/video metric, not a primary renderer realism or physical-legibility metric |

## Required metric bundle fields

The upstream bundle should bind at least:

1. **Identity:** schema version, bundle ID, SHA-256, exact source commit, generator/tool versions and immutable configuration.
2. **Task/domain:** task mode, intended use, scene/workload IDs, artifact class, required semantic channels and protected assets.
3. **Metric entries:** metric name/version/model hash, exact invocation, role (diagnostic/candidate gate/qualified gate), expected input format and declared limitations.
4. **Reference class:** analytic/calibrated reference, pinned path-traced frame, engine comparator capture or human preference data. Record how the reference was created and whether it shares code/assets with the test renderer.
5. **Image conditions:** registration/alignment method, pixel format, transfer function, colour space, tone mapping, peak luminance, output resolution, display dimensions, pixels-per-degree and viewing distance.
6. **Temporal conditions:** sequence length, timestamp/frame alignment, camera and animation state, temporal sampling, dropped-frame treatment and any frame interpolation.
7. **Uncertainty:** metric repeatability, confidence intervals, missing-data rules and known unsupported strata.
8. **Human calibration:** blinded protocol digest, sample-set digest, evaluator count, inter-rater agreement, metric-to-human rank correlation/calibration and confidence interval for each task/artifact stratum.
9. **Disposition:** which metrics are diagnostics versus actual gates for which strata; the justification and held-out reproduction result.

## Human calibration protocol

- Randomize left/right ordering and profile identifiers; do not expose engine or quality labels to evaluators.
- Use pairwise preference for the declared question, not a generic “which looks better?” prompt when the task is engineering or safety inspection.
- Include “equivalent / cannot judge” where appropriate, and record evaluator confidence.
- Stratify by artifact class: silhouette/geometry, roughness/BRDF, lighting/shadow, temporal instability, contact/animation, text/labels, physical overlays, uncertainty/freshness encoding and accessibility/contrast.
- Use a held-out image/sequence set. Do not use the same sample set both to fit a metric mapping and to claim validation.
- Report evaluator count, inter-rater agreement and uncertainty. If a metric-human relationship is weak or changes sign by artifact class, keep that metric diagnostic-only for the affected strata.

A 10 Oct 2026 systematic review of 283 computer-generated-rendering studies found only 8.1% included relevant subjective evaluation and 4.6% directly tested metric–human agreement. This is a real validation gap, not proof that any one metric is bad. The Symtropy opportunity is to close the gap for its own declared tasks and workloads.

## Admission rule

Only `qualified_gate` entries may provide hard bounds to #4530's selector. A metric bundle may include multiple diagnostic metrics and separate qualified gates; the selector consumes the already-computed, receipt-bound bounds, not raw metric implementations. If no metric is calibrated for a required task/stratum, the system must abstain from making that perceptual claim rather than substitute a convenient score.

## Energy / performance separation

Metric bundles do not mix perceived quality with performance:
- frame-time distributions, hitch counts and frame pacing are separate performance measurements;
- GPU energy is a telemetry-derived measurement with its own raw sample-series digest and source semantics;
- VRAM/system memory are separate resource measurements;
- spatial/temporal perceptual errors and human judgments are separate quality evidence.

An improvement claim is made at a matched resource budget or on a disclosed multi-objective Pareto frontier. Lower energy does not compensate for an unmet perceptual or semantic gate.

## Primary references

- HDR-VDP-3 paper: https://arxiv.org/abs/2304.13625
- FLIP perceptual-difference paper: https://doi.org/10.1145/3450626.3459787
- Netflix VMAF, including the 26 Oct 2026 HDR VMAF-v1 model note: https://github.com/Netflix/vmaf
- 10 Oct 2026 systematic review of HDR-VDP-2, LPIPS, DISTS and SSIM in computer-generated rendering: https://doi.org/10.3390/jimaging12100500
