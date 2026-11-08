# Scan-to-BIM paradigm shifts and benchmark targets v0.1

**Status:** research and experiment charter; proposed targets are not achieved results.  
**Owner:** #4116 SCAN-BIM-REPLAY-001.  
**Code track:** PR #4151, replay-contract manifest v0.2.  
**Research review date:** 2026-11-08.

## Executive thesis

Do not build another point-cloud-to-mesh demonstration. Build an **evidence-qualified, decision-aware building reconstruction system**.

The competitive question is not whether a model looks complete. It is whether the system can establish, for each building element and each proposed model mutation:

1. what was directly observed;
2. what is hidden, outside coverage or unknown;
3. what was inferred or completed rather than measured;
4. which reference frame, units, calibrations, tolerances and algorithms were used;
5. what geometric, semantic and topological claims are supported;
6. who or what independently verified the claim; and
7. whether more measurement could change the downstream decision enough to justify its cost.

The resulting artifact should be a **proof-carrying BIM**: an interoperable IFC/graph representation with a revision-bound evidence map, uncertainty and provenance references, explicit unknowns, and independently replayable acceptance decisions. This is a system-level research hypothesis, not an existing product capability or a claim that IFC itself supplies such proof.

## Paradigm shifts

### 1. From complete geometry to evidence-qualified geometry

Every surface patch and object property must carry an epistemic state:
- observed directly;
- observed with declared measurement uncertainty;
- inferred/interpolated from partial geometry;
- imported from a prior model or external record;
- occluded / outside the sensor envelope;
- unknown / contradictory.

A completion algorithm may propose a full wall or opening, but it cannot turn unseen geometry into observed geometry. IFC output should remain interoperable; provenance and dense point-to-surface evidence may be linked through revision-bound external artifacts and the existing Mycelix evidence/custody owners.

**Research question:** can this reduce false as-built claims while retaining useful reconstruction coverage?

### 2. From point-density optimization to decision-sufficient acquisition

More points are not automatically more information. A second targeted viewpoint, range observation or close-range image is valuable when it can change an acceptance, repair, safety-review or coordination decision.

Compose with #1892 SPATIAL-VOI-001. Use its finite decision states and deterministic impact vector. Do not invent an expected value of information from uncalibrated confidence scores.

A scan action should declare:
- predicted surface/element coverage and blind spots;
- time, setup and processing cost;
- measurement uncertainty and correlation with existing captures;
- the finite outcomes it may resolve;
- which decision can change under each outcome.

Benchmark against a density-only policy and a fixed-grid capture policy. A targeted action that changes no decision should not win merely because it adds more points.

### 3. From one best-fit model to competing, falsifiable hypotheses

Maintain multiple geometric/topological hypotheses where observations are insufficient: e.g. door opening versus occlusion, one wall with a recess versus two adjacent walls, a planar wall versus a tilted/non-Manhattan surface.

Each hypothesis must carry:
- supporting observations and contradictory evidence;
- likelihood or score semantics with calibration status;
- affected geometry/topology/identity;
- what additional measurement could discriminate it;
- an explicit abstention state.

The evaluator should reward correct abstention and penalize confident false structure. Do not treat a model's self-reported confidence as ground-truth probability.

### 4. From independent geometry and semantics to joint geometric-topological consistency

A wall is not merely a plane. It participates in spaces, openings, intersections, slabs, columns, containment and circulation.

Evaluate two coupled objects:
- metric geometry (surfaces, dimensions, planes, openings and uncertainty);
- a semantic/topological graph (adjacency, connectivity, containment, openings, element identity and relationships).

A geometrically plausible wall with an impossible room graph is not a valid reconstruction. A correct graph with inaccurate boundaries is not a valid reconstruction either. Preserve both scores and failure reasons separately.

### 5. From a single global confidence scalar to calibrated selective prediction

Do not collapse coverage × distance × normal-distribution similarity into an authority score. Preserve upstream Ersöz–Bosché factors as an attributed baseline, and measure:
- empirical error by declared confidence/uncertainty band;
- interval coverage at 90%, 95% and 99%;
- calibration error and interval width;
- selective risk versus accepted coverage as the system abstains more;
- false confident claims on hidden or unobserved surfaces;
- performance under capture-density, scanner and environment shift.

A confidence method should be better calibrated before it is allowed to control admission or drive an automated repair.

### 6. From a model handover to a persistent physical-asset evidence thread

Bind element identity, IFC revision, source scan, transformation, calibration, measurement, candidate output, approval and post-repair inspection to the existing component and evidence owners:
- #1607 survey provenance, frames and independent checkpoints;
- #3331 stable component/asset identity and supersession;
- #1226 blind-run commitments and receipts;
- Mycelix custody/approval semantics.

