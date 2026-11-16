# SubterraBench v0.1 — Planetary Subterranean Intelligence and Operations

**Date:** 2026-11-16  
**Status:** Proposed benchmark protocol; no Subterra benchmark results claimed

## 1. Purpose

Evaluate a full chain from subterranean candidate interpretation through robotic survey and use-specific engineering assessment. No single score may hide an unsafe access route, unobserved interior, localization drift, lost evidence or invalid planetary-protection state.

## 2. Separate evidence dimensions

Every site scorecard must report separate states for:
- morphology;
- geological-origin hypotheses;
- subsurface connectivity;
- access and egress;
- surveyed interior extent;
- environmental characterization;
- structural/mechanical assessment;
- communication availability;
- science value;
- planetary-protection status;
- use-specific engineering qualification.

No weighted sum may promote an unknown or failed dimension into a pass.

## 3. Evidence tiers

| Tier | Evidence source | Permitted claim |
|---|---|---|
| S0 | Deterministic synthetic geometry, maps and sensor streams | Algorithmic correctness and controlled fault coverage |
| S1 | Analytical/independent reference reconstruction | Numerical correctness within stated assumptions |
| S2 | Bench tests and instrument calibration | Tested sensor/robot subsystem capability |
| S3 | Terrestrial cave, mine or volcanic analogue | Field performance in the declared analogue envelope |
| S4 | Orbital/spaceflight experiment | Performance under the tested mission environment |
| S5 | Planetary surface/subsurface deployment | Performance in the actual destination environment |

Evidence tiers must remain separate. A simulated cave traversal cannot qualify a flight robot; an Earth cave survey cannot certify a lunar cave.

## 4. Benchmark families and proposed targets

All numerical targets below are programme targets for preregistered test envelopes, not universal standards or achieved results.

| Benchmark | Measures | Proposed gate |
|---|---|---|
| SubsurfaceCandidateBench | Candidate detection, false candidates, probability calibration, alternative-origin ranking | Calibrated probabilities on held-out sites; false-positive cost reported separately from recall |
| OriginHypothesisBench | Distinguishing volcanic, collapse, karst, tectonic, ice-related and unknown mechanisms | Correct abstention when evidence cannot discriminate origin; no forced single-class output |
| SubsurfaceMorphometryBench | Width/height profiles, bottlenecks, chamber geometry, branch topology and unobserved extent | Nominal 90% geometry intervals achieve 85–95% empirical coverage on held-out analogue sections; unobserved intervals remain masked |
| ConnectivityBench | Correct inference of entrance-connected topology | No claim of connected extent beyond direct observation or a clearly labelled geophysical inference |
| NetworkEvidenceBench | Separate candidate/geophysically supported links from directly observed connectivity; test topology components, directionality, traversability and route/retreat claims | Candidate links never merge confirmed components; only directly observed and traversal-demonstrated edges may support a robot route; no demonstrated route is reported as unknown, not physical disconnection |
| AccessEgressBench | Descent, traverse, traction, slope, obstacle, tether and recovery feasibility | Zero unrecoverable robot states in the finite high-consequence challenge suite; retreat path must remain independently represented |
| SubsurfaceSLAMBench | Drift, loop closure, localization integrity and map consistency | Error and uncertainty reported by distance/loop closure and geology; no map declared globally consistent without closure evidence |
| SemanticSituationalAwarenessBench | Detection/localization of hazards, targets, relays, resource indicators and science features | Score precision, recall, localization error, false alarm burden and evidence-return rate—not map coverage alone |
| RelayResilienceBench | Data delivery, latency, link availability, relay mass/energy and recovery from node loss | Target at least 99% delivery of declared critical evidence records in the frozen fault suite, or explicit mission-safe store-and-forward disposition |
| TeamCoordinationBench | Multi-robot task completion, duplicate effort, energy, collision, communication and degraded operation | Zero hard safety invariant violations in the finite suite; task efficiency compared to strongest single-robot and centralized-team baselines |
| GeologicalHazardBench | Rockfall, unstable blocks, fractures, collapse precursors, floor bearing and unknown zones | Zero false-safe classifications in the finite high-consequence challenge set; calibrated probabilities and false-alarm counts disclosed |
| EnvironmentalCharacterizationBench | Temperature, pressure, radiation, dust, gas, volatile and illumination mapping | Spatial coverage and uncertainty reported by sensor modality; missing values never interpolated into qualified zones |
| ScienceValueOfInformationBench | Expected reduction in uncertainty or decision regret per observation mass, energy and time | Target at least 20% lower preregistered decision regret than passive/uniform sampling at equal observation budget |
| PreservationBench | Planetary-protection constraints, contamination-control records and sample chain of custody | Reject all unauthorized or policy-incompatible sampling/biological actions in the finite adversarial suite |
| ClaimConsistencyBench | Cross-field consistency of morphology, origin, connectivity, access, survey, engineering-use and preservation claims | Reject contradictory states (for example, direct connectivity with no demonstrated access or survey); no inconsistent site-readiness claim may be promoted |
| UseQualificationBench | Requirements and evidence for storage, industry, habitat or science use | No cross-use inference: each declared use has its own acceptance envelope and independent test records |

## 5. Baseline set

