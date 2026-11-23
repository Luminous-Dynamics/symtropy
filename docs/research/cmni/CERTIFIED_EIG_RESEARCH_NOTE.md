# CMNI certified expected information gain — research note

**Status:** exact linear-Gaussian identity implemented on a research branch; nonlinear bounds are a proposed research programme  
**Reference:** Doumbouya, Vidard & Zahm (2026), arXiv:2609.32282  
**Branch:** `research/cmni-certified-eig-v0`

## 1. Immediate implementation: distinguish D-optimality from EIG

For a linear-Gaussian inverse problem,

[
y = H\theta + \epsilon,\qquad
\theta\sim\mathcal N(m_0,P_0),\qquad
\epsilon\sim\mathcal N(0,R),
]

with observation noise independent of the scientific parameter and positive-definite (R), the posterior covariance is

[
P_1=(P_0^{-1}+H^TR^{-1}H)^{-1}.
]

The Bayesian expected information gain in nats is exactly

[
\operatorname{EIG}
=\frac12\left[\log\det(P_0)-\log\det(P_1)\right].
]

The D-optimal log-determinant reduction is twice the EIG. They must not be labelled interchangeably.

CMNI's exhaustive small-set portfolio oracle now exposes both:
- `d_optimal_logdet_reduction`
- `linear_gaussian_expected_information_gain_nats`

The latter is one half of the former. Regression tests check both values against the common-cause A+B and complementary A+C analytical fixture.

## 2. Shared-cause treatment

For a stacked batch observation with shared uncertain sources (u_s),

[
y_i=H_i\theta+\sum_s J_{i,s}u_s+\epsilon_i,
]

and independent latent Gaussian sources with covariance (C_s), the joint observation-noise covariance is

[
R_{ij}=R_{ij}^{\rm independent}
+\sum_{s\in\mathcal S_i\cap\mathcal S_j}
J_{i,s}C_sJ_{j,s}^T.
]

The exact Gaussian EIG identity uses this complete joint (R), including off-diagonal common-cause blocks. If the source distribution is non-Gaussian, covariance-only marginalization is not generally an exact likelihood; retain the source as a latent nuisance variable and validate the appropriate joint model.

Missing provenance is not a zero-covariance observation. The EIG value may be computed under a declared conditional model, but the result must remain explicitly unqualified when common-cause lineage is incomplete.

## 3. Research frontier: bounds for nonlinear and non-Gaussian settings

Doumbouya, Vidard and Zahm derive EIG upper/lower bounds from dimensional logarithmic Sobolev inequalities and Fisher-information matrices. Their main theorem assumes a smooth joint density for ((\theta,Y)), finite second moment of (Y), and positive-definite matrices that genuinely bound the expected conditional Fisher information and marginal Fisher information:

[
\Sigma_{\rm noise}^{-1}\succeq
\mathbb E[\mathcal I_{Y\mid\theta}],
\qquad
\Sigma_{\rm signal}^{-1}\succeq
\mathcal I_Y.
]

For a candidate observation projection (W), the theorem bounds the EIG by log-determinant ratios of projected, conditionally residualized signal/noise matrices. In their notation,

[
\Sigma[W]=\Sigma-
\Sigma W(W^T\Sigma W)^{-1}W^T\Sigma.
]

The paper presents incremental bounds for information gained by a new observation after an existing batch, and conservative bounds for retaining information from the full observation vector.

**Important:** computing a Fisher matrix or a local covariance approximation is not, by itself, a certificate. CMNI must prove or conservatively bound the matrix inequalities required by the theorem. A plug-in estimate without a valid uncertainty envelope is an approximation, not a certified bound.

The paper's linear-Gaussian case recovers exact EIG. Nonlinear and non-Gaussian cases require estimating or bounding the relevant diagnostic matrices; the bound gap reflects non-Gaussianity and model nonlinearity.

## 4. CMNI adaptation plan

### E0 — exact common-cause Gaussian identity
- Use the exhaustive batch oracle's joint covariance (R).
- Check posterior-covariance and data-space SNR formulae against each other.
- Verify EIG = one half D-optimal logdet reduction.
- Include common references, detector calibration and evaluated-data nuisance sources.
- Add ill-conditioned and singular-covariance rejection controls.

### E1 — latent nuisance and goal-oriented quantities
- Distinguish scientific parameters, measurement-artifact parameters and nuisance variables.
- Keep nuisance variables latent when covariance-only marginalization is not valid.
- Score information about physical quantities of interest separately from information about nuisance parameters.
- Add exact-alias regimes where no candidate can distinguish physical and measurement mechanisms.

