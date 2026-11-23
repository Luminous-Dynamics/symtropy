# Finite-World Incumbent Decision Adequacy Benchmark v0.1

**Status:** synthetic reference evaluator; exact-head validation pending.  
**Owner:** [ORBITAL-INCUMBENT-DECISION-ADEQUACY-001 #7105](https://github.com/Luminous-Dynamics/symtropy/issues/7105).  
**Claim ceiling:** synthetic finite-world incumbent adequacy only.

## 1. Research question

A deployed decision should not be re-optimized simply because the data distribution moved. The right question is whether the incumbent has become materially suboptimal or infeasible under the currently admissible worlds, after accounting for switching cost.

This benchmark composes existing owners:
- #6887 supplies a current surviving-world set with its own anytime-valid evidence contract.
- #6958/#6997 supply scenario-specific feasibility, action, loss and regret semantics.
- #6996/#7013/#7001 supply action-universe closure and independent model-family challenge.
- #6595/#6598 supplies typed scope, evidence freshness and dependency invalidation.
- #7089/#7093 supplies independent evidence-dependency graph adequacy.

The new evaluator does not reproduce their semantics. It consumes an already qualified finite contract and answers only whether the incumbent remains inside a declared retention envelope.

## 2. Exact v0.1 rule: common-action worst-case improvement

For every surviving world (w), let (F(w)) be its feasible action set and (L(a,w)) the declared loss of action (a).

The comparator must not choose a different action for each hidden world. Such a per-world optimum is clairvoyant: it assumes knowledge the decision-maker does not possess. Instead derive the common feasible action set:

[
F_{\mathrm{common}}=\bigcap_{w\in W_{\mathrm{surv}}}F(w)
]

Every candidate in this set is feasible in every surviving world. For each common action, compute its worst-case loss:

[
M(a)=\max_{w\in W_{\mathrm{surv}}}L(a,w)
]

Choose a deterministic reference comparator, minimizing (M(a)) over the common set; break ties by action ID:

[
a_{\mathrm{ref}}\in\arg\min_{a\in F_{\mathrm{common}}}M(a)
]

The incumbent’s worst-case loss and common-action improvement are:

[
M_0=M(a_0),\qquad
\Delta_{\mathrm{common}}=M_0-M(a_{\mathrm{ref}})\geq0
]

The retention threshold is:

[
\tau=\epsilon_{\mathrm{materiality}}+C_{\mathrm{switch}}
]

where materiality and switching cost are separately declared, non-negative values in the same exact integer loss unit.

- **RETAIN_INCUMBENT:** only if all upstream evidence gates are current, both the action domain and dependency graph are closed within their declared grammars, at least one world survives, the incumbent is feasible in every surviving world, and (\Delta_{\mathrm{common}}\leq\tau).
- **REOPTIMIZATION_WARRANTED:** a valid closed finite model shows the incumbent infeasible in any surviving world or (\Delta_{\mathrm{common}}>\tau). This is a review trigger, not a replacement-action selection or execution command.
- **ABSTAIN:** empty surviving set, open/stale/indeterminate upstream state, inadequate model family or dependency graph, stale/contradictory evidence, scope/loss-unit mismatch, empty feasible world or incomplete loss matrix.
- **INVALID_INPUT:** malformed/duplicated identities, invalid costs/losses, unknown feasible actions, duplicate loss rows or arithmetic overflow.

The reference common action is an analytical benchmark only; it is not returned as an action recommendation. Threshold equality retains the incumbent. The evaluator never invents scenario probabilities or averages across worlds.

This v0.1 comparator is a *static non-anticipative baseline*: one action must work across all current worlds without knowing the hidden world ID. More capable observation-contingent policies belong to #6997 and must branch only on admissible observable outcomes.
## 3. Why this differs from a generic drift alarm

A KS/MMD or changepoint alarm asks whether observations changed. It does not establish that the deployed action's optimality gap changed.

RADAR (Zheng, Wiberg & Zhu, 29 August 2026) is a strong research comparator: it uses inverse optimization to infer latent preferences and tests incumbent optimality gaps under current distributions, with two-sample and sequential changepoint procedures and asymptotic Type-I error/power guarantees. The finite-world evaluator here is a deterministic companion using an explicit surviving-world set, not a replacement for RADAR and not a new statistical guarantee.

A distribution can change while the incumbent remains within the finite-world retention envelope. Conversely, a small shift can cross a feasibility or regret boundary and require review.

## 4. Frozen v0.1 corpus

The 16 cases cover:
1. Stable incumbent retention with non-zero exact regret.
2. Decision-irrelevant world changes that leave the incumbent optimal.
3. Opposite world-specific optima that would make a hindsight comparator appear beneficial, while no single common feasible action improves worst-case loss.
4. Common-action worst-case improvement strictly above the materiality-plus-switching threshold.
5. Exact equality at the threshold, which must retain.
6. Incumbent infeasibility in one surviving world, which triggers review.
7. Empty surviving-world set, which abstains.
8. Open action domain, which abstains.
9. Stale evidence, which abstains.
10. Incomplete loss matrix, which abstains.
11. Duplicate loss row, which fails closed as invalid input.
12. Scope mismatch, which abstains.
13. Inadequate model family, which abstains.

A separate typed Rust mutation test checks i64 threshold-addition overflow without losing integer exactness in JSON/JavaScript.

## 5. Baselines and comparison protocol

Compare on identical world sets, feasible sets, objective units and compute budgets:
- never re-optimize;
- fixed-calendar re-optimization;
- re-optimize on any KS/MMD or registered changepoint alert;
- RADAR-style regret-based operational-suboptimality monitoring, under its stated preference/distribution assumptions;
- finite-world common-action worst-case improvement gate;
- expected-regret trigger only where probabilities are justified and calibrated;
- exact exhaustive finite oracle on small cases;
- per-world clairvoyant comparator as an unattainable diagnostic upper bound only; it must never drive the gate.

For stochastic evaluation, preregister regime transitions, observation budgets, preference uncertainty, campaign-wise error levels, minimum worthwhile regret gap, sample size, multiplicity and stopping protocol. Never compare one method on held-out worlds against another on its training worlds.

## 6. Metrics and prospective targets

Targets, not measured achievements:
- zero false-retain outcomes when exact common-action improvement exceeds the registered threshold;
- zero retention when the incumbent is infeasible in any surviving world;
- zero retain decisions with an open action universe, stale evidence or incomplete loss matrix;
- zero false re-optimization on controls where the incumbent remains within the retention envelope;
- 100% exact finite-oracle agreement;
- deterministic input-order permutation invariance;
- zero authority grants.

For larger or stochastic comparisons, report:
- false-retain rate and false re-optimization rate;
- power at a preregistered minimum worthwhile regret gap;
- detection delay after harmful shift;
- cumulative regret while retaining the incumbent;
- incumbent infeasibility exposure time;
- number, duration and cost of switching events;
- abstention rate by cause;
- sensitivity frontier over materiality margin and switching cost;
- runtime, memory and world/action scaling.

Keep decision quality, switching cost, evidence validity and operational authority as separate outputs. Do not collapse them into a composite readiness score.

## 7. Known limitations

The evaluator trusts its supplied surviving-world set, action universe, feasible sets and losses. It cannot prove that the true world is included, that action closure is complete, that loss models match reality, or that the incumbent is physically safe. Those claims remain owned by upstream evidence producers and competent external authorities.

A positive synthetic result means only that the finite inputs satisfy the exact retention rule. It does not authorize an operation or choose/execute a replacement.
