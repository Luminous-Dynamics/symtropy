# Robust finite ICVaR policy selection v0.1

**Status:** executable exact selector over caller-supplied finite observable policy candidates. All candidate probabilities and losses in tests are synthetic. This is not yet an automatic policy-tree generator, full Bellman planner, physical risk estimator, or navigation authority mechanism.

## Implemented API

`select_robust_iterated_cvar_policy` accepts a finite list of complete observable policy candidates. Each candidate contains:
- a nonzero stable policy identity;
- one branch tree per persistent mission model;
- a root sensing action identity;
- observable branch identities and continuation-action identities;
- terminal decisions for each second-stage observable outcome;
- model-specific branch probabilities and conditional continuation-loss distributions.

The selector:
1. validates unique policy identities;
2. requires every candidate to use exactly the same persistent model identity set;
3. independently evaluates two-level ICVaR for each complete model within each candidate;
4. applies an outer maximum across persistent models;
5. selects the candidate with the smallest worst-case ICVaR;
6. resolves exact ties by the smallest policy identity and returns an audit score for every candidate.

The shared root/branch/terminal policy signature is mandatory. Model-specific action choices for the same observable history are rejected, even if the resulting risk score would be lower. Model-specific branch likelihoods and losses are permitted.

## Exact objective-ranking counterexample

Two observable policies are evaluated under the same two persistent mission models.

### Policy 1: lower mean, severe tail

- Model 1: 95% branch at loss 0, 5% branch at loss 100.
- Model 2: 90% branch at loss 0, 10% branch at loss 100.

Worst-model expected loss is 10. Worst-model ICVaR at tail mass 5% is 100.

### Policy 2: higher mean, bounded loss

- Model 1: loss 20 on every branch.
- Model 2: loss 30 on every branch.

Worst-model expected loss is 30. Worst-model ICVaR at tail mass 5% is also 30.

Therefore:
- Risk-neutral expected-value selection chooses Policy 1 (10 < 30).
- Robust worst-5%-ICVaR selection chooses Policy 2 (30 < 100).

The two policies use identical observable branch/action signatures across both candidate models. This is a mathematical counterexample, not an empirical navigation risk result.

## Tests authored

- Risk-neutral tail-mass limit selects the lower-mean policy.
- Robust ICVaR selects the bounded-tail policy.
- Candidate insertion-order invariance and deterministic candidate-ID tie-breaking.
- Candidate model-set equality across all policies.
- Rejection of invalid / duplicate policy IDs.
- Rejection of model-specific root/continuation/terminal action signatures for the same observable history.

Exact-head CI is required before these tests are reported as passed.

## Scope and next step

This selector optimizes only over the finite candidate set supplied by its caller. It does not itself generate feasible candidate trees from the temporal navigation model or prove a full robust ICVaR Bellman recursion. The next integration increment should derive candidate branch distributions directly from the existing navigation policy-tree generator, add state/action costs and connect reachable conditional continuations to the Bellman evaluator.

Keep belief-state immediate-cost CVaR, static mission-path CVaR, iterated CVaR and outer persistent-model robustness as separate outputs. Do not sum them into one risk score without a valid risk-composition argument.

## Claim boundary

No real payloads are admitted. No transition-model coverage, physical calibration, PHMI, safety certification or operational authority is claimed.
