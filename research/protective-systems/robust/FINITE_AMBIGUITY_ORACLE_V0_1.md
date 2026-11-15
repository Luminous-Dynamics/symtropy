# Finite ambiguity-set robust-design oracle v0.1

**Owner:** PROTECT-ROBUST-001 (#5317)  
**Evidence class:** synthetic  
**Claim ceiling:** exact finite decision semantics only

## Decision problem

Three protective design candidates are evaluated across three fixed synthetic scenarios: nominal, compound hazard and distribution shift. The evaluator receives two admissible scenario distributions:
- a historical-like distribution `[0.90, 0.10, 0.00]`;
- a shifted-regime distribution `[0.50, 0.25, 0.25]`.

Each design declares its loss by scenario and whether every domain-specific hard safety floor is met in that scenario. A design that fails any required hard floor is excluded, regardless of a low mean or tail-loss score.

For each remaining design, the oracle computes:
- expected loss under each distribution in the ambiguity set;
- worst expected loss over that set;
- upper-tail CVaR at alpha = 0.80 for each distribution;
- worst CVaR over the ambiguity set;
- maximum scenario loss.

The selection rule is lexicographic: worst-case expected loss, worst-case CVaR, maximum scenario loss, then stable design ID. This is a deliberately transparent finite oracle, not a universal utility function or claim that one risk measure is always correct.

## Hand-checkable results

| Design | Losses (nominal / compound / shifted) | Hard-floor failure? | Worst expected loss | Worst CVaR 80% |
|---|---|---|---:|---:|
| NOMINAL_OPTIMUM | 1 / 10 / 18 | No | 7.50 | 18.00 |
| ROBUST_DESIGN | 3 / 5 / 8 | No | 4.75 | 8.00 |
| CHEAP_FRAGILE_DESIGN | 0.5 / 1 / 0.5 | Yes: shifted | excluded | excluded |

Expected selected design: **ROBUST_DESIGN**.

The CVaR implementation uses the exact upper tail mass (1 - alpha), including a fractional part of the probability atom at the tail boundary. This matters for discrete scenario sets.

## Negative controls

- A cheap design with a hard-floor failure is excluded even when its numerical losses look attractive.
- An incomplete uncertainty set yields `ABSTAIN_INCOMPLETE_UNCERTAINTY_SET`; omitted hazards are not assigned zero probability.
- If every design violates a required hard floor, the result is `NO_ROBUST_FEASIBLE_DESIGN`, not a least-bad PASS.
- Probability vectors that are negative, non-finite or do not sum to one are rejected.

## What this does not prove

This is not an empirical hazard distribution, climate model, vehicle crash model, human protection model or building-physics simulation. It verifies only exact arithmetic and finite decision semantics. Later versions must compare nominal optimization, min-max robustness, stochastic optimization, CVaR, distributionally robust optimization, variance-constrained robust BO, NSGA-II and expert baselines under matched information and compute budgets.

The Rust report also carries a SHA-256 digest of the canonical serialized problem input, so the selected disposition can be tied to the exact finite scenario set, ambiguity distributions, design losses and hard-floor declarations that were evaluated.
