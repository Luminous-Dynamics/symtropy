# Evidence-Preserving Scan-to-BIM Admission Protocol v0.1

**Status:** research protocol draft; no algorithm, geometry engine, sensor, IFC deliverable or engineering decision is qualified by this document.  
**Owner:** Symtropy #4086 — SCAN-BIM-ADMISSION-001  
**Composes:** #1607 survey evidence; #4043 built-asset robot lifecycle; #4062 built-asset synthetic oracle; #3331 component digital thread.

## 1. Purpose

Define a reproducible evaluation contract for the transition from registered, qualified survey products to an editable as-built building representation and a proposed digital-twin model change.

The benchmark is not a new point-cloud registration engine. It evaluates whether a candidate:
- preserves the provenance and uncertainty of its measured inputs;
- reconstructs geometry without inventing unseen surfaces;
- associates observations with the correct persistent building elements;
- preserves semantic and topological relationships;
- distinguishes real physical change from acquisition, visibility, registration and correspondence artifacts;
- emits a revision-bound candidate delta only when the evidence supports it; and
- requests more observation or abstains when the decision is underdetermined.

The core distinction is:

```text
raw capture
  != survey observation
  != registered / adjusted survey product
  != reconstructed surface
  != semantic building element
  != proposed model delta
  != admitted authoritative asset state
```

Each transition must be explicit, versioned and reviewable. A low point-cloud residual, high segmentation score, valid IFC file or visually plausible model does not independently establish an accurate as-built state.

## 2. Ownership boundaries

| Concern | Existing owner | This protocol consumes |
|---|---|---|
| CRS, frame identity, survey sessions, control adjustment, registration provenance, checkpoint separation, uncertainty | #1607 | Qualified survey products and their validity scope |
| Component identity, manufacturing/installation/inspection lineage | #3331 | Persistent component IDs and lifecycle references |
| Evidence validity and invalidation | #1846 | Validity generations, stale-evidence dispositions |
| Interval classification | #2502 | Conservative bounds and indeterminate outcomes |
| Decision-changing measurements | #1892 | Value-of-information and measurement cost |
| Defect/repair synthetic fixture and independent oracle pattern | #4062 | Fixture conventions and bounded Rust oracle practice |
| Existing-building inspect/repair/re-verify lifecycle | #4043 | Accepted model state and downstream task context |
| Functional output and full-workflow accounting | #3605 | Human review burden, accepted output and rework accounting |

No duplicate registration, coordinate-reference, metrology, component registry, IFC authoring, change-event ledger or general geometry engine is introduced here.

## 3. Research and industry reference points

### Research

