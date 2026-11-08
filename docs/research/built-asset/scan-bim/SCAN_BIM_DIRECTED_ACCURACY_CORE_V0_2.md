# Scan-to-BIM directed accuracy core v0.2

**Status:** P1.0 deterministic geometry primitive with two explicit correspondence policies; no official historical score reproduction is claimed.  
**Issue:** #4116 — SCAN-BIM-REPLAY-001.  
**Implementation:** `tools/scan-bim-replay-contract/src/metrics.rs`.  
**Stack:** after #4188 (candidate/runtime/frame/geometry/graph receipts).  
**Research date:** 2026-11-08.

## Why correspondence semantics must be typed

A point-to-surface distance is not a complete metric definition. The benchmark must bind at least:

1. direction (reference → candidate or candidate → reference);
2. source-sample population (reference surface samples, source vertices, or source surface samples);
3. target surface set and visibility mask;
4. correspondence rule (closest point on the bounded surface, or orthogonal projection to a plane under a declared boundary rule);
5. distance cutoff and median/aggregation policy.

The current official ISPRS results page defines accuracy as the median absolute distance between **sample points on reference surfaces** and the **closest surface in the submitted source model**, as a function of cutoff distance. This is naturally represented as reference-to-candidate with Euclidean closest point on the closed submitted surface. A closest point may lie on an edge or vertex; it is not valid to reject every sample whose orthogonal projection falls outside the interior of a triangle.

The 2018 evaluation-framework profile is a different method: source-model vertices are projected orthogonally to observable reference surface planes, and a projection is eligible only when it lies inside the target surface boundary. That is not interchangeable with Euclidean distance to the nearest point of a bounded triangle.

The 2021 final-results paper describes source-surface sampling in its methodology. The exact correspondence implementation and sample-generation details are not yet independently reproduced here. Its manifest profile therefore remains explicitly marked `unresolved_until_reference_method_reproduction`; it is not run through either of the two implemented methods and is not reported as a qualified score.

Primary sources:
- Official ISPRS results page: https://isprs.org/resources/datasets/benchmarks/IndoorModeling/results.aspx
- 2018 evaluation framework: https://doi.org/10.5194/isprs-archives-XLII-4-297-2018
- 2021 final results paper: https://doi.org/10.1016/j.ophoto.2021.100008

## Implemented correspondence policies

### A. `euclidean_closest_point_on_triangle`

For each sample and each observable target triangle, compute the true Euclidean closest point on the closed triangle (face interior, edge or vertex) using region tests. The sample distance is the Euclidean norm to that point.

Use for the current official ISPRS results-page profile:
- direction: `reference_to_candidate`;
- sample population: `reference_surface_samples`;
- target surfaces: submitted candidate model triangles;
- policy: `euclidean_closest_point_on_triangle`.

### B. `orthogonal_projection_inside_triangle`

For each sample and observable target triangle:
1. compute the triangle's unit normal;
2. orthogonally project the sample to the infinite plane;
3. compute barycentric coordinates of the projection;
4. accept the correspondence only when the projection lies inside the triangle boundary (within a small declared numerical epsilon);
5. use absolute perpendicular distance to the target plane.

Use for the 2018 evaluation-framework profile:
- direction: `candidate_to_reference`;
- sample population: `source_surface_vertices`;
- target surfaces: observable reference triangles;
- policy: `orthogonal_projection_inside_triangle`.

An out-of-bound plane projection is reported as no correspondence. It is not replaced with a nearest-edge distance under this profile.

### C. Explicitly unresolved profile

The 2021 named profile uses:
- direction: `candidate_to_reference`;
- sample population: `source_surface_samples`;
- policy: `unresolved_until_reference_method_reproduction`.

This value is permitted as a replay declaration so the protocol can preserve the published source population without pretending the core can score it. The numeric `metrics.rs` API intentionally has no unresolved implementation variant. A caller must refuse to run a score for this declared policy until a source-faithful adapter is added and independently checked.

## Rust API contract

The `evaluate_directed_accuracy_curve` function requires explicit:
- `AccuracyDirection`;
- `AccuracySamplePopulation`;
- `CorrespondencePolicy`;
- sample list in metres;
- target triangles with an explicit observable flag;
- strictly increasing positive cutoff values in metres.

