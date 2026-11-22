# AV Multi-Arm Rank Confidence Sequences v0

**Parent:** Symtropy #6834  
**Retirement child:** Symtropy #6850  
**Programme:** Symtropy #6806  
**Composes:** #6753 all-action EVSI, #6771 Pareto frontier, #6497 response-window/silence, #6564 multi-round executor, #6754 blind custody, Symthaea #12837 / #13179 candidate arms.

## Objective

Implement a reference evaluator for simultaneous anytime-valid ranks over multiple policies scored on the same finite cohort of episodes. The first target is A0–A5 (six arms).

This reproduces the finite-cohort method in:

Khosravi & Huo (2026), *Rank Confidence Sequences: Anytime-valid Leaderboards*, arXiv:2609.32211: https://arxiv.org/abs/2609.32211

The method updates evidence for every ordered arm pair, eliminates weak orders only when their average contradiction wealth crosses the preregistered threshold, and reports only comparisons/ranks supported by all surviving weak orders.

## Statistical boundary

The finite-cohort method assumes:
- a fixed finite episode cohort;
- a single common order uniformly randomized and committed before results are observed;
- every model not yet retired is scored on every episode in that common order;
- one bounded score in [0,1] per active model and episode;
- the score definition and higher-is-better direction are frozen.

It allows arbitrary dependence between models' scores **within the same episode**. It does not cover choosing the next episode, source or stratum adaptively based on previous model scores. Active evidence acquisition by a model **inside an episode** is allowed; the outer episode order remains precommitted.

## Method reproduced

For every ordered pair (j,l), the score difference is Z[t] = X[t,j] - X[t,l]. For each betting fraction lambda in a frozen grid, update the pairwise wealth:

    E[jl,lambda,t] = E[jl,lambda,t-1] * (1 + lambda * (Z[t] - b[t]) / (1 + b[t]))

The pair wealth is the mean over the lambda grid. The finite-cohort offset b[t] is computed from prior paired differences and remaining cohort size, clipped to [-0.99, 1], as specified in the paper. The implementation stores log wealth to avoid overflow.

For each weak order W (ties allowed), the contradiction wealth is the average pair wealth over the ordered hypotheses that W says are true. A weak order is permanently eliminated when its running maximum reaches 1/alpha.

The exact enumerator reports at every look:
- surviving weak-order count;
- certified pairwise dominances and unresolved pairs;
- exact simultaneous rank sets and dominance-derived rank bounds;
- top-k membership;
- dominance tiers;
- each ordered pair's log betting wealth for independent replay.

It also implements the paper's polynomial shortcut in log space and checks that its certified relations are a subset of the exact enumerator's results.

## Predictable model retirement

The implementation also supports the optional model-retirement step in Proposition 4.5 of the paper. An active model set is chosen only from previous looks, is monotone non-increasing, and may never reintroduce a retired model. The model roster, fixed episode order and retirement rule are committed before candidate scores are observed.

Supported rules:
- NONE: score every model on every episode, reproducing the original evaluator.
- TOP_K_RESOLVED: retire a model when its simultaneous rank set proves it is inside or outside the configured top-k.
- RANK_SINGLETON: retire a model when its exact simultaneous rank set has one possible rank.
- PAIRWISE_RESOLVED: retire a model when every comparison involving it is certified in one direction.

For each pair, its betting wealth updates only while both models are active. When either model retires, that pair's wealth is frozen permanently and remains in the weak-order/rank calculations. The next item must contain exactly the active set produced by the preceding look. The outer episode order is not changed, skipped or chosen adaptively.

The input may contain only the evaluated prefix of the fixed cohort when every model has retired. Otherwise the output distinguishes cohort exhaustion from an incomplete input. The evaluator reports per-model invocation counts and the number of invocations avoided relative to scoring that model on the entire cohort. These are protocol-level invocation counts, **not measured GPU time or energy savings**.

Retirement is task-specific. A model retired because its scalar rank or top-k status is resolved must not be treated as irrelevant to a separate multi-objective Pareto frontier, resource metric, task family or another decision. Those need their own score and evidence tracks.

## Input and execution

The v1 input requires:
- benchmark ID and integer schema version;
- 2–8 unique arm IDs (six-arm A0–A5 has 4,683 weak orders);
- one metric ID, alpha and frozen betting fraction grid;
- canonical episode IDs, full fixed episode order, seed commitment and order digest;
- optional predeclared retirement rule and top-k value;
- each item in the exact precommitted order, with a score for every currently active arm and no inactive/retired arm.

Example full-run item:

    {"episode_id":"episode-0001","scores":{"A0":0.4,"A1":0.6,"A2":0.7}}

Example retirement-enabled item:

    {"episode_id":"episode-0001","active_arms":["A0","A1","A2"],"scores":{"A0":0.4,"A1":0.6,"A2":0.7}}

Run:

    python3 -m unittest discover -s tools/av-anytime-valid-evaluator -p 'test_rank_confidence_sequences.py' -v
    python3 tools/av-anytime-valid-evaluator/rank_confidence_sequences.py --input rank-cs-workload.json

The order digest proves integrity of the submitted order envelope; independent custody under #6754 must attest that the order was uniformly randomized and committed before outcome access.

## Explicit v0 limits

- Finite-cohort random-order mode only; no i.i.d. superpopulation mode yet.
- One bounded [0,1], higher-is-better scalar metric only.
- No familywise allocation across metrics/strata/policy families.
- No Pareto-frontier inference across multiple axes; compose #6771 and #6840.
- The evaluator replays a completed score prefix and reports the next active set; it does not itself launch/cancel external model jobs. A cooperating runner must dispatch only the evaluator-issued active set on the next fixed episode.
- It cannot prove the custodian's randomization process, data rights or seed secrecy.
- It does not support data-adaptive outer episode selection.
- Invocation counts are not hardware energy/latency measurements.

## Claim boundary

Passing synthetic known-answer tests validates implementation mechanics only. It is not an A0–A5 model ranking, official SPOT/DIVE/ALIVE/StreamArena score, SOTA result, measured compute/energy saving, or independent blind evaluation.
