# Ersöz–Bosché 2026 PCD–BIM matching-confidence Rust reproduction v0.1

**Status:** deterministic metric-core implementation with hand-built fixtures; no external point-cloud/BIM reproduction or SOTA claim.  
**Issue:** #4246 — SCAN-BIM-MATCH-CONFIDENCE-001.  
**Implementation:** tools/scan-bim-replay-contract/src/matching.rs.  
**Stack:** after #4225 deterministic surface sampling and #4205 directed accuracy.  
**Research date:** 2026-11-08.

## Pinned comparator

Paper:
- Ahmet Bahaddin Ersöz and Frédéric Bosché, “Evaluating confidence in geometric matching between 3D point clouds and BIM models by integrating coverage, distance, and distribution metrics,” Automation in Construction 181 (2026), 106636.
- DOI: https://doi.org/10.1016/j.autcon.2025.106636
- Open article PDF: https://open.metu.edu.tr/bitstream/handle/11511/116924/Evaluating%20confidence%20in%20geometric%20matching%20between.pdf

Released implementation:
- https://github.com/CyberbuildLab/pcd-bim-matching-confidence
- Pinned source commit: ff21962c1c65ae9bfb966f21dfd66af0b8648a19
- utils.py blob: 713122d4118d21250778e032beeba50858566f99
- main.py blob: 9482d2956045c39d791187755d24486c6a2f109a
- LICENSE: Apache-2.0. Any direct code reuse must preserve the Apache license and attribution requirements. The Rust implementation here is an independent implementation of the published equations and the observed released-code behavior.

The paper evaluates 46 PCD/BIM pairs and 4,000+ IFC elements across ISPRS, CV4AEC, BIMNet and custom datasets. Its scope includes walls and diverse architectural/MEP objects. These are author-reported study results; the Luminous implementation has not yet replayed those data.

## Published method — article-text profile

For one BIM element, the method constructs:
- B: uniformly sampled points on the theoretically non-self-occluded BIM surface;
- S: a coarse scan subset from an oriented bounding box expanded by 10 cm;
- B': BIM samples whose nearest point in S lies within delta;
- S': unique scan points paired to B' in the fine-matching pass.

The study used:
- BIM surface sampling resolution: 2 cm;
- scan point-cloud voxel downsampling: 1 cm;
- coarse OBB expansion: 10 cm;
- fine matching threshold delta: 5 cm.

The three indices are:

**Coverage**
`I_coverage = |B'| / |B|`

This is a proportion of theoretically visible BIM surface samples with at least one scan point within delta. It is not raw scan point count or point density.

**Distance**
For each matched BIM sample:
`alpha_i = 1 - d(b_i, S) / delta`

Then:
`I_distance = mean(alpha_i over B')`

It measures closeness only over matched pairs. Coverage separately captures unmatched BIM samples. Do not silently change the distance denominator to all BIM samples.

**Distribution**
Let P be the normalized histogram of normals for S', and Q the normalized histogram of normals for B. Then:
`I_distribution = 1 - D_KL(P || Q)`

The article describes a 40-bin spherical normal histogram: five polar slices of width pi/5 and eight azimuth slices of width pi/4.

**Composite**
The equal-weight experimental form is:
`I_confidence = I_coverage * I_distribution * I_distance`

The paper also permits application-specific weighting. The three indices should be retained separately. The product is not a probability, not an uncertainty calibration, and not a universal admission criterion.

## Released-code profile and discrepancy

The pinned upstream code does not exactly match the article text in two implementation details:

1. **Polar histogram boundaries.** The article text uses five equal polar intervals of pi/5. The released Python code uses boundaries pi/8, 3pi/8, 5pi/8 and 7pi/8. It collapses each polar cap to one azimuth-independent bin, while splitting the three middle bands into eight azimuth bins.
2. **KL similarity handling.** The article displays 1 - D_KL(P || Q). The released implementation clips each histogram probability to epsilon=1e-10, renormalizes both histograms, computes D_KL(scan || BIM), then clamps 1 - KL to [0,1]. The article's displayed equation does not include the same smoothing/clamping.

Both are preserved as explicit profiles:
- `PaperTextV1`: article-text polar partitions; no epsilon smoothing; raw 1-KL value retained, including negative values.
- `UpstreamCodeFf21962V1`: exact pinned-code polar boundaries; epsilon smoothing; renormalization; clamp to [0,1].

Neither profile is silently declared canonical for historical results. A paper reproduction should state which profile was used and compare it with the pinned code.

## Rust implementation

`matching.rs` accepts:
- BIM surface samples with stable sample IDs, points, BIM-derived normals and represented area weights;
- scan samples with stable IDs and coordinates;
- a matching radius in metres;
- an explicit paper-text or upstream-code distribution profile.

