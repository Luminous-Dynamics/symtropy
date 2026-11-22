# CMNI anytime value-of-computation and interval-dominance contract v1

**Status:** Rust interval-dominance gate authored; no end-to-end scheduler or confidence-sequence producer is claimed yet.  
**Tracking issue:** #6769.  
**Related methods:** Coons & Huan MF-EIG (2025), active pilot sampling (Coons, Jivani & Huan, 2026), empirical-Bernstein confidence sequences (Waudby-Smith & Ramdas, 2024).

## 1. Why this is the next systems layer

The kernel can now compute several kinds of scientific utility:
- exact small-set linear-Gaussian parameter EIG and A/D-optimal reductions;
- linear-Gaussian QoI utility;
- finite-scenario lower-tail CVaR;
- common-cause-aware exhaustive and greedy batch portfolios.

For nonlinear experiments, the likely path adds:
- direct predictive-QoI EIG estimates;
- theorem-qualified nonlinear EIG intervals;
- multifidelity estimators and pilot-covariance uncertainty;
- uncertainty over physical and statistical model discrepancy.

A fixed compute budget treats every ambiguous design decision alike. The decision engine should instead ask: *what computation or evidence acquisition most reduces the chance of choosing the wrong experiment?*

## 2. Current code increment

`research/cmni/reference-kernel/src/design_decision.rs` adds:
- `PortfolioUtilityInterval`: finite ordered lower/upper utility bounds with unique candidate IDs;
- `UtilityBoundEvidence`: Exact, TheoremCertified, AnytimeValidFamilywise, Heuristic, or Unqualified;
- `certify_maximum_utility`: certifies a winner only if its lower utility bound exceeds every competitor's upper bound by a caller-declared margin;
- explicit unresolved states for overlapping intervals, heuristic/unqualified bounds, and incompatible familywise receipts.

Validation rules:
- exact values must have equal lower/upper endpoints;
- theorem evidence requires non-empty theorem, assumptions and enclosure receipt IDs;
- anytime-valid confidence-sequence intervals must declare alpha in (0,1) and share the same familywise receipt ID and alpha across the candidate family;
- heuristic or unqualified intervals cannot certify a winner;
- threshold equality is unresolved, not a win.

The shared family receipt is intended to reference a validated simultaneous confidence process covering the full candidate family at all inspection times, including data-dependent stopping. The kernel checks the declared contract; it does not generate confidence sequences or authenticate their receipts. A collection of separate pointwise intervals is not a simultaneous familywise confidence sequence.

## 3. Crucial epistemic separation

These evidence labels have different meanings:
1. **Exact:** exact relative to the declared mathematical model and numerical assumptions; not proof that the physical model is true.
2. **Theorem-certified:** only valid if the theorem assumptions and conservative matrix/numerical enclosures are actually established and independently checked.
3. **Anytime-valid familywise:** only valid if the confidence process controls the complete candidate family uniformly over time and under the declared sampling/stopping process.
4. **Heuristic:** a useful planning interval but not sufficient to certify dominance.
5. **Unqualified:** a result with unresolved source lineage, missing assumptions, or another qualification blocker.

The enum variants and receipt IDs are typed declarations, not proof objects. A downstream verifier must resolve the receipts before treating the corresponding interval as certified.

Monte Carlo uncertainty, theorem-bound uncertainty, model discrepancy and missing evidence are not interchangeable:
- a confidence sequence can control Monte Carlo sampling error under its assumptions;
- it does not cover a biased estimator, low/high fidelity discrepancy, missing common causes or a wrong forward model;
- a theorem bound can control a mathematical approximation only within its theorem assumptions;
- unknown source dependence remains unqualified until modeled or supported by defensible independent evidence.

## 4. Adaptive value-of-computation controller — next layer

Maintain intervals [L_i, U_i] for each candidate portfolio and explicit costs for available actions:
- high-fidelity model evaluation;
- low-fidelity evaluation;
- posterior or nested-MC draw;
- pilot draw to estimate cross-fidelity covariance;
- improved covariance or likelihood evaluation;
- independent calibration / source evidence;
- stop, select, or abstain.

