# SIM-REL-007 — prior-free minimax decision-tree comparator

**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2941  
**Parent implementation:** https://github.com/Luminous-Dynamics/symtropy/pull/2928  
**Date:** 2026-10-30

## Research question

Does a selector that performs well under one assumed prior remain acceptable when we stop trusting that prior? The SIM-REL-005 Bayes oracle gives the exact expected-cost optimum for a supplied prior. This companion computes the exact worst-case optimum over the same finite deterministic hypothesis support without using a prior.

## Mathematical contract

For survivor set S, positive integer intervention costs c(a), and deterministic binary signatures:

- M(S) = 0 when |S| <= 1;
- otherwise M(S) = min over splitting actions a of [c(a) + max(M(S0), M(S1))].

The solver returns the prior-free minimum worst-case cost, first action, memoized-state count, and a separate decision tree. Costs are accumulated as u64 integers. Exact ties select the lower action index.

The public API accepts signatures and costs only. It does not accept an evaluator target or hypothesis prior. It rejects empty or duplicate signatures, zero costs, and state-budget exhaustion.

## Why this is a useful second oracle

Bayes-optimal expected cost and minimax worst-case cost answer different operational questions. A cheap expected policy may have an unacceptable long tail; a minimax policy can protect the tail but pay more on typical cases. Reporting only one number hides this trade-off.

The minimax value is a worst-case benchmark **within the supplied support**. It does not protect against omitted hypotheses, noisy measurements, continuous controls, nonstationary costs, or a wrong observation model. Open-world contamination and noisy outcomes therefore remain separate extensions, not claims smuggled into this API.

## Correctness gates

The implementation is a separate memoized Bellman recursion and tree type. Its independent non-memoized reference checks all 247 nontrivial subsets of eight selected signatures under heterogeneous action costs. The complete 256-function truth-table cube checks that worst-case identification cost equals the sum of all eight action costs, and that the dynamic program visits 6,305 nonterminal partial-observation states.

Additional tests cover input-order invariance, exact integer tie-breaking, singleton terminal states, invalid inputs, fail-closed state budgets, path-cost upper bounds, and the relationship between the Bayes optimum, the expected cost of a minimax policy under a test distribution, and the minimax worst-case bound.

## External benchmark context

Optimal Decision Tree identification is a classical cost-sensitive problem. It is NP-hard to approximate within a factor better than ln(m) in the general formulation; a 2025 paper gives a simple 8 ln(m)-approximation, illustrating why exact dynamic programming is a small-domain oracle rather than a scalable industry solver ([Zhuo & Nagarajan, 2025](https://www.sciencedirect.com/science/article/pii/S0167637725001312)). Work on noisy outcomes has a different model and studies approximation guarantees for Optimal Decision Tree / Adaptive Submodular Ranking with Noise ([Jia et al., JMLR 2024](https://jmlr.org/papers/v25/23-1484.html)).

For physical experimental optimization, GOLLuM (Nature Machine Intelligence, 2026) reports evaluation across 23 chemistry/materials/process tasks with 10 initial observations and a 50-experiment budget; the paper reports 36.3% top-5% coverage and over 40% fewer experiments to match a traditional Bayesian-optimization baseline ([paper](https://doi.org/10.1038/s42256-026-01283-z)). That is not directly comparable to exact identification in an 8-action Boolean cube. It is the appropriate next-tier external benchmark family once SIM-REL moves from finite hypothesis identification to noisy, continuous, expensive experiments.

A 2026 expected-depth decision-tree study also reinforces that average path cost and worst-case structural complexity are distinct deployment criteria, especially under heterogeneous query costs ([Scientific Reports 2026](https://doi.org/10.1038/s41598-026-55215-3)). Its classification-tree selection metric is not the same as this exact hypothesis-identification oracle.

## Promotion targets

These are proposed gates, not results:

1. Exact oracle agreement with an independent implementation on all exhaustively enumerable small supports.
2. Zero invalid-state acceptance: malformed support, duplicate signatures, zero costs, and exceeded state caps must fail closed.
3. Publish both expected-cost and worst-case-cost distributions for every policy; do not collapse them into one score.
4. Report prior sensitivity across beta = 0, 1, 2 and explicit OOD/contamination settings from SIM-REL-004.
5. On physical/noisy benchmarks, compare with random, fixed-order, max-disagreement, uniform EIG/cost, MDL-EIG/cost, cost-sensitive greedy, Bayesian optimization, and a published uncertainty-aware optimizer. Use identical observations, experiment budgets, cost accounting, and seed sets.
6. Use at least 20 seeds where randomness exists, paired uncertainty intervals, calibration and abstention metrics, and independent held-out task families. A 15% cost reduction is a useful research target only if quality, calibration, and worst-case failure rates do not regress.

## Claim ceiling

This crate establishes an exact minimax reference for the finite deterministic model. It does not establish physical robustness, scalable optimality, superiority to SOTA, or transfer to continuous/noisy experiments.
