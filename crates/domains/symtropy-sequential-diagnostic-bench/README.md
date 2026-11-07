# Symtropy Sequential Diagnostic Bench

Exact finite-horizon adaptive selection of passive measurements and stopping. Composes the one-step EVSI contract in #3917 with fault hypotheses and observability from #3993/#4005.

Implements [issue #4027](https://github.com/Luminous-Dynamics/symtropy/issues/4027).

## Run

    cargo test --manifest-path crates/domains/symtropy-sequential-diagnostic-bench/Cargo.toml

## v0.1 algorithm

For an unnormalized belief vector w(s), terminal action value is max over actions of the weighted utility. Every candidate measurement pays its full cost on the branch where it is selected. Its outcome likelihood updates unnormalized belief weights; each outcome branch may stop or select a different remaining measurement.

The exact recursion evaluates:
- stop now;
- each unused, budget-admissible measurement;
- every non-zero-probability outcome;
- the best continuation policy after that outcome.

A measurement is chosen only when its expected continuation value exceeds stopping value by the declared per-unit-mass continuation threshold. No entropy/information-gain proxy substitutes for utility.

## Strict exact-search bounds

- at most 8 hypotheses;
- at most 8 terminal actions;
- at most 5 candidate measurements;
- at most 3 outcomes per measurement;
- horizon at most 3;
- each measurement can be used at most once per policy path.

Oversize inputs are rejected. There is no approximate fallback.

## Evidence boundary

The synthetic fixture illustrates an adaptive policy:
- a coarse passive measurement is taken first;
- one outcome supports immediate STOP;
- the alternative outcome justifies a second, different measurement.

All priors, likelihoods, utilities and costs in the fixture are fabricated. The result is SYNTHETIC_ONLY. Real policies need independent evidence, a decision owner's approved utility, qualified measurement domains and review.

This crate never issues physical actuation or changes machine settings. It can only return a passive measurement candidate, STOP or HUMAN_REVIEW.


## Conditional dependence rule

The policy requires a case-level conditional-independence assumption and a dependence-evidence reference for each measurement. Measurements with the same declared independence group cannot both be selected along one policy path. This prevents obvious double-counting of repeated or shared-principle observations.

Different groups are still a declared modelling assumption, not proof of statistical independence. A later version must support history-conditioned likelihoods and empirically estimated dependence where sequential observations are correlated.

## Synthetic replay

The three-hypothesis example evaluates 18 recursion nodes and produces:
- current stop value: -3.2 synthetic benefit points;
- selected first measurement: `coarse-passive-observation`;
- `A-like` outcome: stop and retain `address-A`;
- `B-or-C-like` outcome: perform `B-vs-C-passive-sample`;
- second-stage outcomes select `address-A`, `address-B` or `address-C`;
- expected adaptive policy value: 7.6178 synthetic benefit points.

The values are fabricated to exercise branching and stopping. They are not a real experiment recommendation, operational utility or mine economics.
