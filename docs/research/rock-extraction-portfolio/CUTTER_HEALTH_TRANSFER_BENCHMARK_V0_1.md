# Cutter Health Transfer Benchmark v0.1

**Parent issue:** #3846  
**Composes with:** #3305, #3591, #3823/#3824, #3830/#3834, #3812/#3814, #3841, #3853  
**Claim ceiling:** evaluation-manifest integrity and benchmark protocol. This benchmark does not qualify a cutter, sensor, predictor, maintenance policy, TBM or excavation operation.

## 1. Research objective

Move from predicting a wear label on one platform to estimating the physical condition and future risk of an identified cutter under changing rock, machine, geometry and sensor conditions.

The benchmark separates six tasks:

1. Load reconstruction: infer normal, rolling and side loads from instrumented signals, with uncertainty and a calibrated reference.
2. Wear state/severity: distinguish normal wear, uneven/eccentric wear, ring fracture/chipping and other declared physical states.
3. Event detection: detect fault and failure events with event-level false alarms, missed events and detection delay.
4. Remaining useful life (RUL): produce a calibrated distribution or interval, with right/interval censoring and competing failure modes preserved.
5. Sensor integrity: distinguish actual tool degradation from sensor drift, disconnection, mounting changes and calibration loss.
6. Applicability: detect geology, machine, cutter-geometry and operating regimes outside the model's evidence domain; abstain rather than force a prediction.

A prediction is not a physical inspection, maintenance authorization or machine-control command.

## 2. 2026 reference results — reproduce, do not conflate

These are useful reference points with different targets, datasets and splits. They are not one leaderboard.

- **Wang et al., Engineering Failure Analysis 181 (2026), DOI 10.1016/j.engfailanal.2026.111153.** A mechanism-informed feature/temporal/diversity-attention approach uses operational TBM parameters and wear-theory-informed labels for front and edge cutters. The article preview reports 95% average test accuracy, with wear estimates updated every 100 mm, on Mumbai tunnel data. This is an in-study reference; a random or within-project test does not establish external-site, rig, cutter-geometry or prospective transfer.
- **Li et al., Engineering Applications of AI 181 (2026), DOI 10.1016/j.engappai.2026.115682.** An instrumented cutter combines temperature/vibration with physics/data-constrained load reconstruction. The authors report 94.29% of sequence samples with reconstructed-load R² above 0.95 and approximately 96.5% wear-state recognition on a full-scale rotary cutting platform. The described 70/30 split is a platform-level baseline, not proof of transfer.
- **Li et al., Automation in Construction 187 (July 2026), DOI 10.1016/j.autcon.2026.106950.** A semi-supervised, multi-task wear-rate framework reports average test R² of 0.867 on a real-world dataset and addresses scarce direct wear labels. It is a separate target and data regime from the classification studies above.
- **Zhao et al., Transportation Geotechnics 61 (June 2026), DOI 10.1016/j.trgeo.2026.102039.** A ring-level cutter-consumption workflow combines conformal prediction and cost-sensitive maintenance thresholds. The reported 90% interval coverage and error metrics belong to that study's target and split; do not compare them numerically with wear-state accuracy or instantaneous wear-rate R².
- **Li et al., Measurement (2026), DOI 10.1016/j.measurement.2026.120356.** An uncertainty-budgeted in-situ measurement workflow reports under-20% transfer error under similar mechanical conditions. Similar-condition transfer is not a claim of arbitrary site or machine generalization.
- **Zhang et al., Lubricants 14(8) (13 August 2026), DOI 10.3390/lubricants14080312.** Post-replacement 3D scans provide morphology labels for eccentric-wear screening. Its morphology grades are cohort-relative, not universal engineering thresholds.

The comparator record must state target definition, target units, sample count, class prevalence, label source, test split, prediction horizon, censoring, uncertainty method and physical scope. A larger headline percentage on different test rows is not a valid superiority claim.

## 3. Evaluation protocol: prevent identity and time leakage

Every sample must retain identifiers and provenance for the dimensions available in the campaign:

