# PHRI-TAC-ACTIVE-CAL-001 — metrology-constrained active calibration design

**Proposed child direction of:** PHRI-TAC-TRANSFER-001 #4776  
**Physical reference owner:** Symtropy #4753 / #4743  
**Transfer protocol owner:** Symtropy #4776  
**Comparative result owner:** Symtropy #4754 / #4755

## Research question

Can we reduce the number and time cost of target-device calibration contacts while preserving the same independently measured, held-out uncertainty coverage and worst-case error across the declared tactile operating envelope?

This is not a claim that active learning itself is new. The proposed gap is the combination of:
- active selection of the next physically realizable calibration stimulus;
- independent reference-channel and phantom applicability;
- explicit coverage of load, location, shear, curvature/material, temperature and loading-history strata;
- a hard distinction between adaptation data and final validation/test data;
- uncertainty- and guard-band-constrained stopping;
- target-device calibration identity that survives transfer and sensor replacement.

A model may recommend which calibration contact would be most informative. It cannot supply the reference force, create calibration authority, widen the physical envelope or mark itself calibrated.

## Current baseline landscape

The calibration programme must compare against published approaches, not assume a sparse adaptive plan is unprecedented.

1. **TouchWGNN (2026, Frontiers in Robotics and AI, 10.3389/frobt.2026.1791424):** its 113-point distributed tactile array uses an automated per-taxel normal-force calibration procedure. The paper describes 113 calibration positions and multiple target force levels; a two-stage position/force controller waits until the applied force is within ±0.01 N of the target over a stability window before recording. This is a strong fixed-grid automation baseline, not a metrological transfer qualification.
2. **SSTID / sparse strain-node tactile interface (Sensors 2026, 26(4), 1378, DOI 10.3390/s26041378):** optimizes a three-module, nine-channel sensor layout via particle swarm optimization; it uses FEM pretraining (14,400 simulated contacts) and 648 real calibration samples for few-shot sim-to-real adaptation. This is a joint hardware-layout/decoder baseline.
3. **GenForce (Nature Communications 17, 2101, 28 January 2026; DOI 10.1038/s41467-026-68753-1):** transfers force-prediction capability across GelSight, uSkin and TacTip using marker-space translation and material compensation. It reports that its process uses less than 10% of the force-paired images required by its fully paired baseline, plus target location-paired tactile observations. This is a mandatory transfer comparator, not proof that a target measurement has inherited traceability.
4. **TacEva (Advanced Intelligent Systems 2026; DOI 10.1002/aisy.202501179):** proposes sensor-agnostic evaluation of vision-based tactile hardware, standard operating performance and robustness. Use its relevant protocol concepts, while preserving direct-force and non-vision families as distinct observables.

Sources:
- TouchWGNN: https://doi.org/10.3389/frobt.2026.1791424
- SSTID: https://doi.org/10.3390/s26041378
- GenForce: https://www.nature.com/articles/s41467-026-68753-1
- TacEva: https://doi.org/10.1002/aisy.202501179

## Calibration design problem

The physical owner supplies a finite candidate stimulus set tied to the exact phantom and reference revision. Each candidate stimulus must bind:
- indenter identity and measured tip geometry;
- target sensor article, tile, firmware and surface/profile revision;
- spatial position and declared surface frame;
- normal and, where qualified, tangential reference load;
- material/curvature/contact-geometry stratum;
- temperature and other applicable influence conditions;
- loading/unloading branch, dwell/stabilization time and prior history;
- reference instrument, current calibration and uncertainty receipt;
- achievable/rejected status and reason.

The candidate planner receives only the candidate stimulus description, permitted model summary and already-observed adaptation data. It does not receive held-out evaluator truth or final-test outcomes.

The independent reference system owns the actual applied force/displacement labels. A commanded stage position, target controller setpoint or candidate sensor self-report is not a reference observation.

## First executable planner slice

The Rust crate now includes a deterministic coverage-first selection primitive. It filters candidates by exact target unit/profile and phantom revision, current independent reference calibration, qualified method, and within-envelope conditions. It satisfies required strata before using a model-supplied information-per-cost ranking for optional contacts.

This is intentionally narrower than full active experimental design: the current primitive does not compute D-optimality, posterior covariance, nonlinear sensitivity, uncertainty propagation or expected reduction in measurement uncertainty. It does not command the fixture. It only creates a candidate contact list from predeclared eligible stimuli.

The initial tests verify mandatory coverage precedence, stale-reference/method/condition exclusion, target/phantom identity binding, budget shortfall, deterministic tie-breaking, duplicate identities and invalid ranking values. These are software contract tests only, not physical calibration results.

## Compared selection policies

Freeze at least these policies against the same candidate set and sample budget:

- **Full reference grid:** the complete feasible calibration grid; practical accuracy/reference ceiling.
- **Uniform stratified:** balanced fixed counts over the declared force/location/history/environment strata.
- **Random stratified:** seeded random selection subject to the same hard quotas.
- **Maximin coverage:** select contacts to maximize minimum distance to already sampled operating points.
- **D-optimal / information-matrix greedy:** select the next candidate that improves the declared calibration-model information matrix, while retaining all hard coverage constraints.
- **Active predictive-uncertainty:** select where the current model predicts the largest uncertainty, subject to independent reference quality and stratum quotas.
- **Transfer + active adaptation:** initialize with a source sensor model/transfer prior, then select target-device calibration points under a disclosed contact budget.
- **GenForce-style transfer baseline:** reproduce the applicable marker-space / material-compensation protocol when the exact sensor family and dataset are available.

