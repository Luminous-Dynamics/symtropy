# Buried Utility Robot Localization — Qualification v0.1

**Status:** Research / benchmark contract only. No localization implementation or robot has been qualified.  
**Parent:** [SUF programme #4910](https://github.com/Luminous-Dynamics/symtropy/issues/4910)  
**Duct qualification:** [#4913](https://github.com/Luminous-Dynamics/symtropy/issues/4913)  
**Localization workstream:** [#4925](https://github.com/Luminous-Dynamics/symtropy/issues/4925)

## 1. Why localization deserves its own qualification

An inspection system can detect a defect correctly and still fail operationally if it cannot tell a maintenance crew where to find it. Conversely, a low average trajectory error does not establish that the robot can select the correct branch, avoid a false access-node association, report a calibrated uncertainty interval or be recovered after losing localization.

The deliverable should therefore be a **task-adequate location claim**:

- route / segment identity, chainage and coordinate reference;
- uncertainty and evidence quality;
- the anchor and map revision used to place that chainage in the wider network;
- sensor, calibration, firmware and time-synchronization identities;
- observed / unobserved / unusable route coverage;
- a decision to accept, reject, abstain or declare the task unsupported;
- independent ability to relocate the reported point during maintenance.

A 1D chainage result must never be represented as a 3D robot pose or a globally registered utility location.

## 2. Current research baseline — important new result

Worley et al., *Acoustic–Visual Sensor Fusion for Robot Localization in Buried Pipes*, first published 23 October 2026 in the *Journal of Field Robotics*, combine direct acoustic impulse-response measurements with optic-flow visual odometry. They compare particle filtering (PF) and pose-graph optimization (PGO).

The paper's experiment used four trajectories at one site:
- 32 m in a 300 mm diameter pipe;
- 30 m, 30 m and 12 m in three 450 mm diameter pipes.

Acoustic measurements were collected periodically (about once every 10 seconds) while the robot was stationary; video was recorded at 30 FPS and processed at a lower effective rate. Ground-truth chainage came from a tether rotary encoder, with stated 0.01 m precision.

Reported results include:
- the acoustic–visual PGO method had a 0.5 m threshold-error rate of 0 for each of the four trajectories under the paper's evaluation;
- the combined error rate across the paper's parameter sweep was 0.09 for acoustic–visual PGO, compared with 0.21 for acoustic–visual PF, 0.26 for acoustic-only PGO, 0.47 for the benchmark acoustic method and 0.69 for visual odometry alone;
- average position error was below 0.15 m across the four pipes, and as low as 0.023 m on one pipe.

These numbers are **paper-specific reference results**, not a universal industry benchmark and not cable-duct results. The authors explicitly limit the current localization state to 1D position along the pipe axis; data came from one site, three of four pipes shared material and diameter, and generalization to cable ducts and other conditions remains future work. The article says the supporting data are available from the corresponding author on reasonable request. Obtain and record data-use terms before using or redistributing those data.

### What the paper changes in our design

It gives us a credible state-of-the-art baseline to reproduce rather than a fictional target. It also exposes the next opportunity:

1. Acoustic echoes can periodically correct visual-odometry drift.
2. Visual motion estimates can initialize / disambiguate acoustic measurements.
3. Pose-graph optimization performed better than the tested particle filter in this 1D experiment.
4. Fixed-rate stationary acoustic measurement adds time overhead. A decision-value policy could choose *when* another measurement is worth the stop, energy and schedule cost.
5. The article's own limitations make cable-duct transfer a genuine open research question—not something we may assume.

## 3. Paradigm shift: active, uncertainty-calibrated localization

### 3.1 From fixed-rate sensing to value-of-information sensing

Rather than sampling acoustics on a fixed cadence, estimate whether a new measurement is likely to change the maintenance decision.

Define the research quantity:

\[
\operatorname{VOI}(a) =
\mathbb{E}[L(d,\hat{x}_{\text{before}})-L(d,\hat{x}_{\text{after }a})]
- C_{\text{time}}(a)-C_{\text{energy}}(a)-C_{\text{disruption}}(a)
\]

where action \(a\) may be continuing motion, stopping for an acoustic chirp, acquiring an extra visual view, reversing to improve an observation, or requesting human localization. The loss function must be task-specific: a fault location that must be found through a particular access chamber has a different tolerance from a general inspection trace.

This is a **research policy**, not a permission to move a robot or manipulate infrastructure. Symthaea may propose a measurement or abstention; the authorized operator remains responsible.

Benchmark:
- localization error and task success per metre, second and joule;
- extra stationary time per accepted location;
- missed opportunities where a measurement would have prevented a wrong location;
- unnecessary stops where the extra measurement did not change the decision;
- calibration and failure rate under held-out route profiles.

### 3.2 Preserve multiple hypotheses under acoustic aliasing

Reverberant pipes can produce repeated or ambiguous echo patterns. Do not collapse competing acoustic correlations into a single confident peak before checking visual, inertial, wheel/tether and route-topology consistency.

Keep multiple candidate associations until evidence separates them. A wrong but confident loop closure is a more dangerous failure than a reported Indeterminate.

### 3.3 Separate local chainage from global route placement

Use a two-layer representation:

- **Local estimator:** relative chainage / pose along one declared duct segment, with uncertainty.
- **Network anchor:** surveyed access-node identity, route topology, coordinate reference, map revision and anchor confidence.

A local estimator cannot silently inherit global certainty from a stale or wrong access-node record. Sol-Atlas owns spatial reference, route topology and source provenance; this benchmark consumes those records read-only.

### 3.4 Qualify task outcome, not just trajectory RMSE

For each reported defect / work point, test whether an independent operator can relocate it within the predeclared task tolerance from the access node. Count wrong segment, wrong branch and failed reacquisition as distinct failures even when the average chainage error looks acceptable.

## 4. Methods and baselines

Use identical observations, ground truth, route profile and evaluation windows wherever a method comparison permits:

| ID | Method | Comparison role |
|---|---|---|
| M0 | Tether encoder + wheel odometry | Strong conventional reference for tethered inspection |
| M1 | Wheel + IMU odometry | Dead-reckoning baseline |
| M2 | Visual odometry only | Continuous motion estimate / drift baseline |
| M3 | Direct acoustic echo only | Absolute / relative acoustic cue baseline |
| M4 | Feature-based acoustic method | Prior acoustic method where faithful implementation/data are available |
| M5 | Acoustic + visual with PF | Published fusion family |
| M6 | Acoustic + visual with PGO | Published leading method to reproduce |
| M7 | Acoustic + visual + IMU / wheel / tether factors | Ablation of heterogeneous factors |
| M8 | M6/M7 plus surveyed route-anchor constraints | Network-context contribution; report anchor information advantage separately |

Vendor-stated capabilities are not measured performance. If raw inputs or code for a published baseline cannot be obtained, mark faithful reproduction **Blocked / Unsupported** rather than fabricating a substitute from plot images.

## 5. Test ladder

### L0 — literature reproduction

1. Request raw data and implementation details from the corresponding author under stated terms.
2. Pin the exact article citation, data receipt, preprocessing, parameter grid and evaluation code revision.
3. Reproduce all four trajectories, including the paper's per-trajectory 0.5 m error rates and the parameter-sweep aggregate.
4. Record any discrepancy before changing methods.

The published 0.5 m threshold exists for comparison with this paper and its cited PAS 128 framing. It is not a universal cable-duct repair tolerance.

### L1 — deterministic synthetic contract

Create analytically known 1D truth trajectories and sensor observations with explicit drift, noise, cadence, time skew, dropouts, repeated echoes and false anchors. Preserve the truth independently from the estimator implementation. The first fixture manifest is a campaign plan, not a generated acoustic waveform dataset.

### L2 — non-energized duct test rig

Use a measured, versioned cable-duct profile with actual wall material, diameter, bend geometry, coupling/joint profile, occupancy and environmental condition. Compare the methods against independent ground truth and the best applicable tether/encoder or conventional proving workflow. Begin with empty duct, draw-rope and inert dummy-cable occupancy only.

### L3 — supervised field evidence

Only after L0–L2 review, data rights, utility / landowner permissions, qualified engineering review and a site-specific safety plan. Sewer, stormwater, cable microduct and larger cable conduit remain separate result strata.

## 6. Metrics

Report per profile, material, sensor configuration and environmental stratum:

- chainage absolute-error P50 / P95 / maximum;
- endpoint error and drift per metre;
- fraction above each frozen task tolerance;
- wrong-anchor, wrong-segment and wrong-branch rates;
- 50%, 90% and 95% interval coverage / calibration on held-out runs;
- localization outage duration, reacquisition time and lost-fix rate;
- high-confidence wrong-location rate and abstention rate;
- performance with individual and correlated sensor degradation;
- CPU/GPU latency, energy per accepted metre and stationary sensing overhead;
- operator intervention and maintenance-point relocation success;
- successful retrieval, recorded separately from localization quality.

The tolerance is frozen before the run from access spacing, repair method, independent truth accuracy and the consequence of a mislocated point. Report the 0.5 m metric as a literature-comparison column, not as a blanket pass/fail limit.

## 7. Frozen hostile-case families

| ID | Hostile condition | Required control |
|---|---|---|
| H01 | Repeated identical couplers / joints create acoustic aliases | Preserve multiple hypotheses; no false high-confidence fix |
| H02 | Visually feature-sparse or blurred duct | Expose visual-only drift / abstention |
| H03 | Acoustic absorption, strong reverberation or multipath | Detect degraded measurement quality |
| H04 | Water, silt, debris or cable occupancy changes impulse response | Treat as a new profile stratum; no silent transfer |
| H05 | Diameter / wall material / bend geometry changes | Reject stale or incompatible calibration |
| H06 | Sensor clock skew, dropped samples or asynchronous streams | Reject or explicitly model time incompatibility |
| H07 | Wrong, stale or ambiguous access-node anchor | No global placement from unsupported anchor |
| H08 | IMU bias, wheel slip or tether slack/friction | Detect disagreement and widen uncertainty |
| H09 | Acoustic or visual sensor dropout | Safe estimator degradation / Indeterminate when required |
| H10 | False loop closure or reversed direction | Independent topology and chronology check |
| H11 | Initial pose outside declared tolerance | Indeterminate until independently re-anchored |
| H12 | Correlated degradation affects multiple modalities | Do not assume independent sensor failures |

## 8. Proposed qualification controls

These are targets, not achieved results:

- 100% rejection of seeded stale-calibration, unit/timebase mismatch, incompatible anchor and unsupported-dimension cases in the frozen finite corpus;
- zero high-confidence localization outputs when the independent oracle labels a sample ambiguous, unobservable or out-of-envelope;
- 100% accounting of route samples as observed, unobserved, unusable, not reached or unsupported;
- all failed, aborted, manually recovered and missing-evidence runs stay in the denominator;
- reproduce the published paper's matched metrics within a tolerance registered before data inspection;
- establish the cable-duct conventional baseline before choosing numerical P95-improvement targets;
- independent maintenance-point relocation is measured separately from estimator error.

No single composite score. Show false confident fixes, abstentions, coverage, latency, energy, task success and retrieval as separate outcomes.

## 9. Luminous ownership

- **Symtropy:** deterministic replay, fixtures, method comparison and independent verifier. Consume #1732 corridor geometry, #1834 capability envelopes and #4913 duct profiles; do not fork them.
- **Symthaea:** propose route hypotheses, sensing cadence and value-of-information actions; reuse the decision-value investigation owner #7727. No dispatch or physical authority.
- **Sol-Atlas:** own route topology, coordinate frames, asset geometry and source / uncertainty provenance. Localization results are observations to be admitted, not permission to overwrite canonical asset truth.
- **Mycelix:** preserve actor, consent, authorization, tool/calibration, raw evidence and independent acceptance provenance under existing owners. A ledger receipt is not a physical truth oracle.

## 10. Claim ceiling

This document defines a research plan. It includes no generated sensor data, no reproduction of the published paper, no measured localization capability, no hardware test, and no cable-duct or field qualification. Synthetic fixtures can establish only the declared synthetic controls. No result authorizes access, excavation, dispatch, isolation, energization, construction or repair.

## References

1. R. Worley et al. (2026), *Acoustic–Visual Sensor Fusion for Robot Localization in Buried Pipes*, Journal of Field Robotics, first published 2026-10-23. https://doi.org/10.1002/rob.70368
2. IEC TR 63439-1-2:2026, *Robotics for electricity generation, transmission and distribution systems — State-of-the-art and standardization roadmap*. https://webstore.iec.ch/en/publication/109098
3. FHWA, *Subsurface Utility Engineering*; ASCE 38-22 / 75-22 quality-level and data-exchange context. https://www.fhwa.dot.gov/innovation/everydaycounts/edc_8/subsurface_utility_engineering.cfm
