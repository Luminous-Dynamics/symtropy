# Bregar UWB indoor raw-CIR dataset v2.0 — candidate assessment

**Status:** candidate only; not downloaded, immutable, admitted, calibrated or benchmarked by Luminous Dynamics.

## Source and rights

- Dataset: Klemen Bregar, “UWB Positioning and Tracking Data Set”.
- Zenodo v2.0 record: https://doi.org/10.5281/zenodo.8280736
- Version: v2.0, published 25 August 2023.
- Payload archive: `data_set.zip`, publisher-reported size 4.4 GB.
- Publisher-reported archive MD5: `030cce5aadeab7b1b3dbd6901a49bfc2`. This is not SHA-256 and is not a content-admission receipt.
- Dataset payload licence: CC BY 4.0 on the v2.0 record. Attribute the authors and cite the DOI for permitted reuse.
- Do not confuse the earlier v1.0 record (Zenodo 7629141), whose licence terms differ (CC BY-NC-SA 4.0), with this v2.0 candidate.

## What it contributes

The dataset contains four indoor environments, eight fixed DW1000 anchors and one mobile tag. The raw-data hierarchy separates environment/location, anchor and channel captures. Per-capture CSVs include NLOS metadata, measured range, first-path index, received signal power, first-path/early-path features, noise metrics, channel information and accumulated channel-impulse-response samples.

This makes it a useful smaller candidate for:
- raw-CIR feature extraction and reproducibility;
- LOS/NLOS label and feature audits;
- per-anchor range residual and bias calibration;
- small-footprint MCU CIR classification experiments;
- range-set calibration and anchor-geometry experiments where the reference coordinates and capture semantics are independently established.

## What it does not establish

This is not a substitute for MILUV's multi-UAV cooperative ranging, passive-listener exchanges, continuous visual-inertial trajectories or Vicon-based trajectory truth. Its predefined tag/reference positions and controlled indoor captures do not constitute a full continuous UAV trajectory benchmark. It should not be used to claim swarm-navigation performance or general outdoor robustness.

## Admission blockers

1. Download only after confirming the exact Zenodo v2.0 payload inventory; record every file path, byte count and SHA-256.
2. Verify that the archive and file hashes match the official v2.0 record. Preserve the publisher MD5 as an additional cross-check only.
3. Inspect raw CSV schemas and document exact time units, DW1000 counter representation, wrap semantics, sample/event identity and channel numbering.
4. Establish anchor-coordinate provenance, tag locations, antenna delays, RF/channel configuration, and calibration artifacts/version.
5. Audit how LOS/NLOS labels were produced and whether any filtering or row selection depends on evaluator-only truth.
6. Freeze environment-level group IDs and a leakage-resistant environment-held-out calibration/test split before feature selection or model fitting.
7. Keep the controller view free of evaluator-only labels, and make event/row existence invariant under mutation of those labels.

Until these checks are complete, the registry record remains `candidate_not_admitted` with `admission_scope=none` and an empty SHA-256 inventory.

## Research positioning

Use this dataset as a **raw-CIR method-development and small-scale indoor ranging calibration** track. Compare against the source paper's baselines and matched versions of:
- handcrafted CIR feature classifiers;
- compact 63-sample CIR MCU classification;
- BERT-LSTM credibility evaluation where the data representation permits a faithful reproduction;
- physics-informed probabilistic range calibration;
- interval-valued DS-TWR / range-set methods.

Report recall and false-accept separately, interval coverage and sharpness, per-environment generalization, compute/energy, and the full retained/excluded event cohort. Never transfer a score from this dataset to MILUV or Hilti without matched physical validation.
