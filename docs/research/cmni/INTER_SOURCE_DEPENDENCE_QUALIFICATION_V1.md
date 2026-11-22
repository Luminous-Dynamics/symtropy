# CMNI inter-source dependence qualification v1

**Status:** typed research implementation authored on `research/cmni-source-dependence-v0`  
**Date:** 2026-11-22  
**Qualification:** authored; not yet Rust-formatted, compiled, or CI-qualified

## 1. Problem

The common-cause covariance assembler represents shared uncertainty components through named latent source blocks. For source (s), experiment (i) contributes an observation sensitivity (J_{i,s}), and the induced covariance is

[
C_{ij}^{(s)}=J_{i,s}C_sJ_{j,s}^{T}.
]

Repeated source IDs create the corresponding cross-experiment covariance. Within a source block, (C_s) may be fully multivariate and contain off-diagonal covariance between calibration components.

However, distinct source IDs are currently accumulated as separate additive blocks. That is mathematically equivalent to treating those latent blocks as mutually independent. Different identifiers alone do not establish independence.

Examples of potentially correlated but separately named sources include:
- detector efficiency calibration and response corrections derived from the same calibration fit;
- reference-spectrum normalization and evaluated nuclear-data inputs sharing measurements;
- background subtraction and dead-time corrections derived from the same acquisition stream;
- multiple reaction channels derived from one common evaluation fit.

## 2. Typed qualification

The new `InterSourceDependenceQualification` distinguishes:

- `JointBlocksModelled { receipt_id }`: dependence between latent components is represented inside one or more full joint source covariance blocks.
- `DistinctBlocksIndependenceAttested { receipt_id }`: distinct source blocks may be added independently because a documented attestation covers their dependencies.
- `Unknown`: cross-source relationships may be unmodeled.

Every qualified state requires a non-empty receipt identifier. Whitespace-only receipt IDs fail closed.

The receipt ID is portfolio-scoped: all experiment records passed to one joint covariance assembly must carry the same receipt ID. This prevents two individually complete but differently scoped records from accidentally being treated as a complete portfolio-level independence proof.

## 3. Joint covariance and score propagation

`JointExperimentCovariance` preserves:
- `lineage_coverage_complete`;
- `inter_source_dependence_complete`;
- sorted `inter_source_dependence_receipt_ids`;
- sorted shared source IDs and observation ranges.

A numeric joint covariance may still be returned when inter-source dependence is unknown. That matrix describes the covariance formed from the currently declared blocks; it is not evidence that omitted cross-source covariance is zero. The output remains explicitly unqualified.

The exact batch portfolio score carries the same qualification state and receipt identifiers. Strict batch mode excludes unknown inter-source dependence and counts those skipped portfolios separately from incomplete lineage and budget exclusions.

## 4. Analytical regressions added

1. A two-component joint calibration source with covariance

[
C_s=\begin{bmatrix}4&1.5\\1.5&9\end{bmatrix}
]

and two different experiment Jacobians must produce the analytically expected joint observation covariance, including the off-diagonal value 7.1875.

2. Two distinct source IDs with unknown inter-source dependence retain their known marginal covariance contributions but produce `inter_source_dependence_complete=false`.

3. Two individually qualified records carrying different receipt IDs do not establish a common portfolio-scope qualification. Their numeric covariance is retained but the joint result remains unqualified.

4. Empty receipt IDs fail closed.

5. Strict batch design excludes a candidate portfolio with unknown inter-source dependence and increments the dedicated skip counter.

Existing checks for source-ID covariance consistency, duplicated source IDs and incomplete lineage remain in place.

## 5. Important boundaries

This implementation does not discover correlations automatically, prove the scientific validity of an independence attestation, or infer a missing covariance from metadata. Receipt identifiers point to external evidence; they are not themselves cryptographic validation of the underlying scientific argument.

A complete evidence graph will still need versioned source manifests, covariance transfer models, uncertainty provenance, and independent review of any claimed independence. Correlated components should be bundled into the same joint latent block when their cross-covariance is known.

## 6. Benchmark expansion

The next benchmark should include:
- detector and reference calibrations derived from overlapping data;
- independent versus shared nuclear-data evaluation pathways;
- partially known cross-source correlation;
- one joint-block representation versus an intentionally split representation;
- missing or mismatched portfolio-scope receipts;
- posterior coverage and portfolio ranking changes under omitted cross-source covariance.

Metrics should include false independence rate, portfolio rank reversals, uncertainty undercoverage, abstention rate, and receipt completeness. No independence certificate should be issued solely because source IDs differ.

## 7. Proposed acceptance goals (not achieved)

- 100% rejection of empty or malformed qualification receipts.
- 100% explicit unqualification when source records disagree on portfolio-scope receipt.
- Zero strict-batch acceptance of unknown inter-source dependence.
- Exact analytical recovery for multivariate joint source blocks.
- No change to numerical covariance when qualification metadata alone changes.
- Separate reporting of source-lineage completeness and inter-source dependence qualification.

The typed implementation is a research increment, not a complete provenance graph or demonstrated physical independence model.


## 8. Quantitative cross-source misspecification negative control

The new fixture `research/cmni/fixtures/inter-source-correlation-undercoverage-v1.json` isolates a failure that source labels alone cannot detect.

Two scalar observations share a two-dimensional calibration latent variable with covariance

