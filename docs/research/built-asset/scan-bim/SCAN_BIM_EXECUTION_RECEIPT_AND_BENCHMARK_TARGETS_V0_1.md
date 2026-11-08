# Scan-to-BIM execution receipt and benchmark targets v0.1

**Status:** P0.3 schema contract and research targets; no geometric reconstruction or external replay is claimed.  
**Issue:** #4116 — SCAN-BIM-REPLAY-001.  
**Implementation:** stacked draft PR after #4151.

## Paradigm shift: every model is a reproducible claim about a measured world

A Scan-to-BIM output is not just a mesh or IFC file. It is a claim that a particular candidate pipeline, using identified measurements in a declared frame and under a frozen tolerance/sampling/metric profile, inferred a particular building state.

The receipt must make this chain replayable:

`dataset + scene lineage → candidate build/config/model → declared frame + units → sampling + normalization → metric/tolerance profiles → candidate IFC/geometry/graph outputs → byte digests → evaluator-only reference → result`

This does not mean the receipt proves the physical building is correct. It proves which inputs, software identity, declared parameters and output bytes were evaluated. Physical accuracy still depends on reference quality, independent metrology, the metric implementation and the task-specific acceptance authority.

## Manifest v0.2 contract

The Rust contract extends the previous v0.1 metadata/visibility manifest with:

### Candidate execution identity

- Stable pipeline identifier.
- Exact 40-character source commit SHA.
- Toolchain/runtime identity.
- Explicit command argument vector.
- Executable, configuration and optional model-weight artifact IDs.
- Local candidate runtime artifacts with safe relative paths and SHA-256 recomputed from actual bytes.
- Exact per-scene candidate output artifacts (at least one IFC output per scene), bound to scene IDs and byte-verified under the candidate root.

The executable/configuration/model artifact IDs must resolve to artifacts with the corresponding typed roles. Candidate output hashes are not merely strings: the verifier opens the referenced files beneath the candidate root and recomputes SHA-256. The JSON receipt emits runtime artifact identities, output artifact IDs, scene bindings, roles, relative paths and digests—not just output counts. Candidate/evaluator relative paths must be unique within their respective roots.

### Coordinate frame and units

Every scene declares:
- frame ID;
- CRS / local-frame identity;
- axis ordering;
- handedness;
- coordinate linear unit;
- vertical datum identity;
- an artifact ID for the scene's registration transform, which must resolve to a typed candidate `registration_metadata` artifact whose exact bytes are verified under the candidate root.

The tolerance and sampling profiles use canonical metres. This avoids silently comparing a 50 mm tolerance with a coordinate frame expressed in feet or millimetres. The verifier records units and emits an explicit unit-to-metre conversion factor (1.0, 0.001 or 0.3048 for the supported units); it does not transform coordinates or prove that the submitted CRS/transform is physically correct. Transform metadata is byte-bound, not independently surveyed. Coordinate transformation artifacts and survey control belong to #1607. Coordinate transformation artifacts and survey control belong to #1607.

### Tolerance profile

A replay identifies the requirement revision and a finite positive surface-distance limit in metres, with a decision policy:
- `report_only`;
- `hard_limit`;
- `statistical_interval`.

The current P0.3 contract records the policy; it does not implement a project-specific acceptance engine. Statistical intervals must state the uncertainty model, coverage factor and assumptions before a later evaluator can make an acceptance decision.

### Sampling and normalization profile

Bind reference surface spacing, scan voxel spacing and metric distance cutoff in metres. The normalization mode is explicit. `reference_derived_full_bounds` is rejected: partial-input normalization must not receive complete-object bounds from evaluator/reference data. This implements the key leakage lesson in Jayasinghe & Brilakis (2026), whose corrected unbounded completion benchmark identifies full-object bounding boxes as hidden information.

### Metric profile and direction

A metric suite is identified by ID/version, scope, accuracy direction, definition authority, surface buffer and accuracy cutoff. The v0.2 contract uses a typed `definition_authority` to prevent two conflicting published descriptions from being collapsed into one unlabeled “official” scorer:

| Definition authority | Direction encoded | Meaning |
|---|---|---|
| `official_results_page` | `reference_to_candidate` | Current official leaderboard page: reference-surface samples to closest surface in submitted source model |
| `final_results_paper_2021` | `candidate_to_reference` | 2021 ISPRS final-results paper equation: submitted source samples to closest reference surface |
| `bidirectional_diagnostic` | `bidirectional_diagnostic` | Compute and retain both directed distributions; not a substitute for either published definition |

Both sources agree on the other reported measures at the high level:
- completeness = intersection area / total reference surface area;
- correctness = intersection area / total submitted source surface area;
- the historical competition reports the metrics for wall elements only.

