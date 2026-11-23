# Decision-Aligned Regret Calibration Benchmark v0.1

**Implementation:** `tools/orbital-observation-surface-bench/src/regret_calibration.rs`  
**Research issue:** [#6942](https://github.com/Luminous-Dynamics/symtropy/issues/6942)  
**Composes with:** [#6835](https://github.com/Luminous-Dynamics/symtropy/pull/6835), [#6918](https://github.com/Luminous-Dynamics/symtropy/pull/6918), [#6939](https://github.com/Luminous-Dynamics/symtropy/pull/6939), and sequential-world research [#6887](https://github.com/Luminous-Dynamics/symtropy/issues/6887).  
**Claim ceiling:** exact finite split-conformal regret-rank algebra over synthetic, predeclared regimes only.

## 1. Why calibrate decisions rather than only parameters?

A parameter residual is not a decision-risk metric. A substantial residual may remain inside a region where the optimizer and realized decision do not change. A much smaller error near a decision boundary can switch the selected action and create substantial regret.

Zhao, Jiang and Qi's November 9, 2026 preprint, *Beyond Prediction Residuals: Conformal Regret Calibration for Robust Contextual Linear Optimization*, studies this mismatch. It calibrates an optimization-derived regret upper-bound score (SPO+) rather than raw objective residuals and gives finite-sample marginal regret guarantees under exchangeability for contextual linear optimization. Their work also makes clear that support/boundedness assumptions matter when a conformal rank runs beyond the available calibration sample. This is a direct comparator only for domains that meet the paper's contextual-linear optimization and upper-bound assumptions; it is not a generic physical-system certification method. https://arxiv.org/abs/2611.11973

Yang and Jin's ICML 2026 *Multi-Distribution Robust Conformal Prediction* is a relevant comparator for multiple heterogeneous source regimes and uniform coverage claims. Kiyani et al.'s ICLR 2026 work on partially calibrated forecasters is a comparator for whether decision-relevant guarantees can be obtained without demanding full predictive calibration. Neither should be treated as equivalent to the simpler worst-regime threshold implemented here. https://proceedings.mlr.press/v306/yang26ad.html https://proceedings.iclr.cc/paper_files/paper/2026/hash/b8b6df58745740e54f00d84534bd182d-Abstract-Conference.html

## 2. Frozen v0.1 calibration object

The input binds:
- a predeclared calibration split identity and fixed stopping rule;
- an opaque score-contract identity and receipt reference;
- exact alpha as a rational;
- a separately planned inventory of operating-regime IDs and digest;
- for each regime, a planned sample-ID inventory and digest;
- complete calibration records, each with unique sample ID, unique replication-unit ID and exact regret upper-bound score;
- an optional independently supported score-range ceiling and its own receipt.

The score contract is essential: each score must be an auditable upper bound on the relevant realized decision regret. Conformal calibration of a score does not prove that this upper-bound relationship holds. The receipt IDs/digests provide structural binding, not issuer authentication or scientific validation.

Each sample and each replication unit must be unique within the corpus. This deliberately rejects duplicated session/run records from being silently counted as additional calibration samples. It does not itself prove exchangeability or independence.

## 3. Exact finite-sample rank and the (n+1) edge

For a frozen regime with (n) calibration scores and error level (alphain(0,1)), the one-indexed split-conformal rank is

[
k=leftlceil(n+1)(1-alpha)ightceil.
]

If (kle n), the threshold is the (k)-th score in exact sorted order.

If (k=n+1), the largest observed score is **not** a valid substitute for the missing ((n+1))-st order statistic. v0.1 has two explicit branches:

- Without an independently supported upper bound on the true regret-upper-bound score, return `NO_FINITE_NONTRIVIAL_THRESHOLD`. Do not fabricate a threshold.
- If a separately receipted support ceiling exists, return that value as `SUPPORT_BOUND_ONLY`. It may be vacuous; it is not an empirical conformal quantile.

A score being encoded as a rational in ([0,1]) is not evidence that actual regret is bounded by one. The support ceiling needs its own domain proof/receipt. It must not be inferred from the parser type.

## 4. Multiple predeclared regimes

v0.1 computes the order statistic independently inside each predeclared regime. For an unknown test regime that is guaranteed to belong to the declared collection, it emits the maximum of the finite regime-specific thresholds:

[
q_{worst}=max_{rinmathcal R}q_r.
]

If each regime's calibration and test score are exchangeable within that regime, this conservative threshold is at least as large as each regime's threshold. Thus it can support a marginal (1-alpha) regret-score coverage statement for a test point from any fixed declared regime or mixture of those regimes. The claim is **not** simultaneous pathwise coverage for all future outcomes, not an anytime-valid guarantee and not a guarantee for an unseen regime.

The implemented max-threshold baseline is not Yang and Jin's max-p multi-distribution conformal method. They are separate methods and must be compared separately. The max threshold is the deliberately transparent, exact baseline; max-p or another distributionally robust conformal method is a matched research comparator.

If any declared regime lacks a finite nontrivial threshold and no supported score ceiling is available, the global unknown-regime threshold is absent. Do not silently drop that regime or pool its scores into a larger sample.

## 5. Assumptions and explicit exclusions

v0.1 supports only a predeclared, fixed-horizon split. It rejects post-hoc regime/score selection and duplicate replication-unit IDs. It does not provide:
- optional stopping or repeated-look validity;
- adaptive sample selection guarantees;
- exchangeability under arbitrary temporal drift or common-cause dependence;
- a joint simultaneous confidence statement over all regimes;
- calibration for regimes outside the frozen inventory;
- a proof that the score is an upper bound on actual domain regret;
- physical qualification or execution authority.

Do not transfer #6887's anytime-valid physical-world guarantee to this calibration object. Even if a sequential world-set guarantee is valid, the loss score, score support, regime classification and calibration samples need their own evidence. Likewise, a valid loss score does not prove the observation/world set is complete.

## 6. Frozen challenge corpora

Three receipts are generated independently:

1. **Two-regime empirical threshold.** Alpha (=1/4), (n=3) per regime, so (k=3=n). The dry-regime scores are (0.1,0.4,0.8) and wet-regime scores (0.1,0.3,0.7). The max-regime threshold is (0.8), attained by `dry`.
2. **Insufficient sample, no support.** Alpha (=1/10), (n=3), so (k=4=n+1). The result must be `NO_FINITE_NONTRIVIAL_THRESHOLD`, never the largest observed score (0.3).
3. **Insufficient sample, independently receipted support bound.** Same sample and alpha, but an external support receipt supplies ceiling (1). The result must say `SUPPORT_BOUND_ONLY`, never `EMPIRICAL_ORDER_STATISTIC`.

The Rust tests and independent JavaScript `BigInt` oracle verify exact rank arithmetic, planned-vs-observed regime/sample inventory, full sample digests, unique replication units, fixed/predeclared selection, worst-regime max and the no-fake-quantile edge.

## 7. Comparator tournament and metrics

On matched datasets, frozen regimes, score contracts and calibration budgets compare:

1. residual conformal calibration of parameter/loss-model residuals;
2. decision-aligned conformal regret score (e.g. SPO+ only where the contextual-linear optimization assumptions apply);
3. this exact per-regime order-statistic + worst-regime threshold baseline;
4. Yang & Jin multi-distribution robust conformal prediction;
5. Kiyani et al. decision-making under partial calibration;
6. robust Bayesian / Wasserstein OT-DRO / credal ambiguity-set baselines;
7. uncalibrated point-loss #6918 and finite joint-scenario #6939.

Report separately by regime:
- exact calibration rank and threshold status;
- empirical coverage and confidence interval on held-out, independently sampled units;
- marginal vs worst-regime coverage;
- 90th/95th percentile regret, CVaR and worst-case regret;
- false-in-profile / missed-OOD rates;
- support-bound-only and no-finite-threshold frequencies;
- action changes / false commitment rates;
- score envelope and joint/rectangular ambiguity-frontier sizes;
- calibration sample count, independent sessions, reference trials, compute, elapsed time and operator burden.

Do not combine coverage, regret and costs into a single aggregate score. Do not call a simulation fixture a statistically independent calibration sample.

## 8. Prospective gates (targets, not achievements)

- 100% exact Rust/JavaScript agreement on the rank, selected sample and threshold for every frozen corpus.
- 100% detection of missing predeclared regimes or samples, digest changes and duplicated replication units.
- Zero substitution of the maximum observed score when (k=n+1) and no external support bound exists.
- 100% distinction between empirical quantile and `SUPPORT_BOUND_ONLY`.
- Zero post-hoc regime selection or unqualified regime pooling.
- Per-regime coverage and its exact interval reported on held-out data; no transfer of split-conformal guarantees to adaptive or dependent sampling without a separate theorem.
- A prospective target of at least 20% fewer calibration/reference trials than the strongest matched comparator at equal regret coverage and false-commitment risk, provided independent held-out evidence supports it.

The intended paradigm shift is to calibrate the thing the system actually owes the user—a bound on decision regret—while keeping physical-world coverage, consequence-model adequacy, and statistical calibration as three separately evidenced contracts.