\[
C_u=\begin{bmatrix}4&3\\3&9\end{bmatrix},
\qquad \operatorname{Corr}(u_1,u_2)=0.5.
\]

Their source Jacobians are \(J_A=[1,0.5]\) and \(J_B=[0.25,1]\), with unit independent observation-noise variance and a unit-variance scalar prior. The correct joint-block representation gives

\[
R_{\mathrm{joint}}=
\begin{bmatrix}10.25&8.875\\8.875&11.75\end{bmatrix}.
\]

The deliberately incorrect split-independent representation keeps the same marginal source variances but drops their cross-covariance:

\[
R_{\mathrm{split}}=
\begin{bmatrix}7.25&5.5\\5.5&10.25\end{bmatrix}.
\]

With (H=[1,1]), exact Gaussian conditioning gives posterior variances (2667/2939\approx0.907452) under the joint model and (705/809\approx0.871446) under the split model. The split model understates posterior variance by exactly (28536/719201\approx3.97\%) relative to the correctly specified posterior.

This is a deliberately constructed arithmetic negative control, not a claim about actual detector correlations. It gives the benchmark a direct quantitative failure signal: an implementation that treats the split components as independent should reproduce the wrong covariance and the overconfident posterior, and therefore fail the oracle.

The next implementation step is to wire this fixture into a fixture-driven Rust regression, so the covariance assembler and its downstream posterior/design scorer are checked against the same external machine-readable expected values. Keep this distinct from verifying that an actual independence attestation is scientifically justified.


## 9. Bounded partial identification when cross-source dependence is unknown

The strict qualification path remains correct: distinct source identifiers do not imply independence, and an unknown dependence must not be silently replaced by zero. However, strict abstention alone discards useful information when the marginal variances and source sensitivities are known and a defensible interval for the cross-correlation is available.

The new research module research/cmni/reference-kernel/src/dependence_envelope.rs adds an exact small-case envelope for:
- one scalar Gaussian parameter with known prior variance;
- two scalar observations with strictly positive independent observation-noise variances;
- two shared scalar uncertainty sources with known marginal standard deviations;
- known source Jacobians for both observations;
- an evidence-backed closed interval for the unknown source correlation rho.

For the stacked observation covariance

\[
R(\rho)=
\begin{bmatrix}
a_0+a_1\rho & c_0+c_1\rho\\
c_0+c_1\rho & b_0+b_1\rho
\end{bmatrix},
\]

the scalar parameter information is

\[
I(\rho)=H^T R(\rho)^{-1}H
=\frac{n_0+n_1\rho}{d_0+d_1\rho+d_2\rho^2}.
\]

Consequently, the derivative numerator is quadratic:

\[
(n_1d_0-n_0d_1)-2n_0d_2\rho-n_1d_2\rho^2.
\]

The algorithm solves that quadratic, retains every real stationary point inside the admissible correlation interval, and evaluates those points plus both interval endpoints. This provides the global information extrema over the entire declared interval, not merely a grid approximation. The current Rust implementation uses ordinary f64 arithmetic rather than directed rounding or formal interval arithmetic, so its outputs are analytical-reference bounds, not machine-certified interval enclosures. It then maps the information bounds to posterior-variance and exact linear-Gaussian EIG bounds:

\[
P_1(\rho)=\left(P_0^{-1}+I(\rho)\right)^{-1},
\qquad
\operatorname{EIG}(\rho)=\tfrac12\log(1+P_0I(\rho)).
\]

### Exact interior-extremum fixture

The machine-readable fixture research/cmni/fixtures/interior-source-correlation-extremum-v1.json uses:
- P0=1;
- H=[2,1];
- independent observation noise variances [1,1];
- source standard deviations [1,1];
- source Jacobians J=[[1,1],[1,-1]];
- admissible correlation rho in [-0.75,0.75].

The resulting observation covariance is diagonal:

\[
R(\rho)=
\begin{bmatrix}
3+2\rho&0\\
0&3-2\rho
\end{bmatrix}
\]

and the information function is

\[
I(\rho)=\frac{15-6\rho}{9-4\rho^2}.
\]

The global information minimum is 1.5 at the interior point rho=0.5; the global maximum is 26/9 at the lower endpoint -0.75. The exact posterior variance envelope is [9/35,0.4], and the exact EIG interval is [0.5 log(2.5), 0.5 log(35/9)] nats.

The Rust tests check the interior extremum, a fixed-correlation interval (which collapses to one value), the full PSD-compatible interval [-1,1], invalid bounds/noise, zero source variance, and an independent dense-grid covariance assembly against the analytical envelope.

### Qualification boundary

This is an exact mathematical enclosure conditional on the input correlation interval. It does not prove that the true detector/calibration correlation lies inside that interval, and it does not turn a placeholder interval into an independence attestation. The bound itself can support a qualified decision only if its correlation bounds, marginal source variances, Jacobians, and model scope have independently reviewable evidence receipts.



This calculation also does not authenticate its input dependence interval. The caller must retain an evidence receipt for the correlation bounds, marginal source variances and source Jacobians. Until that receipt is independently checked, the result remains exploratory/unqualified and must not be promoted to a qualified decision interval.
Scope is intentionally limited to two observations, two shared scalar sources, and one scalar parameter. The next generalization should be a low-dimensional block-correlation uncertainty set with positive-semidefinite constraints and certified outer bounds, before attempting a general high-dimensional covariance-uncertainty solver.
