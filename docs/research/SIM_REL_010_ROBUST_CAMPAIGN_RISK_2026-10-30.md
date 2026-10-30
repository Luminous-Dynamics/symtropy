# SIM-REL-010 — robust campaign safety under scientific model ambiguity

**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2971  
**Date:** 2026-10-30  
**Status:** research specification; no runtime implementation or qualification result is claimed.

## 1. Research question

SIM-REL-009 can calculate campaign risk exactly under one supplied prior and one supplied observation/hazard model. That exactness does not protect against a wrong prior, omitted hazard mechanism, or misspecified observation model.

SIM-REL-010 asks:

**What adaptive experiment policy remains within a declared campaign-risk budget across a defensible set of plausible scientific worlds, and what efficiency is lost by requiring robustness?**

A nominal risk calculation can be numerically exact and scientifically wrong. The objective is to quantify the gap between nominal safety and safety across plausible complete models.

## 2. Finite model-family contract

A finite model family M contains complete plausible worlds. Each world m specifies:
- a prior over shared semantic hypothesis identifiers H;
- deterministic action outcomes for the first exact slice;
- an action/hypothesis hazard map;
- positive experiment costs (shared across worlds in v1).

A campaign policy is one fixed observable-history-to-action mapping shared by every model. The latent model identity is fixed for the entire campaign. It is not revealed to the policy. For each complete policy pi and world m, independently calculate:
- C_m(pi): expected cumulative experiment cost;
- R_m(pi): probability of at least one campaign hazard;
- I_m(pi): probability of exact identification without a hazard.

All policies must use the same action at the same observable history, even when that history has different probabilities under different models. Evaluate that single policy tree in every world. The policy cannot branch on a hidden model index.

For a finite ambiguity set, report the full per-model metric vectors. At least two operating objectives must be exposed:

1. **Nominal-cost / robust-risk:** minimize C_nominal(pi), subject to max_m R_m(pi) <= alpha.
2. **Worst-case-cost / robust-risk:** minimize max_m C_m(pi), subject to max_m R_m(pi) <= alpha.

The first optimizes efficiency under a predeclared reference world while requiring safety across all models. The second is a minimax campaign objective. They answer different questions and must never be silently substituted for one another.

Report the nondominated policy set and explicit projections over (C_nominal, R_worst), (C_worst, R_worst), and identification probability. Preserve the full per-model vectors in receipts so a projection cannot hide which model is worst.

## 3. Global versus rectangular ambiguity: two different adversaries

The primary reference is **globally persistent / non-rectangular**: one complete model m governs the entire campaign. Observations may change the set of compatible models, but nature cannot switch to another physical model independently after every branch.

A separately named **rectangular** comparator permits conditional ambiguity choices at each history. This is a stronger adaptive adversary and can produce a more conservative value. It is not the same uncertainty model. Report its conservatism gap, and do not call its value the fixed-model result.

Preserve joint model coherence. Do not independently choose the worst prior, outcome likelihood and hazard map if those components are physically coupled. Each ambiguity-set element must be a coherent complete world.

If ambiguity is represented by a polytope over weights on complete worlds, the expected cost and hazard probability of a fixed policy are linear in those weights. The support-function maximum is attained at an extreme point. Start with a finite set of rational vertices; later polyhedral extensions must disclose the exact LP or approximation semantics.

## 4. Hand-checkable prior-ambiguity fixture

Use four shared semantic hypotheses h0–h3 and three binary actions:

| Action | Cost | Hazard | Safe outcome partition |
|---|---:|---|---|
| A | 1 | Hazard only under h3 | h0,h1 -> 0; h2 -> 1 |
| B | 3 | Always safe | {h0,h1} -> 0; {h2,h3} -> 1 |
| C | 1 | Always safe | {h0,h2} -> 0; {h1,h3} -> 1 |

Two plausible complete worlds share these semantics but assign different prior mass:

| Hypothesis | Nominal m0 | Alternative m1 |
|---|---:|---:|
| h0 | 0.35 | 0.20 |
| h1 | 0.30 | 0.20 |
| h2 | 0.30 | 0.30 |
| h3 | 0.05 | 0.30 |

