# CMNI common-cause batch-design oracle v1

**Status:** synthetic analytical benchmark specification and frozen fixture  
**Date:** 2026-11-21  
**Scope:** two-parameter linear-Gaussian design with known shared systematic covariance  
**Qualification:** analytical oracle only; not a PFNS result, a validated optimizer, or evidence of real-world superiority

## Why this benchmark exists

A portfolio can contain individually precise measurements that add surprisingly little independent information when they share a dominant calibration uncertainty. Candidate-level marginal variances do not reveal this dependence. An optimizer that sums candidate Fisher information as if each run were independent can therefore select the wrong batch.

This fixture is deliberately small enough to admit exact, independently checkable posterior covariances. Its purpose is to test the covariance and portfolio-ranking plumbing before introducing nonlinear response models, posterior Monte Carlo, or continuous design variables.

## Generative model

Let the dimensionless scientific parameter vector be

\[
\theta = (\theta_{\mathrm{physical}},\theta_{\mathrm{artifact}})^T,
\qquad \theta\sim\mathcal N(0,I_2).
\]

Each candidate has a local linear observation model

\[
y_i = H_i\theta + J_i u_i + \epsilon_i,
\]

where \(u_i\) denotes named shared systematic sources and \(\epsilon_i\) is candidate-specific independent noise. All errors are Gaussian and their covariance is fixed for this benchmark.

The three candidates are:

| Candidate | Sensitivity row | Independent variance | Shared source |
|---|---|---:|---|
| A | \([1,0]\) | 0.1 | calibration:A-B, variance 9 |
| B | \([1,0]\) | 0.1 | calibration:A-B, variance 9 |
| C | \([0,1]\) | 20 | none |

A and B each have marginal observation variance 9.1, but their cross-covariance is 9 because both depend on the same latent calibration. Their correlation is \(9/9.1\approx0.98901\). Candidate C is individually much noisier, but it measures a different scientific direction.

Select exactly two candidates under a unit cost per candidate and total budget 2. Candidate lineages are complete in this synthetic fixture.

## Exact covariance and posterior oracle

For a selected portfolio \(S\), stack its design rows into \(H_S\), and assemble the full observation covariance \(R_S\), including off-diagonal blocks induced by repeated shared-source IDs.

For this linear-Gaussian case,

\[
P_S = (P_0^{-1}+H_S^TR_S^{-1}H_S)^{-1},
\qquad P_0=I_2.
\]

The A-optimal utility used here is the unweighted trace reduction in the declared dimensionless coordinates:

\[
U_A(S)=\operatorname{tr}(P_0)-\operatorname{tr}(P_S).
\]

The D-optimal utility is:

\[
U_D(S)=\log\det(P_0)-\log\det(P_S).
\]

The expected results are frozen in `research/cmni/fixtures/common-cause-batch-design-oracle-v1.json`.

### Portfolio A+B, with the shared calibration retained

\[
R_{AB}=\begin{bmatrix}9.1&9\\9&9.1\end{bmatrix},
\qquad
P_{AB}=\begin{bmatrix}181/201&0\\0&1\end{bmatrix}.
\]

Thus \(U_A(AB)=20/201\approx0.09950249\), and
\(U_D(AB)=\log(201/181)\approx0.10480788\).

### Portfolio A+C

\[
R_{AC}=\begin{bmatrix}9.1&0\\0&20\end{bmatrix},
\qquad
P_{AC}=\begin{bmatrix}91/101&0\\0&20/21\end{bmatrix}.
\]

Thus \(U_A(AC)=311/2121\approx0.14662895\), and
\(U_D(AC)=\log(2121/1820)\approx0.15305117\).

**Correct ranking:** A+C is more informative than A+B for both declared objectives.

### Negative control: pretend A and B are independent

Replacing \(R_{AB}\) with \(\operatorname{diag}(9.1,9.1)\) gives the false posterior

\[
P_{AB}^{\mathrm{naive}}=
\begin{bmatrix}91/111&0\\0&1\end{bmatrix}.
\]

The resulting false utilities are \(U_A=20/111\approx0.18018018\) and
\(U_D=\log(111/91)\approx0.19867069\). Both incorrectly rank A+B above A+C.

This is the core adversarial test: **dropping a known common cause must be capable of changing the selected portfolio, not merely perturbing a reported error bar.**

## Required executable tests