The official ISPRS Indoor Modelling results page says accuracy is the median absolute distance from reference-surface samples to the closest surface in the submitted source model, as a function of cutoff distance. The 2021 *Results of the ISPRS benchmark on indoor modelling* paper describes accuracy in its equation as source-surface samples to the closest reference surface. These are opposing directed distances. Treat the website definition as the leaderboard's current displayed specification, but do not claim that this resolves the historical evaluator implementation. A canonical scorer source/code or written clarification from the benchmark maintainers is still required before P1 can choose one direction for exact historical reproduction. Until then, retain both directions as explicitly labelled diagnostics and do not relabel either one as the other.

Sources:
- Official results page: https://isprs.org/resources/datasets/benchmarks/IndoorModeling/results.aspx
- 2021 final-results paper: https://www.sciencedirect.com/science/article/pii/S2667393221000089

## 2026 research frontier and the real competitive targets

### Historical ISPRS Indoor Modelling leaderboard

The current official results page presents separate leaders for each metric; no single submitted method dominates all three. At the published 10 cm buffer/cutoff:

| Dataset | Best completeness | Best correctness | Lowest median accuracy |
|---|---:|---:|---:|
| TUB1 | 0.93 (Ochmann et al.) | 0.80 (Lim et al.) | 1.34 cm (Tran et al.) |
| TUB2 | 0.84 (Tran et al.) | 0.92 (Ahmed et al.) | 2.12 cm (Tran et al.) |
| Fire Brigade | 0.94 (Tran et al.) | 0.50 (Lim et al.) | 1.41 cm (Tran et al.) |

These are historical published results, not reproduced Luminous results. The Pareto implication matters: completeness, correctness and accuracy are distinct objectives. Optimizing just one can hide over-modeling, omitted surface, or a poorer fit. TUB1 and TUB2 are two captures of the same physical building; they must remain in the same building-level holdout.

### LiBiDT — causal temporal LiDAR–BIM state (Scientific Reports, 15 June 2026)

Chen et al., *Multi-line LiDAR 3D construction environment modeling and BIM consistency update method for digital twins*, Scientific Reports 16 (2026), article 27367, https://doi.org/10.1038/s41598-026-58063-3.

The method composes spatio-temporal global registration, multi-task perception, scan-to-BIM bipartite association/state updating and downstream decision/risk analytics. On CV4AEC 2024 Scan-to-BIM it reports:
- association F1 = 0.837;
- Chamfer Distance = 3.2 cm;
- surface RMSE = 3.5 cm;
- strongest competing CoFiNet (Reg) + S2B baseline association F1 = 0.822;
- temporal IDF1 improvement from 0.854 to 0.861 in the reported comparison.

The authors explicitly leave strict temporal validation under active construction for future work. Luminous should therefore compare the static association/geometry metrics on the same held-out task, then target temporal identity continuity and evidence stability prospectively. We should not call the temporal result externally validated construction-site performance.

### Multi-agent Scan-to-BIM / Scan-to-Graph (Automation in Construction, August 2026)

Pan et al., article 107003, DOI: https://doi.org/10.1016/j.autcon.2026.107003. This work combines query interpretation, point-cloud inference, topology-aware IFC generation and semantic graph generation with intent-adaptive detail. The TUMCMS room-area mean relative deviations reported in the study are 4.70% and 5.10% for two model configurations. Compare room area, boundaries, graph adjacency/connectivity, IFC validity, user-requested level-of-detail control and human correction time separately. Access to study data is a reproduction dependency; do not assume a public corpus.

### Structural reconstruction from incomplete scans (Automation in Construction, 2026)

Trinh, Hou & Wu, article 107161, DOI: https://doi.org/10.1016/j.autcon.2026.107161. Its structural component workflow reports over 97% area accuracy, 98% perimeter accuracy and 92% moment-of-inertia accuracy, with LOA30 reported and some elements approaching LOA40. Its result is a task-specific structural-element baseline. A fair comparison needs the same component families, tolerances, independent metrology and timing definition.

### Occlusion completion without target-bound leakage (Automation in Construction, September 2026)

Jayasinghe & Brilakis, article 107052, DOI: https://doi.org/10.1016/j.autcon.2026.107052. The work demonstrates that complete-object bounds leaking into partial-input normalization make completion benchmarks easier than real-world use. The replay contract rejects reference-derived full-object bounds in candidate-facing normalization. Reproduce the authors' corrected benchmark before comparing EMD/CD; report boundary uncertainty and downstream meshing/IFC outcomes too.

### BIMNet — geometric and topological reconstruction benchmark

Liu et al., Automation in Construction 173 (2025), article 106096, DOI: https://doi.org/10.1016/j.autcon.2025.106096. BIMNet is an IFC-based large-scale real-world point-cloud-to-BIM dataset with geometric and topological metrics. The associated website says data access depends on Matterport3D approval and a separate request; the whole package is not automatically redistributable. Keep this disabled until access and per-subset rights are recorded: https://thucbims.github.io/bimnet/

