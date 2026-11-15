# AFEX nonlinear root-turnover uncertainty benchmark v0.1

**Issue:** [#5436 — AFEX-ROOT-DISTRIBUTION-UQ-001](https://github.com/Luminous-Dynamics/symtropy/issues/5436)  
**Root process owner:** [#5431](https://github.com/Luminous-Dynamics/symtropy/issues/5431)  
**First-order covariance engine:** [#4652 / PR #4654](https://github.com/Luminous-Dynamics/symtropy/pull/4654)  
**Nonlinear distribution engine:** [#3973 — MCM-UQ-001](https://github.com/Luminous-Dynamics/symtropy/issues/3973)  
**Date:** 15 November 2026  
**Status:** synthetic analytic ratio oracle; no Monte Carlo execution or AFEX empirical uncertainty estimate

## 1. Why this is distinct from point-ratio validation

AFEX fine-root turnover is annual productivity divided by standing stock. The numerator and denominator are measured through different but potentially dependent sampling processes. The resulting ratio is nonlinear. Separate standard errors or a single first-order Jacobian may be inadequate when distributions are skewed or denominator uncertainty is large.

This benchmark is a forest-specific consumer only. It must reuse:
- #4652 for first-order covariance propagation via a declared Jacobian;
- #3973 for nonlinear distribution propagation through a joint Gaussian-copula model.

It does not add another covariance or Monte Carlo engine.

## 2. Exact synthetic oracle

Let (P) be annual root productivity and (S) be root standing stock. Assume:

`log(P) ~ Normal(mu_P, sigma_P^2)`

`log(S) ~ Normal(mu_S, sigma_S^2)`

with `Corr(log(P), log(S)) = rho`.

Then:

`T = P/S`

is lognormal with:

`mu_T = mu_P - mu_S`

`variance(log T) = sigma_P^2 + sigma_S^2 - 2 rho sigma_P sigma_S`.

Lifespan (L=365.25/T) is also lognormal. This produces an analytic oracle for output means, standard deviations, medians and quantiles. Monte Carlo samples can then be evaluated against known answers instead of only testing deterministic replay.

## 3. Frozen synthetic parameterization

The benchmark uses normalized reference coordinates:
- productivity median 1.2, log-space standard deviation 0.15;
- stock median 1.0, log-space standard deviation 0.10;
- Gaussian-copula/log-space correlations +0.6, 0 and −0.6.

The marginals remain identical across all cases. Only their dependence changes.

| Log-space correlation | Turnover median | Turnover mean | Turnover SD | Central 95% interval |
|---:|---:|---:|---:|---:|
| +0.6 | 1.200 | 1.209 | 0.146 | 0.948–1.519 |
| 0 | 1.200 | 1.220 | 0.222 | 0.843–1.709 |
| −0.6 | 1.200 | 1.231 | 0.280 | 0.772–1.864 |

The same joint draws yield lifespan:
| Log-space correlation | Median days | Mean days | SD days | Central 95% interval |
|---:|---:|---:|---:|---:|
| +0.6 | 304.4 | 306.6 | 37.1 | 240.4–385.4 |
| 0 | 304.4 | 309.4 | 56.2 | 213.8–433.4 |
| −0.6 | 304.4 | 312.2 | 71.0 | 195.9–472.8 |

This demonstrates why the independence assumption is not harmless: the median remains fixed, while interval width and mean shift substantially.

All values are exact analytic calculations up to floating-point rounding for a synthetic lognormal model. They are not inferred from AFEX's reported treatment means/SEs and are not a claim about its true covariance.

## 4. Comparison tournament

1. First-order delta/Jacobian covariance using #4652.
2. Joint-distribution Monte Carlo using #3973 with correlated lognormal marginals.
3. Exact lognormal ratio oracle above.
4. Explicit dependence ablations (+0.6, 0, −0.6).
5. Same-sample joint output for turnover and lifespan, preserving their inverse dependence.

A fixed Monte Carlo trial count is not enough. Use #3973 batch convergence, order-statistic quantile confidence and invalid-sample diagnostics. Report the Monte Carlo numerical error separately from the physical measurement uncertainty.

## 5. Executable forest analytic oracle

The exact lognormal formulas have a small Rust known-answer verifier:
- `validator/src/afex_root_distribution_kernel.rs`;
- integrated under corpus ID `AFEX-ROOT-DISTRIBUTION-UQ-001` in the forest validator dispatcher;
- CLI result: `PASS_SYNTHETIC_AFEX_ROOT_DISTRIBUTION_ORACLE`;
- five authored Rust unit tests plus one top-level dispatcher/digest test.

Run from the repository root:

```bash
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/afex-root-ratio-distribution-analytic-v0.1.json

cargo test --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml
cargo fmt --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- --check
cargo clippy --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml --all-targets -- -D warnings
```

The verifier checks 36 frozen fields across the three scenarios and binds the exact fixture bytes in a SHA-256 receipt. It also rejects changed marginal parameters, changed 95% quantile critical values, a false empirical-marginal/covariance admission claim, or a claim that Monte Carlo has run.

These Rust commands remain unexecuted in the current runtime because Cargo, rustc, rustfmt and Nix are unavailable. Independent JavaScript formula checks are a separate cross-check, not a substitute for those gates.

## 6. Positive-denominator and nonlinearity boundary

The lognormal reference fixture is strictly positive. Real root-stock measurements must have an admitted positive-support distribution and provenance. If the denominator distribution is not positive or has material support near zero, the ratio can become unstable or heavy-tailed. The engine must preserve invalid-domain samples and return its declared invalid-sample or not-converged disposition; no clipping, zero-fill or silent resampling.

The delta method is a local approximation. If denominator coefficient of variation, skew or tail behaviour makes the approximation inadequate, prefer the joint Monte Carlo distribution. For normal numerator/denominator inputs whose denominator uncertainty crosses zero, a Fieller-type set may be unbounded; do not force it into a finite symmetric interval.

## 7. Unit and scale contract

Productivity (e.g. Mg C ha⁻¹ yr⁻¹) and stock (e.g. Mg C ha⁻¹) do not share a physical unit. The v0.1 #4652 state requires a common canonical input coordinate. Therefore each raw physical input must first pass a separate source/scale transform to normalized dimensionless coordinates; their uncertainty and covariance must be transformed with the same registered scale receipts. The physical turnover scale (yr⁻¹) and lifespan conversion must be restored as explicit output transforms. A shared label is not proof of dimensional equivalence.

## 8. Empirical gate

The AFEX 2026 paper's turnover denominator stock data are available from the corresponding author on request; the public EIDC resource supplies the productivity input. Published aggregate means/SEs do not identify the plot-level numerator/denominator covariance. No AFEX marginal distribution or dependence parameter may be estimated from those summaries alone.

Empirical use requires:
- admitted raw paired plot-level productivity and stock observations;
- exact plot/block/depth/time support;
- measurement and sampling distribution models;
- source-backed covariance or dependence assumptions;
- independent marginal and joint predictive checks;
- model discrepancy and structural inadequacy kept separate from numeric covariance.

## 9. Claim ceiling

This fixture is a synthetic analytic ratio-of-correlated-lognormal known-answer corpus. It does not execute #3973, reproduce AFEX root-turnover uncertainty, estimate field covariance, establish treatment effects or qualify any forest model.
