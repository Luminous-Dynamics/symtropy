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
- Deterministic model ordering and duplicate-identity rejection.

The model aggregator evaluates a fixed policy's separate full-mission loss distribution under each candidate model and takes the maximum model-conditional CVaR. It does not average across epistemic models or allow their identity to switch inside a mission.

## Exact fixture

Two synthetic policies have these distributions:

| Policy | Distribution | Expected loss | CVaR 95% |
|---|---|---:|---:|
| A | 1% loss 1.00; 99% loss 0 | 0.01 | 0.20 |
| B | 100% loss 0.02 | 0.02 | 0.02 |

Expected loss prefers A; CVaR95 prefers B. The binary asserts the exact integer fractions rather than printing rounded floating-point values.

A second demonstration computes the outer worst-case CVaR over a benign and a faulted persistent model.

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
- cross-check against an independently expanded integer-atom oracle on small distributions.

## Reproduce

~~~sh
cargo test --manifest-path research/nav-observation-toy-fim/Cargo.toml navigation_tail_risk
cargo run --manifest-path research/nav-observation-toy-fim/Cargo.toml --bin navigation_tail_risk_demo
~~~

## Limitations and next step

This is an evaluator for distributions supplied by the caller. It does not generate path probabilities from the temporal minimax policy tree, optimize actions for CVaR, estimate model-set coverage, or prove that input probabilities correspond to physical navigation. The next increment should connect complete path-loss distributions from the existing observable policy-tree enumerator, then independently cross-check robust policy selection on tiny exhaustive cases.

No dataset payloads are admitted and no physical campaigns are run by this increment.
