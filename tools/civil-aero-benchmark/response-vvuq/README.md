# AER-01 response-vector VVUQ v0.1

**Tracking:** AERO-RESPONSE-VVUQ-001 / #4190  
**Related claim-admission contract:** #3413  
**Related structural/aeroelastic bridge:** #4093  
**Status:** synthetic covariance/diagnostic implementation; no NASA experimental data admitted.

## Purpose

This crate implements a disclosed multivariate response-discrepancy diagnostic for physically comparable output channels with correlated uncertainty. It fills the quantitative calculation gap left by #3413, which binds multivariate-validation evidence references into claim admission but does not calculate the metric itself.

ASME VVUQ 20.1-2024 is a methodology and benchmark reference. This crate is not a reimplementation of the proprietary standard and makes no conformance claim.

## Core mathematical output

For each channel, the implementation retains:
- physical residual in the channel's original units;
- the normalization scale and normalized residual;
- marginal standardized residual;
- combined standard deviation.
The result also retains the complete provenance receipt for any cross-source covariance contribution.

For the normalized residual vector r and combined covariance Sigma, it computes:

\[
D^2 = r^T \Sigma^{-1} r
\]

The result also reports an infinity-norm covariance-condition estimate and the minimum/maximum Cholesky diagonal pivots (not their squared values).

D² is a covariance-weighted discrepancy diagnostic. It is not automatically a chi-square statistic, probability, p-value, confidence level or validation theorem. A within-limit status is emitted only when the caller provides a predeclared D² limit, a maximum marginal-z limit, a protocol digest and a claim-scope digest matching the request. Without a limit the result is DIAGNOSTIC_ONLY.

Raw channel discrepancies remain visible; a global score cannot erase a local channel result.

## Uncertainty and provenance

The three mandatory uncertainty components are experimental/measurement, numerical/discretization and input-parameter propagated covariance. Each component carries:
- exact covariance channel IDs and matrix;
- source ID and artifact digest;
- estimator build digest;
- calibration dataset digest;
- calibration-envelope digest and per-channel maximum diagonal variance;
- exact state-identity digest;
- exact normalization-basis digest.

For declared-independent sources, the three component covariance matrices are added. If source dependence is declared, an explicit cross-source covariance contribution and source receipt are mandatory. In v0.1, the cross-source matrix represents the symmetric contribution Cov(A,B) + Cov(B,A), not one directional cross-covariance block by itself. This contribution can be indefinite; only the sum of all covariance and cross-covariance contributions must be positive definite for evaluation. An implicit zero cross-covariance is never substituted for unknown dependence. The output retains the cross-source artifact, estimator, calibration and source IDs.

Every covariance matrix is expressed in normalized response coordinates. The normalization basis is hashed from canonical channel ID, quantity, units, frame, sample coordinate and normalization scale. Channel arrays and covariance matrices may use different ordering; the evaluator canonicalizes by channel ID and reorders matrix rows/columns. A channel identity or scale change invalidates the source-bound basis digest.

Calibration dataset digests must differ from the held-out validation dataset digest. Model-form discrepancy is kept as a separate evidence reference and is never added to the covariance budget used to excuse the discrepancy being assessed.

The current schema checks the supplied provenance references and digests structurally. It does not authenticate the publisher, artifact, estimator binary or verifier identity against a cryptographic trust store; that remains in the #3428 trust-boundary work.

## Fail-closed numerical policy

- Covariance components must be symmetric and positive semidefinite.
- The combined covariance must be positive definite and below the locked condition-estimate limit.
- Singular/near-singular, asymmetric, indefinite, non-finite or dimensionally inconsistent inputs are rejected; there is no silent diagonal fallback or arbitrary regularization.
- Missing covariance sources are not replaced with zero.
- Calibration variance caps are checked channel by channel.
- The decision threshold must match the exact claim scope.
- No universal D² acceptance threshold is provided.

## Frozen known-answer fixture

The fixture is synthetic and includes three channels:
- integrated lift force;
- integrated pitching moment;
- sectional twist at eta=0.5546.

The channels use different physical units and reference scales. Three correlated covariance blocks represent experimental, numerical and input uncertainty. The held-out data, calibration datasets, estimator builds, uncertainty artifacts, state identity and normalization basis are all distinct digest-bound values.

Locked result:
- covariance-weighted D²: approximately 18.493035490698507;
- maximum marginal |z|: approximately 4.285714285714286;
- decision result under the synthetic fixture's declared limits: WITHIN_DECLARED_LIMIT.

This fixture verifies arithmetic and evidence contracts only; it is not an AIAA/NASA dataset.

## Local execution

From the repository root:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/response-vvuq/Cargo.toml -- --check
cargo test --manifest-path tools/civil-aero-benchmark/response-vvuq/Cargo.toml
cargo run --quiet --manifest-path tools/civil-aero-benchmark/response-vvuq/Cargo.toml > /tmp/response-vvuq-result.json
python3 tools/civil-aero-benchmark/response-vvuq/verify_independent.py --rust-output /tmp/response-vvuq-result.json
~~~

The independent Python standard-library oracle reconstructs the state and normalization hashes, reorders covariance matrices and independently evaluates D² and diagnostics.

## Claim ceiling

The positive output claim is multivariate_response_discrepancy_only. It is not a physical validation finding, proof of ASME VVUQ 20.1 conformance, a structural adequacy finding, an airworthiness claim or certification approval.

The next admission step is to use this contract on real AIAA CRM static-FSI response data only after exact data bytes, measurement uncertainties, units, station definitions, geometry revision and reference provenance are admitted. The rigid Configuration B and elastic 2a(E)/2b(E) protocols must remain separate.
