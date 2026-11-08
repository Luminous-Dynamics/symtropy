# Scan-to-BIM directed accuracy core v0.1

**Status:** P1.0 deterministic geometry primitive; no official benchmark replay or full ISPRS score is claimed.  
**Issue:** #4116 — SCAN-BIM-REPLAY-001.  
**Implementation:** `tools/scan-bim-replay-contract/src/metrics.rs` in stacked PR after #4163.

## Why this is a separate primitive

The literature does not define one interchangeable point-to-surface accuracy calculation. It differs in direction, sample population and correspondence rule:

- 2018 evaluation framework: source-model vertices → closest observable reference surface; orthogonally project to the nearest reference plane, require projection inside the reference surface boundary, then apply distance cutoff and median.
- 2021 final-results paper: source-surface sample points → closest reference surface plane.
- Current official leaderboard page: reference-surface samples → closest submitted source-model surface.

The code therefore exposes the sample population and direction as separate typed inputs. It will not infer them from a metric label or average the two directions.

## Geometry contract

Inputs are expressed in metres in one already-aligned coordinate frame:

- a non-empty explicit sample list;
- a non-empty set of triangular target surfaces, each tagged observable/not observable;
- a strictly increasing list of positive distance cutoffs;
- a named direction and sample population.

For every sample:
1. Iterate only over observable target triangles (all triangle geometry is still checked for finiteness and degeneracy).
2. Compute the target triangle plane's unit normal.
3. Orthogonally project the sample onto that plane.
4. Compute barycentric coordinates of the projected point and require that the projection lie inside the triangle boundary within a numerical tolerance.
5. Select the eligible target plane with the smallest absolute perpendicular distance. Ties retain the first target in input order, making the result deterministic.
6. Preserve `None` for samples with no valid correspondence; they are not silently converted to a large distance or a zero.

For each cutoff (r), the module sorts the corresponding distances (leq r), reports the median, the accepted count and the number of valid correspondences rejected for exceeding the cutoff. The no-correspondence count is separate and constant across the curve.

### Directed profile types

| Direction | Permitted sample population | Target surface set |
|---|---|---|
| `reference_to_candidate` | `reference_surface_samples` | Candidate model surfaces |
| `candidate_to_reference` | `source_surface_vertices` or `source_surface_samples` | Observable reference surfaces |

Both directions must be evaluated separately. The core returns the direction and sample population in its result so downstream aggregation cannot erase which method was used.

## Synthetic tests added

The unit suite checks:
- 20 mm offset between parallel planar surfaces;
- orthogonal projection outside the target triangle is rejected as no correspondence;
- unobservable targets cannot create correspondences;
- median-versus-cutoff behavior at 20 mm, 100 mm and 250 mm;
- conventional even-count median;
- nearest eligible plane selection;
- direction/sample-population mismatch rejection;
- non-positive and non-monotonic cutoff rejection;
- degenerate triangles and non-finite samples fail closed;
- direction and population survive in the returned result.

All inputs are hand-built synthetic triangles and points. These tests validate numerical semantics; they do not demonstrate performance or score an external dataset.

## Claim boundary and omissions

This module does **not** yet:
- generate the source sample population from IFC meshes or reference meshes;
- parse IFC, E57, LAS/LAZ, PLY or point-cloud files;
- build or validate triangle meshes from actual BIM/scan data;
- estimate observability from sensor poses or reference geometry;
- apply the full 2018 projection-to-boundary correspondence to arbitrary polygons with holes (the v0.1 primitive uses triangles);
- calculate completeness/correctness intersection areas or area denominators;
- implement the official source/target surface buffering, tessellation, area approximation, curve aggregation or historical scorer;
- perform coordinate registration or uncertainty propagation.

The result is a deterministic directed-distance building block, not the official ISPRS accuracy metric and not a geometric SOTA result.

## Next geometry milestones

1. **P1.1 — source sampling and exact method adapters.** Implement mesh vertex and surface-area-weighted sampling with pinned seeds/spacing. Build named 2018 vertex and 2021 sample adapters; keep the official-page reference-sample adapter separate. Verify all sample identities/counts.
2. **P1.2 — polygonal observable reference surfaces.** Replace triangle-only boundary membership with polygon-with-holes support and stable element/face identity. Preserve excluded/unobservable boundary area.
3. **P1.3 — completeness/correctness area oracle.** Add buffered parallel-surface intersection area, source/reference denominator semantics and threshold curves, first on analytic fixtures then independent fixtures.
4. **P1.4 — independent scorer differential.** Compare all directed profiles, sample populations, cutoff curves and 10 cm leaderboard values to canonical scorer output or maintainer-confirmed results. Retain discrepancies rather than tune them away.
5. **P2 — matched dataset experiment.** Only after data-access permission, use building-level holdouts (TUB1/TUB2 stay together), exact input hashes, independent checkpoints and report per-scene curves plus bootstrap intervals. No hidden reference data in candidate runtime, normalization, config or CI artifacts.

## Comparative targets

The first objective is source-faithful reproduction, not beating a published scalar:
- match named 2018 and 2021 methodology profiles independently;
- reproduce the official results-page profile as a separate direction until scorer provenance is reconciled;
- report full cutoff curves, counts and per-scene results, not only the 10 cm median;
- after reproduction, compare completeness/correctness/accuracy as a Pareto set, then add calibration, abstention and temporal identity metrics.

Published 2026 targets remain in `SCAN_BIM_EXECUTION_RECEIPT_AND_BENCHMARK_TARGETS_V0_1.md`. Do not compare LiBiDT association F1, room-area deviation, structural area accuracy and ISPRS wall distances as if they were one task.

## Reproducible run

```sh
cargo fmt --manifest-path tools/scan-bim-replay-contract/Cargo.toml -- --check
cargo test --locked --manifest-path tools/scan-bim-replay-contract/Cargo.toml metrics::tests
cargo clippy --locked --manifest-path tools/scan-bim-replay-contract/Cargo.toml --all-targets -- -D warnings
```
