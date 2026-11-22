# EHD-OED-003 — correlated metrology covariance and two-channel nuisance design

**Status:** manufactured Rust reference and tests authored; Rust execution and exact-head CI pending  
**Parent:** EHD-OED-002 / robust nuisance-adjusted design  
**Tracking:** EHD-OED-001 / #6660

## Why this increment

The previous EHD-OED-002 selector supports one scalar nuisance channel and diagonal observation covariance. That is too narrow for real engineering metrology:

- A force balance can have a constant zero offset and a time-dependent drift.
- Shared calibration creates correlated errors across observations.
- Temperature, cable routing, vibration and cross-axis coupling can create additional correlated measurement channels.
- Physical model changes can alter sensitivities between operating regimes.

The new reference adds two nuisance channels (manufactured offset and drift), full 4×4 correlated covariance, and a second physical-response scenario. It retains the same fail-closed design-selection philosophy.

## Mathematics

For four observations, two physical parameters and two nuisance parameters:

\[
y=X\theta+Z\eta+\epsilon,\qquad \epsilon\sim\mathcal N(0,\Sigma).
\]

The physical information after nuisance profiling is

\[
F_{\mathrm{eff}}
=X^\mathsf T\Sigma^{-1}X-
X^\mathsf T\Sigma^{-1}Z
(Z^\mathsf T\Sigma^{-1}Z)^{-1}
Z^\mathsf T\Sigma^{-1}X.
\]

The implementation does not explicitly form \(\Sigma^{-1}\). It computes the Cholesky factor \(\Sigma=LL^\mathsf T\), whitens each sensitivity column by solving \(L\tilde x=x\), and projects the whitened physical sensitivities onto the orthogonal complement of the two whitened nuisance columns.

The nuisance basis uses two-pass modified Gram–Schmidt. A rank-deficient nuisance block makes the allocation inadmissible. A rank-deficient physical block is also excluded. The implementation adds no jitter, no diagonal loading, no symmetrization and no pseudo-inverse to turn invalid input into an apparent pass.

For frozen scenarios \(\mathcal S\), the design objective is

\[
d^\star=\arg\max_{d\in\mathcal D_4}
\min_{s\in\mathcal S}\lambda_{\min}(F_{\mathrm{eff}}(d,s)),
\]

subject to nuisance rank 2 and physical rank 2 in every scenario.

## Manufactured tournament

Six candidate conditions are indexed by \(x\in[-3,-2,-1,1,2,3]\). The two nuisance sensitivities are \([1,x]\), representing a shared offset and linear drift. The nominal physical sensitivity is \([x^2,x^3]\).

A second scenario changes the second physical sensitivity to \(x^3+0.25x^4\) and changes observation covariance to an AR(1) correlation matrix with \(\rho=0.45\):

\[
\Sigma =
\begin{bmatrix}
1&.45&.2025&.091125\\
.45&1&.45&.2025\\
.2025&.45&1&.45\\
.091125&.2025&.45&1
\end{bmatrix}.
\]

There are \(\binom{6+4-1}{4}=126\) four-observation allocations with replacement. Forty-nine remain full rank after nuisance projection in both scenarios.

| Result | Selection | Worst-case weakest information |
|---|---|---:|
| Nominal-only design | [0, 2, 3, 5] | 27.647171 when evaluated in shifted/correlated scenario |
| Correlated robust maximin | **[0, 2, 4, 5]** | **42.546743** |

For the robust design, the scenario-specific minimum information eigenvalues are 42.546743 (nominal/independent) and 43.295841 (shifted/correlated). The robust design therefore improves the worst-case weakest-direction information by approximately **53.89%** over the nominal-only design evaluated under the shifted/correlated scenario.

These values are manufactured linear-algebra results. They are not EHD operating conditions or physical experiment recommendations.

## Adversarial cases

- Asymmetric covariance is rejected; it is not silently symmetrized.
- Non-finite covariance is rejected.
- Non-positive-definite covariance is rejected; no jitter is added.
- Rank-deficient offset/drift nuisance basis excludes the allocation.
- Rank-deficient residual physical sensitivities exclude the allocation.
- Unsupported observation budgets are rejected.
- All allocations must use the same four-slot covariance boundary and candidate identity ordering.

## Limitations

This is a bounded reference: two physical parameters, two nuisance parameters, four observation slots, six manufactured candidate settings and two frozen scenarios. It does not yet implement arbitrary observation counts, arbitrary covariance dimensions, profile likelihood, parameter-uncertainty propagation, model-discrimination metrics, sequential experiment design or an EHD-specific instrument model.

The next step is a general small-matrix QR/SVD implementation for arbitrary declared nuisance rank and covariance dimension, with independent numerical-oracle comparisons and versioned calibration covariance receipts. Physical instrument covariance must come from calibration evidence; the manufactured AR(1) matrix is not a substitute.

## Industry positioning

This is complementary to ASME VVUQ 20.1's multivariate simulation/experiment validation metric and to the uncertainty frameworks of NISTIR 8298 and ITTC CFD procedures. It is not a replacement for those standards, a thrust-stand calibration standard, or a validated uncertainty budget.

The distinctive Luminous contribution is to make shared calibration covariance and nuisance confounding affect experiment selection before a physical recommendation is emitted.
