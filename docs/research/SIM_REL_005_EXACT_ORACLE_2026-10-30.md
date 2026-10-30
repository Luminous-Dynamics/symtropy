# SIM-REL-005 — exact adaptive decision-tree oracle

**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2923  
**Date:** 2026-10-30

## Scope

A finite-hypothesis, deterministic-outcome, positive-action-cost benchmark has an exact Bayes-optimal adaptive decision tree. This crate implements its Bellman recursion with memoized survivor sets.

Given a current survivor set S, prior masses w(h), and intervention a with cost c(a), the exact continuation value is:

V(S) = min_a [ c(a) + W(S0)/W(S) V(S0) + W(S1)/W(S) V(S1) ],

where Sy contains hypotheses whose deterministic output at a is y, and W(S) = sum over h in S of w(h). A singleton is an identified terminal state with value zero. Actions that do not divide the survivor set cannot reduce uncertainty and are not considered.

## Why this is useful

The exact oracle is a theoretical lower bound on the expected experiment cost of any adaptive policy under the same finite semantic hypothesis support, prior, deterministic binary observation model, fixed positive action-cost vector and exact-identification terminal condition.

It is not a practical competitor to beat, nor a lower bound for other priors, larger hypothesis support, measurement noise, continuous controls, changing experiment costs, physical model discrepancy, or an alternate stopping utility.

## Implementation

The dependency-free Rust library:
- validates unique semantic signatures, finite positive prior masses, and positive integer action costs;
- sorts signatures before memoizing to make decisions independent of input order;
- memoizes exact survivor-set states;
- produces a shared decision tree with conditional expected cost at every node;
- breaks exact cost ties in favor of the lower action index;
- fails closed when the configured state budget is exceeded.

The solver API intentionally receives no evaluator target. A caller may use the root action, observe the actual system, filter survivors, and continue. The oracle itself conditions only on its supplied prior and hypothesis support.

## Independent correctness gates

The test suite compares memoized DP to an independently written non-memoized brute-force recursion on every nontrivial subset of the eight functions over three input bits (all 255 subsets, excluding the empty set and singletons). Additional tests cover exact cost on the complete 3-bit cube with uniform action costs; weighted priors and invariance to common prior scaling; input-order invariance and deterministic first actions; Bellman optimality against every admissible first action; duplicate, invalid prior and zero-cost rejection; fail-closed state-budget exhaustion; and absence of evaluator-target arguments in the API.

## Planned integration

After isolated qualification, integrate the oracle as a benchmark-only comparator in SIM-REL-003:
1. derive the same 109 depth-two semantic candidates and beta-tempered MDL weights from the SIM-REL-003 grammar;
2. compute an exact expected oracle tree for each distinct action-cost vector and beta;
3. compare each policy's realized in-grammar cost with the prior-expected optimum, without pretending the expected oracle cost is an instance-specific clairvoyant lower bound;
4. keep OOD cases separate until SIM-REL-004 defines a full-support prior;
5. report expected-cost gap, absolute cost difference, and policy-tree first action separately.

A future open-world oracle may use all 256 truth tables with an explicit contamination prior. Do not mix that oracle with the 109-hypothesis closed grammar.

## Claim ceiling

An exact solution on this finite model validates the decision-tree algorithm and provides a rigorous benchmark reference for that model only. It does not establish better-than-industry discovery, SOTA superiority, physical-system transfer, or a universal grammar.
