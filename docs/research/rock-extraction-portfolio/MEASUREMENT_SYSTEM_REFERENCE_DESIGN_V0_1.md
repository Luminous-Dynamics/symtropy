# Instrumented hard-rock material-transformation pilot: measurement-system reference design v0.1

**Purpose:** define the minimum evidence-producing architecture for a laboratory/bench pilot that feeds #3876, #3910, #3913, #3890, #3927 and #3961. This is a metrology architecture, not construction drawings, purchase specifications or operating instructions.

## 1. Design principles

1. Measure every physical branch that can carry solid material. An unmeasured branch remains UNKNOWN, never zero.
2. Separate independent reference measurement from online sensor estimates.
3. Capture time at acquisition, retain clock/latency uncertainty, and keep raw records immutable.
4. Qualify the measuring system before interpreting any physical process result.
5. Match the measurement resolution and sample design to the smallest effect the experiment is intended to detect.
6. Preserve raw images, point clouds, weighing data, sensor streams, calibration records, exclusions, source-cohort IDs and exact software/model digests.
7. Propagate uncertainty through nonlinear measurement transformations using distributions where justified; do not assume independence merely because covariance data are absent.
8. No operating limit, setpoint or safety envelope is inferred by this reference design.

## 2. Physical measurement boundary map

The pilot should be designed around explicit sampling/measurement stations:

| Station | Physical question | Primary independent measurement | Online/context evidence |
|---|---|---|---|
| Cutter discharge | What material was generated? | Captured whole-stream sample, dry mass, PSD by mass, morphology sample | Cutter force/torque/vibration and machine-state context where safely instrumented |
| Capture inlet | What fraction entered the removal path? | Dry mass and PSD of captured stream; separately weighed floor residual | Pressure/airflow/current synchronized with feed |
| Conveying inlet | What actually entered the installed path? | Independent dry-solid flow sample and PSD | Differential pressure, airflow, blower current/power |
| Intermediate path/bends | What changes along the line? | Accessible sample ports only where engineered and qualified; weighed inspectable deposits/coupons | Segment pressure differences, temperature and vibration |
| Conveying outlet/separator inlet | What arrived after transport? | Dry mass-flow and PSD sample | Pressure/airflow/current and time/residence evidence |
| Separator coarse outlet | What was separated as coarse product? | Independent mass and PSD | Separator state and differential pressure |
| Filter/fines outlet | What fines/dust were recovered? | Independent collection and weighing under approved dust controls | Filter differential pressure, filter state |
| Accepted discharge | What meets the declared acceptance profile? | Independent accepted dry mass, PSD and quality/acceptance record | Discharge state |
| Floor residual / cleanout / reject | What was not accepted or transported? | Spatial survey plus separate weighing/collection where possible | Event and machine-contact context |

Stations are explicit physical boundaries; a stream is not simply a sensor channel. One station may have multiple material streams or sampling artifacts. A station can be unmeasured; the evidence contract must expose that gap.

## 3. Functional instrument classes

Exact range, accuracy, response time, environmental rating, certification and vendor are **not specified here**. These must be selected from the intended material, line geometry, hazards, measurement objectives and approved equipment envelope.

| Functional class | Minimum evidence role | Independent cross-check |
|---|---|---|
| Calibrated mass measurement | Dry-solid mass at feed, collected outputs, fines, rejects, residue and deposits | Static reference weighing with traceable artifacts |
| PSD measurement | Mass-weighted size distribution for each material stream | Representative sieve/reference method, with sampling/weighting limitations disclosed |
| Dynamic particle imaging | High-count 2D descriptor and size distributions | Certified reference artifacts and blinded repeated scans |
| 3D particle geometry | 3D descriptors for selected samples where acquisition resolves full geometry | Reference geometry artifacts and multi-view/repeat scans |
| Differential pressure | Pressure response by installed path segment | Calibrated reference pressure source and independent transducer check |
| Airflow | Gas-flow context for pressure/solids response | Traceable flow reference or validated independent method |
| Electrical current/power | Blower and drive-state context | Calibrated electrical reference / power analyser |
| Temperature / vibration | Sensor health and equipment-state context where justified | Calibrated reference and known-condition diagnostic check |
| Timebase | Timestamp alignment and residence-time analysis | Common clock event / independent timing verification |
| Wall/liner/coupon metrology | Erosion or wear evidence at declared inspection locations | Traceable dimensional/mass reference before and after inspection |