## Industry comparison: capabilities, not vendor-result claims

Leica Cyclone 3DR documentation currently describes:
- high-accuracy point-cloud/BIM deviation analysis and 3D heatmaps;
- progress monitoring with per-asset reports;
- clash detection;
- visual reporting;
- tolerance checking with within/outside/no-data statuses;
- IFC/RVT input in the AEC edition and BCF 2.1 issue export.

Its BIM analysis guide gives suggested accuracy bands by task and recommends different scanner classes. These are vendor workflow recommendations, not independently measured benchmark results. Reference: https://rcdocs.leica-geosystems.com/v1/docs/cyclone-3dr-cyclone-3dr-which-bim-analysis-feature-to-use

Autodesk ReCap documentation describes scan import, management, registration, manipulation and integration with Revit and other Autodesk products, including construction verification workflows. This is a strong incumbent data-preparation/ecosystem baseline; it is not evidence of an automatic semantic/topological reconstruction score. Reference: https://help.autodesk.com/cloudhelp/ENU/Reality-Capture/files/recap_product_overview.html

A neutral P3 comparison must freeze the product build, license/edition, hardware, operator expertise, scan source, coordinate frame, tolerance and output acceptance. Report acquisition, registration, correction, review and handoff times separately. Feature pages alone do not establish a quality or productivity lead.

## Proposed benchmark ladder and targets

Targets below are aspirations for matched evaluations, not achieved results.

| Gate | Benchmark | Target |
|---|---|---|
| P0 deterministic contract | Fixed synthetic manifest, local artifact bytes, adversarial schema/path controls | 100% deterministic replay; 0 accepted stale/mismatched artifact digests; all frame/unit/tolerance/metric-profile errors fail closed |
| P1 historical ISPRS | Official six-scene wall-only protocol, full buffer/cutoff curves and 10 cm table | Reproduce all accessible published table values to displayed rounding precision, or retain explicit unresolved discrepancies |
| P2 element confidence | Ersöz–Bosché Coverage, Distribution and Distance per element | Exact reproduction at pinned settings before any extension; then improve calibration and stability without changing the reproduced metric |
| P2 structural geometry | Trinh et al. area/perimeter/section moments and independent LOA profile | Non-inferiority on matched element classes before optimizing cost/review time |
| P2 Scan-to-Graph | Pan et al. room area/boundary and graph/IFC outputs | Room-area error no worse than the best reported 4.70% under matched data; improve graph/topology and correction effort |
| P2 temporal state | LiBiDT CV4AEC F1/CD/RMSE/IDF1/IDSW | At least parity with F1 0.837, CD 3.2 cm and RMSE 3.5 cm on the matched task; then improve IDF1/IDSW and uncertainty calibration |
| P3 operational acceptance | Same building, same acquisition, same human and commercial workflow | Reduce false mutations and rework at non-inferior geometry/topology; demonstrate lower total acquisition + compute + human-review cost on matched prospective cases |

Never compress the ladder into one weighted score. Do not use a better pooled point distance to excuse an unobserved surface being called reconstructed or a wrong persistent element identity.

## Further paradigm shifts worth pursuing

1. **Static reconstruction → causal state evolution.** Carry element identity, prior state, observation evidence, association ambiguity and revision lineage forward across acquisitions. The evaluator should detect future-data leakage and ID switches; no look-ahead in the temporal state update.
2. **Coverage score → observability state.** Distinguish visible-and-captured, visible-but-missed, self-occluded, externally occluded, outside coverage and unknown. The current P0 accounting does this for supplied strata; future work must derive them from sensor pose, ray geometry and independently validated surfaces.
3. **Confidence scalar → calibrated selective risk.** Report confidence by element and visibility stratum, calibration and selective-risk-versus-abstention curves. Unknown and unobserved must remain explicit.
4. **BIM object creation → evidence-preserving representation change.** Geometry, semantics, topology, CRS/units, uncertainty, and source correspondence should survive IFC export/import or be reported as explicit losses. Reuse the existing BUILT-INTEROP validators rather than creating a duplicate IFC stack.
5. **One-shot scan → decision-sensitive sensing.** Reuse #1892: request another scan only when a finite, declared outcome can change an acceptance/repair decision. A denser cloud that does not change the decision should not automatically win.
6. **Benchmark score → lifecycle outcome.** Reuse #3331 and #4043 to measure accepted changes, false mutations, repair, re-verification and human correction effort.

## Claim ceiling

The v0.3 execution receipt binds a declared run to local runtime/input/output file digests, per-scene frames, tolerance/sampling parameters and metric definitions. It does not execute or sandbox the candidate, compute geometry, validate CRS transformations, run the official ISPRS scorer, or establish field productivity. The synthetic IFC-placeholder files are byte fixtures, not valid IFC deliverables. P0.3 remains a contract and receipt foundation until exact-head validation and a later independent geometric scorer pass.
