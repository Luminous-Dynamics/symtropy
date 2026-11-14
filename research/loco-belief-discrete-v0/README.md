# LOCO Belief Discrete V0

Dependency-free Rust reference package for finite-hypothesis Bayesian updates and robustly admissible probe filtering.

Research parent: [LOCO-BELIEF-001 / #4100](https://github.com/Luminous-Dynamics/symtropy/issues/4100).

## Scope

V0 implements only two narrow primitives:

1. Exact discrete Bayesian updates over a finite set of named hypotheses.
2. Deterministic filtering of candidate probes against a separately supplied set of robustly admissible action IDs.

It does not implement a general POMDP solver, sensor fusion system, calibration method, information-gain optimizer, contact model, viability solver, or physical controller.

## Arithmetic contract

A belief is represented by non-negative integer weights. Its probability for hypothesis h is exactly:

    P(h) = weight(h) / sum_j weight(j)

Common factors are removed for canonical representation. A likelihood update multiplies prior and likelihood weights with checked u128 arithmetic, removes a common factor and checks the total mass. Overflow returns an error; it is never wrapped, saturated or silently converted to floating point.

The caller supplies a likelihood for every hypothesis. Likelihood weights for one observation must represent the joint observation model. Do not multiply correlated sensor likelihoods under an unverified conditional-independence assumption.

This is exact arithmetic for the supplied finite model, not evidence that the likelihood model is calibrated or physically correct.

## Robust probe gate

robustly_admissible_probe_ids(candidate_ids, certified_action_ids) returns only the intersection. It cannot create a certificate. The allow-list must come from the independent robust viability path (#4061 consuming a sound abstraction from #4083). Candidate probes outside the allow-list are excluded, including informative probes.

This package deliberately does not rank candidates by information gain. First qualify posterior arithmetic, the admissibility boundary and evidence provenance; then add an information objective without weakening the robust gate.

## Included tests

- Hand-calculated exact posterior (prior 2:1, likelihood 3:1, posterior 6:1).
- Common-factor invariance.
- Input permutation invariance.
- Zero-likelihood hypothesis elimination.
- Impossible observation rejection.
- Missing and unknown likelihood rejection.
- Duplicate hypothesis rejection.
- Zero-prior rejection.
- Checked arithmetic overflow.
- Robust probe allow-list intersection.
- Empty allow-list behavior.
- Duplicate probe rejection.

## Verification

    cargo fmt --manifest-path research/loco-belief-discrete-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-belief-discrete-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-belief-discrete-v0/Cargo.toml --locked --all-targets -- -D warnings

A dedicated exact-head workflow is defined in .github/workflows/loco-belief-discrete.yml. No pass is claimed until all three commands complete successfully on the exact PR head.

## Claim ceiling

Tests qualify the implementation's finite integer arithmetic and probe-filter behavior only. They do not establish sensor calibration, Bayesian model validity, robustness under distribution shift, contact feasibility, continuous/hybrid reachability, or robot safety.

## Next work

- Joint observation likelihoods with explicit provenance and dependence assumptions.
- Separate sensor-health state and stale/missing/contradictory evidence behavior.
- Exact tiny-model posterior predictive checks.
- Calibration and selective-risk evaluation on held-out individuals and terrain.
- Safe active probe selection over the robustly admitted candidate set.
