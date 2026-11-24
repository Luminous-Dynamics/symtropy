# EHD-PIV-EXT-001 — External end-to-end PIV benchmark admission

**Status:** V0 external dataset admission and profile metric kernel; no competing-system superiority result is claimed.

**Issue:** [#7248](https://github.com/Luminous-Dynamics/symtropy/issues/7248)

## Why this benchmark matters

The current public PIV landscape has more than one target. PIVPostProcess focuses on classifying valid/erroneous velocity vectors. Its October 2026 *Experiments in Fluids* article reports a test F1 of 0.992 and ROC-AUC of 0.998 and a mean F1 of 0.990 on six additional experimental datasets. This is a strong result for its task, but classification against analyst labels is not equivalent to reconstructing the physical measurement or preserving a downstream statistic. [PIVPostProcess paper](https://doi.org/10.1007/s00348-026-04319-7).

PIVtools is a contemporary open-source planar, stereo and ensemble PIV platform. Its 2026 SoftwareX paper reports a synthetic turbulent-channel benchmark at (Re_\tau\approx1000), and a 117-pairs/s performance figure on a 192-core dual-EPYC system. The throughput is hardware- and pipeline-specific; it is not a portable target independent of measurement settings. [PIVtools paper](https://doi.org/10.1016/j.softx.2026.102967).

The PIVtools Turbulent Channel Validation Dataset is a particularly valuable external challenge because its current Hugging Face card combines clean/noisy synthetic planar and stereo measurements with an experimental image set and a same-facility hot-wire comparison. The card declares CC-BY-4.0 and approximately 171 GB total size. [Dataset card](https://huggingface.co/datasets/MTT69/TurbulentChannel).

## Pinned external data

- Repository: [MTT69/TurbulentChannel](https://huggingface.co/datasets/MTT69/TurbulentChannel)
- Revision: `02ce80485cf859692126cc5f90fdc5b727608951`
- [Pinned revision](https://huggingface.co/datasets/MTT69/TurbulentChannel/tree/02ce80485cf859692126cc5f90fdc5b727608951)
- License: CC-BY-4.0; preserve attribution to Taylor, Lawson & Ganapathisubramani and the dataset.
- Citation: Taylor, Lawson & Ganapathisubramani, *PIVtools: A comprehensive open source software for particle image velocimetry*, SoftwareX 35 (2026), 102967, DOI 10.1016/j.softx.2026.102967.

The revision and license are recorded in `data/research/ehd_piv_external_benchmark_v0_1.json`. This work has not downloaded the full dataset.

### Contents

The current dataset card lists:
- **Case A, clean synthetic:** 4,000 planar and 4,000 stereo pairs, 85,000 particles/image and no added noise.
- **Case B, noisy synthetic:** 4,000 planar and 4,000 stereo pairs, 22,000 particles/image and independent Gaussian frame noise (mean 80, standard deviation 16, stated SNR approximately 8).
- **Experimental Cam4:** 2,100 cropped, gain-normalised image pairs with a same-facility hot-wire profile.
- Synthetic statistics derived from JHTDB turbulent-channel particle trajectories, approximately (Re_\tau=1000).
- Planar images 2048x2048 px; stereo images 1440x2080 px; 8-bit TIFF; synthetic particle diameter approximately 3 px and laser sheet approximately 1.2 mm.

## Reference semantics: what may and may not be claimed

The synthetic `direct_stats.mat` files include (U^+), a 3x3 Reynolds-stress tensor, reference confidence intervals and wall-unit metadata. They represent case-matched particle-statistics references, not instantaneous per-frame velocity ground truth. Case A and Case B use different particle counts and must retain their own case-specific reference statistics.

The experimental hot-wire file supplies a one-dimensional profile for mean streamwise velocity and streamwise variance. Its wire length is approximately (l^+=45.6); the card explicitly notes that the variance is not spatial-resolution-corrected. This is an independent profile-level comparison, not complete spatial or instantaneous truth.

This distinction is central: we must not report framewise RMSE against a statistical profile, or call a single hot-wire probe a full-field ground truth.

## V0 executable profile kernel

The crate `tools/ehd-piv-external-benchmark` accepts one normalized scalar profile at a time. It calculates:
- point coverage, bias, RMSE and NRMSE normalized by reference RMS;
- fraction of estimates within supplied reference intervals;
- fraction of reference values within supplied prediction intervals;
- y+-span coverage;
- trapezoidal bias, RMSE, normalized RMSE and integrated absolute error.

The metric kernel requires strictly increasing y+ coordinates and explicit units. It does not interpolate mismatched grids. Trapezoidal quadrature respects nonuniform y+ spacing and skips adjacent intervals when either estimate is missing. Each channel (mean velocity, each stress component, TKE/trace) is scored separately. Confidence-interval diagnostics are not proofs of uncertainty calibration.

The crate also provides a full six-component symmetric Reynolds-stress tensor scorecard: component-wise RMSE, TKE trace error, full-tensor Frobenius error and positive-semidefinite realizability counts. This is enabled only when all six components are present; planar two-component PIV and hot-wire profiles do not support a full 3D realizability claim.

It intentionally does not parse TIFF/MAT/HDF5 in V0. A later importer must record conversions, units, coordinate orientation, input file digests and source revision.

## Comparison protocol

### Audit of the published reference scripts

The pinned README and `scripts/benchmark_comparison.py` expose important baseline choices that must not be hidden inside a generic "PIVtools pipeline":

- Noisy planar and experimental cases ship separate instantaneous and ensemble configurations. The published instantaneous pipeline uses a `ssmin 7×7` preprocessing filter; the ensemble pipeline uses raw images because filtering changes ensemble stresses. These are separate operation classes and must remain separate matched baselines.
- Stereo results in the published capsule are instantaneous only; do not invent a stereo-ensemble comparator.
- The published profile helper excludes the first and last four x-vectors, x-averages the remaining field, and compares (y^+ = 10ldots500) by default.
- Its illustrative alignment sets a wall-origin offset from the minimum measured y-coordinate and then applies a +1 wall-unit shift. Treat this as a published script choice to reproduce and audit, not as an unconstrained tuning parameter. A candidate-specific y+ shift chosen to improve its score is prohibited.
- The helper reports profile error after x-averaging. That can hide localized wakes, streaks or weak vortices, so every profile report must be paired with the separate field/structure scorecard from [#7209](https://github.com/Luminous-Dynamics/symtropy/issues/7209).

Record the exact configuration digest, preprocessing, interrogation-window pass, run index/window size, x-edge mask, wall-origin rule, y+ shift, unit conversion and evaluated y+ intervals for every baseline. Keep the original shipped configuration as the reproduction lane; any alternate preprocessing or calibration is a new named lane, not a silent "fairness" edit.


### Tier 0 — reproduce the capsule

Use the supplied `reference_results` and ground-truth statistics without reprocessing raw images. Reproduce the dataset's reference figures/statistics and verify paths, case identity, unit metadata, calibration sidecars, scale, dt and coordinate signs. This qualifies our importer/plotting pipeline before any algorithm comparison.

### Tier 1 — matched PIV reprocessing

Reprocess raw images with PIVtools as the primary reference pipeline. Add OpenPIV and PIVlab only when the exact method, calibration, preprocessing, masks, interrogation passes/windows, outlier rule and interpolation policy are matched and recorded. Planar instantaneous, planar ensemble and stereo instantaneous are separate tracks.

### Tier 2 — engineering quality

Report mean profiles, each stress component, Reynolds-stress trace/TKE, confidence-interval agreement and errors by predeclared y+ ranges. Report the physical resolution and interrogation-window transfer alongside pixel/grid spacing. Connect structure-preservation scores from [PR #7210](https://github.com/Luminous-Dynamics/symtropy/pull/7210) to near-wall/shear-region comparisons where the reference admits it.

For the experimental case, compare only supported profile quantities with explicit hot-wire response limitations. Do not mistake the wire's measurement volume and spatial averaging for a pointwise oracle.

### Tier 3 — adaptive measurement value

Use the ADAPT/EVENT workstreams to choose additional frames or measurement settings based on expected reduction in downstream profile/stress uncertainty. Candidate actions include seeding density, exposure/illumination, time separation, interrogation window and stereo acquisition. The policy may not inspect reference truth when selecting the next acquisition.

## Statistical protocol

- Start with metadata, shipped reference outputs and calibration sidecars; never bulk-download the 171 GB capsule during initial qualification.
- Use a tiny image subset only to test import and pipeline correctness; label it as a pilot, not a competitive result.
- Freeze method configuration, software commit, dataset revision, conversion version and compute environment before scoring the remaining public holdout blocks.
- Because the dataset and references are public, describe this as a frozen public holdout, not a cryptographically sealed evaluation.
- Treat contiguous temporal blocks, not individual frames or pixels, as the resampling units.
- Report pair counts, block counts, missingness, failed runs, processing times, CPU/GPU, thread settings and all configuration digests.
- Compare runtime at matched image sizes, passes, preprocessing, calibration and hardware. The published PIVtools 117-pairs/s figure is tied to its stated 192-core dual-EPYC configuration.

## Prospective targets

- Reproduce the capsule's supplied reference outputs under recorded numerical tolerances.
- No aggregate score may conceal a failing near-wall band or a deteriorating individual Reynolds-stress component.
- At matched acquisition/compute budgets, reduce worst-band profile and structure errors while maintaining reference-interval and prediction-interval diagnostics.
- Adaptive acquisition primary target: reach the same predeclared profile/stress error with at least 20% fewer image pairs than a fixed periodic/fixed-grid strategy, aligned with ADAPT-001. A 25% reduction is a stretch target. Both are prospective and require matched evidence coverage, worst-band errors, weak-structure retention and uncertainty diagnostics.
- Every result must identify dataset revision, case, image count, method commit, config digest, units, coordinate convention, and reference semantics.

## Claim boundary

This work admits and specifies a reproducible independent benchmark. It does not establish that Symtropy currently beats PIVtools, PIVPostProcess, OpenPIV, PIVlab or commercial systems. It does not transform statistical DNS profiles into instantaneous ground truth or hot-wire profiles into full-field truth.
