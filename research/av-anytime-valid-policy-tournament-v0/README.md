# AV Anytime-Valid Policy Tournament v0

**Parent:** Symtropy #6806  
**Composes:** #6753 all-action EVSI, #6771 Pareto resource frontier, #6497 response-window scoring, #6564 multi-round executor, #6754 blind custody.

## Objective

Prototype a statistically valid stopping rule for one paired policy comparison. A benchmark operator may inspect the result after every independent source cluster and stop once the candidate is demonstrably superior to a preregistered practical margin, demonstrably not superior, or at the sample cap as inconclusive.

## Method

For paired differences X_i in [-1, 1], with positive values favoring the candidate, the v0 evaluator uses a conservative two-sided Hoeffding confidence sequence.

At look n, each tail receives error budget alpha / (2 n (n+1)). The radius is:

    r_n = sqrt(2 log(2 n (n+1) / alpha) / n)

The interval is the sample mean plus/minus r_n, clipped to the known [-1, 1] range. Since the tail allocations sum to alpha over all looks, the union-bound construction is time-uniform under the stated sampling assumptions.

## Sampling assumptions

The estimand and target population must be frozen before evaluation. Each consumed row must be one independently sampled source cluster drawn from that same target population, with one paired candidate-minus-reference difference in [-1, 1]. The method needs a common target mean across looks (for example, IID sampling from a frozen population). It is not valid merely because cluster IDs are unique. The evaluator can reject repeated IDs but cannot prove that clusters are independent, representative or truthfully identified.

## Decision states

- SUPERIOR: lower bound > preregistered superiority margin.
- NOT_SUPERIOR: upper bound <= preregistered superiority margin.
- INCONCLUSIVE: neither condition is certified by the supplied sample cap.

NOT_SUPERIOR is not equivalence. Equivalence requires a separate two-sided margin test and is intentionally not implemented in this first tranche.

## Input contract

The JSON input must declare integer schema_version 1 and sampling_unit independent_source_cluster. Each observation has a unique cluster_id and one finite numeric paired_difference in [-1, 1]. For score metrics in [0, 1], this naturally represents candidate score minus reference score.

## Local execution

    python3 -m unittest discover -s tools/av-anytime-valid-evaluator -p 'test_*.py' -v
    python3 tools/av-anytime-valid-evaluator/anytime_valid_policy_eval.py --input path/to/input.json

The test suite checks deterministic positive, negative and tie cases, rejects reused source clusters and malformed numeric/schema inputs, and verifies that the output counts only the prefix actually consumed before the stopping look.

The CLI is an **offline replay scorer**: it receives an ordered result file and reports the earliest valid stopping prefix. It does not itself control candidate execution. Real evaluation-compute savings require a streaming driver to supply one independent cluster at a time and honor the stopping state before scheduling the next cluster. A retrospective prefix count must not be reported as measured compute savings.

## Explicit v0 limits

This is not the complete #6806 implementation. It does not yet provide:
- all-model rank confidence sequences or six-arm simultaneous inference;
- alpha allocation across multiple metrics, strata or policy pairs;
- cluster-robust inference for dependent clips within one source;
- equivalence / non-inferiority families beyond the one-sided margin;
- Pareto-frontier certificates with simultaneous uncertainty on every metric;
- inference for unbounded latency, p95/p99 tails or unmeasured energy;
- adaptive sampling designs whose conditional estimand changes during the run.

The input validator requires one row per independent source cluster to prevent accidental pseudo-replication, but does not estimate cluster dependence from repeated clips. The evaluator cannot establish whether the declared sampling population is representative. Those limits must remain visible in every report.

## Claim boundary

Passing these tests qualifies only the mechanics of this two-arm bounded reference method. It is not a Symthaea model result, an official SPOT/DIVE/ALIVE score, proof of real-world superiority, a measured evaluation-compute saving, or a substitute for the independent custodian specified in #6754.