The same adaptive policy A-then-C on safe outcome 0 yields:
- m0: expected cost 1.65 and campaign hazard probability 0.05;
- m1: expected cost 1.40 and campaign hazard probability 0.30.

Thus for alpha=0.10, the nominal selector accepts A under m0, while the robust selector must reject it because its worst-model campaign risk is 0.30.

A risk-free identification policy is C first, then A after outcome 0 and B after outcome 1:
- m0: expected cost 2.70, campaign hazard probability 0;
- m1: expected cost 3.00, campaign hazard probability 0.

This fixture tests that one shared policy is evaluated across complete models, that global model identity is fixed, and that the robust decision does not assemble an incoherent worst case from unrelated component extrema. It is a local golden comparison, not a claim that these are the complete frontier values for every possible tree.

## 5. Ambiguity-set families

Keep these uncertainty semantics distinct:

- finite expert/model-family sets (structural alternatives);
- total-variation / contamination neighborhoods around a reference prior;
- KL or other f-divergence Bayesian ambiguity sets;
- MMD / Wasserstein neighborhoods where the ground metric is defensible;
- data-calibrated set-valued uncertainty based on independent calibration evidence.

The first executable slice should use rational finite ambiguity vertices. Later continuous ambiguity sets can use an exact support-function / LP solver where available, otherwise report approximation error, optimization gap, and resource caps.

The research question is not simply how to make the ambiguity set large. It is whether one can achieve adequate held-out coverage without making every useful experiment infeasible. Measure ambiguity-set coverage and resulting decision conservatism together.

## 6. Required solver and correctness gates

1. On sealed tiny domains (4–8 hypotheses, 3–4 actions, 2–4 complete models), compare the exact policy frontier against independent exhaustive policy-tree enumeration.
2. Evaluate one shared policy tree under every model; the chooser must never receive the evaluator's model identity.
3. Preserve prior / outcome / hazard correlation. Do not combine incompatible component extrema.
4. Verify fixed-model global consistency: the model index cannot switch after observations. Verify rectangular semantics independently.
5. Check worst-case risk equals max_m R_m, worst-case cost equals max_m C_m, and nominal objectives agree with independent per-world replay.
6. Verify alpha monotonicity, frontier dominance, branch probability normalization, expected cost and hazard accounting.
7. Check exact identification only on singleton non-hazard terminals; hazard or unresolved paths are not successful identification.
8. Check input-order invariance, deterministic tie-breaking, duplicate/malformed input rejection, checked arithmetic, and fail-closed state/frontier/vertex caps.
9. Add out-of-set negative controls. If the true world is outside M, the system must not claim the conditional risk guarantee applies; report ambiguity-set miss rate separately.
10. The API accepts no true-world label or evaluator model identity. Keep sealed world truth in the independent evaluator.

## 7. Benchmark programme

### Matched baselines

- nominal SIM-REL-009 frontier;
- TV / contamination-ball robust risk;
- posterior-informed DRO-BAS-style ambiguity sets when the assumptions and divergence match;
- robust Bayesian ambiguity sets / robust posterior predictive alternatives;
- fixed safe policy and hard robust safe-only policy;
- conformal risk-controlled policy selection when reference-policy data and exchangeability conditions are available;
- rectangular robust dynamic policy as a separate higher-conservatism comparator;
- continuous safe Bayesian optimization methods only on matched continuous tasks with equivalent feedback and campaign-risk semantics.

### Predeclared stress panel

Sweep ambiguity radius / contamination in {0, 0.01, 0.05, 0.10, 0.20, 0.40}, campaign risk alpha in {0, 0.01, 0.05, 0.10, 0.20}, at least three structural family shifts, multiple prior shifts, and multiple action-cost schedules.

Separate calibration, ambiguity construction, tuning and sealed evaluation families. Include:
- true world inside the ambiguity set;
- true world outside the ambiguity set;
- prior shift with unchanged mechanism;
- hazard-map shift with unchanged prior;
- observation-mechanism shift;
- coupled shift in prior, outcomes and hazards;
- overly broad ambiguity set (safe but operationally useless);
- overly narrow ambiguity set (apparently efficient but misses reality);
- fixed global model versus history-wise rectangular adversary.

### Required metrics

