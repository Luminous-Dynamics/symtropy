# Scan-to-BIM 2026 frontier comparator matrix v0.1

**Status:** literature-grounded evaluation design; no methods reproduced or compared by Luminous yet.  
**Owner:** #4116 SCAN-BIM-REPLAY-001.  
**Related:** #4086 / PR #4088, #1607, #1892, #3331, #4043, #2005–#2143 BUILT-INTEROP.

## Research question

Can Symtropy improve the quality, reliability and lifecycle utility of an as-built representation by composing the strongest existing reconstruction, graph-generation, measurement-confidence and commercial workflows—without claiming novelty for their established methods?

The 2026 state of the art already contains distinct approaches with different outputs and evidence ceilings. Compare them on their native tasks first, then run matched common-building experiments where data access permits.

## Comparator matrix

| Comparator | Output and demonstrated scope | Published reported result | How Symtropy should compare |
|---|---|---|---|
| Ersöz & Bosché (2026), geometric matching confidence | Per-IFC-element scan/BIM matching confidence based on Coverage, Distance and Distribution; theoretically visible surface accounts for inter-element occlusion | Evaluated on 46 PCD/BIM pairs and 4,000+ IFC elements across ISPRS, CV4AEC, BIMNet and custom data | Reproduce all three factors and upstream visibility semantics at pinned code/revisions; compare identity robustness, calibration and downstream admissions separately |
| Pan et al. (2026), LLM-enabled multi-agent Scan-to-BIM / Scan-to-Graph | Query interpretation, point-cloud inference, topology-aware IFC generation, semantic graph generation and multi-stage validation; three modalities/datasets | TUMCMS room-area mean absolute deviation 1.45 m² (Llama configuration) / 1.51 m² (Qwen configuration), corresponding to 4.70% / 5.10% mean relative deviations vs manually created BIM | Compare room area, boundary error, wall geometry, graph adjacency/connectivity, IFC/IDS gates, prompt-controlled reconstruction levels, correction person-minutes and false structure claims under matched scenes and prompts |
| Trinh, Hou & Wu (2026), incomplete-point-cloud structural reconstruction | Structural walls, columns, beams and slabs; Heaviside slicing, Boundary Condition Modeling for edge correction and Greedy Snake Slice Completion | Abstract reports >97% area accuracy, 98% perimeter accuracy, 92% moment-of-inertia accuracy, LOA30 and approximately 11 min; some elements approaching LOA40 | Reproduce element-level area/perimeter/section-property metrics on the same element families; use independent metrology and disclose the exact LOA specification/tolerance, sample sizes and time scope |
| Jayasinghe & Brilakis (2026), occlusion completion | Point-cloud completion for Scan-to-BIM; explicitly addresses complete-object-bound leakage | Corrected evaluation removes target-bound leakage; paper reports study-specific EMD and Chamfer-distance reductions of approximately 25% and 40% | Reproduce corrected candidate inputs; use partial-only normalization, hidden regions and independent holdouts; score downstream geometry, elements and topology, not only point-set distance |
| Mobile Device LiDAR / RoomPlan vs TLS (Ersoz & Bosché, ISARC 2026) | Low-cost mobile acquisition versus terrestrial laser scanning in residential/office scenes | Reported 3.5 min/m² TLS versus 0.1 min/m² mobile; room layout 3D-IoU up to 0.93; limitations include wall thickness and positional drift | Compare quality/cost/effort Pareto frontier on the same buildings, independent control/checkpoints and declared task acceptance; do not transfer timing between unmatched scenes |
| Leica Cyclone 3DR | Component-level IFC-vs-scan tolerance analysis, coverage/no-data, missing/out-of-tolerance status, manual review, reports/BCF, clash/progress/BIM analysis | Product capabilities, not a published neutral benchmark result | Matched operator workflow, same scan/BIM/revision, declared tolerances, correction minutes, exports, evidence continuity and accepted-change outcomes |
| Trimble RealWorks | Scan registration, design-model comparison, clash/deviation reporting, as-built modeling | Product capabilities, not a published neutral benchmark result | Same matched workflow, registration/control and model output; record tool versions, processing time, manual edits and model exchange fidelity |
| Autodesk ReCap Pro / Revit | Scan-to-mesh, mesh editing and ReCap-to-Revit handoff | Product capabilities, not a published neutral benchmark result | Measure geometry, semantic/IFC preservation, model authoring effort, revision loss and repair/reverification workflow |

### Source references

- Ersöz & Bosché, *Evaluating confidence in geometric matching between 3D point clouds and BIM models by integrating coverage, distance, and distribution metrics*, Automation in Construction 181 (2026), 106636. DOI: https://doi.org/10.1016/j.autcon.2025.106636
- Pan et al., *LLM-enabled multi-agent framework for automated Scan-to-BIM and Scan-to-Graph reconstruction*, Automation in Construction 188 (2026), 107003. DOI: https://doi.org/10.1016/j.autcon.2026.107003
  - Published repository record: https://www.repository.cam.ac.uk/items/acda5588-8321-4f94-bb9b-30014bea1d54
  - Data are stated to be available on request; this is a reproduction/access dependency, not an assumed public corpus.
