# Variable-Aware Forest Observation Aggregation v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Support-conversion work:** [#5237](https://github.com/Luminous-Dynamics/symtropy/issues/5237)  
**Status:** bounded synthetic kernels and 12-case corpus; no production raster or survey adapter

## 1. Why aggregation is a scientific operation

"Resample to a common grid" is not a sufficient observation model. Geometry tells us which measurements overlap a target; variable semantics tell us what mathematical operation is valid.

The same 25 m footprint or 30 m pixel may carry a density, a total stock, a sample count, or a class label. These values cannot all be averaged by area or summed without qualification.

The paradigm shift is to make aggregation **typed by measurement semantics** and return the observed support alongside the result. A value without its covered area, valid-data fraction, units and sampling design is incomplete evidence.

## 2. Four distinct operators

### A. Intensive variable: area-weighted mean

For values $z_i$ over non-overlapping source supports and valid overlap area $a_i$:

`\bar z_{valid} = \frac{\sum_i z_i a_i}{\sum_i a_i}`

The output is the mean over the **valid observed portion**, not automatically the whole target. The receipt must include:
- target area;
- geometric coverage fraction;
- valid-data coverage fraction;
- valid observed area;
- mean and unit, or no mean when no valid observations exist.

Examples include aboveground biomass density (Mg/ha), canopy height (m), or a spatially represented continuous field. The source's own measurement uncertainty and any support-conversion uncertainty must be carried separately in a later production adapter.

### B. Extensive stock: integrate density, do not average totals

Given biomass density $b_i$ in Mg/ha and valid overlap area $a_i$ in m²:

`B_{observed} = \sum_i b_i \frac{a_i}{10,000}`

The result is an extensive stock (Mg) over the observed overlap only. A half-covered target returns half-support stock evidence; this kernel does not multiply by two to invent the unobserved half. Whole-target estimation requires a separately justified model or sampling estimator, with its extrapolation and uncertainty declared.

This operator is intentionally named as an integration from density. It is not a generic rule to prorate any arbitrary extensive source total uniformly within its source cell.

### C. Counts: sampling design, not raster arithmetic

For a finite population of sampling units selected with known first-order inclusion probabilities $\pi_i>0$, the Horvitz–Thompson total is:

`\hat N_{HT}=\sum_{i\in s}\frac{y_i}{\pi_i}`

The implementation requires:
- explicit sampling-frame and sampling-design identities;
- the design to be declared as a probability sample;
- unique sampling-unit identities;
- positive, finite first-order inclusion probabilities.

It does **not** fabricate a standard error from first-order probabilities alone. Variance estimation generally needs second-order inclusion probabilities or a justified alternative design-specific estimator. Non-probability camera-trap detections, opportunistic sightings or convenience plots cannot be promoted to a design-based population total by supplying an arbitrary weight.

### D. Categories: class composition, never numeric averaging

For categorical source classes, return the valid area and proportion for each class identity. Do not average integer class IDs. A mixed cell containing forest and degraded forest is a composition, not a new fractional class code.

Missing/invalid source support contributes to geometric coverage but not valid class area. Unknown categories are not silently mapped to forest or non-forest.

## 3. Executable scope

The Rust implementation is in `validator/src/aggregation_kernel.rs` and is composed with the bounded rectangular support kernel. The public synthetic corpus is `cases/forest-variable-aware-aggregation-synthetic-v0.1.json`.

Twelve CLI cases exercise:
1. area-weighted intensive mean over complete support;
2. partial intensive mean without extrapolation;
3. all-masked intensive support (no numeric zero-fill);
4. unit mismatch refusal;
5. density-to-stock integration over complete support;
6. density-to-stock integration over partial support;
7. Horvitz–Thompson count total with first-order probabilities;
8. zero inclusion-probability refusal;
9. non-probability design refusal;
10. categorical area fractions preserving class identity;
11. masked categorical support preserving the coverage gap;
12. empty-class refusal.

The kernel adds seventeen Rust unit tests. The library has three corpus/mutation tests for the variable-aware corpus.

## 4. Independent acceptance targets

Before any production adapter is promoted:

