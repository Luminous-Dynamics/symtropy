# Locomotion Intervention Design V0

Standalone, dependency-free Rust decision kernel for [LOCO-DESIGN-001 / #3852](https://github.com/Luminous-Dynamics/symtropy/issues/3852).

## Objective

Choose the next admissible intervention from frozen model-conditional predictions. The goal is to maximize the separation of competing locomotor mechanism hypotheses while protecting against the least distinguishable plausible pair.

This is deliberately a pre-outcome operation: a selector must receive predictions frozen before the experiment is delivered or its outcome is observed.

## V0 criterion

Each candidate supplies one predicted mean vector per prior-positive hypothesis, on one exact, immutable observable layout. Observation noise has a common declared diagonal Gaussian covariance:

    Sigma = diag(variance[0], ..., variance[K-1])

For each unordered pair of hypotheses i,j:

    KL(i || j | action)
      = 0.5 * sum_k((mean_i[k] - mean_j[k])^2 / variance[k])

Because the covariance is common, this pairwise KL is symmetric.

For each candidate the selector reports:

- **Worst-pair separation:** minimum pairwise KL across every prior-positive hypothesis pair.
- **Prior-weighted pair separation:** normalized sum of pairwise KL weighted by 2 * prior_i * prior_j.
- **Robust utility per cost:** worst-pair KL divided by candidate cost.

It chooses the admissible, budget-feasible action with maximum robust utility per cost. Exact ties use the lowest stable candidate ID. If the best worst-pair KL is below the caller's threshold, the selector abstains and returns the closest candidate with its full score breakdown.

This is a maximin predictive-separation proxy. It is **not Bayesian expected information gain**: there is no integration over possible observations and no outcome-conditioned posterior update in V0.

## Integrity and receipt binding

- Hypothesis, candidate and forecast IDs must be unique.
- Every candidate must forecast every prior-positive hypothesis exactly once.
- All forecasts must use the exact same observable-layout digest, feature order and vector dimension.
- Priors are positive and normalized; observation variances and costs are finite and positive.
- Inadmissible or over-budget candidates are not scored.
- No hypothesis pair may be dropped to make a candidate appear more discriminating.
- Non-finite or malformed forecasts fail closed.
- The returned score records candidate digest, forecast digest, model digests, observable layout digest, cost and every pairwise separation.

The selector does not authenticate physical delivery. A real action still needs a separate assignment/delivery receipt with onset, affected channel/leg, magnitude, latency and fidelity.

## Deterministic known-answer tests

12 tests cover:
- abstention when two mechanisms make identical predictions;
- selecting a phase-pulse candidate that separates independent and coupled hypotheses;
- worst-pair protection even when the weakly separated model has a small prior;
- strict action admissibility and budget enforcement;
- deterministic tie-breaking;
- closed-form prior-weighted separation;
- cost normalization;
- duplicate hypothesis/candidate rejection;
- missing, duplicate and layout-mismatched forecasts;
- invalid priors, variances, budgets and digests;
- forecast dimension/non-finite checks;
- no-feasible-action and too-few-hypothesis errors.

## How this fits with COPEx and Symtropy planning

COPEx (ICML 2026) already demonstrates constrained Bayesian experimental design with offline amortization and online multi-step scenario-tree planning under costs and physical/action constraints. This V0 does not claim to improve on COPEx's EIG or sequential planner.

The intended composition is:

1. This kernel ranks feasible locomotion interventions using a conservative frozen-forecast separation score.
2. The six-leg benchmark (#3843) supplies phase-pulse, coupling-cut, feedback-delay and leg-failure prediction fixtures.
3. The sequential scenario-tree planner (#3588) can extend candidate selection to multi-step action/outcome branches.
4. A later V1 can implement actual predictive outcome distributions, posterior updates and expected information gain, then compare greedy, robust, random and lookahead strategies at matched compute and action budgets.

## Not implemented

- Bayesian EIG or posterior updating after an observation.
- Scenario-tree/lookahead planning.
- Stochastic predictive distributions or full covariance.
- Parameter fitting, Monte Carlo calibration or empirical coverage.
- Physical safety proofs or delivery authentication.
- Direct simulator integration; candidate predictions are caller-supplied and digest-bound.

## Run

    cargo fmt --manifest-path research/loco-intervention-design-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-intervention-design-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-intervention-design-v0/Cargo.toml --locked --all-targets -- -D warnings