| Problem | Baselines |
|---|---|
| Orbital candidate identification | Existing cave catalogues, standard image classification, conventional geomorphological interpretation and multi-instrument expert analysis |
| Cave morphology | Uniform-cylinder model, classical point-cloud registration, mesh reconstruction, conventional cross-section analysis and statistical morphometry such as the 2026 Ape Cave work |
| Mapping | LiDAR/IMU SLAM, visual-inertial SLAM where lighting supports it, occupancy-grid/OctoMap and Gaussian-mixture mapping |
| Robot mobility | Single wheeled rover, legged rover, tracked rover, tethered descent and human-supervised teleoperation |
| Multi-robot coordination | Centralized task allocation, decentralized task allocation, single robot, homogeneous fleet and heterogeneous fleet |
| Communications | Fixed relay chain, breadcrumb nodes, robot mesh, tether/fiber, high-gain link, store-and-forward and communication-aware relay placement |
| Mission planning | Shortest-path/frontier exploration, information gain, risk-aware planning and human-directed route planning |
| Hazard assessment | Conventional threshold rules, geotechnical assessment and independent sensor-triggered shutdown |
| Situational awareness | Map coverage alone versus semantic feature detection/localization and successful evidence return |
| Planetary protection | Pre-mission rule checklist and policy-constrained planning with negative controls |

Baselines must use equivalent sensing, payload, compute, energy, communication and map prior. Published systems that cannot be reproduced should be cited as architectural references, not as measured benchmark comparisons.

## 6. Experimental design

1. Split by cave/site and geological origin, not random scan fragments from the same cave.
2. Include both well-surveyed and deliberately occluded regions.
3. Freeze ground truth maps, cross-sections, geological labels and hazard annotations before blind evaluation.
4. Preserve each sensor's native resolution, coordinate frame, calibration and time.
5. Include correlated sensor failure cases; multiple products from one source are not independent measurements.
6. Inject low-light, dust, drift, loop-closure error, wheel slip, tether snag, relay loss, dropped packets, sensor bias and partial robot failure.
7. Require semantic target detection, localization and evidence delivery in addition to map coverage.
8. Report safe retreat, robot recovery and lost-data outcomes.
9. Keep scientific claims, engineering claims and planetary-protection permissions distinct.
10. Publish raw/derived data where allowed, model/configuration hashes, score code, uncertainty and a reproducible run manifest.

## 7. Result dispositions

Each claim is assigned exactly one:
- **SUPPORTED:** available evidence meets the declared criterion.
- **CANDIDATE:** evidence supports investigation but not confirmation.
- **UNKNOWN:** insufficient evidence.
- **CONTRADICTED:** credible evidence opposes the claim.
- **OUT_OF_DOMAIN:** assumptions or environment exceed the validated envelope.
- **ABSTAINED:** system correctly declines to infer or act.
- **INVALID:** measurement or test protocol failed.

Do not map UNKNOWN or OUT_OF_DOMAIN to a low confidence pass. ABSTENTION is a successful outcome when evidence is insufficient.

## 8. Superiority claim

Subterra may claim improvement only against a reproduced, strongest relevant baseline using the same held-out sites, observation budget, payload/energy limits, communication assumptions and success definition. Improvements must be statistically supported and must not regress safety, retreat, preservation, evidence integrity or data-return metrics beyond preregistered limits.

## 9. Initial release gate

SubterraBench v0.1 is ready for external review when:
- a synthetic non-uniform cave profile dataset exists;
- at least one blind site/geometry split exists;
- scoring code has positive and negative controls;
- morphology and origin are stored as separate claim dimensions;
- communications and semantic detection are scored separately from map coverage;
- evidence tiers and planetary protection are explicit;
- independent reproduction is possible from frozen inputs and score code.


## 10. Observation lineage and common-cause grouping

The new dataset-level observation-lineage manifest links site observation IDs and derived records to physical acquisitions and declared shared-cause episodes. It is separate from the site-evidence schema so the v0.1 site record remains unchanged.

The manifest captures:
- one record per raw acquisition or derived evidence product;
- stable record, site, observation and physical-acquisition IDs;
- derived-product source-record references;
- shared instrument-calibration, environmental-episode, processing-lineage and reference-label-campaign groups;
- opaque provenance references, without claiming that reference presence authenticates the underlying source.

Required semantic validation still sits beyond JSON Schema: every referenced source record must exist; no source graph cycle is permitted; observation IDs must resolve exactly once to observations in the declared site record; derived products must preserve the acquisition lineage of their source chain unless an explicitly reviewed transformation declares a new physical acquisition boundary; shared-cause groups must be scoped to an episode/campaign, not a global device or software name.

The Symthaea dependence audit consumes corresponding typed declarations and computes transitive split components. Its output is an audit of declared grouping only, not proof of independence. A future adapter should retain original IDs and issue a source-to-audit mapping receipt; it must not silently infer independence from absent group declarations.

| Benchmark | Property | Proposed gate |
|---|---|---|
| ObservationLineageCompletenessBench | Every site observation maps to one or more lineage records, without dangling observation IDs | 100% declared observation association coverage; no silent omission |
| DerivedArtifactGraphBench | Derived products cite existing source records; graph is acyclic | Zero dangling IDs or cycles |
| AcquisitionPreservationBench | Derived records retain physical acquisition identity | Zero accidental acquisition split |
| CommonCauseSplitBench | Shared calibration/environment/processing/adjudication episode IDs are grouped transitively | Zero declared common-cause groups cross qualifying data splits |
| ProvenanceBoundaryBench | Opaque refs are not upgraded into verified authenticity claims | Zero authority/evidence promotion from reference presence alone |

The synthetic fixture contains two sites sharing calibration and environmental episodes, with three related raw/derived records from one acquisition at site A and an isolated third site. It is an adversarial split-design fixture, not empirical planetary data.