- Liu et al., *Dataset and benchmark for as-built BIM reconstruction from real-world point cloud*, Automation in Construction 173 (May 2025), DOI: [10.1016/j.autcon.2025.106096](https://doi.org/10.1016/j.autcon.2025.106096). The BIMNet study describes 25 indoor scans, 116,507,604 points, 382 rooms and 8,710 m², with geometric and topological evaluation. This is an external benchmark candidate; data access and redistribution rights must be checked before use.
- Ersöz & Bosché, *Evaluating confidence in geometric matching between 3D point clouds and BIM models by integrating coverage, distance, and distribution metrics*, Automation in Construction 181 (January 2026), DOI: [10.1016/j.autcon.2025.106636](https://doi.org/10.1016/j.autcon.2025.106636). It reports validation on 46 point-cloud/BIM pairs and more than 4,000 elements. The coverage, distance and distribution decomposition is a useful baseline; self-occluded geometry must not be mistaken for observed evidence.
- Patil, Sowmya & Kalantari, *Automatic Scan-to-BIM: The Impact of Semantic Segmentation Accuracy on Opening Detection*, ISPRS Annals, published 3 July 2026, DOI: [10.5194/isprs-annals-XI-1-2026-437-2026](https://doi.org/10.5194/isprs-annals-XI-1-2026-437-2026). The paper reports task-specific differences between door and window detection and argues that semantic segmentation metrics alone do not establish geometric reconstruction accuracy.
- Jayasinghe & Brilakis, *Adapting occlusion completion methods for scan-to-BIM automation*, Automation in Construction 189 (September 2026), article 107052, DOI: [10.1016/j.autcon.2026.107052](https://doi.org/10.1016/j.autcon.2026.107052). The paper identifies bounding-box leakage as a key source of inflated point-cloud completion results, proposes a corrected benchmark without that leakage, and reports roughly 25% lower EMD and 40% lower Chamfer Distance on its evaluated datasets. Treat these as study-specific results, not a transferable superiority guarantee. Our benchmark must withhold reference bounds and split by building/scene/capture lineage.
- **ISPRS Benchmark on Indoor Modelling:** six point clouds acquired with different sensors and scene conditions, including high-clutter/occluded and non-Manhattan scenes; one scene's windows are hidden by blinds. This is a useful cross-sensor generalization set alongside BIMNet. Verify the available BIM/reference package and terms before using it as a scored benchmark. https://isprs.org/resources/datasets/benchmarks/IndoorModeling/dataset.aspx
- You, Chen & Xue, *Automated scan-to-BIM for construction digital transformation: Conceptual framework, processing methods and best-practice guidelines*, Automation in Construction 182 (February 2026), DOI: [10.1016/j.autcon.2025.106755](https://doi.org/10.1016/j.autcon.2025.106755). A review of 58 documented cases provides a workflow and method map, not a single end-to-end performance baseline.
- *From Measured Geometry to Editable BIM: A Representation-Centered Review of Existing-Building Scan-to-BIM Reconstruction*, Buildings 16(20), 4095 (2026): https://www.mdpi.com/2075-5309/16/20/4095. The review highlights loss of uncertainty/provenance and element correspondence at representation boundaries.

### Commercial capability baselines

The benchmark must compare relevant, version-pinned product workflows rather than assume that any vendor covers the entire lifecycle.

- **Leica Cyclone 3DR 2025.2 and later:** component-level IFC-versus-point-cloud tolerance checking with within-tolerance / out-of-tolerance / missing statuses, adjustable tolerance, exclusion-zone and minimum-coverage parameters, manual BIM-object review, and PDF/BCF reporting. The 2026 documentation also covers deviation analysis, progress monitoring, clash detection, visual reports and tolerance checking. Sources: https://rcdocs.leica-geosystems.com/docs/cyclone-3dr-2025-2-0-cyclone-3dr-release-notes and https://rcdocs.leica-geosystems.com/cyclone-3dr/2026.0/Ex_ToleranceChecking
- **Trimble RealWorks:** processing and analysis of point-cloud data, target/targetless registration, comparison with design models for clash analysis, deviation reports, as-built modeling, geo-referencing and export to other software. Source: https://www.trimble.com/en/products/building-construction-field-systems/trimble-realworks
- **Autodesk ReCap Pro 2026:** local scan-to-mesh conversion, segmented-mesh editing and an integrated ReCap-to-Revit scan-to-BIM workflow. Sources: https://help.autodesk.com/cloudhelp/ENU/RealityCapture-New/files/whats_new_recap.html and https://help.autodesk.com/cloudhelp/ENU/Reality-Capture/files/Scan_to_Mesh_Options/Scan_to_Mesh_Local/scan_to_mesh_revit.html

These are serious comparators, not strawmen. The following are **not novelty claims**: point-cloud registration; geometric deviation/heatmaps; component tolerance status; progress, clash and no-data/coverage reporting; PDF/BCF outputs; scan-to-mesh; and model export. Commercial products already provide meaningful versions of many of these functions.

### Competitive positioning hypothesis

Do not set the objective as “replace Leica/Trimble/Autodesk” or “beat their scan-to-BIM feature list.” Treat those tools and their specialist algorithms as external capability providers. The Luminous research hypothesis is a cross-tool qualification and lifecycle layer:

1. Preserve an independently reviewable evidence chain across proprietary and open tools, asset revisions and representations.
2. Carry coordinate-frame identity, capture lineage, calibration, uncertainty, coverage and independent checkpoint status into the downstream element/change decision.
3. Preserve persistent component identity and semantic/topological relationships through add/remove/modify/split/merge/replacement operations.
4. distinguish true physical change from acquisition differences, object pose, occlusion, registration drift and correspondence error.
5. Make uncertainty and abstention first-class: “unknown / inspect again” is preferable to a falsely confident model mutation.
6. Bind every candidate model delta to an exact base model digest/revision, supporting evidence, rule/profile revision, review authority and invalidation conditions.
7. Measure the full workflow: human preparation, scan setup, calibration, manual editing, review, corrections, downstream repair acceptance and maintenance—not just point-cloud algorithm runtime.

These are **hypotheses to benchmark**, not claims that commercial systems cannot support them. The first competitive demonstration must use matched inputs and a named product/version/configuration, preserve each tool's own outputs, and compare them with an independent referee on false mutations, completeness, geometric quality, uncertainty coverage, review burden and accepted downstream work.

## 4. Paradigm

Move from **best-fit model generation** to **evidence-qualified representation change**.

1. Preserve raw capture, qualified survey, measured surface, fitted geometry, semantic element and proposed model delta as distinct linked artifacts.
2. Treat element correspondence as a hypothesis with explicit alternatives, not an automatic consequence of spatial proximity.
3. Distinguish physical change, changed object pose, occlusion, changed acquisition, registration drift and correspondence failure.
4. Treat unseen or insufficiently observed geometry as unknown. Do not hallucinate hidden surfaces or count them as inspected.
5. Preserve uncertainty through fitting, regularization, segmentation, IFC instantiation and later model updates.
6. Treat a model update as a reversible, revision-bound candidate transaction with preconditions, evidence links and invalidation rules.
7. Select further observations by expected decision change, not raw point count or generic information gain.

## 5. Reference planes and candidate contract

Each case declares separate, digest-bound reference planes:

1. **Design/prior model:** asset, element and assembly identity; exact model revision; units; coordinate frame; topology; expected element classes and relations.
2. **Independent as-built reference:** reference geometry, control/checkpoints, measurement method, calibration and uncertainty. Checkpoints used for scoring must not be used for registration, fitting or tuning.
3. **Candidate observations:** capture/source identity, modality, time/clock domain, sensor/calibration revision, frame transform, visible/unobserved regions, uncertainty and shared lineage.
4. **Evaluator truth:** synthetic known-answer labels and expected dispositions. Candidate code must not read this plane.
5. **Reconstruction and proposed delta:** candidate element geometry/class/correspondence, topology graph, uncertainty, source links, model preconditions and operation (add, remove, modify, split, merge or abstain).

Candidate outputs must retain:
- persistent asset/component identity or an explicit unresolved correspondence;
- the source evidence and lineage for every admitted geometric/semantic assertion;
- geometry, units, frame, revision and uncertainty;
- visibility/coverage and excluded/occluded regions;
- topology relations and their supporting evidence;
- algorithm, configuration and dependency identity;
- disposition and stable reason codes;
- exact base-model generation/digest and proposed delta digest.

A digest establishes byte identity, not measurement correctness. Derived views of one capture share lineage and do not count as independent measurements.

## 6. Required dispositions

Use typed outcomes rather than a boolean success flag:

- `ADMIT_CANDIDATE_DELTA`
- `PARTIAL_WITH_UNKNOWN`
- `REQUEST_ADDITIONAL_OBSERVATION`
- `REAL_CHANGE_CANDIDATE`
- `NO_CHANGE_SUPPORTED`
- `CONFLICTED_CORRESPONDENCE`
- `STALE_OR_INVALID_EVIDENCE`
- `REJECTED_TOPOLOGY`
- `OUT_OF_DOMAIN`
- `NOT_EVALUATED`

No outcome in this benchmark grants structural acceptance, legal survey status, code compliance, repair authorization or return-to-service authority.

## 7. V0.1 synthetic corpus

Begin with bounded room/panel scenes and deterministic point samples. Keep the first oracle independent of point-cloud learning and full IFC authoring.

### Geometry and visibility
- plane, wall, floor, ceiling, corner, opening, door, window, panel, frame and fastener;
- exact/near-exact geometry, sparse points, nonuniform density, missing surfaces and self-occlusion;
- one view, overlapping views, changed viewpoint and duplicate capture lineage.

### Identity, semantics and topology
- correct, ambiguous and incorrect element correspondence;
- moved, replaced, split, merged, duplicated, missing and orphaned elements;
- valid and invalid adjacency, containment, connectivity and opening-boundary relations;
- semantically wrong class with geometrically plausible fit.

### Change and adversarial conditions
- true addition, removal and displacement;
- benign furniture/equipment movement and open/closed door pose;
- registration bias/drift, wrong frame, unit mismatch and stale design revision;
- conflicting observations, common-source correlated observations and overconfident uncertainty;
- unknown region incorrectly extrapolated behind an occluder;
- topology broken by a geometrically small but semantically meaningful model edit.

## 8. Metrics

Never collapse the benchmark into one scalar.

| Plane | Report |
|---|---|
| Geometry | Symmetric point-to-surface/surface-to-point distance distributions; EMD and Chamfer Distance where applicable; median/p95/p99; normal angle; dimensional/datum error; completeness; object-bound error; independent-checkpoint residual |
| Visibility | Visible-surface coverage; unobserved/occluded fraction; coverage by region and modality; missing-view error |
| Semantics | Per-class precision/recall; opening detection; class confusion; component association precision/recall |
| Topology | Precision/recall of adjacency, containment, connectivity and opening-boundary edges; duplicate/orphan/missing relations |
| Change | Precision/recall by change type; false-change rate on no-change pairs; missed-change magnitude; benign-pose confusion |
| Admission | False model-mutation rate; stale-precondition rejection; disposition/reason correctness; abstention and unresolved-region rate |
| Uncertainty | Interval coverage and width for object bounds and element geometry; calibration by class/geometry; risk-versus-coverage; sensitivity to input perturbations |
| Workflow | Accepted candidate deltas per total crew-hour; review time; added measurement cost; false-update rework; time to resolve unknowns |

Use task/project tolerances declared before execution. Do not select a tolerance after seeing results. Separate registration residuals from independent checkpoint errors. Count self-occluded surface separately from visible surface. Retain all rejected, incomplete and indeterminate cases.

## 9. Baselines and fair comparison

With the same scans, model revision, checkpoints, visible-surface denominator, tolerances, operating conditions and human/compute budget, compare:
1. manual expert reconstruction and change adjudication;
2. instrumented expert workflow;
3. relevant, pinned Leica Cyclone 3DR, Trimble RealWorks and Autodesk ReCap Pro/Revit configurations;
4. reproducible published methods and dataset-specific baselines;
5. the Luminous pipeline, with every model, solver, adapter and configuration version frozen.

Record preparation, registration, manual editing, scan setup, review, corrections and rework. Do not compare vendor feature availability or unmatched case-study percentages as if they were causal performance results.

## 10. Promotion gates

### P0 — finite synthetic oracle
- 100% expected disposition and reason-code agreement on the frozen suite.
- Zero unsupported assertions about hidden/unobserved surfaces.
- Zero model mutations on stale frames/revisions or conflicted identity.
- Geometry, semantics and topology failures evaluated independently.

### P1 — external dataset replay
- Reproduce a source baseline before claiming improvement.
- Evaluate BIMNet and the six-scene ISPRS Indoor Modelling benchmark as separate dataset families; preserve their original protocols.
- Validate data access, ground truth, licence and redistribution rights before using BIMNet or any other external corpus.
- Enforce building/scene/capture-group separation and reject any candidate input path containing reference/ground-truth bounds.
- Publish geometric, semantic, topological, visibility and uncertainty results separately.

### P2 — guarded physical article
- Independently measured reference geometry and held-out checkpoints.
- Blind adjudication of element correspondences and proposed changes.
- Metrology uncertainty materially smaller than the decision margin under an approved uncertainty budget.
- No physical actuation or repair authority follows from scan-to-BIM admission.

### P3 — prospective building operation
- Separate site/building holdouts, design revisions, environmental and occlusion strata.
- Demonstrate lower false-mutation and rework burden without degrading independent geometric quality, completeness, safety or maintainability.
- Account for all human review and measurement work.

Do not set numeric superiority targets until the baseline is reproduced on matched inputs. The first target is Pareto dominance: no worse independent geometry/topology quality while lowering false mutations and preserving more provenance/uncertainty without hiding human review cost.

## 11. First implementation sequence

1. Freeze the model-delta/evidence schema and required dispositions.
2. Add an evaluator-independent Rust micro-corpus and known-answer oracle for geometry, element correspondence, topology and admission.
3. Add adversarial/metamorphic tests for hidden surfaces, duplicate lineage, stale frames/revisions, benign movement, topology damage, uncertainty crossing a tolerance and conflicting correspondence.
4. Add public-data adapters only after provenance and licence checks.
5. Add human/commercial matched trials after P0/P1 protocols and reference tooling are reproducible.

## 12. Claim ceiling

Synthetic success establishes only agreement with a finite synthetic contract. It does not establish real sensor performance, field accuracy, IFC completeness, building safety, code compliance, structural adequacy, legal survey validity, repair quality or lifecycle savings. Those require separate, task-specific evidence and competent acceptance authorities.
