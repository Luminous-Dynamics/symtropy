# Orbital Surface Sampling Density — Contract v0.1

**Owner:** #7164  
**Status:** standalone synthetic accounting contract; geometry ray-generation not implemented  
**Claim ceiling:** ray-intersection density accounting on declared surface elements only

## Paradigm

Binary visibility and k-view count do not specify how densely a surface element is sampled. Non-uniform mesh elements make this important: equal ray counts on unequal areas produce unequal sampling density.

## Metric

For each surface element i, candidate view j contributes P_ij ray intersections. For selected views S:

rho_i(S) = sum(P_ij for j in S) / A_i

where A_i is the surface element area. A separate minimum number of contributing view IDs may also be required. This view-count constraint is not interchangeable with density.

The frozen v0.1 encoding uses:

- area_um2: integer surface area in µm²;
- minimum_density_micro_rays_per_mm2: integer target in micro-rays/mm²;
- intersection_count: non-negative integer ray count for each candidate view;
- provenance: either EXACT_SYNTHETIC_ORACLE or CALLER_SUPPLIED_UNVERIFIED.

The exact threshold comparison is:

sum(P_ij) * 10^12 >= minimum_density_micro_rays_per_mm2 * A_i,um²

No binary floating-point comparison is required. The receipt also returns the exact rational density representation as numerator 10^6 * sum(P_ij) over denominator A_i,um².

## Frozen controls

1. Equal area and equal hit count produce equal density.
2. Equal hit count over tenfold greater area produces tenfold lower density.
3. Multiple selected views contribute additively.
4. Unselected views do not contribute to the selected plan.
5. Candidate-pool insufficiency is not hidden by selection.
6. A selected under-dense plan is separated from candidate-pool insufficiency.
7. Unverified ray-count provenance yields INDETERMINATE.
8. Zero area is structured INVALID_INPUT.
9. A well-sampled small element cannot mask an under-sampled large element.
10. Minimum contributing-view count is separate from ray density.
11. Non-uniform multi-element density is checked element-by-element.
12. Element/view row order does not change the receipt.

## Relationship to the dissertation comparator

Staderini's TU Wien dissertation (submitted 1 December 2025), Chapter 4, introduces a sampling-density matrix based on ray intersections per surface-element area, accumulated across viewpoints. It uses that quantity as a set-coverage constraint without remeshing the object. Its full system also includes camera-ray generation, surface sampling, photometric visibility, robot configuration and path planning.

This crate is only the density accounting contract for an already-supplied hit-count matrix. It does not reproduce those upstream geometry/camera calculations or the physical UR10e/Zivid transfer experiments.

## Portfolio integration

- #7002/#6925: geometric candidate views and k-coverage.
- #7136: conditional support-envelope geometry and future occlusion.
- #7164: ray intersections per non-uniform surface-element area.
- #7043/#7049: feature-specific measurability/conditioning.
- #6846: calibrated uncertainty and conformance decision.
- #5920/#5892: route and fleet execution.
- #4258/#6752: observation/noise/correlation.
- Mycelix #6390/#6519: identity, provenance and authority evidence.

No result from this contract constitutes actual sensor sampling, resolved defect size, measurement uncertainty, probability of detection, NDE qualification or engineering acceptance.
