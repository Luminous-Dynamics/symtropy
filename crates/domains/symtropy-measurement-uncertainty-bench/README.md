# Symtropy Measurement Uncertainty Bench

Standalone deterministic Monte Carlo engine for propagating explicitly declared joint input distributions through nonlinear, dimension-checked measurement equations.

Implements [issue #3973](https://github.com/Luminous-Dynamics/symtropy/issues/3973) and complements #3961 measurement qualification, #3927 stage-resolved material transformation, #3814 full-cycle metrology and #3825 energy/resource accounting.

## Run

    cargo test --manifest-path crates/domains/symtropy-measurement-uncertainty-bench/Cargo.toml

## v0.1 input model

- NORMAL, LOGNORMAL, UNIFORM and TRIANGULAR marginal distributions.
- A declared Gaussian-copula latent correlation matrix, strict positive-definite Cholesky factorization, and source/assumption provenance.
- Deterministic SplitMix64 + Box-Muller PRNG; explicit seed and PRNG version.
- Seven-base-dimension SI algebra; every input and output declares a dimension vector and unit conversion.
- Reverse-Polish measurement equations with explicit stack validation.
- Multiple outputs share one joint input draw; the output covariance/correlation matrix is calculated from complete joint-valid draws.
- No clipping, zero filling or silent resampling of invalid samples.

## Output and numerical quality

The engine reports mean, sample standard deviation, standard error of mean, median, configured quantiles and a central coverage interval. For each reported quantile, an exact binomial order-statistic confidence interval estimates the Monte Carlo precision of that quantile estimator. This is distinct from the physical output's own coverage interval.

Batch-wise cumulative summaries track mean, standard deviation and quantile movement. A declared terminal-stability rule records the last batch transition that exceeded a drift threshold and requires a stable tail after it. This is a diagnostic, not a mathematical guarantee of convergence.

## Limits

- Gaussian copula only; it is an explicit dependence assumption, not an empirically inferred or universally correct model.
- Strict positive-definite matrices only; no silent jitter/repair.
- Deterministic reproducibility is scoped to the supported implementation/target; transcendental functions can differ across platform/libm implementations.
- No Bayesian inference, parameter fitting, stochastic physical simulator or adaptive experiment policy.
- This crate is not a full GUM/JCGM certification or an operational decision authority. Synthetic inputs remain synthetic.
