# SCAN-BIM-VISIBILITY-001 — evaluator-only surface observability

Status: **P0 implementation in progress; synthetic semantics only.**  
Parent: [Symtropy #4292](https://github.com/Luminous-Dynamics/symtropy/issues/4292)  
Stack: [surface sampling PR #4225](https://github.com/Luminous-Dynamics/symtropy/pull/4225) → visibility evaluator branch.

## Purpose

Compute an evaluator-side estimate of which sampled reference-BIM surface patches are visible from declared sensor views, given a declared triangle occluder inventory. Produce a deterministic receipt with per-probe and per-surface dispositions and area totals. The output is consumed by the confidence evaluator, not by the reconstruction candidate.

This is not a replacement BIM authoring application, general-purpose ray tracer, survey registration system, or proof that the physical building was completely scanned.

## Architectural boundary

- Candidate/reconstruction package: `scan-bim-replay-contract` remains the metadata and replay-contract auditor.
- Evaluator package: `scan-bim-visibility-evaluator` is a separate Cargo workspace member and executable. It consumes an evaluator request over stdin and writes a receipt to stdout.
- Reference geometry and its deterministic area-weighted sampling receipt are evaluator-side inputs. The request rejects unknown JSON fields and verifies the sampling receipt's canonical digest and numeric accounting before converting samples into visibility probes.
- The evaluator checks that each probe's barycentric coordinates reconstruct its position on the exact declared reference triangle, that the normal follows that triangle's winding, and that represented weights conserve the declared target surface area.
- The occluder manifest digest is recomputed from sorted stable triangle IDs, coordinates and provenance classes. A mismatch is rejected.
- The visibility receipt binds reference-model digest, sampling digest, occluder manifest, sensor-view evidence, visibility profile, per-view blockers and the computed result.

**Package separation is not an OS security boundary.** The evaluator is a separate binary/package, but production evaluation still requires process isolation, resource limits, read-only evaluator inputs, restricted IPC and candidate/evaluator artifact-root separation from #4116. Do not call this sandboxed merely because Cargo has two packages.

## P0 dispositions

| Disposition | Meaning |
|---|---|
| `observable` | At least one qualified view has an unobstructed line of sight with sufficient incidence |
| `grazing` | A view has line of sight but incidence falls below the declared threshold |
| `self_occluded` | Nearest blocker belongs to the reference building |
| `externally_occluded` | Nearest blocker is a declared external obstacle |
| `mixed_occlusion` | Different views are blocked by different known blocker classes |
| `outside_sensor_coverage` | Outside declared range/FOV for all evaluated views |
| `unknown` | Occluder coverage is incomplete or blocker provenance is unknown |
| `contradictory` | Equal-distance blockers in one view have conflicting provenance |

A mixed cause across different views is not the same as contradictory nearest blockers within one view. An unobstructed qualified view makes the probe observable even if another view is blocked. Unknown evidence prevents an affirmative visibility claim unless another qualified view independently establishes line of sight.

### Normal orientation correction

A triangle's winding does not prove that its normal points outward. The evaluator therefore uses the **absolute** normal/view dot product for incidence and labels low-incidence observations `grazing`; it does not infer a front/back semantic from arbitrary mesh winding. Normals are nevertheless checked against the source triangle winding so that probe geometry cannot be silently changed.

## Current implementation

The Rust evaluator provides:
- Deterministic FOV/range and right-handed sensor-basis validation.
- Two-sided ray/triangle intersection with declared distance tolerance.
- Self, external and unknown blocker provenance.
- Multiple-view aggregation and distinct mixed/contradictory states.
- Incomplete-occluder abstention.
- Hard ray-test budget; the traversal stops and returns an error rather than truncating.
- Per-surface and total area accounting, including grazing and mixed-occlusion area.
- Canonically ordered probe/view/occluder inputs and SHA-256 receipt.
- A strict JSON stdin/stdout executable.
- Adversarial tests for FOV, frame mismatch, grazing incidence, reversed winding, blockers, unknown inventory, mixed causes, conflicting ties, malformed probes, ray budget, permutation stability, digest tampering and request/receipt round trip.

The visibility kernel currently performs a bounded brute-force probe × view × triangle traversal. It is intentionally a small correctness oracle, **not a throughput-competitive scene-scale renderer**. The next performance phase should add a deterministic BVH and differential-test it against this brute-force oracle before comparing throughput.

## Research and industry baseline (2026-11-10)

This work must be compared against, not marketed as a replacement for:

1. **Ersöz & Bosché, Automation in Construction 181 (2026), 106636** — coverage, distance and normal-distribution matching-confidence indices, with visibility/self-occlusion handling; the independent baseline reproduction is #4246 / PR #4247. The present oracle is an implementation component for reproducing the evaluator-side visible surface, not a novel replacement metric. DOI: https://doi.org/10.1016/j.autcon.2025.106636
2. **Bayesian task-specific NBV (Zhu, Sellán & Terenin, 2026)** — evaluates view acquisition in terms of downstream task performance, not generic point density. Compare acquisition planners on matched tasks and budgets: https://arxiv.org/abs/2605.05095
3. **GO-PRE (ICML / PMLR 306, 2026)** — selects views using predictive-rendering entropy over a target view manifold. Add as a required baseline for perception/acquisition experiments: https://proceedings.mlr.press/v306/song26k.html
4. **Uncertainty-aware digital twins (Malihi, 2026)** — lifecycle/LoD-aware residual envelopes and calibration. This confirms that uncertainty-aware built-environment twins are an active research direction, not a unique Luminous claim. DOI: https://doi.org/10.1016/j.dte.2026.100140
5. **Representation-centred Scan-to-BIM review (Wang et al., 16 October 2026)** — emphasizes continuity of measurement support, identity, topology, semantics, uncertainty and model state across representation transitions. DOI: https://doi.org/10.3390/buildings16204095
6. **Infrastructure lifecycle BIM systematic review (Pisciotta & Inzerillo, 30 October 2026)** — 97 primary studies; highlights semantic continuity, updating, governance and long-term validation gaps. DOI: https://doi.org/10.3390/app162110749

Commercial baselines:
- **Leica Cyclone 3DR 2026.1.6 (29 October 2026)** includes the established Scan-to-Steel and Scan-to-Pipe workflows; Scan-to-Steel supports point-cloud-driven profile extraction, review/alignment and export, while 2026.1.6 adds building/storey structure to Scan-to-Pipe IFC export. We must not claim point-cloud extraction or basic IFC export as new: https://rcdocs.leica-geosystems.com/docs/cyclone-3dr-2026-1-0-cyclone-3dr-release-notes and https://rcdocs.leica-geosystems.com/v1/docs/cyclone-3dr-release-notes-cyclone-3dr
- **Autodesk ReCap Pro 2026** supports classified mesh editing, IFC4 export and Revit/RCMR handoff. The differentiator must be independently verified identity, information preservation, evidence lineage and uncertainty across tools—not simply IFC export: https://help.autodesk.com/cloudhelp/ENU/Reality-Capture/files/Scan_to_Mesh_Options/Scan_to_Mesh_Local/scan_to_mesh_export_model.html
- **Trimble RealWorks 2026.10** includes registration refinement and point-cloud workflows. Pin versions, modules and data paths in comparisons: https://help.fieldsystems.trimble.com/realworks/2026.10.htm

Standards:
- IFC 4.3.2.0 / IFC4X3_ADD2 is the official published baseline; IFC 4.4 is still a working draft as of this snapshot. Use the exact schema/profile and do not silently convert versions: https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html and https://standards.buildingsmart.org/IFC/DEV/IFC4_4/HTML/
- IDS 1.0 is an official buildingSMART standard for alphanumeric IFC information requirements. IDS does not establish geometric accuracy: https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/
- BOPTEST is a useful downstream operational-control benchmark for digital-building use cases, but it is not a Scan-to-BIM geometry benchmark: https://www.energy.gov/cmei/buildings/boptest-building-operations-testing-framework

## Benchmarks and targets

These are proposed research targets, not achieved results.

| Dimension | P0 target | P1 / external target |
|---|---|---|
| Analytic visibility | 100% expected disposition on frozen plane/box fixtures | Differential agreement with pinned independent ray-query reference |
| Area accounting | Exact conservation within declared floating-point tolerance | Report per-element and per-view visible/excluded area |
| Determinism | 100% identical receipt bytes for identical canonical inputs | Cross-platform reproducibility on pinned toolchain |
| Abstention | Zero observable outcomes for incomplete inventory / invalid frame / tampered receipt | Measure false-visible rate on independently surveyed buildings |
| Identity | Reject duplicate probe, triangle, surface and view identities | Persistent surface identity across model revisions |
| Occlusion causes | 100% classification on frozen self/external/unknown/mixed controls | Real occluder inventory sensitivity and failure analysis |
| Throughput | Report actual ray-test count and memory | BVH vs brute-force at fixed geometry and equal query budgets |
| Confidence | No probability claim from visibility labels alone | Calibrate geometric confidence against independent metrology |
| Acquisition | Compare fixed, coverage-greedy, uncertainty-greedy, Bayesian task-NBV and GO-PRE | Compare task outcome per capture cost and review effort |

For external reconstruction, use whole-building splits; never split points from the same building across training and test. Keep historical ISPRS completeness/correctness/accuracy separate from element confidence, topology, uncertainty calibration, identity, mutation safety and operational cost.

## Claim ceiling

A passing synthetic oracle establishes only the finite semantics of this declared ray-query implementation and its fixtures. It does not establish:
- completeness or correctness of a real occluder inventory;
- calibration or accuracy of sensor poses or sensor range/FOV;
- physical survey accuracy, material condition or building-code compliance;
- true outward orientation of arbitrary source mesh normals;
- calibrated probability of element correctness;
- IFC parsing, IDS conformance, model editability or a lossless round trip;
- SOTA superiority, commercial superiority or safe construction/maintenance decisions.

Those require independently sourced datasets, external metrology, pinned third-party comparators and independent toolchains.
