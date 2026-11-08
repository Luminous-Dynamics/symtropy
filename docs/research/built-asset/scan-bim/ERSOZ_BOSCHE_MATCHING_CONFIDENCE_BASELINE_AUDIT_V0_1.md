# Ersöz–Bosché point-cloud/BIM matching-confidence baseline audit v0.1

**Status:** source inspection completed; exact metric reproduction not yet run.  
**Primary paper:** Ersöz & Bosché, *Evaluating confidence in geometric matching between 3D point clouds and BIM models by integrating coverage, distance, and distribution metrics*, Automation in Construction 181 (2026), 106636. DOI: https://doi.org/10.1016/j.autcon.2025.106636  
**Upstream implementation:** https://github.com/CyberbuildLab/pcd-bim-matching-confidence  
**Pinned source revision inspected:** `ff21962c1c65ae9bfb966f21dfd66af0b8648a19`  
**Declared upstream license:** Apache-2.0, confirmed by the pinned revision's root `LICENSE` file (blob `7cee444ba2f31f5cdb9b9e99cbcb3ebb0fb7e0a3`). Dependency and transitive-license review is still required before redistribution or vendoring.

## Why this baseline matters

This paper and implementation are a direct comparator for element-level point-cloud/IFC matching confidence. Symtropy should reproduce this baseline before proposing novel coverage, point-distribution or distance-confidence indices. The upstream repository is Python/IfcOpenShell/Open3D and consumes an existing IFC model and point cloud. It is an evaluation/matching-confidence tool, not a general point-cloud-to-BIM reconstruction system.

The paper reports evaluation on 46 point-cloud/BIM pairs and more than 4,000 IFC elements across multiple datasets. Those published counts and findings are study evidence, not a claim that Symtropy has reproduced the evaluation.

## Inspected implementation profile

At the pinned revision:

- `main.py` defaults to `IfcWall` elements.
- The default geometric matching threshold is `delta = 0.05` (model units must be confirmed for each input).
- Default mesh sampling resolution is 0.02; scan voxel downsampling is 0.01.
- The program reads IFC through IfcOpenShell, converts product geometry to Open3D meshes, reads PLY point clouds, filters relevant elements, samples expected surfaces and emits per-element CSV metrics and processed geometry.
- Dependencies in the pinned `requirements.txt` blob `fc6c4764c397c4af65a462ae58dfb45e4853da73`: Open3D 0.19, Shapely, pandas, Matplotlib, scikit-learn, NumPy 1.26.4, plyfile 1.1, IfcOpenShell 0.8.1 and trimesh. Exact transitive environment, dependency licences and platform support must be locked before reproduction/redistribution claims.

### Three confidence factors

The implementation computes:

`I_confidence = I_coverage * I_distance * I_distribution`

- **Coverage:** number of matched sampled BIM surface points divided by number of sampled candidate BIM surface points, `len(Bp) / len(B)`.
- **Distance:** for each matched BIM sample, weight `1 - d_i / delta`; the implementation first matches only nearest scan points with `d_i <= delta`, then averages those weights.
- **Distribution:** compare 5×8 spherical bins of surface normals (40 bins total) using a KL-divergence-based similarity. The code computes `1 - D_KL(P_scan || P_BIM)`, clamps the value to [0,1], and uses epsilon clipping/renormalization to avoid log/zero issues.

This scalar product is the upstream method's aggregate. Symtropy must retain the three factors and the matching sample counts separately; never use the product as a substitute for geometry, topology, epistemic calibration or admission.

## Implementation audit: reproducibility and adversarial concerns

The source inspection identifies items to freeze and challenge, not necessarily defects in the peer-reviewed method.

