# Buried Infrastructure Localization — State-of-the-Art Baseline Matrix v0.1

**Snapshot date:** 2026-11-12  
**Owner:** Symtropy #4925  
**Programme:** SUF #4910  
**Related duct qualification:** #4913  
**Spatial interchange:** Sol-Atlas #755

## 1. Why there must not be one localization leaderboard

The latest research spans materially different tasks:

- robot chainage within a buried pipe;
- acoustic feature localization and condition classification;
- pedestrian positioning inside a walkable underground utility tunnel;
- general robot state estimation across changing sensor reference frames.

These are not directly comparable. They differ in state dimension, motion model, geometry, sensors, infrastructure assumptions, ground truth and intended task. A single average localization score would be technically misleading.

This matrix separates four populations and a fifth cross-cutting assurance track. Every future Luminous result must identify its population and must not pool metrics across populations without an explicit, justified normalization.

## 2. Current research baselines

### B0 — acoustic–visual localization in buried sewer pipes

**Worley et al., Journal of Field Robotics, first published 2026-10-23.**  
DOI: https://doi.org/10.1002/rob.70368

- Population: four field trajectories in buried sewer pipes: one 32 m × 300 mm route and three 450 mm routes of 30 m, 30 m and 12 m.
- State: one-dimensional chainage relative to the initial robot pose; it does not provide globally anchored 3D pose.
- Methods: dense optic-flow visual odometry; direct acoustic impulse-response localization; acoustic–visual fusion with particle filtering (PF) and pose-graph optimization (PGO).
- Published results: average position error below 0.15 m across the four pipe runs; as low as 0.023 m on one 27.5 m trajectory; zero estimated samples above the paper's 0.5 m threshold on each of the four PGO-fusion trajectories. Its combined parameter-sweep error-rate was 0.09 for acoustic–visual PGO, 0.21 for acoustic–visual PF, 0.26 for acoustic-only PGO and 0.69 for visual odometry.
- Operational cost: the experiment used periodic acoustic measurements roughly every 10 s, with the robot stationary for synchronization. The 300 mm test had about 300 s of stationary acquisition in a 900 s deployment; the authors estimate about 10 s per stop including acquisition/processing in the tested 30 m configuration.
- Data: raw data are available from the corresponding author on reasonable request; access and reuse terms must be obtained.

**Limits:** one site; three of four pipes had the same material and diameter; 1D relative position; no cable-duct qualification. The 0.5 m threshold is a paper comparison criterion linked to PAS 128 AL5 context, not a universal acceptance requirement for every maintenance task.

### B1 — dual-resolution acoustic inspection robot

**Sun et al., Tunnelling and Underground Space Technology, published 2026-02-01.**  
DOI: https://doi.org/10.1016/j.tust.2025.107133  
Open accepted manuscript: https://eprints.whiterose.ac.uk/id/eprint/239021/

- Population: experimental in-pipe acoustic inspection platform.
- Method: low-frequency acoustic waves for long-range coarse sensing/navigation and high-frequency acoustic waves for short-range high-resolution inspection.
- Published result: average localization variation of 8 cm over 18 m of test length; average feature-classification accuracy of 76% for aligned structures, tilted structures, blockages and empty-pipe conditions.
- Data/material: accepted manuscript is available through the University of Bristol research record under CC BY; preserve the manuscript and third-party material's own rights terms.

**Limits:** “localization variation” must not be silently relabelled as MAE, RMSE or P95; the detailed metric definition and rig profile must be reproduced from the paper before any numerical head-to-head. Its feature classification result is not a localization metric. This is an acoustic-only inspection baseline, not a cable-duct performance claim.

### B2 — UWB/IMU multi-source localization in underground utility tunnels

**Chang et al., Sensors 2026, 26(14), 4651, published 2026-07-22.**  
DOI: https://doi.org/10.3390/s26144651  
Open full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC13416606/

- Population: pedestrian positioning in an underground power utility tunnel, plus a laboratory evaluation.
- Hardware: nine-axis MEMS IMU (100 Hz), UWB anchors/tags (10 Hz), with map constraints and a CNN-assisted denoising variational autoencoder (DVAE-CNN) used for observation credibility assessment and a map-constrained particle-filter stage.
- Data: 18 m × 24 m laboratory area; 1,260 m real utility tunnel; 42 laboratory trajectories and 36 field trajectories (78 total); complete-trajectory train/validation/test split of 70/15/15.
- Published field result: mean position error 0.51 m, maximum 1.31 m and 94% of errors under 1 m. The paper reports a 73.6% average accuracy improvement and 85.2% reliability improvement over results without its credibility-regulation mechanism; reliability is defined against a 1 m threshold in the paper.
- Infrastructure assumptions: UWB anchors at 10–15 m intervals, a structural map and supporting Wi-Fi / Bluetooth / UWB infrastructure.
- Data: the field and laboratory datasets include confidential project information and are not publicly released.

