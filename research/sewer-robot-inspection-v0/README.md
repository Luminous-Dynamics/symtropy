# Sewer Robot Inspection Benchmark v0.1

**Programme:** SEWER-ROBOT-001  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4905  
**Related sanitation benchmark:** https://github.com/Luminous-Dynamics/symtropy/issues/4902  
**Date:** 2026-11-12  
**Status:** Protocol and synthetic contract only. No sensor corpus, robot trial, model validation, product qualification or field performance claim.

## Thesis

A sewer-inspection system is not qualified merely because a detector recognizes defects in selected video frames. It must establish which asset and chainage it inspected, what surface it observed, what it could not observe, how localization and sensor quality changed, and whether a human reviewer can reproduce the condition assessment.

The benchmark unit is a complete mission and an asset-held-out condition record:

robot + sensor configuration -> mission -> observations -> spatial/temporal registration -> evidence quality -> defect candidates -> human adjudication -> accepted asset-condition record

## 1. Research basis and current comparators

### Specific published RGB-D baseline

Shen et al. (2026) report a sewer-robot RGB-D instance-segmentation and size-estimation system with:
- 84.8% mAP@50 for the paper's 16-class dataset;
- 10.2 mm overall MAE and 11.6% MAPE for 3D defect-size estimation;
- 11 frames/s on an embedded Jetson Orin NX;
- an autonomous 6 m inspection in a 0.8 m diameter pipeline, with seven defects identified/quantified within 90 seconds.

These are results for one paper, dataset, model, hardware and pipe setup—not a universal industry SOTA or a benchmark result reproduced here. Any claim to beat it requires the same data, split, sensor geometry, annotation and metric.

### Broader state of the field

Li et al. (2026) review 183 sewer-inspection robotics publications from 2011–2025. Their identified bottlenecks include sensor fusion versus real-time processing; localization drift and spatiotemporal synchronization in signal-denied pipelines; nonlinear mobility/control and training-data quality; asset-data interoperability and communication reliability.

This suggests the next defensible frontier is not only a larger detector. It is mission-level evidence completeness, uncertainty-aware metrology and cross-utility generalization.

### Human inspection standards / industry practice

Use a versioned coding profile from the applicable jurisdiction:
- NASSCO PACP for CCTV inspection and coding in the North American context;
- WRC Manual of Sewer Condition Classification (MSCC) in South African practice;
- EN 13508-2 for coding of external sewer systems.

Do not silently translate between taxonomies. Freeze the exact edition, defect dictionary, severity rules, inspector qualifications and adjudication procedure. Human-coded output is a reference annotation, not infallible truth; preserve inter-rater disagreement and adjudication.

## 2. Paradigm shifts

1. **From frame accuracy to mission completeness.** Require traceable chainage, direction, pipe identity, retrieval, observed/unobserved surface and quality-state for every interval. The denominator is the entire planned mission, not only usable frames.
2. **From black-box defect scores to robotic metrology.** Separate defect detection, extent measurement, localization, severity coding and structural diagnosis. A crack-like pixel pattern is not automatically a structural grade or remaining-life estimate.
3. **From “no detection” to explicit observability.** A water-covered invert, glare-saturated crown, dirty lens, blind bend or low-confidence pose is unobserved or unusable, never defect-free.
4. **From video reports to interoperable evidence objects.** Link observations to segment/chainage, coordinate reference, calibration, robot revision, taxonomy version, reviewer and superseding evidence.
5. **From one sensor to regime-aware sensor fusion.** RGB/RGB-D, structured light/laser profiling, sonar and odometry are complementary. Fuse only where synchronization and uncertainty are bounded; preserve disagreement rather than averaging contradictory modalities away.
6. **From autonomy as a feature to recoverability as a requirement.** A stuck robot, tether fault, battery decline, comms loss or localization jump is a mission outcome to measure. Safe stop, recovery and human handover are part of product performance.
7. **From model confidence to decision utility with abstention.** Report high-risk false negatives, calibration and selective coverage. Unknown asset, unsupported material, poor visibility or out-of-distribution observations must produce Indeterminate, not an overconfident rating.

