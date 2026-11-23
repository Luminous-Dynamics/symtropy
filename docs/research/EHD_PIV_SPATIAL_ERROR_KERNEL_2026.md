# EHD-PIV-UQ-005 — calibrated spatial covariance for overlapping PIV windows

**Status:** standalone Rust module authored; 11 unit tests authored; local Rust execution pending; exact-head hosted CI pending.  
**Branch:** research/ehd-piv-spatial-error-kernel-v0-2  
**Parent:** EHD-PIV-UQ-004 cluster-conformal field and functional uncertainty

## Why this increment is necessary

Common latent modes represent global causes such as camera-scale calibration, pulse timing and surface registration. They do not represent the spatially correlated random errors created when neighboring PIV interrogation windows reuse much of the same image information.

Sciacchitano and Wieneke (2016), *PIV uncertainty propagation*, DOI 10.1088/0957-0233/27/8/084006, derive uncertainty propagation for vorticity, divergence, spatial means and Reynolds stresses. Their paper shows that spatial and inter-component measurement-error correlation changes the uncertainty of derived quantities. In a Monte Carlo example with 32×32 px Gaussian-weighted interrogation windows and 75% overlap, their measured error correlation is approximately 0.45 at two vector spacings and remains significant to about three spacings. They also state that correlation parameters depend on the PIV processing configuration and can be estimated a priori by Monte Carlo for that configuration.

That published value is not a universal parameter for our solver or hardware. The new implementation therefore does not hardcode 0.45 or infer a correlation length from overlap percentage alone.

## Covariance model

The added component is:

`Cov(\delta u_i,\delta u_j)=K(x_i,x_j)C_{component}`

where (K) is an anisotropic compact-support Wendland C2 kernel in three spatial dimensions:

`K(r) = (1-r)^4(4r+1), 0 <= r < 1; otherwise 0`

and (r) is the Euclidean distance after scaling each spatial axis by its fitted correlation length. Wendland C2 is positive definite in three dimensions. (C_{component}) is a symmetric positive-semidefinite 3×3 cross-component covariance.

For functional Jacobian (J_{i,a}), the additional covariance of integrated outputs is:

`C_spatial[a,b] = sum_i sum_j K(x_i,x_j) J[i,a] C_component J[j,b]^T`

This supplies the missing cross-sample terms to the existing local-plus-shared-mode field-functional covariance. Local per-sample covariance must exclude the component represented by this kernel to avoid double counting. Shared latent modes must describe distinct causes.

The reference implementation is (O(N^2)) in surface samples. Production requires spatial bins, compact neighbor lists or a matrix-free kernel operator without changing the mathematical contract.

## Calibration receipt and fail-closed conditions

A kernel is accepted only with:
- a non-empty parameter-calibration receipt identity;
- a frozen processing-protocol fingerprint that matches the target field;
- at least 20 independent acquisition clusters as an initial evidence floor;
- a maximum absolute empirical autocorrelation fit error no greater than the predeclared limit (initial prospective gate 0.05);
- no resolved negative correlation lobe for this specific model.

The nonnegative Wendland C2 family cannot represent the slight negative correlation undershoot reported for some PIV filter configurations. If a negative lobe is resolved, the system rejects this model and should evaluate an oscillatory positive-definite kernel, empirical covariance lookup or another justified covariance family. It must not force a nonnegative model onto incompatible data.

The receipt fields are assertions supplied by an external calibration pipeline/reviewer. This crate does not fit the kernel or cryptographically verify the receipt. A receipt ID alone is not evidence that an adequate empirical calibration was actually performed.

## Prospective benchmark

For each frozen PIV processing configuration (window size, overlap, number of passes, deformation, vector spacing, masks, tracer and imaging/noise regime):

1. Generate synthetic image sequences with known displacement truth or obtain repeated independent reference acquisitions.
2. Estimate error fields using independent reference velocity/displacement.
3. Estimate empirical spatial auto- and cross-correlation functions by region and component.
4. Fit competing models on calibration acquisitions: empirical lookup, Gaussian, Matérn, Wendland C2, and an oscillatory positive-definite alternative when negative lobes exist.
5. Freeze parameters and score them on whole held-out acquisition sessions and flow families.
6. Compare the predicted covariance function at every resolved lag and region; report correlation-function RMSE, maximum absolute fit error, likelihood/proper score and positive-definiteness.
7. Propagate each accepted model to vorticity, mass flow and momentum flux. Compare analytic covariance with Monte Carlo using the same frozen joint distribution and independent seeds.

Initial prospective gates:
- at least 20 independent calibration acquisitions for screening; final sample size set by power analysis;
- maximum absolute spatial autocorrelation error ≤0.05 across the declared lag range and all required regions;
- zero unresolved covariance PSD failures;
- held-out session coverage for derived functionals reported with whole-field cluster conformal calibration;
- no improvement accepted if it is obtained by inflating functional uncertainty without reporting sharpness/interval size;
- runtime, memory and energy measured on identical fields and hardware.

These are proposed targets, not measured results.

## Authored tests

Eleven Rust tests cover:
- neighboring-window correlation increases mass-functional covariance;
- no cross-sample covariance beyond compact support;
- anisotropic length scales change the covariance;
- inter-component covariance reaches cross-functional covariance;
- local independent covariance is retained and augmented, not replaced;
- missing calibration receipt and non-finite coordinates are rejected;
- insufficient independent acquisitions are rejected;
- empirical fit error above the predeclared bound is rejected;
- negative correlation lobes are rejected by this model;
- processing-protocol mismatch is rejected;
- invalid scales/non-finite fit thresholds and non-PSD covariance are rejected;
- total output covariance remains symmetric.

## Limitations and future extensions

This is a stationary anisotropic correlated-component model with a common (C_{component}). It does not yet model spatially varying correlated amplitude, nonstationarity across the field, temporal covariance, negative lobes, or coupling to particle-image formation. The correlation kernel must be fitted for each processing envelope rather than assumed universal.

No raw image field has been reprocessed. No correlation surface, autocorrelation function or kernel parameter has been estimated from laboratory data. Rust execution, formatting, compilation, Clippy, unit tests and hosted CI are pending.

## References

- Sciacchitano, A. & Wieneke, B. (2016). PIV uncertainty propagation. *Measurement Science and Technology* 27(8), 084006. DOI: 10.1088/0957-0233/27/8/084006.
- Wieneke, B. (2015). PIV uncertainty quantification from correlation statistics. *Measurement Science and Technology* 26, 074002. DOI: 10.1088/0957-0233/26/7/074002.
- Wendland, H. (1995). Piecewise polynomial, positive definite and compactly supported radial functions of minimal degree. *Advances in Computational Mathematics* 4, 389–396. DOI: 10.1007/BF02123482.
