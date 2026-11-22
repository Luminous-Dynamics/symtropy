# EHD-PIV-UQ-004 — cluster-conformal field coverage and physical-functional uncertainty

**Status:** standalone Rust reference authored; 19 unit tests authored; Rust execution pending; exact-head hosted CI pending.  
**Branch:** research/ehd-piv-conformal-functional-uq-v0-1  
**Parent:** EHD-PIV-UQ-003 covariance calibration

## Research motivation

EHD-PIV-UQ-003 evaluates predicted two-component covariance by regional marginal and joint coverage, but its cluster-level confidence interval is a Student-t approximation over per-cluster coverage fractions. A stronger complementary construction is to calibrate the maximum nonconformity score across each entire acquisition field.

For vector residual e and supplied covariance Σ, use:

`s_i = e_i^T Σ_i^{-1} e_i`

For acquisition cluster g, define:

`S_g = max_i(s_i), over every vector in the frozen field and all four regions`

For n exchangeable calibration fields and nominal cluster coverage 1-alpha, the split-conformal rank is:

`k = ceil((n+1)(1-alpha))`

The calibrated squared radius is the kth sorted cluster maximum. If k>n, the exact finite-sample rule is an unbounded prediction set; the implementation emits no finite radius. At 95% nominal coverage, n=19 is the minimum for a finite radius. That is a mathematical rank condition, not a claim that nineteen fields are adequate for every engineering risk.

Under exchangeability of complete calibration fields and the future field under the same protocol, the resulting ellipsoid provides finite-sample marginal coverage for the *entire target field*: every scored vector across bulk, near-wall, shear layer and wake is covered simultaneously with probability at least 1-alpha over calibration and target fields. It is not conditional coverage given a fixed calibration sample. A matching metadata fingerprint is necessary but cannot prove exchangeability. Changed flow families, geometries, tracers, acquisition chains, time regimes, masks or field sizes may invalidate the guarantee.

This is deliberately more conservative and more engineering-relevant than treating thousands of correlated interrogation windows as independent calibration observations. It must be benchmarked against pointwise split conformal, region-wise conformal, Gaussian covariance calibration and cluster-max conformal on exactly the same held-out acquisition fields.

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
| Gaussian covariance | UQ-003 Hessian + empirical scaling | joint coverage, NLL and sharpness |
| Field functional uncertainty | finite-difference / Monte Carlo reference | mass/momentum covariance relative error |
| EHD force reconciliation | existing control-volume + direct balance | residual within complete combined uncertainty |

For the cluster-max benchmark, report the number of independent calibration fields, the exact field size and masks, nominal alpha, rank, radius, realized whole-field coverage, region-specific failure frequency, ellipse area, and abstention rate. The 95% guarantee requires matching cluster exchangeability; it does not imply distribution-shift robustness.

For field-functional propagation, freeze a deterministic face-sample fixture and compare the analytic Jacobian covariance against central finite differences and Monte Carlo under the identical local-plus-shared factor model. Proposed initial numerical target: relative covariance error ≤1e-6 against the Jacobian reference for well-conditioned fixtures; Monte Carlo comparisons should use confidence intervals and sample-size convergence, not a hard pointwise tolerance.

## Authored tests

The conformal kernel includes 10 tests:
- exact rank at nineteen clusters for 95% coverage;
- eighteen clusters correctly produce an unbounded set;
- max score taken over all vectors and regions;
- protocol mismatch, changed field size, missing region;
- duplicate cluster/sample identity;
- invalid covariance and invalid alpha/attestation.

The functional kernel includes 9 tests:
- manufactured mass and momentum Jacobians;
- shared calibration mode cross-sample covariance;
- demonstration that shared uncertainty does not average away;
- shared density and area-mode propagation;
- covariance multiplier propagation;
- rejection of non-PSD local covariance, malformed shared modes and duplicate mode identities.

## Limitations and pending work

This implementation is a mathematical reference only. It has not run under Rust locally; formatting, compilation, Clippy, unit tests and exact-head hosted CI are pending. No raw PIV field, PIVtools output, experimental data or force-balance measurement has been processed.

The conformal guarantee is only valid under exchangeability at the complete-field cluster level and a fixed protocol/field-size contract. The software cannot certify that assumption. The functional uncertainty is first-order and only as complete as the local covariances and shared-mode model supplied. Pressure, viscous/body-force and storage uncertainties remain in the existing control-volume assurance layer.