- nominal and worst-case campaign risk;
- false-safe certification / risk-budget violations on held-out true worlds;
- ambiguity-set coverage and miss rate;
- nominal, worst-case and per-world expected experiment cost;
- exact identification probability and no-feasible-policy rate;
- risk–cost–coverage frontier and conservatism gap versus nominal Bayes;
- regret to fixed-model and rectangular exact references separately;
- runtime, memory, number of model vertices, policy/frontier states and approximation gap.

Report conditional safety (the true model belongs to M) separately from empirical performance under an external held-out-world generator. An exact guarantee conditional on M is not an unconditional guarantee that M contains reality.

## 8. Proposed qualification targets (targets, not results)

- 100% agreement with independent exhaustive robust-tree enumeration on all sealed tiny instances.
- Zero campaign-risk-budget violations for every evaluated world inside the declared finite ambiguity set in the exact deterministic harness.
- Explicitly withhold the guarantee when the true world is outside the ambiguity set.
- 100% consistency of global model identity across each fixed-model campaign.
- Paired calibration / held-out evaluation that reports ambiguity coverage, robust risk and experiment cost together.
- Target at least 10% lower worst-case cost than hard robust safe-only selection at matched robust risk and identification probability on at least two predeclared held-out families, with paired uncertainty intervals excluding zero. This is an ambition, not an observed result.

## 9. Relation to existing SIM-REL work

- SIM-REL-004 [issue #2921]: open-world model adequacy and OOD posterior; supplies candidate structural alternatives but does not itself calibrate a robust hazard ambiguity set.
- SIM-REL-005 [PR #2928] and SIM-REL-007 [PR #2941]: exact finite expected and worst-case deterministic experiment-cost references.
- SIM-REL-006 [PR #2930]: stop / abstain / act loss.
- SIM-REL-008 [issue #2944]: noisy and persistent measurement likelihoods.
- SIM-REL-009 [issue #2958] / [PR #2959]: nominal-model campaign risk–cost frontier.

Keep SIM-REL-010 as a separate dependency-free Rust reference crate and receipt schema. Integrate it with SIM-REL-009 only through a benchmark adapter after independent qualification.

## 10. External research context and fair comparison

- Dellaporta, O'Hara and Damoulas, *Decision-Making Under Model Misspecification: DRO with Robust Bayesian Ambiguity Sets*, Entropy 28(4), 430 (2026): https://doi.org/10.3390/e28040430. Relevant for robust posterior predictive and MMD-based ambiguity under misspecification; it is not itself an adaptive experiment-tree algorithm.
- Dellaporta, O'Hara and Damoulas, *Decision Making under the Exponential Family: Distributionally Robust Optimisation with Bayesian Ambiguity Sets*, ICML / PMLR 267 (2025): https://proceedings.mlr.press/v267/dellaporta25a.html. Relevant for posterior-informed ambiguity sets and tractable DRO formulations.
- Prinster et al., *Conformal Policy Control*, ICML / PMLR 306 (2026): https://proceedings.mlr.press/v306/prinster26a.html. Relevant as a finite-sample risk-tolerance policy-control baseline using data from a safe reference policy; its guarantee assumptions must be checked before transfer.
- Bottero et al., *Information-Theoretic Safe Bayesian Optimization*, JMLR 27 (2026): https://www.jmlr.org/beta/papers/v27/23-1687.html. Relevant continuous safe-exploration baseline.
- Tokmak, Schon and Baumann, *Safe Bayesian optimization across noise models via scenario programming*, ACC 2026: https://ieeexplore.ieee.org/document/11615536. Relevant for heteroscedastic and heavy-tailed measurement-noise assumptions.

These methods solve different problems. Do not claim superiority until action space, safety feedback, data budget, risk unit, ambiguity coverage and compute budgets are matched.

## 11. Claim boundary

The reference is exact only for its declared finite model family or ambiguity polytope, coherent joint-world semantics, action set, policy class, cost definition and campaign-risk metric. It does not guarantee that the true physical system belongs to the ambiguity set, validate hazard maps, or establish real-world safety. Any external guarantee is conditional on demonstrated ambiguity-set coverage and its stated assumptions. Physical deployment requires independently validated hazard envelopes, open-world controls and external safety review.
