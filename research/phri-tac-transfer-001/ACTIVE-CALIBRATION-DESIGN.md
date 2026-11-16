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


## Selector / label / evaluator type boundary

The executable comparison now separates three data objects:
- selector-visible candidate stimuli and derived sensor features;
- training-only reference-force observations, stored separately from candidate records;
- held-out target truth, stored in a separate evaluator-owned domain.

Every selection policy accepts only the candidate slice. Model fitting receives the selected candidate indices and separately supplied reference observations. Held-out truth is passed only to the evaluator after policy selection completes. This prevents a future selector from accidentally reading reference labels through a shared candidate record, and is stronger than an informal "no leakage" promise.

A deterministic reference-channel noise term is separate from the sensor readout noise. This is synthetic reference error, not a real instrument calibration or covariance model.


## Paired multi-seed reproducibility tranche

The executable robustness follow-on requires at least 50 preregistered paired seeds. Each seed reuses the same generated data across policy arms, and each arm is compared with the full-grid reference under the same seed. The summary preserves mean, median, p95 and maximum error, synthetic interval diagnostic, generated contact-cost proxy and paired RMSE delta against full-grid.

This establishes whether the first policy ordering is seed-stable enough to justify further study. It does not yet provide bootstrap confidence intervals, cluster-robust statistical inference or model-family robustness; those remain required before any physical-policy selection. The source/target split and selector/evaluator type boundary remain intact across every seed.


## Paired bootstrap interval

For each policy, the campaign forms one RMSE difference per seed against the full-grid reference from the same seed. It then performs a deterministic 2,000-resample percentile bootstrap over those paired seed-level differences and reports the 95% interval for the mean difference. Each bootstrap draw resamples seed-level pairs; contact samples within a seed are never treated as independent replicates.

The interval is conditional on the frozen synthetic generator and seed-selection mechanism. It quantifies seed-to-seed variability for this generator; it does not establish uncertainty for a physical sensor population, an uncertainty budget, or performance transfer to different hardware/model families. Model-family robustness and real article/lot replication remain required.


## First hosted 50-seed campaign — 16 November 2026

Workflow #78 on source head `a419a63a1a8aa9335d6a2ee0dbcde9f4f7c5d5c6` completed with 40 Rust tests passing, rustfmt passing, strict Clippy passing, the 34-case transfer oracle passing and the 50-seed campaign CLI completing.

First 50-seed campaign output (all values are synthetic model estimates; force units are N):

| Policy | Target refs | RMSE mean | RMSE median | RMSE p95 | RMSE max | Prediction-interval diagnostic mean | Paired RMSE delta mean | Paired bootstrap 95% interval | Synthetic cost ms mean |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Full target grid | 72 | 0.004234 | 0.004234 | 0.004234 | 0.004234 | 1.0000 | 0.000000 | [0.000000, 0.000000] | 45,240 |
| Uniform stratified | 16 | 0.033170 | 0.033170 | 0.033170 | 0.033170 | 1.0000 | 0.028936 | [0.028936, 0.028936] | 10,480 |
| Random stratified | 16 | 0.005848 | 0.005460 | 0.008348 | 0.010340 | 0.9610 | 0.001615 | [0.001273, 0.001995] | 10,019.4 |
| Maximin stratified | 16 | 0.062137 | 0.062137 | 0.062137 | 0.062137 | 1.0000 | 0.057903 | [0.057903, 0.057903] | 10,480 |
| D-optimal stratified | 16 | 0.004986 | 0.004986 | 0.004986 | 0.004986 | 0.9833 | 0.000752 | [0.000752, 0.000752] | 10,400 |
| Source-only transfer | 0 | 0.530757 | 0.530757 | 0.530757 | 0.530757 | 0.1000 | 0.526523 | [0.526523, 0.526523] | 45,240 |
| Transfer + D-optimal adaptation | 16 | 0.005161 | 0.005161 | 0.005161 | 0.005161 | 1.0000 | 0.000928 | [0.000928, 0.000928] | 55,640 |

### Interpretation and limits

- On this single synthetic response family, target-only D-optimal is the strongest sparse policy by mean and worst-case RMSE; it uses 16 rather than 72 target reference contacts and approaches the full-grid reference.
- Seeded random stratification is less stable: lower mean RMSE than a poor fixed design, but higher p95/worst errors and a lower mean regression prediction-interval diagnostic than D-optimal.
- Uniform and maximin are poor for this response basis and candidate geometry. A coverage metric alone does not guarantee parameter identifiability or good inverse-response prediction.
- Source-only transfer fails under the deliberately shifted sensor profile. Transfer plus target adaptation recovers but consumes 72 source and 16 target reference contacts, costs more than target-only D-optimal and does not improve its mean or worst-case RMSE. It is not a calibration saving in this test.
- Several seed-level distributions and bootstrap intervals collapse to a single value because this generator has fixed model coefficients, fixed stimulus geometry and only small deterministic noise changes. This does **not** establish policy stability across sensor articles, lots, response families or real physical units.
- The bootstrap resamples paired seed-level differences, but these seeds are not independent physical instruments. The interval is conditional on this synthetic generator. The pseudo prediction interval is not a GUM uncertainty budget, and the cost values are not measured fixture time.