Choose the next action using expected reduction in decision loss per resource cost—not simply the largest predicted variance reduction. Stop when one portfolio is interval-dominant, or when the marginal value of further computation is below its cost. At a fixed budget, return unresolved if no dominance certificate exists.

Do not rank by utility divided by cost unless that is the predeclared decision objective. Prefer explicit resource constraints or a declared cost-adjusted utility, so changing units does not silently change the design.

### Multifidelity branch

The Coons–Huan estimator is unbiased for the high-fidelity EIG mean under its assumptions and uses approximate control variates with optimized allocation. CMNI must reproduce the published nonlinear benchmark before adapting it to nuclear physics.

The 2026 active-pilot line adds uncertainty over the covariance matrix required for multifidelity allocation and allocates pilots where covariance uncertainty is important to the outer objective. This suggests a second-level value-of-computation problem: when should CMNI stop learning the fidelity covariance and start the production estimate?

### Exact and theorem-bound branch

For linear-Gaussian fixtures, use the closed-form posterior/EIG oracle. For nonlinear/non-Gaussian cases, use EIG lower/upper bounds only when all theorem requirements are met. The 2026 log-Sobolev/SNR framework requires genuine matrix bounds on expected conditional and marginal Fisher-information objects. A plug-in Fisher matrix or local linearization is not a certificate.

## 5. Benchmark design

Freeze these before evaluating algorithms:

| Suite | Competitors | Metrics |
|---|---|---|
| Exact portfolio oracle | exhaustive enumeration, greedy | exact winner, regret, cost, tie/abstention behavior |
| Nonlinear EIG | single-fidelity nested MC, published MF-EIG | bias, variance, RMSE, interval coverage, compute calls |
| Pilot allocation | fixed pilot size, adaptive pilot, oracle covariance | wasted pilot cost, final estimator variance, allocation regret |
| Robust design | nominal EIG, CVaR, maximin, risk-aware GO-OED | tail utility, worst-case regret, scenario stability |
| Anytime dominance | fixed-time intervals, pointwise repeated CIs, familywise confidence sequences | false certificate rate, coverage under optional stopping, time-to-decision |
| Source misspecification | complete joint covariance, split-independent negative controls, Unknown qualification | posterior undercoverage, false independence, abstention, rank reversals |

Proposed targets (not achieved):
- zero false dominance certificates on analytic and preregistered stochastic controls;
- no certified winner when any candidate has heuristic or unqualified bounds;
- zero false familywise claims under optional stopping;
- exact-oracle regret of 0 on small enumerated sets;
- median regret <=1% and 95th percentile <=5% on frozen larger suites;
- at least 10x reduction in high-fidelity calls or estimator variance against a matched-budget single-fidelity baseline, with no material bias or coverage loss;
- report total cost, pilot cost, low/high-fidelity calls, posterior draws, wall time, memory, interval width, regret, and abstention rate.

Do not aggregate all dimensions into one score.

## 6. Public research references

- Coons & Huan (2025), “A Multifidelity Estimator of the Expected Information Gain for Bayesian Optimal Experimental Design,” SIAM/ASA J. UQ 13(4), 1990–2021. https://doi.org/10.1137/25M1731812
- Coons, Jivani & Huan (2025), “Bayesian Covariance Uncertainty for Adaptive Pilot-Sampling Termination in Multi-fidelity Uncertainty Quantification.” https://arxiv.org/abs/2508.18490
- Coons, Jivani & Huan (2026), “Active Pilot Sampling for Multi-fidelity Estimation in Stochastic Optimization.” https://arxiv.org/abs/2610.27563
- Waudby-Smith & Ramdas (2024), “Estimating means of bounded random variables by betting,” JRSS B 86(1), 1–27. https://doi.org/10.1093/jrsssb/qkad009
- Doumbouya, Vidard & Zahm (2026), “Certified Bayesian optimal experimental design: from Expected Information Gain to Signal-to-Noise Ratio.” https://arxiv.org/abs/2609.32282

## 7. Current limits

The present Rust module only compares caller-supplied intervals. It does not:
- generate confidence sequences;
- verify theorem assumptions or matrix enclosures;
- estimate multifidelity covariance;
- select compute actions;
- account for physical-model discrepancy;
- establish real-world measurement independence.

The module is a contract and regression-test substrate for those later components, not yet a complete value-of-computation engine.