## 3. Modality and robot families

| Modality | Primary role | Required qualification risks |
|---|---|---|
| Pan/tilt CCTV crawler | Visual inspection and operator review | Light uniformity, camera calibration, focus, motion blur, water/debris obscuration, clock-position mapping |
| RGB-D / stereo | Instance segmentation and 3D sizing | Depth scale, range dependence, reflective/wet surfaces, missing depth, sensor alignment |
| Laser profiling / structured light | Shape, deformation and cross-section measurement | Range/angle calibration, angular coverage, reflectivity, profile registration |
| Sonar / acoustic | Submerged or optically obscured sections | Sound-speed/temperature assumptions, bubbles, sediment, multipath, calibration |
| Wheel/track odometry + IMU | Local chainage and attitude | Slippage, drift, wheel diameter, turns, slope, wet debris |
| Tether length / surveyed landmarks | Independent localization evidence | Slack, stretch, pulley geometry, cable snag and reference quality |
| Gas / flow / temperature channels | Context and hazard observations | Sensor identity, calibration, exposure limits, measurement boundary and jurisdictional controls |

No one platform or sensor stack is assumed to work across all diameters, materials, flow depths, bends, junctions, corrosion states or access conditions.

## 4. Corpus and annotation contract

A benchmark release should bind:
- immutable asset/segment identity, coordinate reference, diameter, material, length, direction and chainage origin;
- source rights, original observation bytes, cryptographic digest, acquisition state and transformation lineage;
- robot/platform ID, hardware revision, firmware, sensor and lens model, calibration/check evidence and mounting geometry;
- mission plan, start/end, movement direction, retrieval result, power/communications state, operator interventions and safe-stop events;
- time-synchronized video, depth/sonar/laser/IMU/odometry/tether channels with explicit clock source, offset, drift and missing intervals;
- observation quality: usable, degraded, unobservable, missing, with a controlled reason vocabulary;
- observed surface/clock-position coverage and chainage range; never infer coverage from frame count alone;
- defect annotation class, geometry, extent, severity code, uncertainty, reference measurement method and reviewer qualification in a privacy-preserving form;
- multiple independent human assessments, disagreements, adjudication and taxonomy version;
- repair/renewal and post-repair evidence only when actually collected.

**Split by physical pipe segment and source campaign**, then hold out at least one utility/geography, one robot/camera family, one operator group and one temporal campaign where data access permits. Adjacent frames, clips, multiple passes and duplicated exports from one asset must remain in one split. Report the actual independent-unit count; do not inflate sample size by treating frames as independent pipes.

## 5. Metrics

### Detection and condition coding
- Per-class precision, recall, F1, PR-AUC and mAP at declared IoU thresholds;
- segmentation IoU/mIoU and boundary-distance error;
- critical-defect false-negative rate and recall at fixed false-alarm budgets;
- per-defect-class and per-severity confusion matrices;
- human inter-rater agreement, adjudication rate and code consistency;
- open-set/unknown detection and abstention coverage.

### Localization and dimensional metrology
- absolute chainage error median, P95 and maximum; drift per metre; landmark/loop-closure residual;
- defect dimension MAE, bias, MAPE and 90/95% interval coverage against calibrated independent references;
- orientation / clock-position error where a standard coding scheme requires it;
- repeatability across repeated passes and reproducibility across robots/operators;
- sensitivity to water surface, pipe geometry, sensor distance and material.

