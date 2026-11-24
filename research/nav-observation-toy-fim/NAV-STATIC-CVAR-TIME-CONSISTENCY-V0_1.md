# Static mission CVaR and receding-horizon replanning: exact counterexample v0.1

**Status:** exact finite mathematical counterexample and test fixture. Synthetic integer probabilities / losses only. This does not implement a nested-CVaR controller or establish physical navigation risk.

## Research question

Does a policy that is optimal for static CVaR of the complete mission loss remain optimal after an observation arrives and the controller recomputes a conditional CVaR plan?

In general, no. Static trajectory CVaR is not automatically time-consistent under receding-horizon replanning. The chosen root policy may optimize its unconditional full-path distribution, while a fresh conditional optimization after a branch is observed may select a different continuation.

This is not a numerical bug. It is a difference between risk semantics.

## Exact two-stage construction

The first observation distinguishes two branches:

- H occurs with 1% probability.
- L occurs with 99% probability.

On branch L, a common exogenous mission event produces loss 300 with 4% total mission probability; otherwise loss is zero. On branch H, the controller chooses one continuation:

- **Action A:** conditional loss 200 with 5% probability, otherwise 150.
- **Action B:** constant conditional loss 180.

All H-branch losses are below the 300-loss L event. The upper 5% of the full mission distribution therefore contains the 4% catastrophic L mass plus the entire 1% H branch.

### Full-mission distributions

| Loss | Root policy A mass | Root policy B mass |
|---:|---:|---:|
| 300 | 4.00% | 4.00% |
| 200 | 0.05% | 0 |
| 180 | 0 | 1.00% |
| 150 | 0.95% | 0 |
| 0 | 95.00% | 95.00% |

### Exact CVaR95 results

For root policy A:

\[
\operatorname{CVaR}_{0.95}(A)
=
\frac{0.04(300)+0.0005(200)+0.0095(150)}{0.05}
=270.5.
\]

For root policy B:

\[
\operatorname{CVaR}_{0.95}(B)
=
\frac{0.04(300)+0.01(180)}{0.05}
=276.
\]

The static root objective selects A because \(270.5 < 276\).

But after H is observed, the conditional CVaR values are:

| H-branch continuation | Conditional mean | Conditional CVaR95 |
|---|---:|---:|
| A | 152.5 | 200 |
| B | 180 | 180 |

A fresh conditional CVaR optimization selects B because \(180 < 200\).

If the controller changes from A to B after observing H, the resulting full-mission distribution becomes root policy B, whose original root CVaR rises from 270.5 to 276.

## Executable test

The Rust test named static_root_cvar_and_conditional_replanning_can_prefer_opposite_actions in navigation_tail_risk.rs checks all four exact rational results:

- root A: 541/2;
- root B: 276/1;
- conditional H-A: 200/1;
- conditional H-B: 180/1.

The fixture uses integer probability weights with total mass 2,000, so no floating-point arithmetic or empirical sampling is involved.

## Consequences for the navigation architecture

Do not claim that the current static finite root-policy CVaR objective is automatically safe under repeated replanning.

Three legitimate deployment choices should be compared:

1. **Commit to the root policy tree:** follow the selected observable continuation after each branch, unless an independent safety/authority gate forces interruption.
2. **Use a time-consistent nested conditional risk measure:** re-optimize with an explicitly defined conditional risk envelope and information filtration.
3. **Carry a remaining risk budget / risk state:** make the mission-level risk allocation part of the controller state and decrement it as evidence and decisions unfold.

The third option is a promising research direction, but the risk-budget state itself must have a justified update law; simply subtracting per-step CVaR values is not a general valid composition rule.

## Required benchmark expansion

Compare static mission CVaR, nested / iterated conditional CVaR, belief-state immediate-cost CVaR plus expected future return, and expected minimax. For each, report:

- root objective value;
- conditional branch objective values;
- selected continuation before and after replanning;
- resulting full-mission loss distribution after replanning;
- time-consistency / risk-budget violation;
- false-safe / false-degraded exposure, abstention, valid coverage and resources.

Keep the persistent mission model fixed across every full path. Do not conflate static-versus-dynamic risk semantics with the separate question of whether an adversary can switch models at each branch.

## Claim boundary

This is an exact synthetic counterexample to automatic time consistency of static CVaR under conditional replanning. It is not a navigation performance result, physical risk estimate, proof of any nested-CVaR algorithm, or safety approval.