Do not treat D-optimality, maximum model variance or expected information gain as a proof of metrological quality. Each is a stimulus-selection policy. The independent reference measurements and held-out validation remain mandatory.

## Hard constraints before optimization

A candidate is ineligible when:
- phantom / sensor article / indenter identity is unresolved;
- the applicable reference calibration is missing, expired or outside range;
- the reference channel does not cover the stated measurand or loading mode;
- force, displacement, timebase or environmental condition evidence is incomplete;
- the candidate lies outside the target device's declared mechanical/electrical envelope;
- the required target calibration, firmware, mounting or surface revision is stale;
- the contact would violate the physical test-cell owner's authorized non-human operating procedure.

The planner cannot trade these constraints for lower sample count or a higher model score.

## Stopping rule and comparison metrics

Stop only when all preregistered strata and output purposes meet their own independent validation requirements, or when the sample/time budget is exhausted and the result remains MORE_EVIDENCE_REQUIRED.

Report separately:
- number of reference contacts and distinct contact sequences;
- elapsed fixture time, stabilization time and operator/automation time;
- coverage by force, position, shear, material, curvature, temperature and history strata;
- held-out bias, RMSE, worst-case error and residual plots;
- held-out uncertainty coverage and interval width under the exact uncertainty method;
- conformity/guard-band disposition for the owner-supplied decision purpose;
- false-valid rate, abstention rate and out-of-profile rate;
- target-unit/lot/family generalization and worst-article results;
- recalibration required after aging, remount, firmware or surface replacement;
- end-to-end calibration cost per target unit.

The principal plot is **reference contacts / elapsed calibration time versus independently held-out uncertainty coverage and worst-case error**, with a curve for every selection policy. Compare policies at the same physical reference, coverage strata and confidence/coverage target. A cheaper calibration is not better if it misses an envelope corner or under-covers uncertainty.

## Proposed acceptance gates

- Zero target-test information leaks into policy selection, adaptation or stopping.
- Zero candidate stimulus is counted as a reference contact without a valid independent reference receipt.
- Zero hard-constraint violations accepted by the planner.
- A selected plan covers every preregistered mandatory stratum or returns an explicit unresolved status.
- Any claimed calibration-burden reduction is demonstrated at matched target-unit uncertainty coverage and worst-case error, not only average MAE.
- All seed, candidate-set, policy/model revision and decision receipt data are reproducible.
- Failed, rejected and unselected candidates remain auditable; no post-hoc deletion from denominators.

Numerical thresholds must be set for the exact target sensor, measurand, reference method and downstream purpose before confirmatory experiments. This document does not invent a universal force tolerance or uncertainty target.

## Cross-project reuse

- Symtropy #4753 / #4743 owns the phantom, reference channels and fixture conditions.
- Symtropy #4776 owns transfer-regime and calibration-transfer semantics.
- Symtropy #4754 / #4755 owns the tactile-coupling comparative scorecards and receipts.
- Mycelix owners are consumed only where their exact instrument/calibration/certificate/timebase/uncertainty contract applies; no duplicate uncertainty engine or certificate ledger is permitted.
- Symthaea #5828 / #5831 consumes task/representation and fault-detection results. HDC/CfC may propose candidate contacts only after a matched baseline comparison; it is not a physical reference.
- Sol-Atlas remains a read-only outcome/spatial consumer and does not recalculate calibration.

## Claim ceiling

This protocol is research design only. No active-selection algorithm is shown to reduce physical calibration cost, improve force accuracy, preserve uncertainty coverage or outperform the fixed grid yet. No sensor has been calibrated by this document. No purchase, human-contact, actuator, certification or deployment authority is created.


## Executable synthetic policy comparison added — v0.1

A standalone Rust experiment now exists in `src/active_benchmark.rs`, and the CLI emits its policy scorecard.

The frozen synthetic harness uses:
- 72 target candidate contacts across four temperature × loading-phase strata;
- 72 source reference contacts from a deliberately shifted source response profile;
- 60 target-held-out cases with interleaved force levels and contact conditions;
- a 16-contact target budget, with four target reference contacts per mandatory stratum.

The policies are full target grid, uniform stratified, seeded random stratified, maximin, D-optimal, source-only transfer, and source-prior plus D-optimal target adaptation. The output reports source and target reference contacts separately, so source training observations are never counted as target-device calibration.

The reported interval is only a regression prediction-interval diagnostic and the contact-cost quantity is a synthetic proxy. Neither is a GUM uncertainty result or measured fixture time. This tranche does not qualify a physical calibration, establish a physical transfer advantage, or authorize operation of the phantom.

Next evaluation gate: reproduce the result over a preregistered seed set; investigate model misspecification and worst-stratum performance; retain all negative results; then compare the selection policies at matched target reference-contact budgets and matched held-out uncertainty requirements once the physical reference chain is qualified.
