# CMNI maximin experiment design under source-correlation uncertainty v1

**Status:** numerical maximin adapter authored; exact-head CI and independent numerical review pending.  
**Branch:** `research/cmni-dependence-robust-design-v0`.  
**Predecessor:** the exact two-source dependence envelope in `dependence_envelope.rs`.

## 1. Motivation

A common shortcut is to plug a single estimated cross-source correlation into the experiment-design objective. That can make a design look optimal only because the uncertain dependence parameter was replaced by a convenient point estimate.

The 2026 robust Bayesian experimental design literature makes model ambiguity part of the design objective. Abdulsamad et al. (arXiv:2603.14094v3, 8 November 2026; accepted NeurIPS 2026) formulate a max-min game against an adversarial model under an information-theoretic ambiguity set, derive Sibson alpha-mutual information as the robust objective, and use PAC-Bayes lower bounds to control finite-sample uncertainty: https://arxiv.org/abs/2603.14094

CMNI's current contribution is narrower and complementary: an exact analytic (ordinary floating-point) envelope for the cross-correlation of two scalar shared sources in a two-observation, one-parameter linear-Gaussian model. This allows a concrete first question: does the candidate design remain valuable across every dependence value in the evidence-backed interval?

## 2. Implemented adapter

`dependence_robust_design.rs` consumes a candidate set and:
- requires a common ordered pair of source identifiers;
- requires a single non-empty joint-model receipt across the candidate set;
- requires common source marginal standard deviations, prior parameter variance and correlation interval;
- computes each candidate's full-interval EIG envelope using `bound_two_source_dependence`;
- computes a nominal midpoint EIG for comparison;
- ranks candidates by midpoint EIG and worst-case EIG separately;
- returns worst- and best-case correlation values and the two ranks.

The ranking is maximin over the supplied correlation interval:

\[
d^* = \arg\max_{d\in\mathcal D}\ \min_{\rho\in[\rho_-,\rho_+]} I(\theta;Y\mid d,\rho).
\]

This is not a claim that the supplied interval contains the true correlation. That is an evidence/provenance question handled outside this function.

## 3. Synthetic rank-reversal oracle

Machine-readable fixture: `research/cmni/fixtures/midpoint-vs-maximin-source-correlation-v1.json`.

Two candidate designs share the same two sources, source standard deviations, prior variance, observation noise and correlation interval ([-0.75,0.75]).

- Nominal-favourite: observations couple to distinct source components. At midpoint correlation 0, EIG is approximately 1.36718 nats. At its worst admissible correlation, EIG is (0.5\log(10)\approx 1.15129) nats.
- Dependence-stable: both observations load on the first source component. Its EIG is invariant to the uncertain correlation, approximately (0.5\log(113/9)\approx 1.26508) nats.

The midpoint-only ranking chooses Nominal-favourite. The maximin ranking chooses Dependence-stable. The robust design gives up some nominal peak utility to improve the minimum utility over the declared uncertainty set.

The values are analytic for this synthetic scalar linear-Gaussian model. They are not physical measurements or a detector calibration result.

## 4. Why this is not yet a formal certificate

The underlying dependence envelope enumerates analytic stationary points and endpoints, giving global extrema for this model up to ordinary f64 arithmetic. It does not use directed rounding or formal interval arithmetic. Therefore:
- `machine_certified_enclosure` is false;
- the score is a numerical maximin ranking, not a certified lower bound;
- the common receipt ID is a scope reference, not proof that the evidence receipt is authentic or scientifically justified;
- the assumed interval itself must be supported by independent calibration, physical knowledge, or a defensible uncertainty analysis.

A future certified path needs outward-rounded polynomial/root enclosures, interval Cholesky/positive-definiteness checks, and independently verified source receipts.

## 5. How this compares with current methods

| Method | Ambiguity handled | CMNI comparison |
|---|---|---|
| Nominal Bayesian OED | One declared likelihood/correlation | Midpoint EIG comparator in the fixture |
| Maximin robust BED / Sibson alpha-MI | Information-theoretic ambiguity set over model distributions | Broader nonlinear comparator; not implemented here |
| PAC-Bayes robust design | Finite-sample confidence for stochastic robust utility | Future comparator for nested-MC designs |
| CMNI two-source dependence maximin | Closed interval for one shared-source correlation parameter | Exact analytic extrema in this restricted model, ordinary f64 |

The Nature 2026 review on AI physics-experiment design emphasizes four linked frontiers: expressive search spaces, fast and reliable simulators, computable scientific objectives, and AI exploration across discrete and continuous design variables: https://www.nature.com/articles/s41586-026-10898-6

The CMNI extension here specifically strengthens the objective and reliability layers: the design is scored against dependence uncertainty rather than an unexamined plug-in assumption.

## 6. Benchmark plan (targets, not achieved)

- 100% agreement with a high-precision independent solver on the two-source analytic envelope fixtures.
- Zero nominal-to-robust rank errors on exact synthetic cases where all candidate minima are known.
- Compare maximin interval EIG with midpoint EIG, expected EIG over a declared correlation distribution, maximin posterior-variance reduction, and Sibson-alpha robust EIG on matched small models.
- Report nominal regret, worst-case regret, worst-case EIG, interval width, selected worst-case rho, and computation cost.
- Sweep correlation interval width and source-Jacobian structure; test rank-reversal frequency and stability.
- Later add three or more shared sources and low-dimensional correlation ellipsoids/polytope ambiguity sets, then validate against global optimization and formal interval methods.

Aspirational engineering targets:
- zero rank reversals relative to an independent high-precision global optimizer on the supported two-source problem;
- zero acceptance of mismatched source scopes or receipts;
- zero machine-certified claims until outward-rounded validation is implemented;
- median regret ≤1% versus exact maximin oracle on frozen tractable sets after discrete batch optimization.

## 7. Limits

This module covers only one scalar parameter, two observations, two scalar shared sources and one cross-correlation interval. It does not implement a general positive-semidefinite correlation ambiguity set, robust Sibson MI, PAC-Bayes bounds, nonlinear model-discrepancy ambiguity, or physical validation.