| Invariant | Target |
|---|---:|
| Intensive area-weighted mean agrees with an independent analytic oracle | 100% of frozen cases |
| Partial support remains explicitly partial | 100% |
| Missing/invalid support contributes no fabricated zero or class label | 100% |
| Density-to-stock unit conversion is correct | 100% |
| Non-probability counts cannot produce an HT total | 100% refusal |
| Invalid inclusion probability or duplicate sampling unit is rejected | 100% |
| Categorical class identity is preserved | 100% |
| Identical inputs produce deterministic outputs | byte-identical receipts |
| Survey-design uncertainty | explicit `NOT_ESTIMATED` until a valid variance method exists |

These are synthetic conformance targets, not claims about field accuracy or ecological performance.

## 5. Industry / SOTA context

The competitive landscape in 2026 supports a focus on data semantics, transfer and downstream scientific validity:

- A systematic review of 190 forest computer-vision studies reports that standardized benchmarking is absent in 73% of the reviewed work and reports 20–45% cross-biome degradation in the studies it synthesizes. It also reports high pooled ViT classification accuracy, but those image-classification metrics do not establish demographic or ecosystem-model accuracy. This is review-level evidence and should not be treated as a universal degradation rate: https://link.springer.com/article/10.1007/s10531-026-03412-x
- An October 2026 review of 57 studies frames digital-twin forestry around continuous coupling of multisensor EO and ecosystem models through data assimilation. It also reports that tropical and boreal representation remains uneven and most forest digital-twin systems are at prototype or early operational stages: https://link.springer.com/article/10.1007/s44564-026-00029-8
- An August 2026 perspective on nature-technology-data integration organizes practice around survey design, cueing, validation, consilience, interpolation/extrapolation and data fusion, supported by metadata, rights, scientific rigor and utility: https://doi.org/10.1002/rse2.70100
- A 2026 review of crown segmentation highlights benchmark resources including NEON Tree Crown, BAMFORESTS, VHRTrees, FOR-instance and OAM-TCD, and argues for cross-site, cross-sensor evaluation and measurement of downstream ecological effects: https://link.springer.com/article/10.1007/s42452-026-09721-8

**What to do better:** use the appropriate specialist models and source products, but refuse to flatten their outputs into a generic common raster. Expose the observation operator, variable type, support and uncertainty; compare their downstream forest-state implications under independent holdouts. Use multi-source consilience (independent signals that agree for a declared ecological reason) before data fusion. Fusion should be an earned step, not an assumption.

## 6. Benchmark programme

For a later held-out empirical tournament, compare:
- nearest-cell / nearest-centre assignment;
- unweighted mean or sum as naïve baselines;
- pixel-count / rasterized-area weighting;
- exact area-overlap weighting;
- source-native product aggregation where documented;
- a high-resolution independently measured reference;
- a design-based estimator for genuine probability samples.

Freeze the same source data, geometry, target support, units, quality masks, sample frame, compute budget and evaluation split. Use independent site/plot/campaign blocks. Report:
- bias, MAE and RMSE for intensive variables;
- total-stock error and budget closure for extensive quantities;
- design-based total error and variance calibration for counts;
- per-class area/fraction error and confusion for categorical products;
- coverage, abstention and unobserved rates;
- sensitivity to registration shifts, resolution and masks;
- runtime, memory, energy and data-transfer cost.

Do not compare a model's full-support prediction against a partial source observation without declaring the operator and coverage. Do not compare class IDs as continuous values. Do not publish a single aggregate score that hides a category, region or rights gate failure.

## 7. Explicit limitations and next work

Not implemented:
- arbitrary polygons, holes, topology repair and fractional raster-cell intersection;
- affine/rotated raster grid transforms;
- CRS transformations and datum/grid-shift provenance;
- positional/registration-error distributions and uncertainty propagation;
- source covariance and spatially correlated error propagation;
- second-order sample inclusion probabilities and survey-design variance;
- source-native calibration, QA semantics and empirical validation;
- real source asset authentication and rights adjudication.

Next: extend #5237 with polygon/affine support and a math oracle; add independent source uncertainty and area covariance propagation; then build adapters only for rights-cleared, source-version-pinned observations. No model should consume an aggregate without its coverage and uncertainty companion fields.

## 8. Claim ceiling

The current successful corpus result, when exact-head Rust tests pass, means only that the declared synthetic arithmetic and semantic rules are internally consistent. It does not demonstrate calibrated stock estimates, valid survey inference, cross-biome transfer, real forest-dynamics accuracy, conservation impact or SOTA superiority.
