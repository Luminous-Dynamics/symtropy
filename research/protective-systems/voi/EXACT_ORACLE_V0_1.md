# Exact-oracle VOI microbenchmark v0.1

**Owner:** PROTECT-VOI-001 (#5268)  
**Status:** finite synthetic oracle; software decision semantics only  
**Frozen date:** 2026-11-14

## Purpose

Test whether a decision policy selects informative experiments using the correct likelihood direction and a fully specified cost function. This is a deliberately small exact problem used to debug the benchmark harness before expanding to multi-step policies.

## Decision problem

There are two latent states, A and B, with prior probabilities 0.6 and 0.4. The final decision chooses A or B. Correct decisions have loss 0; incorrect decisions have loss 1. Without another observation, the Bayes-optimal decision is A and expected loss is 0.4.

Each experiment emits observation A or B. Its likelihoods are explicitly provided as:

- P(observation A | state A)
- P(observation A | state B)

The policy must use both likelihoods. It must not assume that observation A supports state A: an anti-correlated proxy is intentionally included.

The constrained objective is:

`expected Bayes loss after observation + 0.05 × experiment cost`

Only experiments with cost no greater than 4.0 are feasible. No experiment is also a valid baseline, with objective 0.4.

## Frozen known answers

The Bayes risk after an experiment is the sum, over the two possible observations, of the smaller joint state/observation mass multiplied by the incorrect-decision loss. This enumerates the finite outcome space exactly; no Monte Carlo is required.

| Experiment | Cost | Expected Bayes loss | Cost-adjusted objective |
|---|---:|---:|---:|
| CHEAP_WEAK_PROXY | 0.50 | 0.34 | 0.3650 |
| EXPENSIVE_HIGH_INFORMATION | 4.00 | 0.10 | 0.3000 |
| ANTI_CORRELATED_PROXY | 0.75 | 0.05 | 0.0875 |
| UNINFORMATIVE_PROXY | 0.25 | 0.40 | 0.4125 |
| No experiment | 0.00 | 0.40 | 0.4000 |

Expected best experiment: **ANTI_CORRELATED_PROXY**.

The naive policy that maps observation A to decision A and observation B to decision B incurs expected loss 0.95 on the anti-correlated proxy. Correct Bayesian use of the declared likelihoods inverts that signal and incurs expected loss 0.05. This is a negative control against assuming a proxy's direction.

The cheapest experiment is worse than doing nothing under the declared objective. That negative control prevents "cheapest next test" from being mistaken for value-of-information optimization.

## Baselines and next expansion

This v0.1 is an exact-oracle semantics fixture, not a statistically sufficient comparison of discovery algorithms. Later versions should add:
- multiple sequential observations and adaptive stopping;
- priors that vary by held-out task family;
- correlated and conditionally dependent measurements;
- experiments with failure, censoring, delay and unavailable capacity;
- multi-objective constrained decisions and rare hard-constraint violations;
- cost uncertainty, setup/reconfiguration and confirmation costs;
- baselines for random/stratified sampling, DOE, single- and multi-fidelity BO, expected information gain and hypothesis discrimination;
- repeated paired tasks and confidence intervals.

Do not claim superior materials discovery from this binary toy problem. It only verifies the exact finite decision calculation and anti-correlated evidence handling.