A deterministic spatial hash uses delta-sized cells and searches the 27 adjacent cells. Each BIM sample is matched to the nearest scan point within the inclusive threshold. Equal-distance ties use stable scan-sample IDs. BIM and scan samples are sorted by IDs so input permutation does not change the result.

The unique S' subset is formed by scan-sample ID. If multiple BIM samples map to one scan point, its normal is assigned from the first BIM sample in stable ID order, matching the pinned implementation's “first mapping” behavior while making the choice deterministic.

The report includes:
- count-based published coverage index;
- count-based published distance index;
- distribution index under both paper-text and code profiles;
- selected profile and selected distribution value;
- the optional confidence product;
- area-weighted coverage/distance extensions, separately named;
- BIM and S' normal histograms;
- exact matched BIM/scan identities and distances.

The area-weighted extensions use sample represented-area weights from #4225. They are not substituted for the published sample-count indices. Their value is to expose tessellation/sample-allocation sensitivity and to support a later explicitly area-weighted operational score.

## Visibility boundary

The paper's theoretical visibility calculation addresses self-occlusion: it excludes BIM surface samples whose distance to other element surfaces is near zero. This is not the same as proving that a particular scan pose, sensor field of view, range or trajectory actually observed that surface.

The Rust metric API receives the caller-qualified BIM sample population. It does not ray-cast, estimate visibility, infer sensor poses or claim that every non-self-occluded sample was scanned. Preserve the separate visibility accounting from #4151/#4188 and the survey provenance from #1607.

## What this Rust profile does not yet reproduce

- IFC geometry extraction, including representation types, placements, opening subtraction and tessellation.
- The exact Open3D uniform mesh sampling and voxel downsampling used in the upstream executable.
- The author's theoretical-visibility preprocessing implementation and all element-type filters.
- OBB construction/cropping and exact candidate scan-subset preparation.
- Scan-normal handling outside the pinned behavior of assigning normals from BIM matched points.
- The complete CSV/IFC property export pipeline.
- The 46-pair / 4,000+ element paper evaluation, CV4AEC/BIMNet/ISPRS data, or author case study.

The current module is an independent metric core fed by already-qualified samples. It is not yet an end-to-end reproduction of the Python program.

## Hostile tests

The test suite covers:
- exact match and unit indices;
- unmatched and offset samples, keeping coverage and distance distinct;
- inclusive delta threshold;
- unique S' construction when multiple BIM samples map to one scan point;
- sample-count versus area-weighted coverage divergence;
- input permutation determinism;
- article/code polar-cap histogram discrepancy;
- KL-derived values below zero in article-text mode and clamp behavior in upstream-code mode;
- non-finite coordinates, zero normals and invalid delta.

Additional required tests before full paper reproduction:
- exact 40-bin fixtures for axis-aligned and oblique normals;
- KL direction P||Q vs Q||P and smoothing edge cases;
- multi-face wall with one-sided vs two-sided observed distribution;
- duplicate IFC Names and GlobalId identity ambiguity;
- source sampler and upstream Open3D sampling sensitivity;
- visibility false positives from other geometry and temporary occluders;
- independent reference implementation of the KL and matching calculations.

## How Luminous can do better without misrepresenting the baseline

Preserve published coverage, distribution and distance as three independent metrics. Extend the evaluator with:
- per-surface supported area and stable sample lineage;
- explicit non-self-occluded, observed, outside-sensor-coverage, unknown and contradictory strata;
- persistent component identity and topology consistency;
- measurement/registration uncertainty and independent checkpoint provenance;
- calibration tests where the confidence output must predict independent geometric errors;
- selective-risk vs accepted-coverage curves and false confident hidden-surface claims;
- false model mutations per independently accepted physical change;
- expert correction person-minutes and matched acquisition/review cost.

Do not optimize the confidence product alone. A higher product can hide a catastrophic error in one factor, and the distribution value can be negative under the paper-text KL formula. The acceptance layer must use explicit failure reasons and hard gates.

## Benchmark gates

1. **Primitive parity:** all hand-computed matches, coverage, distance, histograms and KL values pass in both profiles.
2. **Source parity:** differential compare against the pinned upstream implementation on synthetic mesh/scan fixtures. Record any differences due to deterministic IDs, point order or chosen sample generation.
3. **Paper parity:** reproduce author-reported metrics on accessible, rights-approved benchmark data under matched sampling and thresholds.
4. **Independent validation:** assess calibration, model-mutation false positives and human correction burden using independent held-out building groups.
5. **Industry comparison:** compare with commercial element tolerance/coverage reporting at matched scans, tolerances, operators and accepted quality.

## Claim ceiling

The Rust module implements the declared mathematical matching profiles on caller-supplied samples. It does not yet reproduce the complete source pipeline, calibrate a confidence probability, qualify a real building model, beat SOTA or establish industry superiority.
