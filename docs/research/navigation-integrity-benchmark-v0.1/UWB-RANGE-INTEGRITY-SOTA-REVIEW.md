# UWB range-integrity SOTA and edge-deployment comparison — 22 November 2026

## Why the benchmark must evaluate ranges, not only LOS/NLOS labels

MILUV exposes raw radio timestamps, clock-skew, received power, full CIR and passive-listener receptions. A useful navigation contribution should turn those into a conservative range interval and a calibrated confidence/abstention decision. A binary LOS/NLOS score alone does not establish that an accepted range is sufficiently accurate for a downstream pose or collision-risk decision.

## Recent published comparators

| Method | Publication-reported result | What to reproduce | Important limitation |
|---|---|---|---|
| BERT–LSTM ranging credibility and mitigation | Credibility error 0.047; reported LOS ranging-error reduction 37.9%, NLOS 80.94%; average localization error reduced 69.68% | joint temporal + energy inputs, fine-grained error causes, retain/correct/discard policy | different multi-scenario dataset and hardware; results are not directly transferable to MILUV |
| Physics-informed evidential NLOS calibration | MAE below 0.12 m across multiple datasets; predicts calibrated distance with aleatoric and epistemic uncertainty | nonnegative delay, Gaussian-derivative pulse and multipath-energy priors; single-pass epistemic/aleatoric output | abstract-level reported result; exact matched MILUV config and per-domain tail coverage must be obtained |
| MCU-optimized CIR truncation + lightweight attention | 63 CIR samples selected; NLOS recall 93.68%; quantized STM32 deployment | complexity/accuracy trade-off and true edge hardware measurements | recall alone says nothing about false-positive rate, range-bias tail or calibration |

Sources:

- Yang et al., Measurement 256 (2025), 117721, DOI 10.1016/j.measurement.2025.117721: https://doi.org/10.1016/j.measurement.2025.117721
- Sun & Gao, Measurement Science and Technology 37 (2026), 256308, DOI 10.1088/1361-6501/ae7c20: https://doi.org/10.1088/1361-6501/ae7c20
- IEEE Communications Letters, DOI 10.1109/LCOMM.2026.3662481: https://doi.org/10.1109/LCOMM.2026.3662481

## Luminous research hypothesis

Combine channel evidence and measurement integrity without equating a high LOS probability with a safe range:

1. raw DS-TWR exchange timing and clock-skew;
2. first-path/leading-edge position, peak power, peak-to-noise and CIR decay features;
3. passive-listener receptions and shared-obstruction dependency labels;
4. per-radio antenna-delay calibration and uncertainty;
5. anchor/tag geometry and observation age;
6. a calibrated positive NLOS-bias interval, or an explicit unbounded/abstain state;
7. downstream range-graph and pose-level integrity propagation.

The currently implemented DS-TWR interval kernel covers only item 1, conditional on caller-supplied duration-error bounds. Its optional positive-bias interval correction defines how a separately qualified NLOS bound can compose with timing uncertainty. It does not yet extract CIR features or infer the NLOS-bias bound.

## Matched benchmark protocol

### Baseline matrix

- threshold rules using received signal power and first-path peak;
- handcrafted CIR features plus logistic regression / random forest / SVM;
- CNN/LSTM over a predeclared CIR window;
- published BERT–LSTM temporal+energy credibility method;
- physics-informed evidential model with aleatoric/epistemic decomposition;
- MCU-optimized 63-sample CIR model;
- raw DS-TWR baseline with the published ranging/skew correction;
- DS-TWR interval arithmetic with timestamp uncertainty;
- interval plus positive NLOS-bias bound and explicit reject/abstain action.

### Required metrics

- NLOS recall and LOS recall at fixed false-alert budgets;
- false-accepted NLOS range fraction and false-safe pose exposure;
- positive range-bias p50/p95/p99/max;
- interval coverage and interval width by LOS/NLOS cause, material and geometry;
- risk–coverage/selective prediction curve and calibration error;
- downstream pose p50/p95/p99/max and divergence rate;
- range-graph consistency and anchor/peer exclusion latency;
- passive-listening value per joule and per additional observation;
- MCU/edge memory, cycles, energy, latency and communication cost.

### Generalization and leakage control

Split on whole MILUV experiments, not randomly sampled CIR windows. Use leave-one-obstacle-material-out, leave-one-anchor-constellation-out, leave-one-trajectory-class-out, leave-one-robot-out and leave-one-tag-count-out. Keep all windows from an exchange and the corresponding passive listener observations in the same partition. Calibrate the range-bias bound on separate experiments; do not infer it from evaluator `gt_range` or Vicon fields during test.

## Promotion target

First reproduce the published MILUV baseline errors. Then require the proposed model to improve false-accepted range exposure and tail-error coverage at a fixed false-alert/availability budget, while reporting interval width, edge cost and worst-domain performance. A higher NLOS recall alone is not enough; a classifier that rejects every range has high recall but zero service value.

No UWB/CIR model or MILUV physical result has been implemented or reproduced by Luminous in this work.