The next required campaign changes the physical/model response family and introduces model misspecification, nonlinearity, hysteresis, reference common-mode faults, aging/remount and unit/lot variation. No physical acceptance decision should be based on this first campaign.


## Adversarial model-family tranche — five first known answers

A separate executable module adds five response families: nominal, cubic-force misspecification, force × temperature × history interaction, sensor saturation, and common-mode reference bias. The current inverse-model basis is deliberately held fixed across families so that a changed response law can expose estimator and policy failure rather than being silently learned into the assumed model.

Every family compares the same seven policy arms over at least 50 paired seeds. The training labels are generated separately from selector-visible stimuli; the held-out evaluator truth remains separate and inaccessible to selection and fitting. The reference-bias scenario injects a shared 0.25 N offset into training reference observations while leaving held-out truth unbiased, so an internally consistent but wrong fit can be exposed through held-out error and its regression interval diagnostic.

This is a first stress tranche only. It does not yet model physical aging curves, remount geometry changes, dropout, multi-channel common-mode correlation, or independently measured sensor-unit/lot variability. Those remain future response-family extensions and require physical fixture characterization before parameter values can be called realistic.


## Independent reference-channel integrity tranche

The follow-on contract adds a second synthetic reference stream that is not used in model fitting. Qualified synthetic fits require both:
1. pairwise primary/check disagreement within the named synthetic-only tolerance (0.02 N in this executable fixture); and
2. an explicit synthetic reference-ancestry attestation indicating distinct calibration ancestry.

The report always states that pairwise agreement does **not** prove independence. A sixth response family injects the same 0.25 N offset into both reference streams. Its pairwise disagreement is small, but its ancestry attestation is false, so the qualified fitting path rejects it. A separate family biases only the primary stream; the numerical disagreement guard rejects it.

The adversarial campaign still executes the unguarded diagnostic fit to quantify the damage from ignoring a failed reference gate. These policy metrics are counterfactual diagnostics only and must not be treated as qualified measurements. The normal active-calibration benchmark and fit helper now enforce the reference gate before fitting.

The 0.02 N threshold and ancestry boolean are synthetic test-fixture parameters, not physical metrology thresholds or proof of actual instrument independence. Real qualification requires the #4753 owners to establish independent instrument identities, calibration ancestry, timebases, mounting/range applicability, uncertainty, and common-cause analysis.


## Reference integrity: two separate gates

The reference-channel prototype now requires both numeric agreement and an attested distinct calibration ancestry before the normal calibration-fitting path accepts a domain. The synthetic cross-check tolerance is 0.02 N and is named in code as a synthetic-only fixture value. It is not a physical metrology threshold.

A sixth adversarial family, shared reference common-mode bias, gives both numerical channels the same 0.25 N offset. Pairwise disagreement alone would accept it; the fixture therefore marks its ancestry attestation false, and the qualification path rejects it. The primary-only reference-bias family is rejected by a large pairwise difference. The report always states that numerical agreement does not prove independence.

The adversarial campaign retains explicit diagnostic-only functions that bypass reference qualification to measure the counterfactual error from using faulty data. They are isolated from the normal active calibration path and never qualify a measurement. The standard benchmark's source and target fits now require the integrity gate.

Real hardware still requires independent calibration ancestry, calibration validity/range, clock/time alignment, mounting/force-path analysis, uncertainty and common-cause review from the physical phantom/reference owner. A software ancestry boolean is not a calibration certificate.


## First hosted reference-integrity result

Hosted workflow #112 on source head `a431b6cf8d4688d0321e8769da1e0c18eeacbad3` passed 48 Rust tests, rustfmt, strict Clippy, the tracked 34-case transfer oracle, the 50-seed campaign and the six-family adversarial CLI. The subsequent workflow writeback commit `150237be23450c31078f3cc71fcea710a6142196` contains rustfmt-only changes to the source already tested.

Reference integrity results for 72 synthetic target calibration contacts (0.02 N synthetic-only tolerance):

| Response family | Max primary/check disagreement | Ancestry attested | Gate |
|---|---:|---:|---|
| Nominal | 0.004343 N | Yes | Pass |
| Cubic force | 0.004343 N | Yes | Pass |
| Force × temperature × history | 0.004343 N | Yes | Pass |
| Sensor saturation | 0.004343 N | Yes | Pass |
| Primary-only 0.25 N reference bias | 0.254343 N | Yes | Reject |
| Shared 0.25 N reference bias | 0.004343 N | No | Reject |

The last row is the critical common-cause result: both numeric streams agree because both share the injected 0.25 N offset. Pairwise agreement cannot detect that. The separate synthetic ancestry attestation rejects the scenario, and the output explicitly states that agreement does not prove independence.

The 48-test hosted pass belongs to source head `a431b6cf...`. Follow-up commit `150237be...` is formatting-only. The current branch/PR head may move again when a later hosted run writes generated formatting; do not silently treat an action-required bot event as a new test pass.

## New outstanding issue: range observability

The saturation family shows that a reference chain can be internally consistent while the tactile transducer itself has stopped responding at high loads. An independent reference channel does not fix loss of sensor information. This is now tracked separately under PHRI-TAC-OBSERVABILITY-001 / #5616. The future gate must detect flatlined response increments, clipped taxels and unidentifiable ranges and mark those operating points out-of-profile or more-evidence-required rather than interpreting model confidence as observability.
