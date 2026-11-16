# PHRI-TAC-TRANSFER-001 - metrology-preserving tactile transfer protocol

Owner: Symtropy #4776
Physical reference: Symtropy #4753 / #4743
Comparative scorecards: Symtropy #4754 / draft PR #4755
Synthetic executable slice: this directory, v0.1

## Research question

Can a transferable tactile predictor reduce target-device labelled calibration effort while preserving the distinction between a model estimate and a target-qualified physical estimate under held-out sensor units, manufacturing lots, contact sequences, materials, calibration revisions, aging and environmental shift?

The paradigm shift is not to assume that a universal embedding makes sensors identical. It is to treat transfer as a conditional capability with an explicit evidence envelope:
- source experience can inform a target predictor;
- target-unit calibration remains target-unit evidence;
- adaptation and validation samples remain separate from final test samples;
- uncertainty and guard-band validation are required for a target-qualified estimate;
- a model cannot issue, inherit, mutate or replace a calibration certificate;
- outside the transfer/calibration envelope, the output degrades to a model estimate, Unknown, or a recalibration request.

## State of the art and comparator positioning

The benchmark must include, not claim novelty over:

1. GenForce - Chen et al., Nature Communications 17, 2101 (28 January 2026), DOI 10.1038/s41467-026-68753-1. The paper unifies tactile signals as marker representations, translates marker-to-marker, and learns spatiotemporal force prediction across homogeneous and heterogeneous sensors. Its reported examples include GelSight, uSkin and TacTip; the paper reports mean force MAE below 0.92 N for normal force and below 0.22 N / 0.30 N for its two shear axes averaged over six transfer combinations. One uSkin-to-TacTip normal-force result reports MAE improving from 7.76 N to 0.52 N. These are paper-reported study outcomes, not independently reproduced Symtropy results.
2. TacVerse - 106,800 tactile images from seven vision-based tactile platforms, with within-sensor, zero-shot cross-sensor and few-shot adaptation protocols for shape, grating and force-regression tasks. Its paper reports that force regression is notably sensitive to sensor shift and few-shot adaptation does not fully close the gap to within-sensor performance. DOI 10.1002/adrr.70169.
3. RCT - 29,279 DIGIT frames across 1,832 full contact sequences, 122 materials and three sensor instances. It exposes contact-sequence leakage: the paper reports a 17.7 percentage-point Recall@1 drop after removing sequence overlap, and recommends unit-of-contact-sequence rather than frame-random splitting.
4. Sparsh / TacBench - a learned tactile representation baseline where the sensor modality is vision-based tactile input. Keep representation performance separate from force metrology.
5. Within-device fitted calibration and per-target-device calibration - practical upper bounds for task learning and physical measurement on the exact tested articles.
6. No-transfer source model and simple sensor-native baseline - low-complexity controls.

References:
- GenForce: https://www.nature.com/articles/s41467-026-68753-1
- TacVerse: https://arxiv.org/abs/2606.25877 and https://doi.org/10.1002/adrr.70169
- RCT: https://arxiv.org/abs/2606.31694 and https://faerber-lab.github.io/RCT/
- Sparsh: https://ai.meta.com/research/publications/sparsh-self-supervised-touch-representations-for-vision-based-tactile-sensing/

Do not pool image pixels, magnetic fields, resistive taxels, capacitive channels and direct force outputs as if they were one measurand. Compare only independently referenced common outputs for which each exact sensor/method has a valid mapping. A sensor-family bridge is an evaluated hypothesis, not an ontology equivalence.

## Transfer regimes

- within_device: same physical article; hold out complete contact sequences.
- cross_unit_zero_shot: unseen target unit and lot, no target labelled adaptation.
- cross_unit_few_shot: unseen unit/lot with a disclosed, capped adaptation set; final test sequences are disjoint.
- cross_family_zero_shot: different transduction families, only where an exact validated bridge profile exists.
- cross_revision: same physical unit across firmware, geometry, skin/gel or calibration revisions.
- aging: repeated measurements over operating cycles/time, with a declared maximum transfer envelope and drift-triggered recalibration.

