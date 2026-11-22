# EHD-PIV-UQ-004 — cluster-conformal field coverage and physical-functional uncertainty

**Status:** standalone Rust reference authored; 41 unit tests authored; Rust execution pending; exact-head hosted CI pending.  
**Branch:** research/ehd-piv-conformal-functional-uq-v0-1  
**Parent:** EHD-PIV-UQ-003 covariance calibration

## Research motivation

EHD-PIV-UQ-003 evaluates predicted two-component covariance by regional marginal and joint coverage, but its cluster-level confidence interval is a Student-t approximation over per-cluster coverage fractions. A stronger complementary construction is to calibrate the maximum nonconformity score across each entire acquisition field.

For vector residual e and supplied covariance Σ, use:

`s_i = e_i^T Σ_i^{-1} e_i`

For acquisition cluster g, define:

`S_g = max_i(s_i), over every vector in the frozen field and all four regions`

This is not the only group-conformal construction. Dunn, Wasserman and Ramdas (2023), *Distribution-Free Prediction Sets for Two-Layer Hierarchical Models*, develop CDF pooling and single/repeated subsampling for a new observation in a new group, with their repeated-subsampling approach recommended when finite-sample guarantees are desired. EHD-PIV-UQ-004 uses a deliberately different estimand: simultaneous coverage of every vector in one complete future acquisition field. It therefore scores each group by its maximum vector nonconformity and applies ordinary split conformal to exchangeable *whole fields*. It should be compared against the hierarchical CDF-pooling and repeated-subsampling methods rather than presented as a replacement for them.

For n exchangeable calibration fields and nominal cluster coverage 1-alpha, the split-conformal rank is:

`k = ceil((n+1)(1-alpha))`

The calibrated squared radius is the kth sorted cluster maximum. If k>n, the exact finite-sample rule is an unbounded prediction set; the implementation emits no finite radius. At 95% nominal coverage, n=19 is the minimum for a finite radius. That is a mathematical rank condition, not a claim that nineteen fields are adequate for every engineering risk.

Under exchangeability of complete calibration fields and the future field under the same protocol, the resulting ellipsoid provides finite-sample marginal coverage for the *entire target field*: every scored vector across bulk, near-wall, shear layer and wake is covered simultaneously with probability at least 1-alpha over calibration and target fields. It is not conditional coverage given a fixed calibration sample. A matching metadata fingerprint is necessary but cannot prove exchangeability. Changed flow families, geometries, tracers, acquisition chains, time regimes, masks or field sizes may invalidate the guarantee.

This is deliberately more conservative and more engineering-relevant than treating thousands of correlated interrogation windows as independent calibration observations. It must be benchmarked against pointwise split conformal, region-wise conformal, Gaussian covariance calibration and cluster-max conformal on exactly the same held-out acquisition fields.

## Dimension-matched 3D conformal path

The first conformal scorer is explicitly planar (2D). To avoid applying a planar radius to an unmeasured out-of-plane component, this increment now also adds `conformal_3d.rs`, using a full positive-definite 3×3 covariance and three-component Mahalanobis residual. It uses the same field-maximum score, fixed protocol/field-size contract and exact split-conformal rank. Both paths are separate typed APIs; the 2D radius cannot be passed off as 3D. The functional reducer's local covariance is also 3D, so a field-functional multiplier can be sourced from the 3D scorer when the protocol fingerprints, vectors and covariance model truly match.

## Spatially correlated covariance to physical functionals

The existing control-volume reducer correctly refuses to treat missing pressure, viscous, body-force, storage, advective or direct-balance terms as zero. Its uncertainty treatment currently accepts componentwise absolute bounds and combines them conservatively. EHD-PIV-UQ-004 supplies an intermediate field-level reducer that preserves shared spatial covariance before any force-balance reconciliation.

For each face sample i, with velocity u_i, density rho_i and oriented surface-area vector dA_i:

`mass_flux_i = rho_i (u_i · dA_i)`

`momentum_flux_i = rho_i u_i (u_i · dA_i)`

The Jacobians propagate local covariance and shared nuisance modes into the full 4×4 covariance matrix of [mass flow, momentum-x flux, momentum-y flux, momentum-z flux]. Shared modes can encode correlated one-sigma perturbations from camera scale, pulse timing, density calibration, area/registration and other common causes. The full functional covariance retains both cross-sample terms and cross-covariance between mass and momentum.

