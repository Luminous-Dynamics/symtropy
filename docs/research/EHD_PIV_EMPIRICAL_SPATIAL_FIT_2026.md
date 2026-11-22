# EHD-PIV-UQ-006 — empirical spatial-error fitting and untouched-session validation

**Status:** research implementation / prospective benchmark. No raw experimental PIV cohort has been processed by this implementation.

## 1. Why this increment exists

UQ-005 propagates a calibrated spatial covariance component into integrated mass and advective-momentum functionals, but its calibration receipt was external. UQ-006 closes that implementation gap: it fits a candidate spatial model from independent-reference residual fields and evaluates it on disjoint complete acquisitions before emitting the UQ-005 kernel type.

This is a measurement-model increment, not a new image-correlation solver. It must not be presented as a demonstrated accuracy improvement until independent data are processed.

Sciacchitano & Wieneke (2016) showed why the dependency matters: in one Gaussian-weighted 32×32 px interrogation / 75% overlap case, error correlation was significant to about three vector spacings, with a value near 0.45 at two spacings. The paper also observed a small negative lobe and demonstrated that derived-quantity uncertainty depends on this correlation structure. Those observations are configuration-specific. Their rotation-flow assessment found vorticity-uncertainty agreement within 5–10% for tested windows/overlaps; that is a useful comparator, not an assumed result for this code. DOI: 10.1088/0957-0233/27/8/084006.

## 2. Estimand and data

For acquisition cluster a and spatial point i, the input is a 3-component residual:

`e_{a,i} = u^{PIV}_{a,i} - u^{ref}_{a,i}`

Coordinates use metres; residuals use m/s. The reference may be synthetic known-displacement truth, an independently calibrated measurement, or a separately qualified diagnostic. A unique reference-evidence ID is required for every acquisition. The fit configuration also requires a reference-uncertainty assessment ID. Residual covariance includes uncertainty from both PIV and the reference measurement. This crate does not deconvolve or infer the reference contribution. The fitted covariance may be interpreted as PIV-only error only when the reference contribution is negligible under the cited assessment or has already been separately modelled and removed by an externally qualified procedure.

The independent unit is a complete acquisition/dependence cluster, not a vector, interrogation window, image pair, or video frame. Calibration and validation cohorts require at least 20 independent acquisitions each as an initial screening floor. IDs and reference-evidence IDs must be disjoint. Every field in both cohorts must share the exact ordered grid and processing-protocol fingerprint.

The caller supplies SHA-256-shaped immutable manifest identities. The software checks their shape and stores them; it does not hash files or prove provenance, reference independence, or exchangeability.

## 3. Current model

The initial estimator fits one isotropic stationary Wendland C2 candidate in three-dimensional coordinate space:

`K(r) = (1-r)^4(4r+1)` for `0 ≤ r < 1`, otherwise zero.

For distinct locations:

`Cov(e_i,e_j) = K(||x_i-x_j|| / ell) S`

At a location:

`C(0) = S + N`

- `S` is the correlated spatial covariance component.
- `N` is the local independent nugget.
- Both matrices must be positive semidefinite.
- The calibration mean residual profile is preserved as a separate bias diagnostic. It is not silently removed from a reported engineering result.

The range candidates are provided in a strictly increasing, predeclared configuration. Each acquisition receives equal weight. A candidate is rejected when no identifiable spatial component can be estimated, when `S` is not positive semidefinite, or when `N=C(0)-S` is not positive semidefinite.

## 4. Untouched-acquisition validation

The validation cohort is never used to select the range or estimate `S`. Pair separations are binned in physical units using the median nearest-neighbour spacing. For every lag bin and component pair, the implementation compares the mean held-out correlation with the prediction from the frozen fit. It reports the maximum absolute discrepancy.

A same-component negative correlation lobe is flagged only when the mean across independent acquisition clusters is below zero by the predeclared z threshold. The validation disposition is one of:

- `Qualified`: held-out maximum absolute correlation discrepancy is within the frozen bound and no resolved negative lobe is detected.
- `RejectedNegativeCorrelationLobe`: the fitted nonnegative Wendland family cannot represent the held-out evidence.
- `RejectedHeldOutCorrelationFit`: the held-out correlation discrepancy exceeds the frozen bound.

