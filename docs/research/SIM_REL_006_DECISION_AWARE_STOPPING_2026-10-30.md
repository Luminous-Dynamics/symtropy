# SIM-REL-006 — decision-aware stopping and abstention

**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2930  
**Implementation branch:** `research/sim-rel-006-decision-aware-stopping`  
**Date:** 2026-10-30

## Research shift

Information gain and exact hypothesis identification are proxies. In deployed scientific and engineering systems, the real question is whether another experiment changes the quality of a downstream decision enough to justify its cost.

This first executable slice solves a deterministic finite-horizon version of:

`V_t(b) = min { L_stop(b), L_abstain, min_a [ c(a) + sum_y p(y|b,a) V_(t-1)(b_(a,y)) ] }`

The terminal decision is a finite class label. With zero-one class loss, stopping incurs the posterior probability of choosing the wrong class, `1 - max_c P(c|b)`. Abstention incurs an explicit, declared fallback/human-review loss. Experiments incur positive costs in the same scalar utility units.

## Implemented model

- Eight deterministic binary actions encoded by an 8-bit semantic signature.
- Finite hypotheses, each with a decision class and positive prior mass.
- Finite remaining experiment horizon, capped at eight actions.
- Positive finite action costs and a nonnegative abstention loss.
- Memoized Bellman recursion over sorted survivor support and remaining horizon.
- Policy tree nodes are exactly one of: stop-and-choose-class, abstain, or experiment with two outcome branches.
- Expected experiment cost, decision error, abstention loss and abstention probability are reported separately.
- The selector API receives no evaluator target.
- Duplicate signatures, invalid prior masses/costs and state-budget exhaustion fail closed.

The implementation is an exact finite-model solver, not a learned acquisition heuristic. Cost and risk inputs must be calibrated into the same utility scale before the result has operational meaning.

## Independent correctness gates

The test suite compares memoized recursion with a separate non-memoized reference over all 247 nontrivial subsets of eight hypotheses. It also checks decision-vs-abstention-vs-query choices, finite horizon, same-decision-class hypotheses that need not be distinguished, prior scaling, input-order invariance, deterministic tie-breaking, risk decomposition, malformed inputs and fail-closed state limits.

## Why this is a paradigm step

A query can have high information gain while having no value for the downstream decision. Conversely, a single cheap query can be worthwhile when it prevents a high-cost wrong action. The correct objective is expected decision loss plus experimental expenditure and fallback burden—not information gain by itself and not universal exact identification.

This also gives a principled connection between:
- SIM-REL-003: MDL/EIG acquisition heuristics;
- SIM-REL-005: exact expected-cost identification;
- SIM-REL-007: prior-free worst-case identification;
- SIM-REL-004: explicit model-misspecification mass;
- SIM-REL-008: noisy and persistent observation likelihoods.

The deterministic solver is the first rung. SIM-REL-008 must replace survivor elimination with proper likelihood updates, and decision quality must be evaluated under held-out generating processes and calibration checks.

## Proposed benchmark targets (not claimed results)

1. Exact agreement with an independent finite-horizon reference over every small-domain support and declared loss/cost configuration.
2. Zero false terminal actions in adversarial negative controls where abstention is optimal.
3. Zero unnecessary queries when all remaining hypotheses imply the same downstream decision.
4. Lower expected total decision loss than EIG-only, fixed, random and max-disagreement baselines under predeclared loss matrices.
5. Report query cost/count, wrong-action probability, abstention rate, risk-coverage, Brier/log score once noise is introduced, calibration by budget, and regret to the exact oracle.
6. Require paired uncertainty intervals and at least 100 independently generated worlds with family-level heldouts before physical-transfer claims.

## External comparison

Sequential Bayesian design with an explicit stopping action is studied by Cheng & Huan (2025 preprint, revised June 2026): https://arxiv.org/abs/2509.21734. This finite implementation is a small deterministic oracle for that decision framing; it does not reproduce the paper's general design model.

For eventual noisy/expensive physical experiments, GOLLuM (Nature Machine Intelligence, 2026) evaluated 23 chemistry/materials/process optimization tasks with 10 initial observations, 50 experiment budgets and 20 seeds. It reported 36.3% top-5% coverage and over 40% fewer experiments to match a traditional Bayesian-optimization baseline. That is a distinct optimization objective, not an apples-to-apples result against this Boolean identification solver. It is a suitable later benchmark family once likelihood-based continuous experimental design exists.

## Claim boundary

The solver is exact only for its finite support, deterministic observations, specified prior, fixed costs, zero-one decision loss, abstention loss and finite horizon. It does not establish calibration under model misspecification, noisy physical experiments, SOTA superiority, or operational safety. Critical actions still need independent verification and a fail-safe handoff.
