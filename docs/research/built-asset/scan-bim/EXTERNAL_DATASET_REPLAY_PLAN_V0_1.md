# External Scan-to-BIM Dataset Replay Plan v0.1

**Status:** adapter and evaluation plan; no dataset has yet been downloaded, executed or scored by Luminous.  
**Owner:** Symtropy #4086 — SCAN-BIM-ADMISSION-001  
**Composes:** `SCAN_BIM_ADMISSION_V0_1.md`, #1607 survey evidence and #2358 benchmark catalogue.

## 1. Strategic decision

Use the public ISPRS Indoor Modelling benchmark as the first external replay family, and BIMNet as a second AEC-specific dataset after confirming data access.

Do not train or claim superiority from the 23-case synthetic corpus. That corpus tests a small admission contract only. External replay must evaluate geometry, visibility, semantic/topological fidelity and abstention on real captures.

**ISPRS is a research-only dataset, not a redistributable asset.** Its test submission portal is closed; however the six scans and corresponding BIM models remain available for researchers to evaluate methods independently. Dataset conditions prohibit non-research use and redistribution to third parties. A publication must acknowledge ISPRS WG IV/5 and cite the benchmark paper. Luminous must obtain each dataset through the ISPRS route and must not commit the source files or derived files that are effectively redistributable copies into public repositories.

Official references:
- Dataset and scene descriptions: https://www.isprs.org/resources/datasets/benchmarks/IndoorModeling/dataset.aspx
- Original benchmark and use conditions: https://www.isprs.org/resources/datasets/benchmarks/IndoorModeling/Default.aspx
- Published results and metric definitions: https://www.isprs.org/resources/datasets/benchmarks/IndoorModeling/results.aspx
- Khoshelham et al. (2017), *The ISPRS Benchmark on Indoor Modelling*, DOI: https://doi.org/10.5194/isprs-archives-XLII-2-W7-367-2017

## 2. ISPRS scene matrix

The benchmark contains six indoor point clouds collected with different sensors and with materially different scene complexity.

| Dataset | Acquisition / scene facts | Benchmark stress and required handling |
|---|---|---|
| TUB1 | Viametris iMS3D mobile system; 10 rooms, 23 doors, 7 windows; trajectory and timestamps supplied; low clutter. | Mobile trajectory and timestamps; open/closed doors; wall-thickness variation; distinguish structural points from people in the scan. |
| TUB2 | Zeb-Revo scan of the **same building as TUB1**; two floors connected by stairs; 24 rooms, 21 windows, 51 doors; trajectory and timestamps; low clutter. | Multi-floor connectivity, stairs, varying wall thickness/ceiling height and a different sensor. TUB1/TUB2 must share one building-level split group; never train on TUB1 and claim unseen-building generalization on TUB2. |
| Fire Brigade | Static Leica C10 terrestrial laser scan; 9 rooms, 10 doors, 53 windows; high furniture clutter; occlusion gaps; curtain walls. | Static-scan occlusion, furniture clutter, curtain-wall and window confusion. Unseen surface must remain unobserved/unknown, not a reconstructed fact. |
| UVigo | Prototype backpack mobile mapping; one room plus entrance hall; 20 windows, 7 simple doors (including elevator doors); stairs, circular columns, curtain wall, varying ceiling heights; moderate clutter; trajectory/time available. | Mobile mapping, curved columns, stairs, elevator openings, curtain walls and varying ceiling. |
| UoM | Zeb-1; 7 rooms, 14 doors; moderate furniture clutter; windows covered by blinds and therefore not visible; stairs down to lower level. | Explicit negative-observation control: windows behind blinds are **not observed**. Do not score hallucinated or assumed window geometry as observed scan evidence, and do not count absent visibility as a detector miss unless the benchmark's declared task asks the candidate to infer hidden geometry. |
| Grainger Museum (GM) | Zeb Revo RT, indoor and outdoor acquisition; 18 rooms, 13 doors, 41 windows; highly cluttered; non-Manhattan geometry. | Non-Manhattan topology, non-orthogonal walls, artefacts/clutter and mixed interior/exterior capture. Strong generalization test against Manhattan-room priors. |

Source for all scene facts: official ISPRS dataset descriptions. Verify actual downloaded package contents, revision and available BIM references before building the adapter; do not assume every expected semantic label or hidden surface is present in every scene.