**Limits:** pedestrian state estimation in a walkable tunnel, not a small in-pipe robot; anchor installation and maintenance are part of the system cost; do not compare its 0.51 m field average with the pipe-robot 1D figures as if they were the same task. The reported percentage improvements are relative to that paper's own baselines and are not comparable to other papers' percentage improvements.

### B3 — general task/setup-agnostic factor-graph localization

**Nubert et al., IEEE Transactions on Robotics, published 2026-07-23.**  
DOI: https://doi.org/10.1109/TRO.2026.3714645  
Official implementation: https://github.com/leggedrobotics/holistic_fusion

- Population: general field robotics, with five real-world scenarios across three robotic platforms.
- Method: factor-graph formulation that jointly estimates local/global robot state and dynamic reference-frame/context variables; fuses absolute, local/relative and landmark measurement types; emphasizes online smoothness, consistency and cross-frame alignment.
- Value to SUF: a strong reusable general fusion baseline and a design reference for admitting heterogeneous sensors with explicit frames, rather than hand-coding one bespoke estimator per sensor pairing.
- Data/code: official source repository and example datasets are linked by the project; inspect exact licenses, dependencies, sensor types and dataset terms before use.

**Limits:** not demonstrated as a buried-pipe or cable-duct system. It is a software framework candidate, not evidence that any chosen sensor model works underground. Evaluate it on the same admitted SUF observations and ground truth as other methods.

### B4 — emerging acoustic micromachine research

**Pipebots / EPSRC project, posted 2026-10-26:** “Bio-inspired micromachines for condition assessment of buried water supply pipes.”  
https://pipebots.ac.uk/bio-inspired-micromachines-for-condition-assessment-of-buried-water-supply-pipes/

- Project objective: develop theory and MEMS sensors measuring high-order acoustic pressure derivatives in waveguides with imperfections; deploy sensors on a robot for water-filled pipe condition assessment.
- Funded value shown by Pipebots: £2,095,442; project dates 2025-11-17 to 2028-11-16.
- Relevance: emerging alternative acoustic sensor physics, especially for water-filled pipes and condition detection.
- Status: ongoing research programme, not a published field-performance baseline in the project update.

**Implication:** monitor and benchmark the sensor concept when results become available; do not claim that Luminous has equivalent sensor hardware or that this programme has already achieved field performance.

## 3. Matched comparison tracks

| Track | Population and state | Fair baselines | Primary metrics | Excluded claims |
|---|---|---|---|---|
| P1 — pipe-chainage localization | In-pipe robot; 1D chainage | Tether + wheel; wheel/IMU; visual-only; acoustic-only; acoustic–visual PF/PGO; dual-resolution acoustic where same task fits | P50/P95/max absolute chainage error; error rate at 0.5 m and task tolerance; drift/m; outage/reacquisition; dwell time; energy | No global 3D pose or cross-pipe transfer |
| P2 — utility-tunnel positioning | Pedestrian and robot reported separately; 2D/3D pose | UWB/IMU/map; IMU-only; UWB-only; Holistic Fusion; map-constrained PF/DVAE as faithfully reproduced | P50/P95/max position error; availability; outage; anchor density/cost; map/frame consistency; latency | No 1D sewer chainage comparison |
| P3 — acoustic feature inspection | In-pipe feature localization and classification | Dual-resolution acoustic; camera/CCTV; direct acoustic; combined acoustic/visual if available | Feature position error using exact paper metric; per-class precision/recall/F1; macro-F1; obstruction false negatives; coverage | Do not substitute class accuracy for localization accuracy |
| P4 — localization integrity | All populations, separately stratified | Best matched method plus declared independent oracle | False high-confidence fix; interval coverage; risk/availability; abstention; common-mode failure behavior | No average score hiding integrity failures |
| P5 — maintenance task outcome | Profile-specific inspection/repair point | Best conventional proving/CCTV/tether process | Independent defect re-location success; first-pass acceptance; work time; retrieval; damage; operator interventions | No claim of field repair from simulation alone |

## 4. What Luminous should do differently

