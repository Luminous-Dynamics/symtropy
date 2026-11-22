# EHD-PIV-UQ-001 — spatially resolved PIV quality, uncertainty coverage and PIV/LPT reconciliation

**Status:** summary-level Rust reference and 26 authored unit tests across classifier, probability-score and diagnostic-comparison modules; Rust execution and exact-head CI pending  
**Published classifier comparator:** PIVPostProcess, *Experiments in Fluids* (2026), DOI 10.1007/s00348-026-04319-7; code DOI 10.5281/zenodo.18794853. The implementation now reports both class polarities and macro metrics so score comparisons cannot silently depend on positive-class choice.  
**EHD multimodal comparator:** Pöppe, Rich & Kriegseis, LXLASER 2026, KIT institutional record 1000197349

## Why F1 is not a PIV quality certificate

PIVPostProcess is a strong current vector-validity baseline. It reports F1 0.992, ROC-AUC 0.998 and log loss 0.037 on a stratified random vector-level test set (250,000 training and 50,000 testing vectors from six experiments), and mean F1 0.990 across six additional experimental datasets. It uses expert manual post-processing as the reference for the six additional datasets.

The same paper exposes important gaps:
- subtle synthetic noise at 150% of clean-flow RMS and 1% spatial coverage gives mean F1 0.62;
- the MHD DNS dataset gives F1 0.68;
- high-noise synthetic cases can produce F1 around 0.91;
- the reported classifier is a vector-validity model, not a calibrated per-vector velocity-uncertainty estimator.

Therefore the benchmark must separately report vector validity, false removal of valid vectors, interval coverage, spatial-region coverage and cross-diagnostic agreement. Expert labels are valuable but not independent physical velocity truth.

## 1. Vector validity: do not hide valid-vector removal

The internal confusion-count convention treats an invalid vector as the positive class:
- TP: invalid vectors correctly rejected;
- FP: valid vectors incorrectly rejected;
- FN: invalid vectors retained;
- TN: valid vectors correctly retained.

For fair comparison, the evaluator now reports both invalid-class and valid-class precision/recall/F1, macro-F1, balanced accuracy, Matthews correlation coefficient (MCC), false valid-vector removal and valid retention. A published single F1 score must not be compared directly until its positive-class convention and underlying confusion matrix are aligned. Prospective gates are invalid-class F1 ≥ 0.97, invalid-vector recall ≥ 0.90 and false valid-vector removal ≤ 1%. Those are targets, not results.

A vector cleaner that improves F1 by removing a coherent near-wall shear layer may be scientifically worse. The false-removal rate is therefore explicit.

## 1b. Probability scores: compare more than one threshold

The PIVPostProcess paper reports ROC-AUC 0.998 and log loss 0.037 in addition to F1. Confusion-count metrics alone cannot reproduce either statistic because they discard the predicted probabilities and ranking. The new `piv_classifier_scores.rs` module evaluates the supplied probability that each vector is invalid and reports:

- tie-aware ROC-AUC using average ranks;
- binary log loss in nats/sample, clipping exact 0/1 probabilities at 1e-15 only to keep finite scores;
- Brier score as a strictly proper probability score;
- ten-bin reliability data and expected calibration error (ECE) as descriptive calibration diagnostics;
- overall and per-region reports for bulk, near-wall, shear layer and wake.

The benchmark must compare probabilities on exactly the same vector labels and holdout cohort. The adapter includes a polarity-complement invariance test to guard this conversion. Report ROC-AUC and proper scores before selecting a threshold; then report both class-specific F1, macro-F1, balanced accuracy, MCC, false-valid removal and retention. ECE is binning-dependent and must never replace log loss or Brier score. A one-class region has undefined ROC-AUC, which remains explicitly unavailable rather than being coerced to zero.

## 2. Velocity uncertainty: calibrate by physical region

