# Fragment morphology benchmark v0.1

**Status:** research protocol + synthetic conformance artefact. No field dataset, transport prediction or mine performance is claimed.

## 1. Research motivation

The parent fragmentation/transport evaluator (#3876) measures dry-solid generation, non-overlapping particle-size mass bins, recovered/residual/rejected material and installed-path stage capacities. Its executable schema does not encode morphology distributions.

That is an important representational gap:

- ISO 9276-6:2008 was reviewed and confirmed current in 2022. It states that averaging shape over a sample is ineffective and calls for distributions of particle characteristics alongside size distributions. https://www.iso.org/standard/39389.html
- ISO 13322-2:2021 covers dynamic particle-image analysis and instrument qualification, including motion blur, depth of field and particle orientation. https://www.iso.org/standard/72566.html
- Boribayeva et al. (2026), Powder Technology 477, 122435, use dynamic particle tracking, Fourier descriptors and quasi-3D reconstruction for highly irregular particles. Their DEM validation is a rotating-drum rolling/cascading flow, not pneumatic hard-rock conveying. https://doi.org/10.1016/j.powtec.2026.122435
- Erken, Ooi & Hanley (2026), Chemical Engineering Science 328, 123719, find particle shape affects vertical plug porosity and velocity; particle size, inlet velocity and rolling/sliding friction also matter. They report differences between vertical and horizontal conveying mechanisms; their study is not a mine-scale hard-rock qualification. https://doi.org/10.1016/j.ces.2026.123719
- Faramarzi & Esmaeili (2026), Intelligent Geoengineering 3(1), 36–56, describe LiDAR point-cloud instance segmentation for rock-fragment morphology. Their SoftRock model was trained using synthetic labelled data and evaluated on three material piles; it is a measurement/segmentation comparator, not evidence of morphology-conditioned pneumatic transport transfer. https://doi.org/10.1016/j.ige.2026.04.002
- Faramarzi & Esmaeili (2025), Powder Technology 456, 120861, provide a 3D LiDAR fragmentation-analysis precedent with synthetic labelled point-cloud training and tests related to lighting, point density and size thresholds. https://doi.org/10.1016/j.powtec.2025.120861
- Ettoumi, Harrou & Ben Amar (published 2026-10-03) compare YOLOv8s-seg, YOLO11s-seg, YOLO26n-seg and YOLO26s-seg on 569 images and 128,767 annotated fragments across blast-pile and excavator-bucket stages. Their headline results show stage-specific degradation: approximately 52% versus 26% performance and approximately 12% cross-stage transfer in the reported setup. They also measure D20/D50/D80 distribution error, corruption sensitivity and hardware cost rather than relying only on detection accuracy. This is an adjacent open-pit image benchmark, not an underground cutting or vacuum-conveying benchmark. https://jidmis.org/index.php/jidmis/article/view/5071
- Raju & Esterhuyse (2026), Mining, Metallurgy & Exploration, report that removal rather than breakage limited sustainable operation in their seven-month South African undercutting/vacuum-extraction trial. https://doi.org/10.1007/s42461-026-01722-w

These sources motivate morphology-aware measurement; none independently establishes that a particular shape descriptor improves hard-rock pneumatic transport.

## 2. Paradigm shift

Replace the one-dimensional PSD-only representation with a **joint, size-conditioned material morphology distribution**:

`size_stratum \rightarrow morphology_descriptor_distributions`

Do not collapse it into one "shape score". Retain separate distributions and definitions for:
- 2D Feret aspect ratio, circularity and solidity;
- method-defined 2D Fourier angularity and roughness descriptors;
- 3D sphericity, aspect ratio and convexity where a calibrated 3D acquisition/reconstruction exists.

A 2D projection must never be promoted to a 3D claim. A particle shape descriptor must not stand in for particle-wall friction, particle-particle rolling/sliding friction, density, moisture, strength or abrasivity.

The parent PSD remains the mass authority. Morphology sampling is a particle-count sample and must not be silently reweighted into a mass distribution.

## 3. Executable v0.1 contract

The Rust crate `symtropy-fragment-morphology-bench` validates:

1. Exact parent case and interval identity.
2. Trial, machine-configuration and material-cohort matches.
3. Parent PSD bin indices and exact lower/upper boundaries.
4. Sample population accounting: valid + occluded + ambiguous + rejected = sampled.
5. Independent measurement or explicit derivation lineage.
6. Versioned descriptor definitions and explicit 2D/3D basis.
7. Descriptor-specific finite domains and ordered p05/p25/p50/p75/p95 distributions.
8. Descriptor sample count bounded by valid particles in the stratum.
9. Explicit calibration, reference material, sampling, segmentation, orientation, repeatability and raw-source references.
10. Explicit unsampled parent PSD bins. No interpolation, average-shape fill or false completeness.

The v0.1 evaluator reports sample counts, size-bin coverage and per-bin descriptor distributions. It does not estimate transport, pressure drop, blockage or productivity.

## 4. Model tournament

Freeze inputs, outputs, code versions and group partitions before comparison.

- **B0 — PSD-only:** existing #3876 mass-size distribution; no morphology.
- **B1 — PSD + material covariates:** add independently measured density, moisture and material cohort.
- **B2 — PSD + 2D morphology:** measured 2D descriptor distributions stratified by parent size bins.
- **B3 — PSD + 3D morphology:** independently measured 3D geometry and shape distributions.
- **B4 — Non-spherical DEM:** measured/reconstructed polyhedral or other non-spherical geometry, with separately calibrated friction parameters.
- **B5 — Hybrid physics/data model:** B4 plus pressure, airflow, electrical power/current and observed recovery response.
- **B6 — Symtropy/Symthaea:** evaluated on identical frozen inputs, output horizons and held-out groups.

Ablations must establish whether shape adds value beyond size, density, moisture, friction and path geometry. The null result "shape did not improve this target on this transfer cohort" is a valid result.

## 5. Benchmark tasks and measures

### Morphology measurement quality
- Distribution error and quantile error for each declared descriptor.
- Repeatability across repeated scans and operators.
- Sensitivity to orientation, occlusion, dust, motion blur, point-cloud density and segmentation errors.
- 2D-to-3D claim rejection rate.
- Per-size-bin sampling coverage and explicit unobserved fraction.

### Fragmentation transfer
- D20, D50 and D80 error on a volume-weighted size curve, keeping mass/volume weighting authority explicit.
- Bin-wise mass-distribution error and distribution distances.
- Held-out parent rock block, fragmentation campaign, imaging session and acquisition stage.
- Worst-stage and worst-material subgroup performance.

### Transport response
Only after physically independent labels exist:
- pressure-drop and pressure-fluctuation interval error;
- flow-regime / plug / dune / blockage event precision and recall;
- recovered-mass-flow error;
- residual accumulation error;
- calibrated predictive intervals and OOD abstention.

### System-level value
- accepted solid volume per full-cycle elapsed time;
- full-boundary kWh per accepted solid m³;
- nuisance alarms per operating hour at matched event recall;
- blockage recovery and cleanout downtime;
- cutter wear per accepted volume.

## 6. Splits and leakage prevention

Never make random adjacent frames or particles from one specimen the headline transfer test.

Use separate:
- train partition;
- calibration partition;
- untouched test partition.

Hold out entire parent rock blocks/specimens, fragmentation campaigns, imaging sessions/instruments, sites, and machine/transport configurations. Add a prospective chronological holdout. Keep stage-transfer tests explicit (e.g., face/cutter discharge to hose inlet, horizontal line to vertical riser, separator outlet), and do not silently mix stage-specific domains.

A reported model selection score must not obscure task-specific performance. Multi-criteria weights, if used for a decision, are a separate declared utility profile and sensitivity analysis; they are not a substitute for raw task metrics.

## 7. Preregistered hypotheses (not achieved results)

- H1: morphology-conditioned models improve held-out transport response prediction over B0 after controlling for size, density, moisture, friction and installed path.
- H2: dynamic or multi-view acquisition reduces orientation-induced descriptor variance relative to uncontrolled single-view imaging.
- H3: 3D shape features add value over 2D descriptors on at least one physically justified high-irregularity transport regime.
- H4: shape-aware DEM improves held-out pressure/flow response only when non-spherical geometry and friction parameters are independently calibrated.

Do not set a universal 20% improvement target before a pilot estimates between-campaign variance, repeatability and sensor uncertainty. Preregister the minimum meaningful effect size, sample-size calculation, exclusion rules, prediction horizon, confidence/coverage targets and stopping rule.

## 8. Adversarial conformance corpus

- same PSD bin boundaries and mass, materially different aspect-ratio/angularity distributions;
- 2D circularity mislabeled as 3D sphericity;
- a single uncontrolled orientation passed as representative 3D shape;
- orientation-biased imaging, particle occlusion, over/under-segmentation and motion blur;
- parent case, trial, machine configuration or material cohort mismatch;
- duplicate, missing, unsorted or mismatched parent PSD bin references;
- parent PSD gaps/overlaps and final-bin errors;
- sample population not reconciling into valid/occluded/ambiguous/rejected;
- descriptor sample count exceeding valid particles;
- descriptor formula/version drift;
- model estimate promoted to measurement;
- friction inferred from shape and mislabeled as independent physical input;
- absent shape coverage silently interpolated from neighbouring size bins;
- adjacent particles or frames from one specimen crossing the train/test boundary.

All planted structural controls must be rejected. This is contract conformance, not a physical performance claim.

## 9. Integration ownership

- #3864/#3876 own parent PSD, generated/recovered/residual/rejected mass, installed transport capacity and mass balance.
- #3891/#3892 (this issue/PR) own size-conditioned morphology evidence.
- #3877 owns transport event-state fusion.
- #3888/#3890 own spatial residual and support-footprint geometry.
- #3863/#3846 own cutter-health and wear transfer.
- #3812/#3814 own accepted output and full-cycle metrology.
- #3825 owns energy/resource accounting.
- #3853 owns robust strategy eligibility and Pareto evaluation.

## 10. Non-claims

This benchmark does not qualify a scanner, image segmentation model, DEM geometry, pneumatic line, mining machine, geotechnical condition or operating envelope. It issues no machine controls. Synthetic fixtures are conformance data only. Any claim of improved transport, productivity, safety or economics requires a prospective, independently measured physical campaign.