1. **Name-based exclusion is not persistent identity.** In visibility setup, elements are excluded from blocker geometry using `ref['name'] != geometry_name`. Duplicate or missing IFC `Name` values can exclude the wrong product or multiple products. A Symtropy adapter should use validated `GlobalId` / stable identity plus exact IFC revision; ambiguous identity must abstain.
2. **Visibility blockers are a selected product subset.** The theoretical-visibility path uses `IfcWallStandardCase`, `IfcWall`, `IfcSlab`, `IfcCovering`, `IfcColumn`, and `IfcBeam` as blockers. Other products may affect visibility but are not included in that blocker list. Reproduction must preserve this published implementation setting; any expanded blocker taxonomy is a separately named extension.
3. **Visibility is sampled, not analytic.** The code samples mesh surfaces with a count based on area/resolution, uses Open3D voxel downsampling and ray casting, and retains sampled points using a distance-to-blocker threshold of 0.5×resolution. Random-state behavior, mesh tessellation, triangle orientation, ray origins and threshold sensitivity need explicit characterization.
4. **Inner-visibility is a separate heuristic.** The code casts sampled normal rays against other IFC geometry and excludes samples whose absolute Z-normal component is at least 0.99 when calculating its inner-visibility percentage. Preserve the original output as a baseline but report its denominator and exclusion count explicitly.
5. **Normal histogram is sampling- and estimator-dependent.** Distribution scores depend on surface sampling, voxel resolution, normal estimation/orientation, binning, epsilon and direction of KL divergence. Test rotated/curved/non-Manhattan surfaces, reversed normals, sparse samples, zero bins and different voxel resolutions.
6. **KL similarity is asymmetric and clipped.** `1 - KL(P_scan || P_BIM)` is not a symmetric distance and clipping values below zero to zero can saturate poor matches. Reproduce the upstream calculation exactly as one baseline; investigate Jensen–Shannon or other calibrated alternatives only as separately named metrics, never as a silent replacement.
7. **Deduplication changes denominators.** The implementation maps each matched scan index to the first BIM sample and makes `Sp` unique, while `Bp` retains every matched BIM sample. This matters for density, duplicate points and many-to-one matches; expose both sample cardinalities and repeat the baseline under controlled density perturbations.
8. **Coarse crop is an oriented bounding box.** The scan is cropped to an extended OBB of the sampled target element. This is a matching efficiency heuristic, not proof of visibility or valid element correspondence. Test nearby parallel walls, repeated bays, duplicate names, clutter and elements close to crop boundaries.
9. **The aggregate can hide failure modes.** Preserve all three factors, per-element denominators, visibility fractions, matched point counts and distance percentiles. Never compare only the scalar product.

## Required reproduction and extension protocol

### Reproduction track (must match upstream)

- Pin this exact Git revision, dependency lock and runtime environment.
- Preserve original default parameters and provide any changed values as separately named runs.
- Fix or record random seeds wherever the upstream runtime permits; repeat stochastic runs to estimate run-to-run variation.
- Use the same IFC and scan input digests, IFC units/coordinate frame, element selector, tessellation, mesh/point sampling and output definitions.
- Independently reimplement or independently recompute each factor for a small deterministic fixture; compare the upstream output against the independent result within declared numerical tolerance.
- Report per-factor values and per-element distributions, not only the product.
- Record failures and unsupported element types rather than silently skipping them.

### Extension track (must not be mislabeled as reproduction)

- Replace name-only identity with stable GlobalId + model revision + explicit split/merge/unknown correspondence.
- Condition coverage on theoretically visible area and separate self-occlusion, external occlusion, field-of-view exclusion and unknown visibility.
- Quantify uncertainty calibration against independent checkpoints, not the score factor itself.
- Add opening geometry, topology/IFC relationship and persistent identity metrics.
- Test KL alternatives and normal representations as independently named metrics.
- Include change attribution across epochs and revision-bound model mutation admission.

## P0/P1 benchmark fixtures motivated by the code audit

- Two IFC elements with identical `Name` but distinct GlobalIds.
- One IFC element with missing Name but valid GlobalId.
- One target sharing a name with a distinct product that should still occlude it.
- A blocker of a product type outside the upstream blocker allowlist.
- A wall, floor, ceiling, curved column and non-Manhattan plane with correctly oriented normals.
- A normal-orientation reversal with otherwise identical geometry.
- Uniform versus highly nonuniform sampling at fixed surface geometry.
- Duplicated scan points and density changes that leave geometry unchanged.
- Distances exactly at 0, just below delta, exactly delta and just above delta.
- Zero matched points and tiny point counts where normal distributions are unstable.
- Candidate/reference bounding-box leakage controls for completion methods (separate from this matching-confidence baseline).

## Claim boundary

Inspection of source code is not reproduction of the paper. A passing synthetic test suite is not reproduction of the paper. Reproduction on the paper's original/allowed data is not external generalization. This audit identifies a strong baseline and specific experimental controls; it makes no superiority, safety or commercial-product claim.