For nominal 95% prediction/measurement intervals, the proposed empirical coverage band is 90–98%. The kernel reports a Wilson 95% binomial interval and disposition separately for:
- bulk flow;
- near-wall region;
- shear layer;
- wake.

A minimum 200 samples per region is a prospective evidence-volume threshold. Missing or undersampled regions cannot be rescued by a large bulk-flow sample. No aggregate-only pass is permitted.

The Rust module verifies supplied coverage counts and calculates the regional summary. It does not estimate velocity uncertainty from image pairs or generate calibrated intervals.

## 3. Cross-diagnostic comparison: PIV and LPT/STB

The LXLASER 2026 EHD presentation by Pöppe, Rich and Kriegseis describes two complementary routes:
- planar PIV for high-resolution near-wall velocity and an integral momentum estimate that accounts for velocity fluctuations;
- four-pulse Shake-the-Box LPT for particle trajectories and material acceleration, enabling a differential Navier–Stokes momentum approach.

The source reports that LPT velocity fields agree with PIV velocity fields and that wire-to-grid acceleration fields reveal channel-like acceleration regions and a significant pressure-gradient contribution.

The executable receipt compares matched 3-component velocity estimates by coordinate frame and condition identity. It adds absolute componentwise uncertainty bounds rather than assuming independent Gaussian errors or using RSS. It also discloses whether PIV and LPT share the same acquisition identity. Different acquisition IDs do not prove independent errors; shared raw images/seeding/illumination must remain visible.

The module does not calculate thrust from LPT acceleration. A differential momentum estimator must still include pressure gradient, density, viscous/body forces and uncertainty. Existing control-volume and direct force-balance receipts remain the proper force qualification route.

## 4. Benchmark against published PIVPostProcess and classical UQ

Compare PIVPostProcess and classical PIV-UQ methods on the same raw image sets:
- image-matching uncertainty;
- particle disparity;
- correlation statistics;
- correlation peak ratio / signal-to-noise measures;
- PIVPostProcess vector-validity classifier.

Freeze the source data, masks, hyperparameters and annotation reference. Add full geometry/session/tracer/pressure holdouts, and separate expert annotation agreement from independent Pitot/hot-wire/LPT/force evidence.

For every method report:
- valid-class and invalid-class precision/recall/F1, macro-F1, balanced accuracy and MCC on the same manually annotated vector labels;
- tie-aware ROC-AUC, log loss, Brier score and reliability diagrams on the same probability labels and frozen test cohort;
- false valid-vector removal and retention;
- spatially resolved interval coverage, width and proper score where full distributions are available;
- vector-field completeness and masked area by region;
- sensitivity to subtle noise near the flow RMS;
- runtime including image preprocessing and evidence export.

## 5. Proposed EHD benchmark gates

Targets, not achievements:
- F1 ≥ 0.97 on matched vector labels;
- invalid-vector recall ≥ 0.90;
- false removal of valid vectors ≤ 1% on sham/dummy/reversal and expert-confirmed valid controls;
- nominal 95% interval coverage 90–98% separately in bulk, near-wall, shear and wake regions;
- ≥200 coverage samples per region before making a regional calibration claim;
- PIV/LPT component residuals within conservative combined uncertainty bounds, with acquisition dependence disclosed;
- no boundary-layer claim when spatial footprint is too coarse for the measured boundary-layer thickness;
- no PIV-to-thrust promotion without all-surface momentum, pressure/viscous/body force, flow fluctuation and uncertainty terms, and a direct force balance.

The Jiang, Noelle and Zwolińska-Glądys source cohorts must remain different geometries/conditions. Do not train one pooled PIV model across tracer, pressure and surface conditions and then infer universal transfer.

## Limitations

The Rust module is a summary-level receipt: it computes metrics from supplied confusion counts, checks coverage counts and compares supplied PIV/LPT vectors. It is not a PIV image processor, correlation-plane uncertainty estimator, particle tracker, CFD solver, pressure-gradient solver or thrust reducer. It does not validate physical EHD measurements. The numerical examples are manufactured.
