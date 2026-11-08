# Scan-to-BIM surface sampling receipt v0.1

**Status:** deterministic area-uniform sampling primitive and receipt; no historical ISPRS sample-generator reproduction is claimed.  
**Issue:** #4222 — SCAN-BIM-SURFACE-SAMPLING-001.  
**Stack:** after #4205 directed accuracy core and #4188 experiment receipt.  
**Research date:** 2026-11-08.

## Why sampling is part of the metric

A distance metric is not reproducible from its name, direction and cutoff alone. The sample population and its construction affect the measured distance distribution. In particular, sampling every mesh triangle equally weights triangulation density rather than physical surface area. A denser tessellation can then change the result even when the represented physical surface is unchanged.

Sampling parameters therefore belong in the replay receipt alongside correspondence policy and cutoff. The sampler emits stable sample identities, barycentric coordinates, area weights, visibility accounting and a digest of the exact ordered sample set.

## Implemented algorithm: uniform_area_halton_v1

Input is a set of triangles carrying:
- stable surface ID;
- stable triangle ID, unique within the surface;
- three finite 3D vertices in metres and one already-aligned frame;
- an explicit visibility class.

Triangles and surfaces are sorted by their stable IDs before sampling. This makes the output invariant to permutation of the input array.

For each surface:
1. Validate every triangle, including excluded/hidden triangles, for finite coordinates and non-degenerate area.
2. Sum the area of only triangles marked Observable.
3. Record excluded area separately for self-occluded, externally occluded, outside-sensor-coverage, unknown and contradictory classifications.
4. Compute the density-equivalent requested count as ceil(observable_area / target_spacing²), with at least one sample for positive observable area.
5. Fail closed if the total count exceeds the declared maximum; never silently truncate or change the density.
6. Choose triangles in proportion to triangle area using a deterministic Halton coordinate.
7. Generate an area-uniform point inside the selected triangle using the square-root barycentric transform.
8. Assign every point the surface ID, per-surface ordinal, selected triangle ID, barycentric coordinates and equal represented-area weight for that surface.

The sequence uses Halton bases 2, 3 and 5 with a stable FNV-derived per-surface seed rotation. The sampler and seed are versioned by the algorithm ID and recorded in the manifest.

## Receipt semantics

The sampling receipt reports:
- sampler algorithm ID;
- target spacing in metres;
- seed and hard sample budget;
- total observable and excluded area;
- per-surface observable/excluded areas and per-reason exclusions;
- sample count per surface;
- each sample's stable identity, coordinates, barycentric weights and represented area;
- SHA-256 of the canonical receipt content.

An all-unobservable input is valid observability accounting and emits zero samples. It is not a zero-distance or perfect-accuracy result. The downstream accuracy evaluator must reject an empty sample population.

The per-sample represented area is observable_surface_area / sample_count_for_that_surface. Its sum across a surface equals that surface's observable area up to floating-point rounding. This preserves the area represented by samples when aggregating across surfaces with different areas.

## Manifest binding

The v0.3 manifest adds:
- `surface_sampling_algorithm = uniform_area_halton_v1`;
- `surface_sampling_seed`;
- `surface_sampling_max_samples`.

The already-declared `reference_surface_spacing_m` is the density-equivalent spacing for this profile. It is not a minimum point separation. The contract bounds the configured maximum to 10,000,000 samples and rejects unsupported sampler IDs and zero/excessive budgets.

The sampler output itself carries a `sample_set_sha256`; an evaluator that materializes samples must preserve that digest with the metric result. The manifest records the requested algorithm and parameters before the evaluator sees the ground truth.

## Relationship to ISPRS

The current ISPRS results page specifies wall-only completeness, correctness and accuracy. It says accuracy uses reference-surface sample points to their closest source-model surface and reports a curve over distance cutoffs. The results page does not specify every implementation detail required to reproduce its historic sample population, tessellation or exact sample generator.

The 2021 final-results paper describes source-surface sampling for its methodology, while the official results page uses reference-surface samples. The 2018 evaluation framework is a separately named source-vertex-to-reference-plane method. These populations and correspondence rules must remain distinct.

This Luminous sampler can produce a controlled reference-surface or source-surface sample population for new experiments. It must **not** be retroactively labelled the historical ISPRS sampler unless the canonical implementation/source and published tables establish that equivalence. Historical scoring remains blocked on source-faithful sampling and full evaluator reproduction.

Primary sources:
- Official ISPRS results and metric definitions: https://isprs.org/resources/datasets/benchmarks/IndoorModeling/results.aspx
- 2018 ISPRS evaluation framework: https://doi.org/10.5194/isprs-archives-XLII-4-297-2018
- 2021 ISPRS final results: https://doi.org/10.1016/j.ophoto.2021.100008

## Adversarial and analytic tests

The Rust test suite covers:
- known right-triangle and square areas;
- per-surface sample counts proportional to area;
- area weights conserving each surface's observable area;
- barycentric weights summing to one and points lying inside the source triangle;
- invariance to triangle input ordering;
- seed-dependent point locations with stable sample identities/counts;
- excluded visibility area accounting and no hidden-surface samples;
- empty samples under fully unknown visibility;
- duplicate surface/triangle identity;
- degenerate and non-finite geometry even when excluded;
- invalid spacing, zero budget and budget overflow without truncation.

## Limits and next gates

This is a deterministic sampler, not a historical benchmark scorer or a uniform-minimum-distance point process.

- A target spacing controls sample density, not guaranteed nearest-neighbour spacing. There is no Poisson-disc or blue-noise separation guarantee.
- The exact output depends on the frozen triangle tessellation and IDs. A retessellated mesh is a different sampling input even if its continuous surface is geometrically equivalent.
- Visibility is caller-supplied; the sampler does not estimate occlusion or theoretical visibility.
- Samples are not independent buildings or independent statistical observations.
- The implementation does not parse IFC/E57/PLY, create meshes, register frames, or propagate metrology uncertainty.
- A source-faithful historical ISPRS scorer still needs the original surface construction, tessellation, sample generator, buffer/intersection rules, cutoff aggregation and differential reproduction against published values.

Next:
1. Feed the receipt's samples and explicit sample population into the directed-accuracy core without copying or hiding sample semantics.
2. Add independent area/coverage/correctness primitives and analytic oracles for buffered surface intersections.
3. Differentially compare historical sampling and scoring against canonical implementation or maintainer-confirmed outputs.
4. Add sensitivity studies over spacing, tessellation, seed and visibility mask; report the resulting metric variance instead of selecting a seed that improves a score.

## Claim ceiling

The implementation establishes deterministic, area-uniform sampling with a byte-bound receipt on synthetic triangle meshes. It does not establish SOTA, official ISPRS reproduction, calibrated measurement uncertainty, a real-building accuracy level or commercial superiority.