### E2 — theorem-qualified nonlinear bounds
- Reproduce the published estimator and bound construction on its original examples.
- Map each CMNI forward-model and noise type to the theorem's regularity and Fisher-matrix assumptions.
- Construct conservative positive-definite matrix bounds with numerical error receipts.
- Store the bound method, assumptions, matrix-enclosure method, tolerances and source lineage with every result.
- Reject a purported certificate if any required matrix inequality cannot be established.

### E3 — value of computation
- Maintain lower/upper EIG intervals for each candidate portfolio.
- Certify dominance only when utility intervals are disjoint in the correct direction.
- When intervals overlap, estimate the value of another gradient evaluation, low-fidelity simulation, high-fidelity simulation or nested Monte Carlo batch.
- Compare adaptive allocation against fixed-budget nested Monte Carlo and the reproduced Coons–Huan MF-EIG estimator at matched total compute.

## 5. Benchmark contract

| Suite | Purpose | Required result |
|---|---|---|
| Linear Gaussian | exact oracle | posterior, D-opt and EIG identities agree |
| Correlated observations | common-cause correctness | off-diagonal covariance changes portfolio utilities and ranking |
| Nonlinear controlled models | bound coverage | true EIG lies inside every claimed valid interval |
| Non-Gaussian priors | assumption stress | report bound width and coverage by regime |
| Latent nuisance | goal orientation | distinguish parameter EIG from QoI EIG |
| Exact aliases | epistemic abstention | zero false certification of distinguishable mechanisms |
| Value of computation | resource allocation | cost, evaluations, interval width, regret and false certificates |

## 6. Proposed targets (not achieved)

- Linear-Gaussian EIG absolute error (le 10^{-10}) on well-conditioned fixtures.
- Zero false dominance certificates on preregistered analytic tests.
- Empirical bound coverage with confidence intervals on all nonlinear benchmark strata.
- At least 10x fewer high-fidelity calls than fixed-budget nested Monte Carlo on selected cases, without loss of bound validity or posterior coverage.
- Explicit unresolved outcome whenever utility intervals overlap.
- Separate reporting for A-optimality, D-optimality, EIG, predictive-QoI utility, false attribution and compute.

## 7. Scientific boundary

The exact Gaussian EIG identity is established analytically and is being encoded in the research kernel. The new output field is authored but awaits exact-head Rust formatting, Clippy and test qualification. No nonlinear EIG bound, theorem certificate, MF-EIG reproduction, or computational acceleration is claimed by this note.


## Status addendum — 2026-11-22

The exact linear-Gaussian EIG correction is now included in the principal CMNI branch. The implementation uses

\[
\Delta_D=\log\det(P_0)+\log\det(P_1^{-1}),\qquad \operatorname{EIG}=\tfrac12\Delta_D,
\]

where the kernel computes the posterior precision factorization directly. A non-unit-prior regression uses \(P_0=\operatorname{diag}(2,3)\), a unit-noise observation of the first coordinate, and checks \(P_1=\operatorname{diag}(2/3,3)\), \(\Delta_D=\log 3\), and \(\operatorname{EIG}=\tfrac12\log 3\). This catches a sign/scale error that unit-prior-only fixtures would not expose.

The preceding exact-head baseline `56140d6ee279c86d8a3ae6a1a290127d83d0e17d` passed workspace formatting, Clippy, physics-core tests, standalone CMNI formatting, standalone CMNI Clippy, 219 CMNI tests, and workspace build on Windows, macOS and Ubuntu. That qualification does not transfer automatically to later source-dependence, QoI/risk, or interval-dominance commits; each new head requires its own exact-head run.

The nonlinear log-Sobolev/SNR EIG bounds remain a research integration plan, not an implemented certificate. CMNI must produce validated conservative enclosures for the theorem-required conditional and marginal Fisher-information matrices. The current typed interval dominance gate records theorem/assumption/enclosure receipt identifiers but does not authenticate them or implement the theorem. Local Fisher matrices and plug-in estimates are not certificates.

The current programme has also added:
- typed source-dependence qualification and a synthetic posterior-undercoverage negative control for split correlated sources;
- linear-Gaussian QoI scoring and finite-scenario lower-tail CVaR;
- interval-based utility dominance with abstention when bounds overlap or epistemic evidence is unqualified.

These are research-only engineering increments. Their exact-head CI and physical-data validation remain separate gates.
