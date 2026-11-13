# Superlet DAS Traffic Reproducibility Audit v0.1

**Workstream:** SUPERLET-TRAFFIC-REPRO-QUAL-001 (#5034)  
**Paper:** Sun et al. (2026), Optics & Laser Technology 201, 115310; DOI 10.1016/j.optlastec.2026.115310  
**Public implementation:** https://github.com/sqs1212/Traffic-flow-monitoring-based-on-distributed-acoustic-sensing-DAS  
**Date:** 2026-11-13  
**Status:** source/metadata audit only; no dataset acquisition or metric reproduction

## 1. Paper-reported task

The article reports phase-consistent superlet preprocessing for 10–80 Hz vehicle signatures, trajectory segmentation IoU of 68.75%, vehicle classification accuracy of 93.31%, and under 50 ms inference latency per sample on its own real-world traffic dataset. These are separate traffic-task results; they are not utility defect POD, pipe condition accuracy, fibre route error or asset attribution.

## 2. Repository completeness

A recursive listing of the public default branch contains 36 paths. It includes preprocessing, model definitions, data loaders and two split lists. The inspected tree does not include LICENSE, pyproject.toml, requirements.txt, a scripts directory or the README-described src/das_traffic_monitoring package root. README installation/training instructions therefore do not match the observed tree.

The vehicle-signal extraction module is 63 bytes and contains only a module docstring. The vehicle-classification metrics module is also only a module docstring. No visible end-to-end training or inference program exists in the inspected tree.

No explicit software licence is visible. Treat the implementation as rights-unresolved; do not copy, modify, bundle or commercially deploy it until the maintainers provide a licence or written permission. The README links a dataset on Baidu Netdisk but gives no visible dataset licence/redistribution terms. Dataset rights are also unresolved.

## 3. Split metadata audit

The two committed vehicle classification split lists were parsed as metadata only.

| Measure | Training | Validation |
|---|---:|---:|
| Rows | 12,111 | 2,332 |
| Unique paths | 12,111 | 2,332 |
| Label 1 | 2,603 | 663 |
| Label 2 | 3,000 | 742 |
| Label 3 | 2,367 | 593 |
| Label 4 | 1,355 | 334 |
| Label -1 | 2,786 | 0 |

The visible train and validation paths have zero overlap. This does not prove independence at recording-session, vehicle, day, physical-event or road/site level because grouping metadata and raw payloads are not in Git.

The loader treats label -1 as unlabeled: mode labeled filters it, while mode all retains it. The public tree does not include the training objective, pseudo-label procedure, loss weighting or configuration explaining how these 2,786 unlabeled training rows are used. Validation contains only labels 1–4. This is an unresolved reproduction gap, not evidence of what the authors did in their full experiment.

## 4. Code-level findings

### 4.1 Computed denoised data is discarded by the main path

The preprocessing loader computes both raw and wavelet-denoised arrays. The main run_das_pcse function receives both, but discards the denoised array and calls the next stage with the raw array. Thus the visible main entry point computes wavelet denoising and then does not feed that result into the bandpass/superlet/envelope chain, despite the README describing wavelet denoising as part of DAS-PCSE. This must be clarified with the authors and source-exact pipeline before comparing metrics. It does not establish which data path the paper's reported results used.

### 4.2 Superlet dependency is not portable in the public tree

The preprocessing imports superletcx. If import fails, it adds a developer-specific absolute path under /home/satadist/sqs/Das_process/new_das and retries. The implementation itself is not present in the observed tree, and no environment lock is included. A clean reproduction needs the exact dependency source/version, licence, configuration and runtime lock.

### 4.3 Further unresolved pipeline questions

- Per-file acquisition metadata is not visibly compared before arrays are stacked; sample rate, spatial interval, channel count and clock consistency need explicit validation.
- Unparseable timestamp filenames are assigned datetime.min during sorting rather than rejected or explicitly marked unknown.
- Spatial slicing and channel averaging need boundary/empty-window tests plus an exact channel-to-distance convention.
- Per-sample min-max normalization may remove amplitude information; raw, calibrated and normalized ablations are required.
- The classifier defaults to five output classes, while the visible split labels are 1–4 and -1. Label remapping and ignore-index semantics are not documented in a visible training entry point.
- No tests appear in the inspected public tree for preprocessing, model or data-loader behavior.

## 5. Required reproduction gates

1. Obtain explicit software and dataset rights.
2. Freeze the exact complete source revision.
3. Obtain the exact superletcx implementation and dependency environment.
4. Obtain missing extraction, metric, training and inference code or author-confirmed equivalents.
5. Clarify label -1, label mapping, training/validation/test protocol and use of unlabeled rows.
6. Obtain raw waveform and image/mask data with file counts, byte sizes and SHA-256.
7. Validate per-file acquisition metadata, timestamps, ordering and spatial calibration before stacking.
8. Resolve the raw-versus-denoised dataflow difference with the authors.
9. Reproduce the paper's exact preprocessing, split, IoU, accuracy and latency before creating alternate experiments.
10. Separately test event/session/vehicle/site grouped splits to assess leakage-resistant transfer.
11. Evaluate denoising using event-preservation, trajectory topology and timing/localization in addition to SNR.
12. Preserve the paper's traffic results as traffic-task baselines, not utility-fault or pipe-defect performance.

## 6. Luminous opportunity

The stronger contribution is a signal-to-decision benchmark harness that records source bytes, acquisition metadata and transformation receipts; runs raw and candidate denoising/trajectory pipelines side-by-side; measures event/track preservation and topology; tests held-out road/site/session transfer; evaluates speed/classification only with independent truth; and measures latency, memory, energy and storage. A Rust-first reimplementation is a separate candidate method until the author's exact behavior has been reproduced.

## 7. Claim ceiling

This is a public source/metadata audit. No dataset bytes were downloaded, no code was executed, no licence was adjudicated by counsel and no paper metric was reproduced. Public repository behavior may differ from the authors' private paper pipeline; obtain author clarification before treating any source finding as a discrepancy in their reported experiment.