## Mandatory split boundaries

Never use frame-random splitting as the primary protocol. Group and report by physical sensor serial/unit, manufacturing lot, contact sequence/press episode, material or physical article, contact position and geometry, calibration revision, firmware and sensor-skin/gel revision, aging/cycle window, and temperature/environmental profile.

Target adaptation, target validation and final test must use distinct sample IDs and contact sequences. Hold out materials/articles when the transfer claim includes material generalization. Report multiple held-out units/lots and their distributions, not one convenient split.

## Separate result families

P1 physical metrology: force bias, RMSE, worst-case error, calibration residuals, uncertainty coverage, guard-band/conformity behavior, cross-axis sensitivity, hysteresis, drift, repeatability, range and recalibration interval.

P2 task prediction: contact localization, normal/shear estimates where physically observable, material/shape/texture recognition, slip timing, grasp stability and manipulation outcomes. Label outputs as model estimates unless the target-unit physical qualification gate is satisfied. A good classification score is not a force calibration result.

P3 transfer efficiency and lifecycle: target labelled contacts required, calibration time, adaptation compute, data volume, inference latency, per-unit commissioning time, recalibration frequency, repair/replacement burden and useful measurement availability.

P4 evidence integrity: split leakage, wrong-unit calibration reuse, source/target identity alias, unsupported sensor-family mapping, profile/revision mismatch, hidden adaptation leakage, stale validation receipt, invalid reference calibration, common-mode dependence, uncertainty coverage failure, false-confidence rate and correct abstention.

Do not compress these into one weighted score. A P2 gain cannot offset a P1 or P4 failure.

## Acceptance targets (prospective)

- Zero sample, contact-sequence, target-unit or target-lot leakage in the declared holdout regime.
- Zero target-qualified measurement claims from source-only calibration identity.
- Zero target-qualified transfer without exact target unit/profile/revision, held-out target validation, current independent reference calibration, uncertainty coverage and applicable guard band.
- Publish zero-shot-to-few-shot adaptation curves and the exact target labelled-contact budget.
- Report target-unit distributions and worst-device outcomes; no mean-only report.
- Demonstrate calibration-burden reduction only at matched target-unit uncertainty coverage and accepted error bounds.
- Drift, age, firmware, geometry, skin and temperature changes trigger explicit requalification/recalibration or out-of-profile status.
- Retain model-estimate-only outputs as useful task evidence where appropriate; do not relabel them as physical measurements.

No universal numerical force threshold or uncertainty coverage target is frozen here. It must be selected for the exact measurand, reference chain, device and purpose before confirmatory data.

## Cross-project ownership

- Symtropy #4753 / #4743: physical phantom, fixture, independently calibrated reference and the fixed 32x32 normal-pressure reference.
- Symtropy #4776: cross-unit / cross-family / cross-revision / aging transfer protocol and data splits.
- Symtropy #4754 / #4755: comparative pHRI benchmark scorecards and provenance-bound run receipts.
- Mycelix metrology owners: instrument/calibration/certificate/uncertainty/timebase records where their canonical, exact contracts apply. Do not create a second certificate or uncertainty engine.
- Symthaea #5828 / #5831: tactile task/representation and synthetic fault baselines. HDC/CfC remains a candidate representation, not a new physical witness.
- Sol-Atlas: read-only spatial/service projections where applicable; no recalibration or reinterpretation of source sample time.

This transfer oracle is not the physical phantom, pHRI controller, a safety authority, a calibration service, or a replacement for the owner-native evidence ledger.

## Synthetic oracle and claim ceiling

cases-v0.json contains 34 synthetic known-answer cases. The Rust oracle validates closed typed structures, split integrity, method applicability, target calibration/revision, independent held-out validation, uncertainty/guard-band evidence and expected dispositions.

A software PASS means only that the synthetic contract implementation reproduces these known answers. It does not reproduce GenForce, TacVerse or RCT, establish real sensor transfer, qualify a calibration, prove uncertainty coverage on hardware, establish physical performance, or authorize human contact or actuation.