The compact factor representation is:

`C_field = blockdiag(C_local_1,...,C_local_N) + sum_k g_k g_k^T`

where each g_k concatenates the per-sample one-sigma sensitivities of one shared latent source. The mode sources are assumed independent after factorization; correlated sources must be mapped to independent modes through an externally validated covariance factorization. Any residual spatial dependence not represented by local covariance and declared modes is omitted.

This is a first-order delta-method calculation. For strong nonlinearities or non-Gaussian inputs, the next comparison should be a Monte Carlo / polynomial-chaos reference using the same frozen joint distribution, with conservation-preserving sampling and independent random seeds.

## Integrated-functional calibration against independent references

The field-functional reducer produces the 4D predictive covariance of mass flow and advective momentum flux. That covariance is still a model output until it is checked against independent functional references. The new `functional_conformal.rs` module takes reference-minus-predicted residuals for the same four-component observable, with a full 4×4 predictive covariance and unique reference evidence identity. It calibrates the maximum 4D Mahalanobis score over all functional observations within each independent laboratory session.

The result is a session-level prediction region for integrated functionals, not a thrust certificate. The reference measurements must be independent and traceable, and the functional contract must freeze surface, sign convention, units, density/reference conditions and estimator definition. The software standardizes the covariance to a dimensionless correlation matrix before Cholesky scoring so mixed physical units (kg/s and N) do not dominate numerical pivot tests.

The calibrated functional radius can only qualify the exact integrated functional definition represented by the observations. It cannot fill missing control-volume pressure, viscous, body-force or momentum-storage terms, and cannot override a failed direct force-balance reconciliation.

### Spatially correlated PIV vector errors

The shared-latent-mode representation above captures global causes such as common calibration scale, timing and surface registration. It does not by itself capture local spatial dependence caused by overlapping interrogation windows. Sciacchitano and Wieneke (2016) derive uncertainty propagation for vorticity, divergence, means and Reynolds stresses, and show that spatial error correlation is needed. Their Monte Carlo case for 32×32 px Gaussian-weighted windows with 75% overlap reports correlation around 0.45 at two vector spacings, with significant correlation out to roughly three spacings. They emphasize that correlation parameters depend on the PIV processing configuration and may be estimated with Monte Carlo for that configuration. The published value is not a universal kernel parameter.

The new `spatial_correlation.rs` path adds a compact-support anisotropic Wendland C2 component:

`Cov(delta_u_i, delta_u_j) = K(x_i,x_j) C_component`

where `K` is positive definite in 3D and the component covariance is symmetric positive semidefinite. It propagates the complete i,j cross-sample terms through the same mass/momentum Jacobians as the field-functional reducer. It is explicitly an additive component: local per-sample covariance must exclude the same correlated component to avoid double counting.

The kernel has a deliberately strict external calibration receipt: at least 20 independent acquisition clusters, a frozen processing-protocol fingerprint matching the target run, a bounded empirical correlation-fit error, and no resolved negative correlation lobe. The current nonnegative Wendland family cannot represent the slight negative undershoot that some PIV processing filters produce. When such a lobe is observed, the correct result is to reject this model and compare an alternative positive-definite oscillatory/spectral kernel or an empirical covariance model, not force a fit.

The reference implementation is O(N²) in the number of face samples. Production use needs a compact neighbor list or matrix-free sparse operator while preserving exact covariance accumulation. The current model is stationary and uses a common correlated-component covariance; spatially varying amplitude, flow-dependent anisotropy and temporal correlation remain future extensions.

## Existing literature and why this matters

Sciacchitano & Wieneke (2016), *PIV uncertainty propagation*, DOI 10.1088/0957-0233/27/8/084006, explicitly derive propagation to vorticity, divergence, means and Reynolds stresses. They show that spatial correlation of displacement errors matters for derivatives and integrated/statistical quantities, and report experimental assessments with roughly 5–10% differences for vorticity uncertainty under their tested setup and within 5% / 10% for selected time-averaged quantities / Reynolds stresses. This is not a gap that the Luminous work invented; our opportunity is to carry that logic into complete field functionals and EHD force/mass/momentum boundaries with traceable shared modes.