## 3. Original benchmark metrics and historical reference

The official ISPRS results define three metrics **for wall elements only**:

- **Completeness** at buffer (b): area of intersection between source and reference surfaces divided by total reference surface area.
- **Correctness** at buffer (b): area of intersection between source and reference surfaces divided by total source surface area.
- **Accuracy** at cut-off (r): median absolute distance from sampled reference-surface points to the closest surface in the source model. Lower is better.

The official tables report (b = 10\,\mathrm{cm}) and (r = 10\,\mathrm{cm}), alongside curves across thresholds. Those definitions and thresholds must be reproduced exactly before any comparison with the historical table.

| Scene | Historical completeness range | Historical correctness range | Historical wall accuracy range |
|---|---:|---:|---:|
| TUB1 | 0.57–0.93 | 0.46–0.80 | 1.34–4.92 cm |
| TUB2 | 0.53–0.84 | 0.48–0.92 | 2.12–5.99 cm |
| Fire Brigade | 0.44–0.94 | 0.14–0.50 | 1.41–4.84 cm |
| UVigo | 0.44–0.87 | 0.18–0.64 | 0.51–6.66 cm |
| UoM | 0.60–0.87 | 0.41–0.77 | 0.92–4.36 cm |
| GM | 0.29–0.65 | 0.49–0.71 | 3.70–4.03 cm |

**These ranges are historical entries shown on the ISPRS results page, not a current 2026 SOTA claim.** They span different submissions and methods; they are not one method's end-to-end result, nor proof that a present-day commercial workflow achieves or fails to achieve those values. The ISPRS test is closed for new official submissions, so any Luminous run is an independent reproduction using the published protocol.

## 4. Luminous extension: separate from official ISPRS scoring

The original ISPRS score is wall-focused. It must remain untouched for reproducibility. Add a second, clearly labelled Luminous analysis with the following planes:

1. **Openings and component elements:** door/window detection and geometry, open/closed door pose, single/double leaves, elevator doors, curtain walls and component identity. Report class-wise precision/recall, instance association, dimensions and boundary error.
2. **Topology:** room connectivity, wall/opening incidence, stair/floor connectivity, containment and adjacency graph edge precision/recall, orphan elements and disconnected components.
3. **Visibility and observability:** visible-surface coverage; explicitly unobserved region masks; sensor pose/trajectory consistency where supplied; occlusion-conditioned performance.
4. **Uncertainty:** calibrated geometry intervals, interval coverage/width, risk-versus-coverage curve, and correct abstention when a tolerance or element identity is not decidable.
5. **Change/admission:** false model mutation and missed-change rates only where paired revisions or independently grounded changes exist. Do not synthesize change truth from a single static scan and then claim physical-change detection performance.
6. **Workflow:** human correction time, accepted model elements per total crew-hour, added measurement burden and rework. These require a matched human/commercial baseline and must not be inferred from point-cloud metrics.

A candidate must not receive credit for completing a hidden object from a reference bounding box or complete-cloud normalization. The 2026 occlusion-completion work demonstrates that leaked reference bounds can inflate completion results. Candidate inputs must use only the partial observation, sensor pose/trajectory, calibration and genuinely available upstream metadata. Ground-truth geometry and bounding boxes remain evaluator-only.

## 5. Required benchmark implementation protocol

### Dataset admission and digests

Before running any model, record:
- ISPRS dataset name and official source URL;
- access date, terms accepted and authorized research purpose;
- package manifest, file names, byte lengths and SHA-256 digests (do not publish restricted source bytes);
- exact scan/BIM pairing and coordinate-frame metadata;
- source sensor/acquisition metadata and any trajectory/timebase;
- missing/corrupt files, repairs or conversions, with immutable original and derived digests;
- software/tool version and complete conversion/registration configuration.

Do not change coordinate frame, unit, scale, axis order or registration silently. The #1607 survey contract remains authoritative for frame/registration qualification. Separate original scan, derived scan, candidate reconstructed model and reference BIM.

### Reproduction order