Electrical-current monitoring is a valuable baseline, but the 2026 current-only pneumatic study is scoped to very dilute negative-pressure conveying; it does not establish current-only reliability on coarse, abrasive hard-rock streams. https://doi.org/10.1016/j.measurement.2026.121117

## 4. Sampling architecture

### 4.1 Whole-stream representativeness

Prefer a validated sampling design that represents the complete stream cross-section and operating interval, not a convenient handful of visible fragments. Document:
- where and when increments are taken;
- how material is randomized, split and preserved;
- how fines, oversize, adhering dust and moisture are handled;
- how sample mass is related to the independent bulk flow measurement;
- how no particle is counted twice;
- how each sample maps to a parent cohort, station and time window.

Do not infer mass fractions from particle counts without a qualified sampling and weighting model.

### 4.2 Paired measurements

Where practical, retain:
- a sealed reference aliquot for independent sieving/gravimetry;
- an imaging aliquot for dynamic 2D morphology;
- a 3D reconstruction aliquot for selected particles or cohorts;
- a retained archive sample for blinded repeat measurement.

Splitting and handling can change segregation and fines representation. Record split method, mass recovery and loss.

### 4.3 Sample sufficiency

Use #3961 to test quantile precision (p05/p50/p95) against:
- instrument resolution;
- reference error envelope;
- independent-particle sampling premise;
- predeclared confidence level;
- maximum interval width.

The November 2026 sample-size paper on large-scale particle imaging explicitly motivates resolution-limited sample size and false-convergence protection. Its specific method is not implemented by #3961; the current crate uses an independently specified exact order-statistic interval and rejects clustered acquisition blocks. https://doi.org/10.1016/j.powtec.2026.122851

## 5. Synchronization and residence-time evidence

All process channels and material samples must be bound to a declared clock protocol. Record:
- source clock and acquisition timestamp;
- timestamp uncertainty;
- hardware/software latency and its calibration;
- synchronization event evidence;
- clock loss, holdover or drift events;
- sample start/end windows;
- residence-time evidence and its uncertainty.

Do not select a fixed timestamp tolerance in this design. Derive the allowable timing error from the shortest physical transient or residence-time feature that the specific hypothesis must distinguish. A sensor signal shifted by a material fraction of the feature duration can create false cause/effect or apparent mass imbalance.

## 6. Measurement-system qualification gates

Before a process campaign:
1. Reference artifacts span the physical feature and descriptor range of interest.
2. Spatial resolution, descriptor resolution, motion blur, depth of field and orientation effects are characterized.
3. Repeated scans span sessions, operators, orientations and instrument restart/recalibration conditions.
4. The measured reference error envelope satisfies a predeclared experiment-owner tolerance.
5. Mass-flow measurements reconcile with independent static weighing within their declared uncertainty.
6. PSD method is compared against an independent reference on representative samples, including fines and oversize.
7. Timebase/latency uncertainty is independently measured.
8. Sensor degradation/failure controls are exercised and recorded.
9. Sample splitting and recovery are checked for mass loss and bias.
10. Calibration expiry, environmental drift, fouling and sensor health are detectable or explicitly flagged.

ISO 13322-2:2021 specifically addresses dynamic particle motion, blur, depth of field, orientation and particulate reference-material qualification. https://www.iso.org/standard/72566.html

## 7. Uncertainty architecture