1. Reconstruct the A+B joint covariance exactly, including the off-diagonal value 9.
2. Reconstruct the A+C covariance and both posterior covariance matrices to relative error no worse than \(10^{-10}\) on this well-conditioned fixture.
3. Reproduce A- and D-utility values and their exact ordering.
4. Run the negative control with the shared-source term deliberately removed and show the ranking reversal.
5. Duplicate the same source ID with a conflicting covariance definition and require fail-closed rejection.
6. Remove lineage-completeness evidence and verify that the numerical result remains explicitly unqualified; missing provenance must never be reinterpreted as independence.
7. Verify deterministic tie-breaking, exact batch size, cost-budget enforcement, and no duplicate candidate selection.
8. Compare exhaustive enumeration (the oracle for small candidate sets) with greedy selection and report regret. Do not claim greedy optimality from this fixture.

## Benchmark expansion

After the analytical fixture passes, expand in stages:

- **Covariance topology:** shared reference evaluation, detector efficiency calibration, evaluated cross-section library, background subtraction, time-base and common response corrections.
- **Identifiability:** orthogonal, partially confounded, exact aliases, and nuisance-only signatures. Exact aliases must not be “resolved” by a more aggressive optimizer.
- **Portfolio algorithms:** random feasible portfolios, cost-aware greedy, exhaustive enumeration on small sets, discrete joint optimization, and measure-relaxed batch A-optimal design with explicit discretization regret.
- **Uncertainty-aware design:** prior sensitivity, model discrepancy, uncertain covariance hyperparameters, missing lineage, and lower-quantile utility over declared scenarios.
- **Decision relevance:** distinguish reduction in parameter variance from discrimination between physical and measurement-artifact hypotheses and from improvement in downstream integral quantities.
- **Compute efficiency:** wall time, forward-model calls, Cholesky factorizations, memory, and utility per unit compute. Compare multifidelity estimators only against equal-cost high-fidelity baselines and verify estimator bias/coverage.

## Qualification targets (proposed, not achieved)

| Metric | Initial target |
|---|---:|
| Analytical oracle posterior / covariance relative error | \(\le 10^{-10}\) |
| Correct common-cause portfolio ranking on preregistered fixtures | 100% |
| Exact-oracle regret on enumerable sets | 0% for exhaustive solver |
| Budget and cardinality violations | 0 |
| False independence under incomplete lineage | 0 |
| Confident attribution in exact-alias controls | 0 |
| 95% interval coverage in calibrated simulation strata | 93–97%, reported per stratum |
| Greedy / joint solver regret | Report distribution; target median \(\le5\%\), not a universal guarantee |

Targets must be frozen before hidden labels are exposed. Report A-, D-, and E-optimality, nuisance-profiled information, posterior coverage, false-attribution rates, and compute separately; do not compress them into one score.

## Research connections and limits

- Mäkinen, Duncan and Helin (2026), [Batch-based Bayesian optimal experimental design in linear inverse problems](https://doi.org/10.1007/s11222-026-11022-4), gives a rigorous Bayesian formulation for a measure-relaxed batch A-optimal design and a Wasserstein gradient-flow optimizer. Its regularization and continuous relaxation are important comparators, but the relaxed result still needs conversion to a feasible discrete portfolio and measured rounding regret.
- Coons and Huan (2025), [A Multifidelity Estimator of the Expected Information Gain for Bayesian Optimal Experimental Design](https://doi.org/10.1137/25M1731812), report unbiased multifidelity EIG estimation and one-to-two-order variance reductions on their selected examples. CMNI's scalar ACV primitive is not that full estimator; reproduce the estimator before comparing.
- The 2026 AIACHNE \(^{252}\mathrm{Cf}\) PFNS measurement using \(^{12}\mathrm C\) and \(^{9}\mathrm{Be}\) elastic-scattering efficiency references is a useful independence case study: it provides a measurement route uncorrelated with earlier measurements, while also identifying deficiencies in the evaluated reference-reaction data that produced small unphysical structures. [Phys. Rev. C 114, 014604 (2026)](https://doi.org/10.1103/w7z1-l32n).
- [Dakota (the current documentation lists version 7.0.0)](https://snl-dakota.github.io/) is a broad optimization, UQ, parameter-estimation and design-of-experiments reference. [BoTorch's batch qEI/qNEI](https://botorch.readthedocs.io/en/stable/acquisition.html) is an important batch Bayesian-optimization baseline, but optimizing improvement in an objective is not equivalent to scientific parameter identifiability or common-cause-aware A/D-optimal design. Comparisons must match the decision objective, model, constraints and compute budget.

## Scientific boundary

This benchmark establishes only that a correctly represented shared Gaussian source can change a small linear-Gaussian portfolio ranking. It does not demonstrate AIACHNE parity, PFNS predictive performance, causal recovery on real measurements, a superior optimizer, a complete provenance graph, or real-world experimental validity. Those require their own preregistered datasets, independent implementations and qualification receipts.
