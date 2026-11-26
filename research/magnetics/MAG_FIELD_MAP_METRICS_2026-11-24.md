# MAG-FIELD-METRICS-001 — exact-grid magnetic field-map comparison

**Prepared:** 2026-11-24  
**Status:** isolated mathematical reference; Rust execution, CI and physical validation are not claimed  
**Symtropy issue:** [#7332](https://github.com/Luminous-Dynamics/symtropy/issues/7332)  
**Parent process twin:** [MAG-FUNC-PROC-001 #7310](https://github.com/Luminous-Dynamics/symtropy/issues/7310)  
**Symthaea design owner:** [MAG-FUNC-001 #13521](https://github.com/Luminous-Dynamics/symthaea/issues/13521)

## Purpose

Make field-map comparison executable before integrating a finite-magnet solver or accepting a physical map. The comparator receives two vector maps on the same frozen spatial sample locations: a reference map and a predicted or observed map.

Same-length arrays are not sufficient evidence that two maps represent the same measurement domain.

## Map identity contract

Each map is bound to a non-empty coordinate-frame identifier, a non-empty frozen grid identifier, and a non-empty ordered collection of unique sample IDs. Each sample carries position in metres and B vector in tesla.

Before scoring, the comparator requires equal frame IDs, equal grid IDs, equal sample counts, identical ordered sample IDs and exactly matching sample positions. Non-finite inputs, duplicate IDs, missing samples, reordering and coordinate mismatches are rejected. No interpolation, resampling or silent sample deletion is permitted.

Grid and frame IDs are opaque caller-supplied identities in this first version; they are not cryptographic hashes or authenticated provenance. A later evidence envelope should bind them to canonical geometry, grid-generation recipe, sensor calibration, measurement condition and immutable input digests.

## Metric definitions

For N matched samples, reference vector B_i and candidate vector Bhat_i, define e_i = Bhat_i - B_i.

- Component MAE: mean absolute error over all 3N scalar components.
- Vector MAE: (1/N) sum_i norm(e_i).
- Vector RMSE: sqrt((1/N) sum_i norm(e_i)^2).
- Reference RMS: sqrt((1/N) sum_i norm(B_i)^2).
- Normalized vector RMSE: vector RMSE divided by reference RMS, defined only when reference RMS is nonzero.
- Maximum vector error and nearest-rank p95 vector error.
- Absolute-tolerance coverage: fraction of samples with norm(e_i) less than or equal to the declared tolerance. The all-samples flag requires every sample to meet the same tolerance.
- Direction-angle error: atan2(norm(u_ref cross u_candidate), u_ref dot u_candidate), only where both field magnitudes exceed the declared direction floor.
- Direction-reversal fraction: fraction of direction-qualified samples whose normalized dot product is negative.

No composite score is created. A high coverage fraction cannot compensate for a high maximum error, incorrect field direction, or poor application response. The caller inspects the full score vector.

The normalized metric is undefined, not zero, when the reference map has zero RMS. Directional metrics are undefined when no sample clears the magnitude floor. These conditions are represented as absent optional values.

## Sampling caveat

The crate computes unweighted sample metrics. They are not automatically volume integrals. Comparisons require a frozen, shared grid and declared sampling domain. A uniformly spaced volumetric grid can support an equal-weight approximation to a volume average; a nonuniform grid, surface-only scan or adaptive sampling needs explicit quadrature weights and/or a separate leaderboard. Surface and volume maps must not be pooled.

## Why strictness matters experimentally

Behera, Lv & Singamneni (2026), [Patterned magnetic pole configurations in bonded magnets](https://doi.org/10.1038/s41598-026-43131-5), report patterned poles in a two-island, five-layer (50 micrometres per layer) multi-material SLS proof of concept. The paper distinguishes its 230 mT in-process field, 1.5–1.9 T post-print magnetizing field and measured surface responses under external-field conditions. The differential 80–100 mT north / 50–100 mT south observations are not intrinsic Br or Hcj. The paper reports raw data as available from the corresponding author on request and discusses an approximately 150 mT local anomaly associated with excess FeSi deposition. A contour image is not a substitute for a coordinate-identified, pointwise three-dimensional reference map.

## Manufacturing tournament update

Sarwar et al. (2026), [State-of-the-art additive manufacturing of NdFeB permanent magnets](https://doi.org/10.1016/j.jsamd.2026.101226), reviews AM NdFeB and reports a broad 0.6–1.0 T remanence range across reviewed AM work. It identifies material extrusion as a strong remanence route among the reviewed AM methods and contrasts AM results with approximately 1.4–1.5 T anisotropic sintered grades. This means the earlier 0.52–0.59 T half-gap arithmetic is a target only against the stated 0.70–0.84 T commercial bonded-grade reference, not a universal additive-manufacturing SOTA target.

The route tournament should include:
1. Material extrusion (MEX) with aligned bonded NdFeB.
2. Multi-material SLS and other laser powder-bed bonded routes.
3. Lithography-based composite manufacturing with field alignment and post-cure.
4. Recycled powder extrusion moulding plus debinding and sintering.
5. Conventional anisotropic injection-moulded bonded grades.
6. Conventional anisotropic sintered magnets.

These are separate material-form and process strata, not one pooled remanence ranking. A higher Br does not demonstrate better spatial field topology or application function. Match density, material form, geometry, measurement direction, temperature, magnetization history, demagnetization correction and application boundary before comparing performance.

## Qualification boundary and next increment

This crate validates metric arithmetic and strict map identity only. It does not validate the field producer, sensor calibration, magnetic constitutive model or manufactured component.

Future uncertainty scoring must specify measurement covariance, prediction covariance, calibration set, coverage levels and out-of-distribution behaviour. For anisotropic or correlated sensor error, a scalar standard deviation is insufficient; use a declared covariance contract. Do not backfill unknown uncertainty.

A future solver adapter should first be compared against independent analytic finite-geometry references (for example, analytical uniformly magnetized cuboid solutions), then against a qualified FEM/BEM implementation, and only then against a measured map. A point-dipole far-field oracle is not a substitute for finite-magnet references.

## References

1. Behera et al. (2026), [Patterned magnetic pole configurations in bonded magnets](https://doi.org/10.1038/s41598-026-43131-5).
2. Sarwar et al. (2026), [State-of-the-art additive manufacturing of NdFeB permanent magnets](https://doi.org/10.1016/j.jsamd.2026.101226).
3. He et al. (2026), [Active data selection and regression for sintered NdFeB](https://doi.org/10.1038/s41524-025-01914-w).
4. IEC 60404-5:2015, [methods of magnetic property measurement](https://webstore.iec.ch/en/publication/22142).
5. IEC 60404-8-1:2023, [permanent-magnet material specifications](https://webstore.iec.ch/en/publication/68440).
6. IEC 60404-18:2025, [open-circuit SCM/VSM method and self-demagnetization correction](https://webstore.iec.ch/en/publication/76495).

## External solver and analytics baselines

Do not build a general field solver as the first response to this problem. Magpylib provides independently implemented closed-form field solutions for cuboid and cylindrical permanent magnets, triangular surface-charge geometries and current loops. Use those finite-geometry analytic solutions as an independent known-answer layer for simple homogeneous geometries before any external numerical solver is trusted. Its assumptions and accuracy boundary must be respected: these analytical models do not establish the behaviour of heterogeneous, nonlinear soft-magnetic regions, process-induced microstructure or hysteresis.

Commercial engineering baselines already support optimization around electromagnetic models. COMSOL describes parameter, shape and topology optimization, including optimization of permanent-magnet position/shape against motor torque. JMAG describes integrated FEA and optimization, sensitivity analysis, response surfaces, Pareto analysis and topology methods. These tools should be part of the baseline tournament rather than dismissed or reimplemented.

The Luminous differentiator should therefore be the evidence-carrying bridge between target field function, multi-material/process feasibility, measured manufacturing variability and repeatable field-map qualification—not a claim to have a universally superior FEM kernel or optimizer.

References:
- Magpylib 6, [Physics and Computation: analytical field solutions](https://magpylib.readthedocs.io/en/stable/_pages/user_guide/guide_resources_01_physics.html).
- COMSOL, [Optimization Module](https://www.comsol.com/optimization-module).
- JMAG, [Optimization and topology optimization](https://www.jmag-international.com/products/jmag-designer/optimization/).