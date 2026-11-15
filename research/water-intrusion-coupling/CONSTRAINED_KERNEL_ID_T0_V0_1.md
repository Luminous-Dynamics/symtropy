# Constrained Positive Kernel Identification T0 v0.1

**Programme:** WATER-MIXING-CONSTRAINED-IDENTIFICATION-001 / [#5298](https://github.com/Luminous-Dynamics/symtropy/issues/5298)  
**Physical invariants:** [#5294](https://github.com/Luminous-Dynamics/symtropy/issues/5294), [draft PR #5297](https://github.com/Luminous-Dynamics/symtropy/pull/5297)  
**Dynamic forward operator:** [draft PR #5288](https://github.com/Luminous-Dynamics/symtropy/pull/5288)  
**Evidence class:** synthetic constrained regression and known-answer oracle  
**Evidence ceiling:** no physical junction identification or qualification

## 1. Objective

Estimate a finite causal outlet-resolved tracer kernel while enforcing its passive physical constraints during optimization. Compare:
- ordinary weighted least squares;
- post-hoc clipping of unconstrained least-squares coefficients;
- joint constrained least squares on the declared nonnegative coefficient simplex for each inlet.

The supplied per-inlet steady gain is independent input to this first increment. The solver preserves that gain exactly (within tolerance); it does not estimate uncertainty in the steady-gain matrix or silently force every inlet to unit recovery.

## 2. Statistical contract

The T0 supports a declared independent diagonal observation covariance:

\[
\operatorname{Cov}(e)=\operatorname{diag}(\sigma_1^2,\ldots,\sigma_m^2),
\quad w_i=1/\sigma_i^2
\]

It applies square-root weights to both the design rows and observations before computing the least-squares objective. It verifies that each supplied sample weight is the reciprocal of its supplied observation variance. Unknown/full covariance structures are rejected in v0.1 rather than treated as independent samples. Zero, negative, nonfinite or inconsistent variance/weights are refused.

The holdout receives its own diagonal variance/weight contract. The reported training and blind holdout MSEs are weighted MSEs under the declared measurement-noise model.

## 3. Constrained optimization

For coefficient vector h, the deterministic projected accelerated-gradient optimizer solves:

\[
\min_h \|W^{1/2}(Xh-y)\|_2^2
\]

subject to h >= 0 and each declared inlet coefficient group satisfying:

\[
\sum_{r \in G_i} h_r=P_i,\quad 0\leq P_i\leq1.
\]

This is a convex quadratic program over a product of simplices. The per-inlet target preserves genuinely sub-stochastic transfer; it does not normalize observed outputs to one. The solver records convergence and iteration count and refuses to describe a nonconverged iterate as identified.

The design matrix is weighted before rank/condition diagnostics. Rank-deficient designs return UNIDENTIFIABLE; full-rank designs above the declared condition ceiling return ILL_CONDITIONED.

## 4. Frozen eight-case controls

1. **Noisy negative unconstrained fit:** LS estimates [1.43, -0.57], clipping yields [1.43, 0] with integrated gain 1.43, while the constrained fit returns [0.8, 0]. On the frozen blind pulse, constrained weighted MSE is 0 versus 0.3969 for clipping.
2. **Sub-stochastic preservation:** known transfer gain 0.8 remains 0.8; no unit-gain normalization.
3. **Rank-deficient excitation:** repeated columns refuse unique identification.
4. **Ill-conditioned full-rank excitation:** condition number exceeds the declared limit and the estimator abstains.
5. **Static zero-lag limit:** recover the one-coefficient steady transfer.
6. **Invalid gain target:** a target above one is rejected.
7. **Unsupported full covariance:** rejected rather than silently diagonalized.
8. **Invalid sample weight:** zero weight/inconsistent inverse variance is rejected.

A separate JavaScript arithmetic mirror generated the frozen expected outcomes. The oracle is stored independently from the Rust implementation, and the manifest binds the case identities.

## 5. What this does not yet solve

- It is a finite-dimensional synthetic linear-regression fixture, not a physical experiment.
- The exact steady-gain constraint is taken as known; uncertainty in P must be carried in a later joint-estimation or robust-constraint formulation.
- Only independent diagonal observation covariance is supported. Correlated outlet/time sensor errors require whitening with a validated covariance factor or a structured likelihood; they must not be approximated by diagonal weights without evidence.
- The current group constraints require all coefficient indices to be assigned to exactly one inlet group.
- Regularization, uncertainty intervals, confidence regions, experiment-design optimization and physical parameter identifiability are not claimed.
- A stable optimizer and full-rank design do not prove the model structure is correct.

## 6. Next benchmarks

Proposed targets (not achieved):
- exact frozen-oracle agreement and exact constraint compliance;
- zero nonpositive/nonfinite variance or weight accepted;
- zero false unique-identification outcomes on deficient/ill-conditioned designs;
- lower blind weighted error than clipping on the deliberately adversarial negative-coefficient case;
- no loss of a known sub-stochastic 0.8 transfer;
- publish condition number, training/holdout weighted residual, coefficient error, active constraints, solver gap/iterations, runtime and memory;
- compare diagonal NNLS to generalized least squares only after a separately validated full covariance is available.

## 7. Evidence status

Authored: Rust estimator, nine unit tests, eight frozen input cases, separate oracle and identity manifest, and exact-revision workflow. A separate JavaScript numerical mirror matches the eight oracle outcomes. Native Rust formatting, tests, Clippy and exact-head CI remain pending until the configured workflow completes.

No result qualifies a physical junction, tracer, sensor, network model, drinking-water system, potability, contamination, toxicology, SANS 241 compliance or release authority.
