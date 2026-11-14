# Weak-IV Anderson–Rubin Confidence Set V1

A dependency-free Rust implementation of Anderson–Rubin test inversion for one endogenous source variable and one downstream outcome, with one or more instruments.

Parent issue: #3555 — LOCO-IV-INFERENCE-001  
Scalar point estimator: PR #3553  
Dynamic/time-series identification: #3547

## Why this exists

A first-stage F statistic is a relevance diagnostic, not a universal identification certificate. Conventional 2SLS normal/t confidence intervals can be unreliable when instruments are weak. A weak-IV-robust confidence set may be wide, unbounded, a union of two rays, the whole real line, or empty. V1 preserves these outcomes instead of forcing them into a finite interval.

## Method

For a candidate structural coefficient beta0, form the transformed outcome:

    w(beta0) = y - beta0 * x

Regress this transformed outcome on the intercept and the full instrument matrix. The Anderson–Rubin statistic is the classical joint F test that all instrument coefficients are zero. V1 inverts this test over beta.

For q instruments and n observations, the classical reference statistic is F(q, n-q-1). Under the classical finite-sample reference distribution, the errors must be independent, homoskedastic and Gaussian conditional on the instruments, and the instruments must satisfy the relevance / exogeneity / exclusion design assumptions. The caller must declare the statistical profile and provide a design-evidence reference. This is an auditable attestation, not proof that an instrument is valid.

The confidence-set boundary is represented by a quadratic inequality in beta. V1 preserves:
- empty set;
- all-real set;
- bounded closed interval;
- single point;
- lower or upper unbounded ray;
- two unbounded rays outside a finite interval.

No plotting-range truncation is performed.

## Numerical approach

- Instrument columns are centered and orthonormalized using twice-reorthogonalized modified Gram–Schmidt.
- Rank-deficient instruments are rejected; the code does not add a ridge term to manufacture identification.
- F-distribution CDF / quantiles are computed from the regularized incomplete beta function and a bounded bisection inversion.
- Confidence-set roots are found from the inverted AR quadratic, retaining unbounded set topology.
- The statistical reference scope is explicit. V1 does not claim heteroskedasticity-robust, cluster-robust or serial-correlation-robust finite-sample coverage.

## Evidence boundary

The estimator does not detect exclusion violations automatically. A caller-supplied invalid/unknown instrument audit blocks causal qualification but the numerical statistic may still be returned as a diagnostic. A non-rejected AR null does not prove that an instrument is valid.

All fixtures are synthetic. No animal data or Drosophila VNC connectome data are ingested.

Do not use this i.i.d. estimator on adjacent neural time points as if they were independent. Temporal dependence/history correction belongs to the separate #3547 track.

## Run

    cargo fmt --manifest-path research/weak-iv-ar-confidence-set-v1/Cargo.toml -- --check
    cargo test --manifest-path research/weak-iv-ar-confidence-set-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/weak-iv-ar-confidence-set-v1/Cargo.toml --locked --all-targets -- -D warnings

## Benchmarks for promotion

Before claiming inferential superiority, compare nominal 90%, 95% and 99% coverage over strong, medium, weak and irrelevant instrument strata; report bounded-set diameter and the rate of unbounded/empty/all-real outcomes separately. A narrow interval with undercoverage is a failure. Overidentification tests and HAC / cluster-robust inference are not included in this V1.
