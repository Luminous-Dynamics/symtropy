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

## 2. Exact v0.1 rule

For every surviving world (w), let (F(w)) be its feasible action set and (L(a,w)) the declared loss of action (a).

For incumbent (a_0):

[
R(a_0,w)=L(a_0,w)-min_{ain F(w)}L(a,w)
]

[
R_{max}(a_0)=max_{win W_{mathrm{surv}}}R(a_0,w)
]

The retention threshold is:

[
	au=epsilon_{mathrm{materiality}}+C_{mathrm{switch}}
]

where (epsilon_{mathrm{materiality}}) and (C_{mathrm{switch}}) are separately declared, non-negative values in the same exact integer loss unit.

- **RETAIN_INCUMBENT:** only if all upstream gates are current, the action universe is closed within its declared scope, at least one surviving world exists, the incumbent is feasible in every surviving world, and (R_{max}(a_0)leq	au).
- **REOPTIMIZATION_WARRANTED:** a valid closed finite model shows the incumbent infeasible in any surviving world or (R_{max}(a_0)>	au). This is a review trigger, not a replacement-action selection or execution command.
- **ABSTAIN:** empty surviving set, open/stale/indeterminate upstream state, inadequate model family, stale/contradictory evidence, scope mismatch, empty feasible world, or incomplete loss matrix.
- **INVALID_INPUT:** malformed/duplicated identities, invalid costs/losses, unknown feasible actions, duplicate loss rows or arithmetic overflow.

Threshold equality retains the incumbent. The evaluator never invents scenario probabilities or averages across worlds.

## 3. Why this differs from a generic drift alarm

A KS/MMD or changepoint alarm asks whether observations changed. It does not establish that the deployed action's optimality gap changed.

RADAR (Zheng, Wiberg & Zhu, 29 August 2026) is a strong research comparator: it uses inverse optimization to infer latent preferences and tests incumbent optimality gaps under current distributions, with two-sample and sequential changepoint procedures and asymptotic Type-I error/power guarantees. The finite-world evaluator here is a deterministic companion using an explicit surviving-world set, not a replacement for RADAR and not a new statistical guarantee.

A distribution can change while the incumbent remains within the finite-world retention envelope. Conversely, a small shift can cross a feasibility or regret boundary and require review.

## 4. Frozen v0.1 corpus

The 13 cases cover:
1. Stable incumbent retention with non-zero exact regret.
2. Decision-irrelevant world changes that leave the incumbent optimal.
3. Different world-specific optimal actions while incumbent regret remains within the retention margin.
4. Worst-case regret strictly above the materiality-plus-switching threshold.
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
- finite-world worst-case regret retention gate;
- expected-regret trigger only where probabilities are justified and calibrated;
- exact exhaustive finite oracle on small cases;
- clairvoyant selector/trigger as unattainable upper bound only.

For stochastic evaluation, preregister regime transitions, observation budgets, preference uncertainty, campaign-wise error levels, minimum worthwhile regret gap, sample size, multiplicity and stopping protocol. Never compare one method on held-out worlds against another on its training worlds.

## 6. Metrics and prospective targets

Targets, not measured achievements:
- zero false-retain outcomes when exact finite oracle regret exceeds the registered threshold;
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