The returned result preserves direction, population and correspondence policy alongside:
- per-sample distance or `None`;
- sample count;
- correspondence count;
- no-correspondence count;
- per-cutoff median;
- per-cutoff accepted count;
- valid correspondences rejected for exceeding each cutoff.

`None` remains distinct from an out-of-cutoff distance. Unobservable targets never establish correspondences. Every declared triangle is checked for finite coordinates and non-degeneracy even if it is marked unobservable.

The API returns deterministic results for an identical ordered input. If target surfaces tie, the first target in input order wins; callers must preserve and bind a canonical target order where byte-identical replay matters.

## Validation suite

The synthetic test corpus now exercises:
- Euclidean distance for an interior face projection;
- Euclidean closest point when orthogonal projection falls outside a triangle but the edge remains part of the surface;
- a point whose true closest point lies on an edge;
- the 2018 plane-projection rejection for a projection outside the target boundary;
- exclusion of unobservable target surfaces under both policies;
- cutoff-dependent medians and rejected-above-cutoff counts;
- odd/even sample median behavior;
- nearest eligible plane selection and deterministic tie behavior;
- direction/population mismatch rejection;
- invalid, non-positive and non-monotone cutoffs;
- degenerate target geometry and non-finite sample/triangle coordinates;
- finite-input arithmetic overflow;
- preservation of the chosen direction, sample population and policy.

These are hand-constructed numerical fixtures. They do not parse IFC/E57/PLY data, generate samples, reconstruct geometry, estimate visibility or reproduce the ISPRS tables.

## Manifest binding

The v0.3 replay manifest now carries `metric_profile.correspondence_policy`. For the named ISPRS wall suite, `definition_authority`, direction, sample population and correspondence policy must agree:

| Authority | Direction | Sample population | Correspondence policy |
|---|---|---|---|
| `official_results_page` | Reference → candidate | Reference surface samples | Euclidean closest point on triangle |
| `evaluation_framework2018` | Candidate → reference | Source surface vertices | Orthogonal projection inside triangle |
| `final_results_paper2021` | Candidate → reference | Source surface samples | Explicitly unresolved pending source reproduction |
| `bidirectional_diagnostic` | Both, separate distributions | Both-direction diagnostic | Bidirectional diagnostic |

Other/custom suites must retain `project_defined` authority. They cannot borrow a published benchmark authority label.

The manifest records the policy; it does not run the accuracy metric or establish that the chosen source samples, target mesh, observability mask or coordinate alignment are valid.

## Important omissions

This core does **not** yet:
- generate reference/source surface samples from IFC meshes or reference geometry;
- implement area-weighted sampling at a fixed spacing/seed;
- support polygons with holes or non-triangular boundary topology directly;
- generate or validate meshes from point clouds;
- estimate theoretical visibility from scanner pose and blocker geometry;
- propagate registration, calibration or surface uncertainty into distance uncertainty;
- calculate completeness/correctness area intersections or their denominators;
- implement official source/target surface buffering, tessellation, area approximation, median population, curve aggregation or a historical scorer;
- execute the #4188 candidate pipeline or read evaluator references under a process sandbox.

## Next steps and benchmark gates

**P1.1 — sample generators.** Pin mesh tessellation, sample spacing, area weighting, random seed, duplicate handling and sample IDs. Keep the reference-surface and source-surface populations separate.

**P1.2 — exact polygonal methods.** Add polygon-with-holes target surfaces and independently tested point-to-polygon correspondences. Reproduce the precise 2018 boundary rule; only resolve 2021 after source-level method reproduction.

**P1.3 — completeness/correctness.** Implement buffered source/reference surface intersection areas with exact observable denominators and cutoff curves. Start with analytic rectangles and non-Manhattan planes, then independent fixtures.

**P1.4 — differential scorer.** Compare each named method to the canonical implementation or benchmark-maintainer-confirmed outputs. Reproduce published 10 cm table values to displayed precision; retain unexplained discrepancies rather than tuning thresholds.

**P2 — matched external replay.** Only after rights/access are confirmed, evaluate the six ISPRS scenes, keeping TUB1/TUB2 in the same building-level split. Publish per-scene curves and counts, not only pooled medians.

## Claim ceiling

This is a deterministic directed-distance primitive with explicit correspondence semantics. It is **not** an official ISPRS scorer, a complete Scan-to-BIM pipeline, a reconstruction algorithm, a calibrated uncertainty model, or evidence that Symtropy beats SOTA or commercial tools.