Do not create a duplicate component registry, generic evidence event log, survey engine or approval ledger. A signature proves an attestation's origin and integrity, not the truth or engineering acceptability of the geometry.

## Current benchmark landscape and what it actually proves

### Historical ISPRS Indoor Modelling

The official results page evaluates walls only and defines:
- completeness as intersecting reference surface area divided by total reference surface area;
- correctness as intersecting source surface area divided by total source surface area;
- accuracy as the median absolute distance from reference-surface samples to the closest source-model surface, as a function of cutoff distance.

The official table reports values at buffer/cutoff 10 cm and plots curves through 0–15 cm. Source: https://isprs.org/resources/datasets/benchmarks/IndoorModeling/results.aspx

The displayed 10 cm results show a **Pareto trade-off**, not one universal winner. Examples:
- TUB1: Ochmann et al. report 0.93 completeness, while Lim et al. report 0.80 correctness; Tran et al. report 1.34 cm accuracy with 0.84 completeness and 0.46 correctness.
- TUB2: Tran et al. report 0.84 completeness and 2.12 cm accuracy; Ahmed et al. report 0.92 correctness with 0.76 completeness and 3.06 cm accuracy.
- Fire Brigade: Tran et al. report 0.94 completeness but 0.38 correctness; Lim et al. report 0.82 completeness and 0.50 correctness.
- UoM: Tran et al. report 0.87 completeness, 0.77 correctness and 0.92 cm accuracy.

These are historical table values, not results reproduced by Symtropy. No single number is enough to rank the methods. Reproduce the curves and all three dimensions at the table's rounding precision. Treat TUB1/TUB2 as two captures of one physical building; they provide two acquisition conditions but only one independent building group.

### Geometric element matching confidence

Ersöz & Bosché (2026), Automation in Construction 181, 106636, DOI 10.1016/j.autcon.2025.106636, combine coverage, distance and distribution indices and account for theoretical visibility under inter-element occlusion. Their reported study covers 46 point-cloud/BIM pairs and 4,000+ IFC elements.

Upstream implementation: https://github.com/CyberbuildLab/pcd-bim-matching-confidence

Required baseline protocol:
- pin the upstream Apache-2.0 source revision and dependencies;
- reproduce its original definitions and sampling exactly;
- retain component scores, sample counts and denominators;
- test duplicate/missing IFC Names against stable GlobalIds;
- test normal orientation, voxel density, KL asymmetry and blocker taxonomy;
- do not call the aggregate product calibrated uncertainty or use it as an admission authority.

### Occlusion completion and leakage

Jayasinghe & Brilakis (2026), Automation in Construction 189, 107052, DOI 10.1016/j.autcon.2026.107052, identify reference/complete-object bounding-box leakage during partial-input normalization. Their corrected benchmark reports study-specific EMD and Chamfer-distance reductions of approximately 25% and 40% after method changes.

Source: https://www.repository.cam.ac.uk/items/98958304-1013-47cc-a6e6-66a121a23e55

Symtropy must run partial-only normalization and prove that full target bounds, reference geometry, held-out membership and evaluator labels are inaccessible to candidate execution. Score completion on hidden surfaces and downstream geometry/IFC, not Chamfer/EMD alone.

### Agentic Scan-to-BIM and Scan-to-Graph

Pan et al. (2026), Automation in Construction 188, 107003, DOI 10.1016/j.autcon.2026.107003, introduce a multi-agent framework with intent-aware IFC and graph generation, topology-aware connectivity and user-controlled reconstruction detail. The TUMCMS experiment reports room-area mean relative deviations of 4.70% and 5.10% for two model configurations.

Source: https://www.sciencedirect.com/science/article/pii/S092658052600244X

Compare room area, boundary distance, graph topology, IFC validity, controllable reconstruction level and expert correction effort on matched data. Data access is a reproduction dependency; do not assume the paper's corpus is freely available.

### Incomplete structural-element reconstruction

Trinh, Hou & Wu (2026), Automation in Construction 191, article 107161, DOI 10.1016/j.autcon.2026.107161, report over 97% area accuracy, 98% perimeter accuracy and 92% moment-of-inertia accuracy for their evaluated structural components, with LOA30 and some elements approaching LOA40. Their method combines boundary correction, slicing and geometric completion.

Source: https://www.sciencedirect.com/science/article/pii/S0926580526004024

Reproduce by structural element family and identical tolerance/LOA definitions. This is not comparable to an indoor wall completeness score or a room graph metric.

### Measurement uncertainty

