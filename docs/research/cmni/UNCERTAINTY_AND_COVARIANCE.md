# CMNI uncertainty and covariance model

**Status:** statistical design, v0  
**Tracking:** https://github.com/Luminous-Dynamics/symtropy/issues/6517

## 1. Why marginal error bars are not enough

The 2026 study defines relative thick-target yields against a common reference at E_ref = 6.5 keV (center-of-mass frame):

R_i = Y_i / Y_ref

In log space:

r_i = ln(Y_i) - ln(Y_ref)

If numerator yields are independent of each other and the same reference measurement is reused for all points:

Cov(r_i, r_j) = delta_ij Var(ln Y_i) + Var(ln Y_ref)

for non-reference points. The common reference creates positive off-diagonal covariance. Correlated beam-charge, detector, background, loading and sample-state errors add further covariance terms.

A vector of marginal sigma_lnR_i values is not sufficient to reconstruct this covariance unless the underlying error sources are independent or a specific decorrelation/cancellation is justified. The source paper mentions correlated beam-current and cathode-potential uncertainty, so the full analysis implementation and Supplementary Table 2 should be inspected before judging the reported fit.

## 2. Likelihood hierarchy

Retain three distinct levels:

1. **Poisson count-space model (preferred when raw counts are accessible):** model signal and background counts, live time, detector efficiency and charge dose with separate likelihood terms. Avoid Gaussian approximations where counts are low.
2. **Diagonal log-space weighted least squares:** reproduce the stated Eq. 17-style objective as a baseline, using marginal uncertainties.
3. **Correlated log-space generalized least squares:** use the full covariance matrix Sigma:
   chi2 = (ln R_obs - ln R_pred)^T Sigma^-1 (ln R_obs - ln R_pred).

Use a Cholesky solve; do not explicitly invert Sigma. Reject non-symmetric, non-finite, singular or non-positive-definite matrices.

## 3. Covariance construction

For independent numerator log-yield variances v_i and one shared reference variance v_ref:

Sigma = diag(v_1,...,v_n) + v_ref * 1 * 1^T

Further correlated contributions can be added as covariance matrices:
- beam current / delivered charge calibration;
- background subtraction shared across energy points;
- detector efficiency, solid angle and response matrix;
- energy calibration and beam energy spread;
- loading state / cathode potential;
- target preparation and sample-to-sample effects.

Do not claim that common systematics cancel without demonstrating that they enter the ratio identically. If the numerator and reference share correlated errors, include the covariance terms rather than assuming independence.

The reference measurement is a common denominator, not an additional independent normalized observation. Do not include R_ref=1 as another independent point in the fit.

## 4. Rust reference implementation

The isolated research kernel now includes:
- a common-reference log covariance constructor;
- a correlated log-space generalized least-squares score using Cholesky factorization;
- the earlier diagonal weighted objective as the baseline.

The implementation is intentionally generic: the caller supplies the full covariance, provenance and observation ordering. It does not infer correlations from unstructured marginal error bars.

## 5. Benchmark

Synthetic generator:
- draw numerator and reference counts;
- normalize all energy points by the same reference;
- inject shared beam-current and loading-state nuisance effects;
- fit with diagonal and full-covariance likelihoods;
- compare parameter bias, interval coverage, model selection and residual calibration.

Required metrics:
- parameter bias and RMSE;
- 50/90/95% interval coverage and sharpness;
- posterior predictive checks;
- false model preference under known generating models;
- sensitivity to shared-reference count level;
- sensitivity to correlated systematic amplitudes;
- robustness to background-subtraction uncertainty.

A covariance-aware model earns credit only if it improves calibration and parameter recovery on blinded synthetic data at matched information. It is not automatically better for every dataset.

## 6. Source boundary

The 2026 article states R_exp(E)=Y_exp(E)/Y_exp(6.5 keV) and gives a marginal relative-error formula in Eq. 19. The methods also describe beam-current and loading uncertainties as correlated across energies. The published text available in this audit does not fully specify whether the fit code used the full induced covariance. This is a reproducibility question, not a claim that the reported result is invalid.

Source: Karahadian et al. (2026), Methods, Eqs. 10 and 17–19:
https://pmc.ncbi.nlm.nih.gov/articles/PMC13500673/