1. Reproduce the original wall-only ISPRS metrics on one low-clutter case (TUB1) first.
2. Run TUB2 as a cross-sensor, same-building repeat, but do not use it as an unseen-building test after tuning on TUB1.
3. Run Fire Brigade and UVigo for clutter, static/mobile acquisition and curtain-wall failure modes.
4. Run UoM as a required hidden-window / non-observation negative control.
5. Run Grainger Museum as the non-Manhattan, high-clutter generalization case.
6. Only after source protocol reproduction, run the Luminous opening/topology/uncertainty/admission extension.

Keep official metric outputs separate from Luminous extension outputs. Publish configuration and result manifests, not restricted data files.

### Split and leakage policy

- Group by **building identity**, scan session, source lineage and augmentation parent—not random point rows or frames.
- TUB1 and TUB2 are two acquisitions of the same building and must not be split across train/evaluation for an unseen-building claim.
- Hold out whole building groups. If only six scenes exist, report the small and heterogeneous number of independent building groups; do not imply that six scans give a statistically broad population estimate.
- For any point-cloud completion experiment, derive input normalization from the partial scan and permitted sensor metadata only. No complete-cloud/reference bounding boxes, reference centroids, hidden extents, evaluator meshes or target-derived preprocessing.
- Keep evaluator truth and labels outside candidate code and CI. Use an independent data custodian for blind held-out evaluation; a JSON allowlist alone is not a security boundary.
- Freeze seeds, capture/session groups, preprocessing, model checkpoints and all tuning sets before final evaluation.

### Baselines

Compare, where reproducible under the same input and the official metric protocol:
- exact published ISPRS submission implementations and their available configuration;
- established geometric/parametric indoor modelling methods;
- current commercial scan workflows only where product versions, input conversions and output model scope can be pinned and the output can be evaluated by the same independent reference method;
- Luminous reconstruction and admission layers.

Do not use a commercial heatmap screenshot or feature list as a quantitative baseline. Where product export, data access or reproducibility blocks a matched comparison, mark that comparator UNASSESSED / NOT_COMPARABLE with reason.

## 6. Second dataset family: BIMNet

Liu et al. (Automation in Construction 173, May 2025, article 106096, DOI: https://doi.org/10.1016/j.autcon.2025.106096) present 25 indoor scans, 116,507,604 points, 382 rooms and 8,710 m² of floor area with IFC-oriented annotations / corresponding BIM models and geometric/topological benchmark metrics.

BIMNet is the higher-value AEC-specific follow-on once:
- the actual source dataset package is available under terms compatible with the intended research;
- the BIM/IFC annotations and ground-truth scope are inspected;
- exact split/group identity and benchmark preprocessing are reproduced;
- a clean baseline implementation is available.

Do not assume that publisher article access means unrestricted dataset access or redistribution. Verify source-data licensing and obtain it through the dataset custodian. Do not put third-party BIMNet files in public Git history without explicit permission.

## 7. Proposed P1 acceptance criteria

**Reproduction gate**
- Exact reproduction of original ISPRS wall-only metric definitions and (b/r=10\,\mathrm{cm}) outputs for every included case.
- All scene exclusions, preprocessing, registration and input revisions declared.
- Historical comparator results quoted as historical, not current SOTA.
- Independent checkpoint/ground-truth usage is documented; fitting and scoring data are separated.

**Generalization gate**
- No train/evaluation overlap by building, scene, capture lineage or augmentation parent.
- TUB1/TUB2 grouped together for building-level split.
- Performance reported per scene and per metric; no pooled average hiding a weak scene.
- No bounding-box leakage or hidden reference geometry in completion model inputs.
- Explicit unknown/abstention output for UoM's blind-hidden windows and other unobserved regions.

**Luminous extension gate**
- Original ISPRS wall metrics reproduced without modification.
- Opening, topology, uncertainty, visibility and admission metrics reported in separate tables.
- Zero false assertions about unobserved geometry in the held-out audit cases.
- Candidate model deltas are revision-bound and rejected on stale base, unsupported identity or topology conflict.
- No superiority claim until matched results are produced against available baselines.

## 8. Non-goals and claim ceiling

This is a research evaluation plan only. No dataset has been downloaded, no result reproduced, no external tool run, and no SOTA claim established by this file. ISPRS submission is closed and the dataset is research-only/non-redistributable. BIMNet access/licence is still to be confirmed. Passing the original wall metric does not validate doors/windows, topology, hidden geometry, building safety, code compliance or a physical robot workflow.