- Trinh, Hou & Wu, *Automatic scan-to-BIM framework for building structures with incomplete point cloud data*, Automation in Construction (2026), 107161. DOI: https://doi.org/10.1016/j.autcon.2026.107161
- Jayasinghe & Brilakis, *Adapting occlusion completion methods for scan-to-BIM automation*, Automation in Construction 189 (2026), 107052. DOI: https://doi.org/10.1016/j.autcon.2026.107052
- Ersöz & Bosché, *A Comparative Study of Mobile Device and Terrestrial Laser Scanning-Based Scan-to-BIM Workflows*, Proceedings of the 43rd ISARC (2026), DOI: https://doi.org/10.22260/ISARC2026/0286
- buildingSMART IFC 4.3.2.0 / IFC4.3 ADD2 (ISO 16739-1:2024): https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/
- buildingSMART IDS 1.0: https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/
- USIBD Level of Accuracy Specification v3.1: https://usibd.org/level-of-accuracy/

## Matched-comparison design

### Common inputs

Where source permissions allow, compare on the same building, acquisition, coordinate frame, units, prior model revision, visible/occluded region definitions, acceptance tolerances and independent checkpoints. Freeze source hashes, model hashes, processing options, software versions, hardware, run seeds, and time boundaries.

The exact same holdout building cannot be used for training/tuning of one method and declared unseen by another. Views, scans, synthetic derivatives and model states from one physical building stay in the same building-level group.

For proprietary software, record the purchased version/build and operator expertise. Feature availability and product documentation do not count as benchmark runs.

### Separate outcome tracks

1. **Geometry:** surface distance distributions, area/perimeter/volume, section properties where relevant, dimensions, centroid, angular error, independent checkpoint error.
2. **Room and spatial layout:** room area/volume, boundary alignment, adjacency, connectivity, multi-floor links and space graph accuracy.
3. **Elements:** per-class precision/recall, persistent identity, opening geometry, split/merge/replacement.
4. **Topology and IFC:** wall-wall and wall-slab connectivity, openings, containment, schema validation, IDS compliance, structural semantic diff and graph consistency.
5. **Epistemic quality:** theoretically visible/captured/missing/occluded/unknown surface, interval coverage, calibration, selective risk and abstention.
6. **Human intervention:** correction person-minutes, number/type of manual edits, unresolved issues, rework and review burden.
7. **Lifecycle:** false model mutations per independently accepted change, rejected unsafe changes, time-to-accepted-change, and post-repair re-verification.
8. **Acquisition/compute economics:** scan time, setup time, post-processing time, compute cost, memory/VRAM, energy where available, and quality-versus-cost frontier.

Do not collapse these tracks into a single scalar. A room-area result cannot stand in for wall geometry. A successful IFC parse cannot stand in for semantic preservation. A graph with correct adjacency cannot stand in for accurate geometry. A confidence product cannot stand in for calibrated probability.

## Where Luminous can differentiate

Do not attempt to beat each method at every native subtask in the first phase. The stronger hypothesis is to compose the best qualified components under one lifecycle protocol:

- Reuse #1607 for survey, coordinate frames, control/checkpoints and uncertainty provenance.
- Reproduce Ersöz–Bosché matching confidence as a published baseline.
- Include Pan et al. as a topology-aware IFC and Scan-to-Graph comparator.
- Include Trinh et al. as a structural element-geometry comparator.
- Include corrected completion from Jayasinghe–Brilakis as an occlusion benchmark.
- Use #2005–#2143 for IFC syntax/schema/IDS and source-to-target semantic preservation.
- Use #3331 for stable component identity across manufacturing, installation, inspection and repair.
- Use #1892 for decision-sensitive next-measurement choice.
- Use #1226 for blind-run and reveal receipts.
- Use #4043 for prospective inspect → repair → re-verify outcomes.

The testable Luminous hypothesis is that a cross-tool, evidence-qualified lifecycle can reduce false mutations and rework while maintaining or improving geometric/topological quality and keeping acquisition/verification cost competitive. It remains a hypothesis until a matched P2/P3 experiment demonstrates it.

## Target levels (proposed, not achieved)

- **Reproduction:** match each published baseline on its original or authorized data within its stated numerical tolerance and implementation details.
- **Parity:** on matched held-out buildings, be statistically non-inferior on each native output metric before using the method as a baseline component.
- **Reliability advantage:** improve calibration/abstention and false-mutation rates without reducing geometric/topological quality.
- **Operational advantage:** reduce human correction time and end-to-end cost at matched independently accepted quality.
- **Generalization:** improve worst-building / worst-scene results on truly unseen buildings, not only pooled point-weighted averages.

The first reported frontier should be a Pareto set over accepted geometry quality, topology/semantic fidelity, epistemic risk, human review effort and total cost/time, with all denominators and uncertainty intervals disclosed.