Xu, Wysocki & Holst (2026), ISPRS Annals XI-1-2026, 463–471, DOI 10.5194/isprs-annals-XI-1-2026-463-2026, investigate point-level MLS uncertainty prediction under limited ground truth. They report XGBoost comparable to Random Forest and approximately three times faster in their evaluated setting.

Source: https://isprs-annals.copernicus.org/articles/XI-1-2026/463/2026/

This is a useful prediction baseline, not a substitute for instrument calibration, independent checkpoints or uncertainty traceability.

### Opening detection

Patil et al. (2026), ISPRS Annals XI-1-2026, 437–445, DOI 10.5194/isprs-annals-XI-1-2026-437-2026, study how segmentation accuracy affects opening detection.

Source: https://isprs-annals.copernicus.org/articles/XI-1-2026/437/2026

Separate door/window detection, opening dimension error, centroid error, 3D IoU, wall boundary quality and wall-opening topology. Segmentation quality is not geometric opening accuracy.

## Industry baseline: compare capabilities, then measure outcomes

| Platform | Publicly documented workflow | What Symtropy must measure to claim an advantage |
|---|---|---|
| Leica Cyclone 3DR | IFC/RVT support, model-vs-scan inspection, deviation colour maps, BCF notes, PDF/OBJ/FBX reports, progress monitoring and scripting | Same scan/model/tolerance/operator; accepted deviation detection, correction time, BCF/IFC round-trip fidelity, false alarms, review evidence and cost |
| Trimble RealWorks | Large point-cloud processing, as-built workflows, clash analysis, multi-trade deviation reports and office-to-field integration | Same scan/model/control; registration quality, as-built review time, deviation coverage, deliverable fidelity and cost |
| Autodesk ReCap Pro / Revit | Point-cloud indexing/registration/cleanup, scan-to-mesh workflows, local meshing, export to downstream Autodesk products | Time/cost for registered scan and usable mesh, downstream semantic authoring effort, IFC exchange fidelity, geometry and revision loss |
| buildingSMART IFC 4.3.2.0 / IDS 1.0 | Open exchange schema; machine-readable alphanumeric information requirements and IFC checking | Schema validity, semantic preservation, IDS compliance and explicit separate geometric/topological validation |

Official industry sources:
- Leica Cyclone 3DR BIM analysis: https://leica-geosystems.com/products/laser-scanners/software/leica-cyclone/leica-cyclone-3dr/powerful-bim-analysis
- Trimble RealWorks: https://www.trimble.com/en/products/building-construction-field-systems/trimble-realworks
- Autodesk ReCap support: https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/Role-of-Autodesk-ReCap-in-scan-to-BIM-workflows.html
- Autodesk Scan to Mesh: https://help.autodesk.com/view/RECAP/ENU/?contextId=scan_to_mesh
- IFC official version table: https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/
- IDS 1.0 scope and limitations: https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/

Important standards boundary: buildingSMART lists IFC 4.3.2.0 / IFC4.3 ADD2 as the official IFC release corresponding to ISO 16739-1:2024. IDS 1.0 is an approved information-requirements standard for IFC object attributes, properties, quantities, classifications and relations; buildingSMART explicitly states that IDS does not cover geometric aspects. Therefore geometry, topology, survey accuracy, safety and code compliance require separate evaluators.

USIBD LOA v3.1 introduces guidance for standard-deviation-based tolerance specification. Use its project-defined requirements as an acceptance-profile reference; do not infer a hard tolerance from sigma without distributional assumptions and the applicable coverage factor. Source: https://usibd.org/level-of-accuracy/

## Benchmark scorecard

Keep these tracks independent. Do not make one weighted score that can hide unsafe epistemic or identity failures.

| Track | Metrics | Required controls |
|---|---|---|
| Historical geometry | ISPRS completeness/correctness/accuracy curves and 10 cm table values | Exact historical protocol, wall-only, TUB1/TUB2 grouped by building |
| Element geometry | Surface distance median/p95/p99, area/perimeter/volume, dimension/centroid/orientation error, independent checkpoint error, 3D IoU | Non-Manhattan planes, curved elements, sparse/noisy scans, outlier and unit/frame controls |
| Semantics | Per-class precision/recall/F1; door/window detection | Imbalanced classes, duplicate IFC names, missing names, unseen classes |
| Topology | Room adjacency/connectivity precision/recall/F1, wall-slab, wall-wall, opening-boundary consistency | Disconnected rooms, false openings, missing walls, repeated bays and multi-floor links |
| Identity | GlobalId correspondence, split/merge/replacement/unknown accuracy | Duplicate/missing names, cloned GUIDs, superseded models and element replacement |
| Epistemic quality | 90/95/99% interval coverage, interval width, calibration error, selective risk vs abstention, unobserved-region false-claim rate | Independent control checkpoints, occluded/hidden areas and sensor shift |
| Admission/lifecycle | False acceptance, false rejection, false model mutation per accepted physical change, stale-reference rejection | Hard-negative and canary controls; inspect → repair → re-verify |
| Human operations | Expert correction person-minutes, unresolved issues, edits per accepted object, rework rate | Same operator expertise, same inputs and task definition |
| Acquisition economics | Scan/setup/processing/review time, cost, memory/compute, quality-vs-cost Pareto frontier | Matched buildings, capture modality, hardware and quality target |

