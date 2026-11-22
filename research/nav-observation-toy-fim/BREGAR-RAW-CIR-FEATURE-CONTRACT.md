# Bregar UWB v2 raw-CIR feature extraction contract

**Status:** deterministic Rust feature kernel and ten synthetic unit tests implemented; raw Zenodo payload has not been downloaded or inspected.

**Dataset candidate:** [Bregar UWB Positioning and Tracking Data Set v2.0](https://doi.org/10.5281/zenodo.8280736), currently candidate-only in [registry PR #6779](https://github.com/Luminous-Dynamics/symtropy/pull/6779).

## 1. Source-verified data representation

The v2.0 Zenodo metadata and authors' public code identify these source fields:

- TAG_ID, ANCHOR_ID, tag/anchor XYZ positions;
- NLOS, RANGE;
- FP_INDEX, RSS, RSS_FP, FP_POINT1/2/3;
- STDEV_NOISE, CIR_POWER, MAX_NOISE, RXPACC;
- channel / frame / preamble parameters;
- CIR followed by the complex CIR sample values through the end of the CSV row.

The authors' preprocess/save_raw_data_to_json.py records the exact prefix-field ordering and treats the CSV values from index 26 onward as CIR samples. The versioned upstream reference source inspected for this contract is KlemenBr/uwb_positioning at commit a39472cf5bf828dce0ee1a403438744ea76cd1a8.

## 2. Publisher-aligned normalization and windowing

The authors' technical_validation/cir_min_max_mean.py and technical_validation/los_nlos.py define:

- CIR window length: 152 samples.
- Offset before first path: 5 samples.
- First-path index: integer truncation of FP_INDEX toward zero for positive values.
- Complex-to-magnitude conversion: magnitude of each complex CIR sample.
- Accumulation normalization: magnitude divided by RXPACC.
- Window: from integer FP_INDEX minus 5 through the next 151 samples.

The Rust kernel preserves the original FP_INDEX and its fractional component while applying the publisher's integer truncation for window alignment. It rejects invalid FP_INDEX, nonpositive RXPACC, nonfinite samples, zero-energy windows and incomplete 152-sample windows.

The publisher's plotting script calculates a time axis from PRFR, using separate 16 MHz and 64 MHz formulas, but hardcodes PRFR=64 in the inspected reference plot. The new Rust kernel therefore requires the caller to supply a separately qualified sample_period_ns; it does not silently infer a time base from the row's PRFR field.

## 3. New deterministic feature vector

The module src/bregar_cir_features.rs returns the complete normalized 152-sample magnitude window and these explicit features:

- first-path and maximum magnitude;
- peak index and peak delay relative to integer first path;
- RMS magnitude in the five pre-first-path samples;
- peak-to-prepath-noise ratio in dB, represented as unavailable when the reference is exactly zero rather than infinity;
- first-path-to-peak magnitude ratio;
- total squared-magnitude energy;
- energy fractions before first path, at the first-path sample, in the early postcursor window, and in the late tail;
- energy-weighted delay centroid and RMS delay spread relative to first path.

The derived features are transparent deterministic transforms of the CIR. They are not a trained LOS/NLOS model, calibrated bias estimator, range correction or integrity probability.

## 4. Label and truth isolation

The feature extractor input contains only:
- stable measurement identity;
- FP_INDEX;
- RXPACC;
- caller-supplied sample period;
- ordered complex CIR samples.

It deliberately contains no NLOS label, measured range, tag position, anchor position or evaluator truth. NLOS labels and position metadata must be joined later through a separate evaluator capability, after the controller-facing features and event identities are frozen.

This API shape is only one layer of isolation. A production dataset adapter must still prove that source rows and controller event existence are not filtered or selected using evaluator labels.

## 5. Tests

Ten synthetic tests cover:
1. Exact five-sample pre-offset and 152-sample extraction.
2. FP_INDEX integer truncation and fractional retention.
3. RXPACC normalization and energy scaling.
4. First-path/peak delay feature alignment.
5. FP_INDEX below the required pre-offset.
6. Incomplete window.
7. Invalid RXPACC, sample period and measurement identity.
8. Nonfinite FP_INDEX and CIR values.
9. Zero-energy rejection.
10. Zero prepath-noise reference represented as unavailable, and energy-fraction partition.

These tests validate the deterministic feature transformation. They are not results from the Zenodo archive.

## 6. Next work

1. Freeze the exact v2.0 archive and per-file SHA-256 values before parsing.
2. Implement a bounded-memory CSV row reader that handles the two-line version/header prefix, the 26 metadata fields and the complex CIR tail without loading the 4.4 GB archive into memory.
3. Verify complex-number serialization, missing values, sample count, RXPACC units, FP_INDEX semantics and PRFR sample spacing on actual source files.
4. Compare Rust windows against the authors' Python reference on a small independently selected sample, with a declared floating-point tolerance.
5. Verify whether NLOS labels and location coordinates remain available for every raw row and whether any preprocessing or row-selection depends on those labels.
6. Freeze environment-level splits before training or tuning.
7. Reproduce the authors' CNN and compare with compact 63-sample CIR and newer credibility/evidential baselines under the same environments and sensor features.
8. Add interval calibration and false-accept measures; classification accuracy alone is not a ranging-integrity result.

## Claim boundary

This module implements the documented CIR magnitude/RXPACC/FP_INDEX/window convention and derived features. It does not parse the dataset archive, validate physical units, authenticate radio sessions, establish range truth, predict LOS/NLOS, calibrate NLOS bias or demonstrate physical performance. The v2.0 dataset remains candidate_not_admitted until payload and lineage review is complete.