### Observation completeness and mission reliability
- mission planned length, reached length, usable length and accepted length;
- surface/clock-position coverage, unobserved fraction, degraded fraction and reinspection rate;
- launch/retrieval success, stuck/lost robot incidence, recovery time, mission duration and time per accepted metre;
- energy per metre, comms outage fraction, frame/sensor dropout and localization degradation;
- image-quality gate sensitivity: unusable evidence correctly marked incomplete versus false-accepted coverage.

### Operational utility
- inspector review minutes per accepted metre and correction rate;
- mobilisation + inspection + cleaning + traffic management cost per metre and per accepted asset record;
- time from mission to human-reviewed condition record, work decision and accepted maintenance;
- repeat defect / post-repair verification where longitudinal data exist;
- worker confined-space exposure and service interruption avoided only where measured using a declared method.

## 6. Proposed targets (not achieved)

### Hard gates
- **Zero silent coverage gaps:** every planned interval ends observed, degraded, unobservable, not reached or missing, with reason and provenance.
- **Zero false-safe from invalid evidence:** stale/uncalibrated sensor, localization jump, unobservable surface, broken synchronization or unsupported regime cannot support an absence-of-defect claim.
- **Zero auto-certified critical negatives:** human review required for critical/high-consequence defect decisions until a class- and jurisdiction-specific safety case is accepted.
- **100% traceability** of each accepted condition code to source observation interval, asset, taxonomy revision, calibration and reviewer.
- Every scheduled mission gets exactly one terminal disposition: complete/accepted, partial/indeterminate, failed or unsupported; missing result is not success.

### Competitive/stretch targets
- Reproduce the 2026 paper's results on its stated benchmark before claiming an improvement. A candidate stretch is mAP@50 > 84.8% and dimensional MAE < 10.2 mm on the same held-out split, while preserving or improving critical-defect recall and confidence calibration. If exact data/split cannot be obtained, report a non-comparable result instead.
- On new asset-held-out tests, target at least 10% relative reduction in localization P95 and measurement MAE over a matched CCTV/RGB-D baseline while keeping 95% uncertainty intervals empirically calibrated. Set the final number after baseline reproduction and reference-metrology uncertainty review.
- On supported pipe classes, target >=95% observable wall coverage at planned speed, with remaining surface explicitly marked unobserved; no coverage percentage can override a critical blind zone.
- Target >=99% successful safe recovery/retrieval over a preregistered mission corpus, reporting confidence intervals and all stuck/lost cases. This is a research target, not a current robot rating.
- Demonstrate at least 2x reduction in reviewer time per accepted metre without worse critical-defect recall, severity agreement or asset-held-out error. Measure full human correction time and reinspection overhead.
- Benchmark at least three sensor configurations and two robot/mobility families before choosing a product architecture.

## 7. Failure and adversarial corpus

Include lens fouling; low illumination; glare; reflections; bubbles; turbidity; high water; sediment; grease; roots; deposits; cracks; fractures; joint displacement; deformation; infiltration/exfiltration; lateral intrusion; corrosion; camera/depth miscalibration; stale calibration; wrong pipe ID; reversed direction; chainage drift; clock skew; duplicate/out-of-order frames; missing/partial sensor stream; localization jump; communications/tether loss; battery brownout; robot immobilization; retrieval failure; blind bend; junction confusion; unseen crown/invert; corrupted video; unknown taxonomy class; class-imbalance; unsupported material/diameter; adversarial near-duplicate training/test clip; and forged inspection completion.

Cases must specify a disposition and independent oracle. Include positive controls, difficult-but-usable cases, null/unknown cases and cases where trained manual CCTV inspection is legitimately preferred.

## 8. T0 synthetic mission contract

