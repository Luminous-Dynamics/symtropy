# Orbital Truss Inspectability — Evidence-Vector Roadmap v0.5

**Status:** research roadmap and matched-comparison protocol  
**Current implementation:** synthetic 2D geometry and k-view planning; 3D conditional support-envelope and rectangular-frustum preconditions  
**Current claim ceiling:** synthetic and caller-preconditioned geometric results only; no physical instrument qualification, probability of detection, structural acceptance or flight readiness

## 1. Paradigm shift: stop optimizing "visibility"

The field's central abstraction should move from *which viewpoints see the part?* to:

**Which combination of design, views, sensors, routes and measurements can establish each required engineering characteristic with adequate independent evidence and known uncertainty?**

A CAD patch being geometrically visible is only a necessary condition. One camera ray may intersect a large surface element yet produce too few samples; a dense point cloud may be saturated or underexposed; a feature can be photographed without sufficiently constraining its dimensions; an apparently precise fitted parameter can still have a poorly calibrated uncertainty budget; and a theoretically good view may be unreachable by a real robot.

The chain is therefore:

\`asset identity -> support geometry -> camera/frustum admission -> GSD/incidence -> occlusion -> sample density -> photometric usability -> feature identifiability -> calibrated uncertainty -> executable acquisition -> observed measurement -> conformity decision -> independently retained evidence\`

Do not collapse these into one weighted "inspection score". Treat them as **separate, non-compensatory constraints** at the feature or surface-element level. A strong k-view count cannot compensate for an under-sampled characteristic, a saturated image, an unobservable parameter, or an unbounded uncertainty.

## 2. The evidence vector

Every feature or surface element should carry its own typed evidence record. The axes below have different units and semantics; aggregate only after the engineering requirement defines an explicit decision rule.

| Axis | Question | Existing owner |
|---|---|---|
| Geometric support / frustum | Is the entire declared support inside the physical camera image boundary and sensing envelope? | #6925 / #7136 |
| Requested view count | Are at least k distinct admitted views available, and does the pool actually contain them? | #6925 / #7002 |
| Failure-domain diversity | Are the admitted observations separated across declared dependencies, without inferring physical independence from labels? | #6925 plus future dependency model |
| GSD / projected resolution | Can the smallest relevant feature be resolved at the required pixel scale? | #7002 / #7136 |
| Sampling density | Are enough measurement rays distributed over each non-uniform surface element per unit area? | #7164 |
| Photometric usability | Are the observed pixels within usable exposure and material/illumination response limits? | #7222 |
| Feature identifiability | Do the observations constrain the required geometric parameters with adequate rank and conditioning? | #7043 / #7049 |
| Metrological uncertainty | What is the calibrated measurement uncertainty and conformity decision risk? | #6846 |
| Execution feasibility | Can the robot reach and acquire the selected measurements under collision, energy and fleet constraints? | #5920 / #5892 |
| Decision value | Does the valid measurement change a public-outcome engineering decision? | #6956 / #2239 / #1892 / #4997 |
| Evidence identity and authority | Can source geometry, measurement, calibration, interpretation and authorized disposition be linked without silent identity loss? | Mycelix #6390 / #6519 |

The record must retain raw inputs and per-axis deficits. For a safety-critical or acceptance feature, an overall mean across features must never hide one under-covered element.

## 3. 2026 research and industry comparator map

### 3.1 NASA Design for Inspection

NASA's Design-for-Inspection work treats inspectability as a design-time property, using simulation to study defect findability and variability rather than waiting until final inspection. It is a design framework and a comparison target for future design iteration, not the same contract as #6925.

Reference: [NASA NTRS — The Inspectability Metric: A Formalized System of Measurement Enabling the Design for Inspection Framework](https://ntrs.nasa.gov/citations/20240007593).

### 3.2 GSD-constrained voxel k-coverage

Waleed Bin Khalid and Chul Min Yeum, *Ground Sample Distance Constrained k-Coverage Viewpoint Planning via Set Multi-Cover Formulation*, arXiv:2610.36450, submitted 27 October 2026.

- Paper: https://arxiv.org/abs/2610.36450
- Full HTML: https://borowiecki.dev/pdf/2610.36450

The paper combines candidate viewpoints, a voxel visibility matrix, GSD, viewing-angle and occlusion filters, candidate augmentation and a set-multicover selector. Its experiment sweep includes voxel sizes 0.04/0.06/0.08/0.10 m, maximum GSD 0.5 mm/pixel, 0.25 mm/pixel setpoint, k = 1/3/5, overlap thresholds 50%/70%, augmentation tilt 1–90 degrees, azimuth 0–360 degrees, and a 3,600-second ILP time limit.

The paper uses an attainable target k*(v) = min(requested k, candidate views that can see voxel v). This is an explicit relaxation and the paper reports remaining shortfalls. Our v0.2 planner instead preserves original requested k and exposes candidate-pool availability. A fair comparison must report both original-k attainment and the capped attainable target for both methods.

Its reported dynamic greedy selection used 3.00–16.39% more viewpoints than ILP in its tested settings; only certified ILP optima justify a proven-minimum comparison. The paper is limited to two structures, known static geometry and voxel-centre sensing; it does not model UAV reachability/dynamics, collision-safe trajectories, minimum viewpoint separation or downstream measurement quality.

Existing work: [#7002 — ORBITAL-GSD-KCOVER-001](https://github.com/Luminous-Dynamics/symtropy/issues/7002).

### 3.3 Integrated sampling density, photometric usability and robot executability

Vanessa Staderini, *Integrated Planning and Optimization Framework for Robotic Visual Inspection*, TU Wien dissertation, 2026. DOI: 10.34726/hss.2026.138640.

- [University publication record and abstract](https://repositum.tuwien.at/handle/20.500.12708/226537)
- [Dissertation PDF](https://repositum.tuwien.at/bitstream/20.500.12708/226537/1/Staderini%20Vanessa%20-%202026%20-%20Integrated%20Planning%20and%20Optimization%20Framework%20for...pdf)

This is a broader integrated comparator than centre-sampled k-coverage. It distinguishes spatial resolution from the density/distribution of measurement points. Its sampling-density matrix counts camera-ray intersections per non-uniform mesh element and divides by element area; contributions accumulate across selected views. It also introduces a photometric visibility matrix using Blinn–Phong and Cook–Torrance BRDFs to exclude saturated or underexposed surface regions. Kinematic pose refinement and a set-coverage generalized travelling salesman problem (SCGTSP) are used to enforce executable robot trajectories under workspace/collision constraints.

The work is validated in synthetic and real experiments across 3-, 6- and 8-DoF systems. Its physical setup includes an industrial robot, stages and a Zivid structured-light sensor. Results are specific to those models, sensor, materials and experiments; they are not orbital flight or NDE validation.

This maps to three separate Symtropy owners:
- [#7164 — ORBITAL-SAMPLING-DENSITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/7164)
- [#7222 — ORBITAL-PHOTO-USABILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/7222)
- [#5920](https://github.com/Luminous-Dynamics/symtropy/issues/5920) for exact small route/fleet feasibility; it should remain distinct from the thesis' integrated SCGTSP.

### 3.4 Feature-oriented measurement quality for structured-light inspection

*Feature-oriented automatic viewpoint planning for structured-light 3D measurement*, Optics & Laser Technology 203 (2026), Article 116282, DOI: 10.1016/j.optlastec.2026.116282.

- [Publisher record](https://doi.org/10.1016/j.optlastec.2026.116282)

This work is a closer comparator to inspection of holes and slots than voxel k-coverage. Its Feature Measurement Quality (FMQ) considers effective depth along the feature normal and tangential constraint quality in the measurement plane. It screens views using sensor geometry and binocular visibility before selecting views and planning paths. The article reports a circular-hole measurement maximum mean absolute error of 0.0936 mm and standard deviation 0.0108 mm in its own experimental conditions, and reports planning about 26 features in 41 s. These are paper-specific outcomes, not universal performance targets.

Existing owner: [#7043 — ORBITAL-FMQ-001](https://github.com/Luminous-Dynamics/symtropy/issues/7043) and [PR #7049](https://github.com/Luminous-Dynamics/symtropy/pull/7049).

### 3.5 Traceable metrological digital twin

Brahim Ahmed Chekh et al., *Traceable Metrological Digital Twin for Robot-Based 3D Inspection: Calibration, Uncertainty Propagation, and Cross-Validation*, Metrology 6(4), 88 (18 November 2026), DOI: 10.3390/metrology6040088.

- [Open-access article](https://www.mdpi.com/2673-8244/6/4/88)

The system combines robot kinematic identification, optical sensor characterization, sensor-to-robot referencing and a machine-readable calibrated metrological state. It propagates calibrated parameter uncertainty through the measurement chain with Monte Carlo methods aligned to GUM, and validates against a CMM-characterized four-sphere artefact and a complex part. It compares dense surfaces to a calibrated Triple Scan reference and 67 feature characteristics to CMM/VCMM reference values.

The authors explicitly limit validation to the environmental conditions and short-term stability of their experiment; thermal drift and long-term stability are not modelled. That limitation suggests a valuable next benchmark for Symtropy: calibration-age, thermal-change and sensor-to-robot-reference drift challenges, with explicit stale-calibration abstention.

Existing owner: [#6846 — covariance-aware 3D datum/metrology](https://github.com/Luminous-Dynamics/symtropy/issues/6846).

### 3.6 Interoperability and conformity standards

- [NIST STEP File Analyzer and Viewer](https://www.nist.gov/services-resources/software/step-file-analyzer-and-viewer) supports STEP AP242, AP203 and AP214, reports semantic and graphical PMI and validation properties, and checks basic STEP-format errors.
- [MBx-IF / CAx-IF recommended practices](https://www.mbx-if.org/) provide persistent-ID and cross-domain exchange practices and test cases. Mycelix #6390 is the portfolio owner for AP242/QIF identity preservation.
- [ISO 14253-1:2017](https://www.iso.org/standard/70137.html) provides decision rules for proving conformity or nonconformity with measurement uncertainty.
- [BIPM/JCGM publications](https://www.bipm.org/en/committees/jc/jcgm/publications) include the GUM family. Use the applicable GUM/JCGM treatment for the measurement model instead of interpreting synthetic perturbation bounds as calibrated uncertainty.

## 4. New frustum gate and its exact limits

The current draft [PR #6925](https://github.com/Luminous-Dynamics/symtropy/pull/6925) adds a distinct rectangular camera-frustum precondition with both horizontal and vertical FOV and camera roll. It evaluates the entire caller-asserted spherical support-direction cap against the five image frustum half-spaces and returns WITHIN, OUTSIDE or INDETERMINATE.

At exact head \`4067bb9029668c46b889db82f6772dbcfb91b3d1\`, the dedicated PR and push workflows passed:
- rustfmt;
- the complete Rust test suite and strict Clippy;
- geometry, planning and support-envelope deterministic replays;
- independent geometry, planning and support-envelope oracles;
- the nine-case frustum corpus, deterministic replay and independent Python receipt comparison;
- exact-head manifest and artifact retention.

Evidence:
- [Push benchmark](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45390312899)
- [PR benchmark](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45390324779)

Repository-wide CI is pending on this head. These dedicated results qualify synthetic contracts only.

The new frustum certificate is not a visibility result: \`may_count_toward_visibility=false\`. The support enclosing-ball assertion is still a caller-supplied precondition; it is not independently proved by the frustum module. Occlusion, GSD, calibration, route feasibility and physical measurement remain separate gates. The next integration must not let a frustum pass set the voxel k-coverage matrix until the other relevant gates have also passed.

## 5. Paradigm shift: feature-conditioned, evidence-constrained inspection

Optimise the *evidence needed for a particular engineering decision*, not the number of photographed patches. For every required feature, preserve:

- candidate-view availability and requested-k shortfall;
- whole-support rectangular frustum and sensing-envelope status;
- GSD, incidence and ray/occlusion result;
- rays per unit surface area and spatial distribution;
- photometric usable fraction, underexposure and saturation;
- feature information rank, singular spectrum, condition and contour-sector coverage;
- calibrated uncertainty, covariance and conformity guard band;
- route/access feasibility, acquisition and calibration provenance;
- independent verification and final decision disposition.

This is a product of typed constraints, not a score in which high values on one axis can compensate for a critical failure on another.

A useful north-star metric is **cost per independently verified, decision-useful characteristic**. Always report its numerator and denominator: inspection cost, number and criticality of required characteristics, valid measurements, independently verified outcomes, false-clearances, abstentions, rework and post-repair verification. Do not use the ratio alone; publish all component metrics and their uncertainty.

## 6. Matched benchmark programme

| Stage | Benchmark question | Comparator / evidence | Gate |
|---|---|---|---|
| B0 | Are geometry and frustum semantics correct? | Frozen controls, negative controls, independent oracles, frame transforms | Zero false-WITHIN outcomes in sealed controls |
| B1 | Is requested k attainable? | Exact set multicover, certified ILP and paper greedy on identical visibility pools | Original-k and attainable-k results both published |
| B2 | Is GSD adequate? | Paper's camera/GSD equations, identical pixel dimensions, camera model and candidate pool | No hidden resolution or candidate-pool denominator changes |
| B3 | Is the surface sampled adequately? | #7164 sampling-density matrix on non-uniform meshes, plus uniform/remeshed baseline | Per-element density shortfalls; no compensation by averaging |
| B4 | Is each view photometrically usable? | #7222 BRDF/radiometry model, Lambertian/Blinn–Phong/Cook–Torrance controls, coupon data | Zero false-usable negative controls; UNKNOWN for missing calibration/material/light inputs |
| B5 | Is the engineering feature identifiable? | #7043 FMQ, matched hole/slot/edge features, CMM/VCMM reference | No rank/conditioning false-accepts on sealed degeneracy tests |
| B6 | Is the measurement metrologically adequate? | #6846 calibrated covariance, GUM-aligned uncertainty propagation, ISO 14253-1 decision rules | Guard-band and uncertainty-aware decision; no covariance without a declared model |
| B7 | Is the view executable? | #5920 exact route/allocation/charging oracle and #5892 source-faithful fleet baseline | Zero collisions, unreachable mandatory views, battery and return-reserve violations |
| B8 | Does the measurement change the decision? | #6752/#4258 observation models plus #6956/#4997 non-anticipative policies | Decision regret and post-action outcome, not entropy-only value |
| B9 | Is evidence traceable and interoperable? | AP242/QIF, NIST STEP Analyzer, persistent-ID round trips | Zero silent identity, datum, PMI or measurement-reference loss |
| B10 | Does the physical procedure detect defects? | Blind reference truth, calibrated instrument, procedure-specific NDE/POD study | Method/material/defect-specific evidence; synthetic results excluded |

## 7. Prospective targets (not achieved)

- 100% exact agreement between Rust and independent oracle for each admitted small instance.
- Zero silent relaxation of requested k; any attainable/capped target must be published with original demand and pool shortfall.
- Zero false-WITHIN frustum certificates in analytically constructed negative/boundary controls.
- 100% separate reporting of view count, domain diversity, sample density, photometric usability, feature conditioning and metrological uncertainty.
- Zero false-usable classifications in frozen photometric negative controls.
- Zero feature-conditioning false-accepts in known rank-deficient cases.
- For larger matched selection pools, report certified ILP lower bounds, feasible incumbents, optimality gap, timeouts and memory separately. A target median gap below 3% and 95th percentile below 10% can be preregistered only after the candidate distribution and runtime budget are fixed.
- Calibration drift: detect stale/out-of-family calibration and abstain; quantify thermal and time-dependent changes before setting numeric tolerances.
- Physical NDE/POD targets must be derived from the actual instrument, procedure, material, defect class, specimen and reference truth.

These are prospective research gates, not achieved results or industry-standard thresholds.

## 8. Portfolio owners and execution order

1. #7002: reproduce the paper's 3D GSD candidate generation, voxel visibility and k-coverage selection.
2. #6925: maintain the exact small k-view oracle, declared failure-domain axis and rectangular whole-support frustum precondition.
3. #7136: combine conditional support-envelope bounds with conservative support-level occlusion; frustum pass alone never sets visibility.
4. #7164: ray-sampling density per non-uniform surface element.
5. #7222: material/illumination/BRDF photometric usability.
6. #7043/#7049: feature measurability and parameter conditioning.
7. #6846: calibrated uncertainty, covariance and conformity decision.
8. #5920/#5892: kinematic access, routes, multi-robot assignment, charging and execution.
9. #6752/#4258, #6956/#2239/#1892/#4997: sensor observations, common causes, decision value and active measurement.
10. Mycelix #6390/#6519: persistent identity, AP242/QIF, evidence custody and authority evidence.

Do not build one universal optimiser in #6925. Compose the specialist contracts through explicit, source-bound interfaces. A planned view is not an acquired measurement, and an acquired point cloud is not a validated characteristic.

## 9. Final success criterion

On the same held-out structures, mesh/element partition, feature requirements, sensors, illumination, robot capabilities and budgets, does the end-to-end workflow produce more independently verified and decision-useful characteristics per lifecycle cost—while reducing false clearance, missed features, unnecessary reinspection and evidence loss?

If it does, show the matched data, independent references, confidence bounds, optimality status and physical domain limits. Until then, call it a promising benchmark result, not "better than SOTA".