- cutter serial and installation/replacement episode;
- parent rock block, specimen and rock cohort;
- source study, campaign and test run;
- site, tunnel/mine, TBM/rig and machine configuration;
- cutter geometry, material, radial position and wear state;
- operator, acquisition clock, sensor position/mounting and time window;
- operating regime, geology/face composition and inspection interval.

Required split classes:

| Split | Permitted claim |
|---|---|
| Random-row diagnostic | Debugging only; never transfer evidence |
| Parent-block/specimen grouped | Generalization to held-out physical specimens/blocks |
| Rig / machine / geometry grouped | Transfer to a held-out rig, machine or cutter configuration |
| External-site | Evaluation on a site absent from training and calibration |
| Prospective-field | Frozen model evaluated forward in time after the prediction protocol is locked |

Do not place adjacent windows, repeated cuts, the same cutter serial, sister specimens from one parent block, or post-inspection records in both training and test when the corresponding dimension is declared as held out. Calibration data must be identified separately from the test set. A test site used for calibration is not an external-site holdout.

A manifest records immutable train, calibration and test sample digests, counts, group IDs and holdout dimensions. A semantic validator checks disjointness; JSON Schema alone cannot prove group independence.

## 4. Physical ground truth and metrology

Wear labels should be tied to independent teardown/inspection measurements where feasible: calibrated 3D scans, dimensional metrology, microscopy or another declared inspection method. Preserve:

- measurement instrument, calibration and traceability reference;
- resolution, repeatability, reproducibility and uncertainty budget;
- assessor independence and blinding status;
- cutter identity, radial/edge/front position and inspection access limitations;
- installation and replacement times, operating hours, excavated volume and geology interval;
- label construction rule and any operator/expert adjudication.

Wear-theory pseudo-labels, machine parameters, vibration, temperature, reconstructed loads and model outputs are predictors or derived labels, not independent inspection truth. Their use is permitted, but the manifest must not describe them as independent teardown labels.

For load reconstruction, the reference is a separately calibrated load/force measurement, not the same vibration signal used as input. For RUL, preserve right-censored and interval-censored examples; a cutter that has not failed by the end of observation is not a failure-free infinite-life label.

## 5. Required metrics

Report each task independently, by split and by relevant subgroup.

### Load reconstruction
- per-axis MAE, RMSE, signed bias and R²;
- peak and impulse errors where relevant;
- prediction interval coverage and width;
- sensor/machine calibration uncertainty separate from model residual.

### Wear classification and severity
- macro-F1, per-class precision/recall and confusion matrix;
- severe-wear recall and class prevalence;
- regression MAE/RMSE/bias for measured physical wear dimensions;
- morphology-specific error, not only mean wear depth.

### Fault/event detection
- event-level recall and missed critical events;
- false alarms per operating hour and per accepted excavated volume;
- detection delay and lead time before inspection/failure;
- performance by fault type and severity, not only pooled accuracy.

### RUL and maintenance
- MAE only on explicitly defined uncensored estimands;
- survival-aware likelihood/concordance and pinball loss where appropriate;
- 90% and 95% interval empirical coverage, width and subgroup calibration;
- early/late replacement loss under a declared action-cost model;
- prospective decision utility, downtime and inspection burden only when actions and outcomes are independently observed.

### Sensor and applicability
- drift/fault detection recall and false alarms;
- OOD recall, false-acceptance rate and abstention rate;
- coverage/error by site, geology, rig, machine, cutter geometry, radial position and sensor configuration;
- worst-group tails and calibration degradation, not only pooled means.

No metric should be computed from test labels used during training, threshold selection or interval calibration.

## 6. Model tournament

Evaluate on identical frozen partitions, labels and horizons:

- B0: last-observation / persistence and cohort-median baselines;
- B1: transparent wear-theory or empirical baseline;
- B2: conventional tabular regression/classification;
- B3: temporal CNN/GRU/TCN or attention model;
- B4: semi-supervised / multi-task wear-rate model;
- B5: survival or competing-risk model for RUL;
- B6: physics-informed load/wear model;
- B7: hybrid physics + learned residual / calibrated uncertainty;
- B8: Symthaea research model, only when its predictions can be scored on exactly the same frozen examples.