- schema/mission-case.schema.json defines a runtime-neutral mission envelope. It carries no image or sensor bytes and makes no defect truth assertion.
- fixtures/synthetic-localization-loss-v0.1.json is a synthetic negative control: a mission has a missing telemetry interval, a localization discontinuity and degraded wall coverage. Expected outcome is Indeterminate; the unseen interval cannot be rated defect-free.
- fixtures/synthetic-complete-mission-v0.1.json is a synthetic positive control. Accepted means only that the observation envelope meets this synthetic completeness profile; it does not mean the pipe is defect-free or serviceable.
- The fixtures bind a deliberately illustrative T0 threshold profile of at least 90% surface coverage and at most 0.10 m P95 localization error. Those values are test-vector constants, not industry acceptance limits or field specifications.
- A one-off local sanity pass parsed the schema and both fixtures and checked required top-level fields, interval ordering/contiguity, full chainage coverage, recomputed fully-qualified observation length, expected disposition and the no-defect-absence invariant (15 checks passed). This was not a full JSON-Schema implementation run, Rust test, CI, physical robot test or defect-detection evaluation.

## 9. Luminous integration boundary

- **Symtropy:** mission/sensor replay, synthetic localization and mobility stress cases, uncertainty analysis and benchmark scoring.
- **Sol-Atlas:** asset network geometry, pipe segment/chainage coordinate reference, GIS provenance and spatial source quality.
- **Symthaea:** read-only anomaly triage and uncertainty-aware multi-observation explanation; never certify absence/severity.
- **Mycelix:** evidence provenance, reviewer sign-off, appeals, approvals and accepted maintenance receipt; no reputation or model-confidence authority.
- **Robotics:** mission execution, safe-stop, recovery and sensor health.
- **Luminous platform:** BOM, robot revision, sensor calibration, mission configuration, design control and lifecycle/service record.

No repair, cleaning or valve actuation is authorized by an AI finding. Each physical intervention remains a distinct, scoped, site-approved workflow with interlocks and independent completion observation.

## 10. First implementation sequence

1. Freeze the mission-case schema and synthetic fail-closed corpus.
2. Define a versioned WRC MSCC / PACP / EN 13508-2 crosswalk with human inspection partners; keep unmapped classes Unsupported.
3. Secure and hash a rights-cleared human-coded CCTV dataset, with asset-level split metadata.
4. Reproduce one published RGB-D benchmark without changing the split, preprocessing, metric or hardware boundary.
5. Add localization/dimensional metrology and sensor-quality tracks separately from defect detection.
6. Add paired repeated-pass, camera-family-held-out, utility-held-out and operator-held-out tests.
7. Only after offline qualification, plan a supervised wet-pipe robot test with physical retrieval, isolation and worker safety review.

## References

1. Shen, Zhang, Jin & Song (2026), [An RGBD fused real time defect detection and size estimation method for sewer robots](https://doi.org/10.1016/j.measurement.2025.119694), Measurement 2026; reported 84.8% mAP@50, 10.2 mm MAE and 11 fps in its own setup.
2. Li, Chen & Chen (2026), [Robotic technologies for sewer inspection: From automated assessment to explainable AI-enabled solutions](https://doi.org/10.1016/j.aei.2026.104605), Advanced Engineering Informatics 74; systematic review of 183 publications.
3. 2025 review, [AI-based sewer pipe condition assessment / evaluation challenges](https://doi.org/10.1016/j.undsp.2025.06.004); review reference for dataset/taxonomy/evaluation heterogeneity.
4. [NASSCO PACP / CCTV inspection resources](https://www.nassco.org/resource/cctv-inspection-quality-in-sewer-cleaning).
5. Republic of South Africa, [Compulsory National Water and Sanitation Services Standards, Government Gazette 52814 (6 June 2025)](https://www.gov.za/sites/default/files/gcis_document/202506/52814gon6292.pdf). Includes annual sewer/faecal-sludge collection inspection, treatment process audits and maintenance/evidence requirements.

## Nonclaims

This protocol does not qualify a robot, dataset, defect detector, dimensional estimator or structural condition grade. Paper results are not reproduced here. A defect detection is not an engineering assessment of remaining life. No inspection model authorizes repair or proves that an unobserved pipe surface is defect-free.