The maximum-error gate and negative-lobe z threshold are stored in the fit record; validation cannot override them. Correlation normalization uses one variance scale estimated across the complete holdout, while every acquisition contributes exactly one lag-bin statistic and one independent-cluster observation. Individual vector pairs are not counted as independent samples. The report is bound to the exact fit configuration, reference assessment, calibration manifest, selected range and spatial covariance; it cannot be applied to a substituted fit. A qualified report can create a UQ-005 `SpatialErrorKernel` using its required `wendland_c2_anisotropic_3d` model identifier (the current isotropic fit is represented by equal x/y/z length scales). The fit exposes `local_nugget_covariance_for_uq005()` for N, and the validation report exports S. The report is bound to the fit configuration, reference-uncertainty assessment, calibration manifest, selected length scale and covariance matrix.

**Double-counting rule:** the fitted `N` must be supplied as each face sample's local velocity covariance. UQ-005 adds `S` through cross-location covariance. Supplying `C(0)` as local covariance and then adding `S` again is invalid.

The validation report separately exposes the shift in mean bias profile. A covariance pass does not imply that the bias shift is acceptable for a particular engineering use.

## 5. What this still does not model

The current implementation is deliberately narrow:

- isotropic and stationary spatial covariance only;
- one global covariance component, not region-specific or nonstationary amplitudes;
- no temporal covariance;
- no anisotropic range fitting, despite UQ-005 being able to consume anisotropic scales;
- no negative-lobe covariance family;
- no raw image, correlation-plane, LPT-track, pressure, viscous traction, body-force, storage or direct thrust ingestion;
- no proof that reference evidence is independent, exchangeable, or physically adequate;
- no deconvolution of reference measurement uncertainty: residual covariance contains both PIV and reference contributions unless an externally qualified method has already bounded or removed the reference contribution.

If validation rejects this model, the next research branch should compare empirical lag tables, Gaussian, Matérn 1/2 and 3/2, anisotropic and nonstationary models, plus oscillatory positive-definite kernels where a negative lobe is resolved. Do not expand the model family by relaxing the held-out gate.

## 6. Industry and state-of-the-art comparison

These are related but not interchangeable benchmark tracks.

### PIV processing and throughput

PIVtools (SoftwareX 2026, DOI 10.1016/j.softx.2026.102967) brings planar, stereoscopic and ensemble PIV workflows together, with distributed execution and direct Reynolds-stress extraction from ensemble correlation. Its 2026 results report a 1.6% mean relative streamwise-stress error over a stated synthetic channel-flow range, experimental mean velocity within 1.5% of hot-wire data down to approximately y+ = 40, 117 four-megapixel pairs/s at 128 cores, and 77% parallel efficiency at that point. A separate preprint describes a 10.5× matched-setting comparison against commercial software on a 20-core workstation. These are configuration-specific PIV-processing results, not covariance-calibration results.

UQ-006 does not compete with PIVtools on image throughput. The useful comparison is whether the full system can preserve or consume its PIV outputs while adding calibrated correlation, uncertainty propagation and decision-grade receipts at a measurable computational cost.

### Vector-validity post-processing

PIVPostProcess (Experiments in Fluids, DOI 10.1007/s00348-026-04319-7) reports classifier results including F1 around 0.992 and ROC-AUC around 0.998 on its declared evaluation cohort. That is a strong comparator for vector-validity classification. It does not by itself establish calibrated joint velocity covariance, whole-field coverage, or coverage of integrated momentum functionals. Keep the tasks and cohorts separate.

### Classical uncertainty propagation and industrial V&V

Sciacchitano & Wieneke (2016) remain the direct methodological comparator for spatially correlated error propagation into derived PIV quantities. UQ-006 should reproduce their qualitative lesson on new configurations rather than hard-code their reported correlation values.

For engineering validation, align the evidence argument with ASME V&V 20-2009 (R2021) and NISTIR 8298: state the quantity of interest, comparison error, experimental and numerical/input uncertainty, validation conditions, and the scope over which conclusions apply. A software fit receipt is not a V&V certification.

## 7. Benchmark programme and prospective targets

The benchmark must preserve independent acquisition/session/geometry/flow-family boundaries. Random vector splits are invalid for the primary claim.