No model gets credit for using a richer label or easier split. Keep feature-availability timestamps so future inspections, maintenance outcomes and post-outcome geology cannot leak into a forecast.

## 7. Benchmark gates and improvement targets

### Hard conformance gates
- 100% rejection of planted cutter-identity, parent-block, site, rig, geometry and temporal leakage for declared holdout dimensions.
- 100% rejection of test-set interval calibration and non-independent labels presented as independent inspection.
- 100% rejection of impossible counts, invalid digests, mismatched dataset/split references and invalid units/metrics.
- Exact deterministic replay of the frozen manifest validator.
- Every model reports its test partition, feature horizon, label source, uncertainty method and claim ceiling.
- A random-row result is always labelled diagnostic only.

### Scientific targets (hypotheses, not achieved results)
- Reproduce the published in-study baselines only when the original target, data, split and metric are accessible.
- On external-site or prospective test sets, beat the strongest transparent/physics and data-driven baseline with a paired, cluster-aware confidence interval that excludes no improvement.
- As a useful first effect-size target, seek at least a 5 percentage-point absolute macro-F1 gain or 20% fewer nuisance alarms at matched severe-event recall. Preregister the exact primary metric and power calculation before observing the holdout; do not switch metrics after seeing results.
- Nominal 90%/95% RUL intervals should achieve coverage consistent with a preregistered uncertainty tolerance and be no wider than the strongest comparable baseline. Report sample count and clustered uncertainty; do not use a fixed tolerance when the cohort is too small to support it.
- Zero missed catastrophic faults in the frozen adversarial conformance set. This is a challenge-corpus gate, not a real-world safety guarantee.
- Demonstrate that OOD/abstention prevents confident predictions on deliberately shifted rock, rig, cutter geometry, sensor mounting and operating regimes.

### Decision-value target
A better prediction is not automatically a better operation. Only after the prediction benchmark passes should it be connected to #3853: compare a predeclared inspection/maintenance policy against a baseline under the same costs, downtime, production and safety constraints. No live maintenance instructions or autonomous control are emitted by this crate.

## 8. Adversarial corpus

Include independent cases for:
- adjacent windows from one cutter serial split across train/test;
- same parent block or sister specimens across train/test;
- same source campaign under different row identifiers;
- changed cutter diameter, edge geometry, material or radial position;
- new rig/site with different stiffness or sensor mounting;
- sensor drift mimicking wear, and wear mimicking sensor drift;
- missing/late inspection and selective teardown access;
- right-censored RUL and competing failure modes;
- operator compensation changing thrust/penetration without a wear-state change;
- geology transition, mixed-face cutting and unseen abrasivity;
- label built from the predictor signal itself;
- test labels used for threshold or conformal calibration;
- calibration/test site overlap in an external-site claim;
- severe event absent from the test cohort;
- class imbalance hiding poor severe-wear recall;
- uncertainty intervals that are narrow but miscalibrated;
- model abstention counted incorrectly as a correct classification.

## 9. Integration

- #3846 owns cutter-health transfer research; this PR implements the first manifest/semantic-validation slice.
- #3841 owns cutter/rock physics and force/fracture model comparison.
- #3305 owns ring-level TBM geology, energy, cutter and maintenance data.
- #3591 owns metrology reproducibility.
- #3823/#3824 and #3830/#3834 own mechanism-response channels and their evidence validator.
- #3812/#3814 own preregistered full-cycle trial data and accepted-output accounting.
- #3853 owns robust excavation strategy decisions; this benchmark supplies evidence only after independent evaluation.
- #3631 owns accepted lifecycle cost.

## 10. Execution and claim boundary

This v0.1 slice validates the evaluation manifest and its declared split/label/metric semantics. It does not train a predictor, reproduce a publication, authenticate source artifacts, independently inspect a cutter, prove field transfer, set maintenance intervals, guarantee RUL, authorize work or control machinery.

The fixture is fabricated and exists only to test validator behavior. Rust tests are authored but must be executed with Cargo before merge; a structural fixture check is not a Rust test pass.