## Proposed target ladder (targets, not current achievements)

### Gate A — exact reproducibility
- P0 replay manifest binds source data, candidate source/build/model/config, output artifacts, coordinate frame/units, tolerances, sampling and metric definitions.
- Candidate and evaluator artifacts have independently declared roots and actual byte hashes.
- Candidate execution is isolated from evaluator references by OS/process controls; a reference-only canary proves no read path.
- P1 matches published ISPRS values within displayed rounding precision, or publishes unresolved differences without threshold adjustment.

### Gate B — geometry and semantic parity
- Non-inferior to the strongest appropriate published baseline on each native output metric.
- Per-scene and per-building results, not only pooled point averages.
- No increase in false hidden-surface claims or false model mutations.
- IFC schema/IDS pass, geometric and topology gates separately qualified.

### Gate C — selective reliability advantage
- On a frozen negative-control suite: zero false acceptance of declared hard blockers.
- 100% detection of injected unit, frame, stale revision, duplicate identity, target-bound leakage and revoked-digest controls.
- Empirical 95% uncertainty intervals cover independent ground-truth errors at the declared rate, with interval width reported.
- As abstention rises, selective risk decreases monotonically on the controlled corpus; report risk–coverage curve and operating point.

### Gate D — operational advantage
- At matched independently accepted quality, target at least 20% reduction in expert correction person-minutes versus a documented expert workflow; require a confidence interval and no increase in false acceptance.
- Demonstrate lower total acquisition + processing + review cost on a quality-vs-cost Pareto frontier, not merely faster inference.
- For decision-sensitive capture, demonstrate fewer scans or lower review cost without degrading the downstream decision or acceptance quality.

The 20% target is an internal product hypothesis, not a published SOTA result or a universal industry threshold. Reassess it after measuring a credible manual/commercial baseline.

## First hostile experiment matrix

Start with small, deterministic, hand-checkable scenes and include matched pairs that differ in exactly one causal factor:

1. Full-object bounds supplied to candidate versus partial-only normalization.
2. Identical wall observed at multiple densities/voxel sizes.
3. One scan with a hidden window versus a scan from a second viewpoint that reveals it.
4. Furniture occlusion versus truly missing wall surface.
5. Self-occluded surface versus outside-FOV surface.
6. Door-sized aperture versus wall patch removed by occlusion.
7. Two parallel walls with repeated geometry and duplicate IFC Names but distinct GlobalIds.
8. Missing IFC Name but valid GlobalId.
9. Same geometry in metres, millimetres, international feet and US survey feet.
10. CRS/frame offset, rotation, axis-order swap and stale control network.
11. Non-Manhattan wall, curved wall and sloped ceiling.
12. Uniform versus non-uniform point sampling with identical ground-truth geometry.
13. Reversed surface normals with otherwise identical geometry.
14. One-room IFC with correct geometry but broken adjacency graph.
15. Correct topology but inaccurate room boundaries.
16. An inferred hidden wall marked as measured — expected hard failure.
17. A second measurement that changes a repair decision versus a denser scan that does not.
18. A physical change, an unchanged scene, and a model-only revision change.
19. Duplicate candidate/evaluator bytes, altered candidate output, stale reference and path/symlink escape.
20. Same accepted repair before/after independent verification with stable component identity.

Run a single-factor ablation for each proposed improvement; publish the exact frozen inputs, candidate digest, output digest, baseline and failure mode.

## Execution order

1. Finish P0 candidate execution/output/profile binding and OS isolation design without weakening any exact-head gate.
2. Reproduce official ISPRS scoring from the official definitions and historical outputs.
3. Reproduce Ersöz–Bosché factors and corrected occlusion completion, with upstream attribution.
4. Add topology/opening/identity evaluators and independent uncertainty calibration.
5. Run prospective expert and commercial comparisons on matched buildings with recorded labor and cost.
6. Connect admitted model changes to #3331/#1226/#4043 lifecycle evidence and #1892 decision-sensitive capture.

**Do not claim SOTA superiority before Gates A and B.** The first defensible breakthrough is a measurable reliability and lifecycle advantage at matched independently accepted geometry/topology and cost—not a more impressive render or a larger pooled point count.
