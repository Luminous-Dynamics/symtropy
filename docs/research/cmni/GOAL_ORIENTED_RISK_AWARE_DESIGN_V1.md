# CMNI goal-oriented and risk-aware experiment design v1

**Status:** exact linear-Gaussian QoI scoring authored on `research/cmni-qoi-design-v0`; nonlinear and risk-aware methods are research targets  
**Date:** 2026-11-22  
**Qualification:** synthetic analytical regression only; no nonlinear predictive-QoI, risk-aware or physical validation claimed

## 1. Why parameter-space information is not enough

Conventional Bayesian OED often maximizes expected information gain (EIG) about model parameters. A nuclear-data application, however, may care about an evaluated spectrum integral, a reaction-rate response, a particular energy band, or the separation of a physical mechanism from a detector artefact.

Information about a nuisance parameter can be statistically real but scientifically irrelevant to a particular decision. Conversely, a parameter direction that appears weakly identified globally may dominate one high-consequence output.

The objective therefore needs to be declared at the scientific-output level.

This is an established research direction:
- Zhong, Shen, Catanach and Huan, [Goal-Oriented Bayesian Optimal Experimental Design for Nonlinear Models Using Markov Chain Monte Carlo](https://doi.org/10.1137/24M1649344), SIAM/ASA Journal on Uncertainty Quantification 14(1), 19–47 (2026; published online 2026-01-02). This work optimizes EIG in predictive quantities of interest (QoIs), rather than parameters, using nested Monte Carlo, posterior MCMC, kernel density estimation and Bayesian optimization.
- Jakeman et al., [Risk-Aware Goal-Oriented Bayesian Optimal Experimental Design](https://arxiv.org/abs/2609.08023) (2026-09-07). This preprint composes risk across three levels: posterior predictive uncertainty, risk across the prediction domain, and risk across datasets. It develops a relaxed continuous-weight design and nested differentiable quadrature.
- Abdulsamad et al., [Maximin Robust Bayesian Experimental Design](https://arxiv.org/abs/2603.14094) (v3 revised 2026-11-08; accepted NeurIPS 2026). This preprint treats model misspecification as a max-min design game with information-theoretic ambiguity constraints and PAC-Bayes lower bounds.

These are comparators, not CMNI results.

## 2. Exact linear-Gaussian QoI score implemented

For a scientific parameter vector with Gaussian prior and posterior

\[
\theta\sim\mathcal N(m_0,P_0),\qquad
\theta\mid y\sim\mathcal N(m_1,P_1),
\]

define a vector of linear scientific quantities

\[
q=L\theta.
\]

Then the induced prior and posterior QoI covariance matrices are

\[
P_{q,0}=LP_0L^T,\qquad P_{q,1}=LP_1L^T.
\]

The exact linear-Gaussian QoI EIG in nats is

\[
\operatorname{EIG}_{q}
=\frac12\left[
\log\det(P_{q,0})-\log\det(P_{q,1})
\right].
\]

The corresponding QoI A-optimal trace reduction is

\[
U_{A,q}=\operatorname{tr}(P_{q,0})-\operatorname{tr}(P_{q,1}).
\]

The new `qoi_design.rs` module exposes:
- `score_linear_quantity_of_interest`;
- `rank_portfolios_by_linear_quantity_of_interest`;
- projected prior and posterior QoI covariance;
- QoI A-optimal and D-optimal utility, reported separately;
- exact linear-Gaussian QoI EIG in nats.

The QoI map must have full row rank for the projected covariance log determinants to be defined. Rank-deficient maps and malformed covariance inputs fail closed. Parameter and QoI coordinates must be explicitly scaled before interpreting trace-based objectives.

## 3. An executable portfolio-rank reversal

Use the common-cause synthetic fixture with prior covariance (P_0=I_2), and let the parameter coordinates represent one physical component and one measurement-artifact component.

The correctly assembled batch posterior covariances are:

\[
P_{AB}=\begin{bmatrix}181/201&0\\0&1\end{bmatrix},\qquad
P_{AC}=\begin{bmatrix}91/101&0\\0&20/21\end{bmatrix}.
\]

For a physical-component QoI (q_p=[1,0]\theta):

\[
\operatorname{EIG}_{q_p}(AB)=\frac12\log(201/181),
\quad
\operatorname{EIG}_{q_p}(AC)=\frac12\log(101/91).
\]

The shared pair A+B is marginally better for this one narrowly declared QoI.

For a measurement-artifact QoI (q_m=[0,1]\theta):

\[
\operatorname{EIG}_{q_m}(AB)=0,
\quad
\operatorname{EIG}_{q_m}(AC)=\frac12\log(21/20).
\]

The complementary A+C portfolio is better for this target.

For the identity QoI map (L=I), the QoI EIG equals full parameter-space EIG. This is a regression identity.

**Interpretation:** there is no universally best experiment portfolio independent of the scientific objective. CMNI must preserve the target definition alongside every design score.

## 4. Boundary: parameter QoI versus predictive QoI

The current implementation scores a deterministic linear transformation of parameters. It does not yet implement the nonlinear predictive QoI estimator in Zhong et al.

For predictive outputs, the quantity may be

\[
q=g(\theta,\xi),
\]

where \(\xi\) represents future process or measurement randomness, model discrepancy, or a prediction-domain variable. In that case, posterior predictive uncertainty includes those additional sources. It is not generally sufficient to project parameter covariance with a local Jacobian and call the result exact QoI EIG.

The nonlinear implementation must:
1. reproduce the published GO-OED estimator on controlled examples;
2. distinguish latent scientific parameters, predictive noise and model discrepancy;
3. test posterior predictive density estimation and nested Monte Carlo bias;
4. validate coverage and convergence over prediction-domain strata;
5. compare against the original parameter-space EIG objective on identical compute budgets.

## 5. Common-cause-aware QoI design

All candidate portfolios must first assemble a joint observation covariance that includes shared reference, detector calibration, evaluated nuclear data, background and response sources.

The QoI projection is applied to the resulting posterior covariance; common-cause dependence must not be discarded before the experiment-selection calculation.

If provenance is incomplete, the score remains unqualified. Unknown dependencies must not be encoded as zero covariance.

For nonlinear or non-Gaussian common-cause uncertainty, retain the uncertain source as a latent nuisance variable when covariance-only marginalization would not preserve the likelihood.

## 6. Risk-aware design extension

The 2026 risk-aware GO-OED preprint motivates three explicit risk levels:

1. **Posterior uncertainty risk:** which deviation measure describes predictive uncertainty (variance, quantile, tail loss, or regret)?
2. **Prediction-domain risk:** should utility average over all outputs, emphasize a particular energy region, or guard against worst-case regions?
3. **Dataset/scenario risk:** should utility be expected over datasets, use a quantile/CVaR-like criterion, or protect against a declared ambiguity set?

For CMNI, these correspond to:
- energy-window-specific PFNS integrals and spectrum shape;
- worst-case deviations over energy bands and downstream reaction-rate responses;
- robustness over detector calibration, reference lineage, campaign, library and model-discrepancy scenarios.

Do not collapse these preferences into an arbitrary penalty. Record the selected risk functional, domain, threshold, ambiguity set, weights and units in the design receipt. The risk-aware model must be preregistered before hidden labels or outcomes are revealed.

### Finite-scenario lower-tail baseline authored

The research kernel now also includes `rank_portfolios_by_finite_scenario_lower_cvar`. It accepts a common finite set of scenario utilities for each portfolio, reports expected utility, worst-case observed utility and empirical lower-tail CVaR, and ranks by the selected lower-tail mass.

For (N) equally weighted scenarios (u_{(1)}\le\cdots\le u_{(N)}), and (alpha\in(0,1]), the implementation averages the worst (alpha N) mass with a fractional contribution from the next scenario when (alpha N) is not an integer. At (alpha=1), this reduces to the arithmetic mean; as alpha decreases, the score focuses more strongly on adverse scenarios.

A synthetic negative-control test demonstrates that an expected-utility winner with utilities ([0,0,100]) loses under worst-third CVaR to a steady alternative with ([10,10,10]).

This is an empirical finite-scenario risk baseline only. It does not establish a probability guarantee, solve a distributionally robust optimization problem, estimate Sibson alpha-mutual information, or reproduce the Abdulsamad et al. PAC-Bayes method. Scenario construction, weighting, ambiguity-set calibration and coverage still require independent validation.

A weighted variant is now authored as well: `rank_portfolios_by_weighted_finite_scenario_lower_cvar`. It accepts a common nonnegative scenario-weight vector, normalizes it, ignores zero-probability scenarios for worst-case reporting, and integrates exactly the worst alpha probability mass with fractional boundary weight. The equal-weight API delegates to this weighted implementation.

For example, utilities `[0, 10, 20]` with weights `[0.1, 0.7, 0.2]` have expected utility 11, while their worst-quarter lower-tail CVaR is 6. Tests also reject negative weights and a zero-total weight vector. These are deterministic arithmetic regressions; they do not validate the plausibility of any chosen scenario probabilities.

## 7. Benchmark ladder

| Suite | Baseline | Required report |
|---|---|---|
| Q0: Gaussian QoI | Exact analytic covariance projection | EIG and covariance error, rank checks, rank reversal |
| Q1: nonlinear predictive QoI | Zhong et al. nested-MC/KDE estimator | bias, variance, posterior predictive coverage, compute |
| Q2: risk-aware design | Mean QoI EIG and risk-aware GO-OED | tail utility, prediction-domain coverage, regret |
| Q3: model uncertainty | Nominal design and Abdulsamad et al. robust design | worst-case utility, ambiguity radius, robust certificate validity |
| Q4: CMNI PFNS | matched physical and detector/reference lineages | integral uncertainty, energy-band coverage, false attribution, cost |
| Q5: discrete feasibility | exhaustive small-set oracle and relaxed portfolio | selection regret and relaxation-to-discrete rounding regret |

### Proposed goals (not achieved)

- Linear-Gaussian QoI EIG relative error \(\le10^{-10}\) on well-conditioned fixtures.
- Zero rank-deficient QoI maps silently accepted as finite-information designs.
- QoI interval coverage calibrated separately by prediction-domain and mechanism strata.
- At least 10× fewer high-fidelity evaluations than fixed-budget nested Monte Carlo on selected nonlinear cases, without material bias or coverage degradation.
- Zero severe-event misses attributable to substituting mean EIG for a preregistered risk-sensitive objective.
- Separate parameter-EIG, QoI-EIG, A-optimal QoI reduction, risk utility, predictive error, coverage and compute reporting.

## 8. Scientific boundary

The exact linear-Gaussian QoI score and its synthetic rank-reversal tests are authored on `research/cmni-qoi-design-v0`. They have not yet been Rust-formatted, compiled or CI-qualified. The nonlinear GO-OED estimator, risk-aware objective, robust ambiguity-set optimization and real PFNS validation remain outstanding. No improvement over the published methods is claimed.