The differentiator should not be “we fuse more sensors.” The 2026 literature already demonstrates multiple fusion architectures, and generic factor-graph tooling is openly available.

The higher-value contribution is **evidence-qualified localization as an operational service**:

1. Every estimate carries segment identity, chainage/pose, frame, timebase, uncertainty, source evidence, calibration and exact estimator/configuration revision.
2. Local chainage and global network placement are separate claims. A stale or ambiguous access-node identity cannot silently create a globally anchored coordinate.
3. Sensor disagreement, aliases, occlusion and missing data remain visible; abstention is a measured outcome, not a hidden failure.
4. Localization methods are selected per population and maintenance task; not every duct should receive UWB anchors or a crawler.
5. Active sensing decides whether another acoustic, visual or inertial observation is worth its time/energy cost.
6. Robot mobility, localization, defect classification, recovery and independent maintenance acceptance remain separate qualification dimensions.
7. The spatial evidence is mapped to OGC MUDDI concepts through Sol-Atlas #755, with sensitive geometry disclosed only to authorized purposes.

## 5. Proposed research goals (not achieved capabilities)

- Reproduce each public/reference baseline on its own matched data before changing the model.
- On frozen synthetic adversarial cases, achieve zero false high-confidence locations for stale frames, incompatible anchors, clock errors, ambiguous repeated structures and declared unobservable intervals.
- On held-out runs, demonstrate calibrated 90% and 95% uncertainty intervals; report coverage by profile and environmental stratum.
- On cable-duct rig data, set the task tolerance before testing from the real repair procedure and metrology capability; report P95/max, not only mean.
- Active sensing: target at least 25% reduction in stationary acquisition/dwell time at non-inferior task-level localization performance against the matched fixed-cadence baseline.
- Infrastructure efficiency: compare installed anchor count and lifetime cost against anchor-heavy methods; investigate whether mobile acoustic/visual sensing can achieve similar task performance with less fixed infrastructure.
- Acoustic inspection: establish per-class confusion matrices first; investigate macro-F1 ≥0.90 as a stretch goal only after building an independent, multi-profile dataset.
- Cross-domain generalization: leave-one-material/diameter/profile out; no train/test leakage across adjacent frames or same route; report domain-shift degradation and calibration.
- Robot task outcome: measure independent maintenance-point relocation and safe retrieval, not just estimator error.

No single weighted score. Keep accuracy, integrity, coverage, latency, energy, dwell, operator effort, hardware/anchor capital, retrieval, and maintenance acceptance separately visible.

## 6. Benchmark hygiene and reproducibility

- Freeze exact paper revision, implementation, raw-data terms, preprocessing, test split, parameters and measurement definitions.
- When exact raw data are unavailable, mark faithful reproduction blocked; do not reconstruct data from plotted curves and call it a reproduction.
- Split by complete route/trajectory and, for generalization studies, by material/diameter/site as appropriate.
- No tuning on held-out test routes.
- Preserve all failed, aborted, manually recovered and missing-evidence trials in denominators.
- Bind raw observation artifacts, calibration, clock synchronization, ground truth, estimator code, build and config to exact digests.
- Use an independent oracle that does not consume the candidate's asserted position or confidence.
- State the claim ceiling per dataset; no sewer, tunnel, drain or cable-duct score transfers automatically to another population.

## 7. References

1. Worley et al. (2026), acoustic–visual pipe localization: https://doi.org/10.1002/rob.70368
2. Sun et al. (2026), dual-resolution acoustic in-pipe inspection: https://doi.org/10.1016/j.tust.2025.107133
3. Chang et al. (2026), underground utility tunnel UWB/IMU + DVAE-CNN: https://doi.org/10.3390/s26144651
4. Nubert et al. (2026), Holistic Fusion: https://doi.org/10.1109/TRO.2026.3714645
5. Holistic Fusion implementation: https://github.com/leggedrobotics/holistic_fusion
6. Pipebots buried-water-pipe acoustic micromachine project update (2026-10-26): https://pipebots.ac.uk/bio-inspired-micromachines-for-condition-assessment-of-buried-water-supply-pipes/
7. IEC TR 63439-1-2:2026 robotics for electric power systems roadmap: https://webstore.iec.ch/en/publication/109098
8. OGC MUDDI Part 1 v1.0: https://docs.ogc.org/is/23-024/23-024.html
9. BSI PAS 128:2022: https://accord-checkout.bsigroup.com/products/pas-128-underground-utility-detection-verification-and-location-specification