| Lane | Comparator / dataset | Measurements | Prospective target (not achieved) |
|---|---|---|---|
| Spatial covariance | Empirical lag table; Gaussian; Matérn 1/2 and 3/2; Wendland C2 | Held-out max absolute correlation error by lag and flow region; negative-lobe detection; PSD; range stability | ≤0.05 absolute correlation discrepancy on the declared primary holdout; zero false qualification on manufactured negative-lobe controls |
| Whole-field uncertainty | Cluster-max conformal; hierarchical group conformal | Whole-acquisition coverage, interval/ellipsoid volume, sharpness, cluster-level confidence intervals | Nominal 95% whole-field coverage; report confidence interval and coverage by region, not a pooled vector count |
| Derived quantities | Sciacchitano–Wieneke propagation; independent reference functionals | Vorticity, divergence, mean/stress statistics, mass flow and momentum flux; coverage and normalized error | 95% functional coverage with no region/session subgroup hidden by pooling |
| Vector filtering | PIVPostProcess published cohort, with matched labels and polarity | F1 both classes, ROC-AUC, log loss, Brier, valid-vector removal, calibration | Reproduce source-reported scores on the exact matched cohort before claiming any superiority |
| End-to-end PIV processing | PIVtools and available commercial workflows | Same images, windows, hardware, core count, output definitions; throughput, memory, accuracy | Report matched benchmark; no unsupported speedup claim |
| Dataset generalization | FFS-S-PIV plus independently sourced experimental fields | Hold out complete flow families, geometries, sessions and reference sources | No leakage; report worst-slice performance and calibration failure, not only macro-average |
| EHD engineering closure | PIV plus genuinely independent LPT/probe and direct force-balance paths | Full mass/momentum terms, uncertainty closure, pressure/viscous/body-force/storage terms, observer perturbation | No propulsion/thrust claim until all required terms and independent force balance are complete |

The FFS-S-PIV public anchor (DOI 10.5281/zenodo.21732072) contains 12,466 five-frame 256×256 synthetic sequences, nine scenarios and known displacement truth. It is a useful displacement and generalization benchmark, but it is not a substitute for physical tracer response or EHD laboratory data.

For finite-sample 95% split-conformal field coverage, 19 independent calibration fields is the mathematical minimum for a finite rank; it is not a robust benchmark-size recommendation. Use substantially larger independent calibration and test cohorts where available, and publish uncertainty intervals on the observed coverage itself.

## 8. Paradigm-shift roadmap

1. **Measurement-to-decision twin:** link each raw asset, processing configuration, field residual, covariance model, uncertainty set, integrated quantity and decision receipt by immutable evidence identity.
2. **Experiment co-design:** choose seeding, pulse separation, window overlap, camera arrangement and reference diagnostics to maximize identifiability and reduce the uncertainty of the actual engineering quantity—not merely minimize vector RMS error.
3. **Active-observer accounting:** treat tracer, illumination, probes and LPT as possible interventions. Compare diagnostic-on/off states with randomized, drift-balanced schedules and record persistence after removal.
4. **Multi-path closure:** require independent routes to agree within combined uncertainty; explicitly model shared causes so that PIV/LPT agreement from one acquisition is not mistaken for independent validation.
5. **Model-falsification-first:** benchmark under deliberate negative lobes, nonstationary variance, pressure/tracer shifts, out-of-plane motion, missing regions, timing drift and geometry changes. A safe rejection is a valid result.
6. **Accelerated covariance operators:** after reference correctness, replace O(A*N²) fitting and O(N²) field propagation with compact neighbour lists or matrix-free operators. Preserve exact operator semantics and compare outputs against the reference implementation.

## References

- Sciacchitano, A. & Wieneke, B. (2016), *PIV uncertainty propagation*, Measurement Science and Technology 27, 084006. DOI: 10.1088/0957-0233/27/8/084006.
- Wendland, H. (1995), *Piecewise polynomial, positive definite and compactly supported radial functions of minimal degree*, Advances in Computational Mathematics 4, 389–396. DOI: 10.1007/BF02123482.
- Taylor, M., Lawson, J. & Ganapathisubramani, B. (2026), *PIVtools: A comprehensive open source software for particle image velocimetry*, SoftwareX. DOI: 10.1016/j.softx.2026.102967.
- PIVPostProcess (2026), *An open-source, physics-aware, data-driven tool for automated particle image velocimetry post-processing*, Experiments in Fluids. DOI: 10.1007/s00348-026-04319-7.
- Yeo, D. (2020), *A Summary of Industrial Verification, Validation, and Uncertainty Quantification Procedures in Computational Fluid Dynamics*, NISTIR 8298. DOI: 10.6028/NIST.IR.8298.
- ASME V&V 20-2009 (R2021), *Standard for Verification and Validation in Computational Fluid Dynamics and Heat Transfer*.
- FFS-S-PIV dataset, DOI: 10.5281/zenodo.21732072.