A 2026 grouped-validation/conformal study by Allohibi (Mathematics 14(21), 4055; DOI 10.3390/math14214055) compares pooled, equal-cluster and subsampling calibration on 1,565 outcomes from 144 economies. At 90% nominal coverage, pooled country-weighted coverage is 0.794, equal-economy calibration 0.875, and subsampling 0.937, with increasing interval width. This illustrates why vector-weighted coverage alone can be misleading for clustered data; it does not validate a PIV-specific method.

Noonan (2026), *Effective Sample Size for Conformal Calibration under Shared Ancestry*, Zenodo DOI 10.5281/zenodo.21626229, further emphasizes that within-cluster dependence reduces effective calibration information and that the relevant intracluster correlation can depend on the target coverage level. It is a preprint and its asymptotic model should not be transplanted directly to laboratory fields without checking assumptions.

## Prospective benchmark lanes

| Lane | Comparator | Primary target |
|---|---|---|
| Pointwise conformal | ordinary split conformal on vector scores | pointwise coverage and mean ellipsoid area |
| Region-wise conformal | separate calibration by bulk/wall/shear/wake | worst-region coverage and width |
| Cluster-max conformal | this implementation | ≥95% whole-field coverage on exchangeable future fields |
| Hierarchical group conformal | Dunn–Wasserman–Ramdas CDF pooling / repeated subsampling | new-group pointwise coverage and set size; compare with field-max estimand separately |
| Gaussian covariance | UQ-003 Hessian + empirical scaling | joint coverage, NLL and sharpness |
| Field functional uncertainty | finite-difference / Monte Carlo reference | mass/momentum covariance relative error |
| EHD force reconciliation | existing control-volume + direct balance | residual within complete combined uncertainty |

For the cluster-max benchmark, report the number of independent calibration fields, the exact field size and masks, nominal alpha, rank, radius, realized whole-field coverage, region-specific failure frequency, ellipse area, and abstention rate. The 95% guarantee requires matching cluster exchangeability; it does not imply distribution-shift robustness.

For field-functional propagation, freeze a deterministic face-sample fixture and compare the analytic Jacobian covariance against central finite differences and Monte Carlo under the identical local-plus-shared factor model. Proposed initial numerical target: relative covariance error ≤1e-6 against the Jacobian reference for well-conditioned fixtures; Monte Carlo comparisons should use confidence intervals and sample-size convergence, not a hard pointwise tolerance.

## Authored tests

The planar conformal kernel includes 10 tests:
- exact rank at nineteen clusters for 95% coverage;
- eighteen clusters correctly produce an unbounded set;
- max score taken over all vectors and regions;
- protocol mismatch, changed field size, missing region;
- duplicate cluster/sample identity;
- invalid covariance and invalid alpha/attestation.

The 3D conformal kernel includes 4 tests for 19-field finite rank, correlated covariance, non-positive-definite covariance and field-size mismatch.

The field-functional kernel includes 10 tests, including central finite-difference checks for all mass and three-component momentum Jacobian entries on an oblique face:
- manufactured mass and momentum Jacobians;
- shared calibration mode cross-sample covariance;
- demonstration that shared uncertainty does not average away;
- shared density and area-mode propagation;
- covariance multiplier propagation;
- rejection of non-PSD local covariance, malformed shared modes and duplicate mode identities.

The integrated-functional conformal kernel adds 8 tests for:
- exact 95% rank at nineteen independent sessions and unbounded result at eighteen;
- 4D Cholesky Mahalanobis scoring with cross-functional covariance;
- standardization of mixed mass-flow / force units before Cholesky;
- rejection of nonsymmetric or singular covariance;
- protocol and functional-contract identity mismatch;
- duplicate independent-reference evidence.

The spatial-correlation kernel adds 9 authored tests for neighboring-window covariance, compact-support cutoff, anisotropy, cross-component/cross-functional propagation, preservation of local covariance, calibration-receipt and protocol gates, poor fit/negative-lobe rejection, and non-PSD component covariance.

## Limitations and pending work

This implementation is a mathematical reference only. It has not run under Rust locally; formatting, compilation, Clippy, unit tests and exact-head hosted CI are pending. No raw PIV field, PIVtools output, experimental data or force-balance measurement has been processed.

The conformal guarantee is only valid under exchangeability at the complete-field cluster level and a fixed protocol/field-size contract. The software cannot certify that assumption. The functional uncertainty is first-order and only as complete as the local covariances and shared-mode model supplied. Pressure, viscous/body-force and storage uncertainties remain in the existing control-volume assurance layer.
