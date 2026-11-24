# Exact discrete mission tail-risk evaluator v0.1

**Status:** executable finite mathematical reference. Synthetic inputs only. This is not a physical navigation model, policy optimizer, calibrated probability source, safety case, or authority API.

## Implemented

The navigation_tail_risk module adds:

- Exact expected-loss calculation over integer-weighted discrete outcome distributions.
- Exact upper-tail CVaR using integer PPM confidence levels.
- Fractional inclusion of a probability atom at the quantile boundary.
- Overflow-checked arithmetic and reduced rational results.
- Overflow-safe exact rational comparison using continued fractions.
- Outer worst-case CVaR across complete, mission-persistent model distributions.
- Independent reconstruction of full mission-path loss distributions from the existing temporal observable policy tree, including no-action padding, one-stage stop branches, and contingent second actions.
- A mass-conservation check at the exact two-stage PPM denominator and a mean-loss cross-check against the existing minimax oracle output.
- Deterministic model ordering and duplicate-identity rejection.

The model aggregator evaluates a fixed policy's separate full-mission loss distribution under each candidate model and takes the maximum model-conditional CVaR. It does not average across epistemic models or allow their identity to switch inside a mission.

## Exact fixture

Two synthetic policies have these distributions:

| Policy | Distribution | Expected loss | CVaR 95% |
|---|---|---:|---:|
| A | 1% loss 1.00; 99% loss 0 | 0.01 | 0.20 |
| B | 100% loss 0.02 | 0.02 | 0.02 |

Expected loss prefers A; CVaR95 prefers B. The binary asserts the exact integer fractions rather than printing rounded floating-point values.

A second demonstration computes the outer worst-case CVaR over a benign and a faulted persistent model. The binary also reconstructs each model's full path-loss distribution from a selected temporal minimax policy, verifies its mean against the solver's exact expected-loss numerator, and computes mission-global worst-case CVaR95.

## Qualification tests

The module contains tests for:

- exact fractional tail mass;
- CVaR at alpha zero matching expectation;
- constant-loss equality;
- monotonicity in alpha;
- positive translation and scaling;
- input-order invariance;
- outer model maximum and model-set expansion;
- overflow-safe fraction comparison;
- empty, zero-mass, invalid-alpha, duplicate-model and arithmetic-overflow rejection;
- cross-check against an independently expanded integer-atom oracle on small distributions;
- full mission-path extraction for no-action and contingent multi-action policies;
- exact reconstruction of model-wise expected loss from the existing mission-minimax policy tree;
- rejection of missing actions, missing outcome branches and malformed policy trees.

## Reproduce

~~~sh
cargo test --manifest-path research/nav-observation-toy-fim/Cargo.toml navigation_tail_risk
cargo run --manifest-path research/nav-observation-toy-fim/Cargo.toml --bin navigation_tail_risk_demo
~~~

## Limitations and next step

This increment reconstructs path-loss distributions from the existing finite temporal policy tree and evaluates each fixed policy under mission-persistent models. It still does not optimize the policy tree for CVaR; the current minimax planner selects an expected-risk policy and the evaluator reports its tail risk. It does not estimate model-set coverage or prove that supplied probabilities correspond to physical navigation. The next increment should enumerate candidate policy trees and select the robust-CVaR-optimal observable policy, then independently cross-check that selection on tiny exhaustive cases.

No dataset payloads are admitted and no physical campaigns are run by this increment.
