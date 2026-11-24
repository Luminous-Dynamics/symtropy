# Temporal robust CVaR policy optimizer v0.2

**Status:** executable finite mathematical reference, synthetic inputs only. This is not a physical navigation result, calibrated model set, safety argument, or authority API.

## Implemented

Building on the exact discrete tail evaluator, this increment adds a separate robust-CVaR policy optimizer over the existing horizon-two observable policy family.

- Enumerates the same finite non-anticipative policy trees as mission-global minimax expected risk.
- Reconstructs complete stochastic mission-path loss distributions independently for each fixed mission model.
- Evaluates CVaR within each model, then selects the policy minimizing the outer worst-case model-conditional CVaR.
- Uses expected-risk ordering as a deterministic tie-break only when exact CVaR values are equal.
- Preserves the expected-risk, minimax-regret, and rectangular-switching objectives as separate methods.
- Does not permit a policy to observe the persistent model or common-cause identity.
- Keeps authority transitions outside the planner.

The optimizer is bounded by the existing four-action / 70,306 policy-tree envelope. The exact arithmetic remains checked integer PPM and rational arithmetic; an unrepresentable tail calculation fails closed.

## Exact policy-ranking reversal

The runnable fixture constructs two observations with the same action and resource constraints:

| Policy action | False-safe | False-degraded | Expected loss | CVaR95 |
|---|---:|---:|---:|---:|
| A: rare catastrophic | 900 ppm at loss 100 | 50 ppm at loss 1 | 0.09005 | 1.801 |
| B: steady small | 0 ppm at loss 100 | 100,000 ppm at loss 1 | 0.1 | 1.0 |

Expected-loss minimax selects A. Robust CVaR95 selects B. The same ranking reversal is reproduced by the policy-tree optimizer using the exact terminal loss distribution under both persistent mission models.

These are deliberately constructed finite probabilities and losses, not measured UWB failure frequencies or empirical risk estimates.

## Qualification tests authored

- All CVaR evaluator metamorphic and malformed-input tests from v0.1.
- Complete path-loss reconstruction for no-action and contingent second-action policies.
- Exact equality between reconstructed expected loss and the existing minimax oracle's per-model risk numerators.
- A policy-ranking reversal where expected-risk and robust-CVaR select different first sensors.
- Candidate-action order invariance of robust-CVaR objective and selected action.

Exact-head CI is still required. Tests authored are not reported as passed until the relevant workflow is complete.

## Reproduce

~~~sh
cargo test --manifest-path research/nav-observation-toy-fim/Cargo.toml cvar_policy_tests
cargo test --manifest-path research/nav-observation-toy-fim/Cargo.toml navigation_tail_risk
cargo run --manifest-path research/nav-observation-toy-fim/Cargo.toml --bin navigation_temporal_cvar_demo
~~~

## Claim boundary and next work

This v0.2 optimizer selects only within the current finite horizon-two policy family and supplied finite candidate models. It does not infer transition-model coverage, calibrate probabilities, address continuous pose state, or establish physical navigation integrity. The next scale-up comparator is an explicitly observable finite-state controller distilled from a recurrent memory policy, independently evaluated against this exact oracle on tractable instances and against persistent joint models at larger scale.