Separate:
- reference certificate uncertainty;
- instrument calibration uncertainty;
- repeatability and between-session/operator effects;
- spatial/descriptor resolution;
- segmentation/reconstruction uncertainty;
- sampling and split bias;
- mass-flow calibration and tare/drift effects;
- clock/latency uncertainty;
- residence-time uncertainty;
- model-form/discrepancy uncertainty.

Do not combine correlated components using root-sum-square unless the dependence structure is supported. Maintain the measurement equation, units, distributions, correlation/covariance evidence and transformation version. For nonlinear image-to-PSD or geometry transformations, consider Monte Carlo propagation with declared input distributions and reproducible seeds, consistent with the JCGM GUM suite. The 2026 amendment to JCGM 100 explicitly covers nonlinearity in measurement models. https://www.bipm.org/en/web/guest/publications/guides

The conservative interval arithmetic in #3876/#3927 remains appropriate for mass-flow closure; it is not a substitute for a complete probabilistic uncertainty budget.

## 8. Sensor-fusion challenge protocol

Use #3913 to create a frozen event corpus with:
- synchronized pressure differential, airflow, blower current/power, recovered mass rate and filter state;
- independently observed process events and separate sensor-fault labels;
- startup/shutdown transients;
- high-solids flow, stratification, deposition, developing restriction, complete blockage;
- filter loading, water accumulation, air leakage, blower/VFD degradation;
- pressure/airflow sensor drift, fouling, disconnection and loss;
- unknown/out-of-domain material/path conditions.

Hold out complete material cohorts, campaigns, path configurations and time blocks. Compare pressure-only, current-only, pressure+airflow, fully fused and sensor-health/OOD models at matched critical-event recall. Report nuisance alarms per operating hour, detection lead time, calibration and worst held-out subgroup—not just random-window accuracy.

## 9. Physical pilot outputs

The minimum frozen outputs:
- stage-wise and stream-wise dry-solid mass-flow intervals;
- mass-weighted PSD by station and branch;
- morphology distributions by exact parent PSD stratum;
- size-selective losses, residual and deposit mass;
- residence-time and clock uncertainty;
- pressure/airflow/current response and independent event labels;
- reference repeatability/reproducibility and sample adequacy;
- accepted solid volume per complete elapsed cycle;
- total-boundary energy, wear and maintenance/downtime.

A downstream PSD shift is not causal attrition evidence by itself. Attribution requires matched material cohorts, time/residence alignment, measured branch streams and representative independent samples.

## 10. Safety and procurement boundary

A competent design authority and mine operator must separately determine:
- mechanical pressure/negative-pressure design and rating;
- dust/explosion protection, ventilation, guarding and grounding;
- electrical/intrinsic safety and mine certification requirements;
- hose integrity, supports, inspection, cleanout and emergency response;
- allowable operating envelope, test exclusion zones and stop conditions;
- sensor placement, access ports and maintainability.

No line pressure, airflow, solids loading, velocity, cutter depth, clearing sequence or machine operating limit is prescribed in this document.

A purchase-ready BOM requires actual design pressure/rating, particle size/abrasivity/moisture, flow envelope, environment, certifications, site restrictions, calibration range, response time and vendor quotations. None should be guessed from a generic architecture.

## 11. Integration

- #3864/#3876: parent fragmentation, mass and installed-capacity evidence.
- #3910: morphology distribution.
- #3913: synchronized pneumatic event state and sensor health.
- #3890: floor residue and support-contact geometry.
- #3927: stage-resolved stream and PSD flow ledger.
- #3961: reference and sample-adequacy qualification.
- #3814/#3825: full-cycle output and resource accounting.
- #3917/#3853: experiment value, strategy eligibility and decision comparison.

**Acceptance boundary:** this design defines evidence architecture. It does not qualify a physical instrument, laboratory rig, mine, sensor installation or excavation machine.
