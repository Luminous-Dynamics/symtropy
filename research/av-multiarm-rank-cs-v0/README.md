# AV Multi-Arm Rank Confidence Sequences v0

**Parent:** Symtropy #6834  
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
- a single common order for all arms;
- a uniformly random episode order drawn before results are observed;
- each arm is scored on every episode, with one bounded score in [0,1];
- the score definition and higher-is-better direction are frozen.

It allows arbitrary dependence between models' scores **within the same episode**. It does not cover choosing the next episode, source or stratum adaptively based on previous model scores. Active sensing by a model **inside** an episode is allowed; the outer episode order remains precommitted.

The input includes a seed commitment, canonical episode IDs, the precommitted episode order and a digest binding the order envelope. The evaluator verifies the permutation and digest. It cannot prove that the custodian generated a uniform random order or that the commitment existed before outcome access; that must be supported by the independent custody process under #6754.

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
- longest-chain dominance tiers;
- each ordered pair's log betting wealth for independent replay.

It also implements the paper's polynomial shortcut in log space and checks that its certified relations are a subset of the exact enumerator's results. Exact weak-order enumeration is capped at eight arms to avoid unbounded combinatorial expansion; six arms have 4,683 possible weak orders.

## Running

    python3 -m unittest discover -s tools/av-anytime-valid-evaluator -p 'test_rank_confidence_sequences.py' -v
    python3 tools/av-anytime-valid-evaluator/rank_confidence_sequences.py --input rank-cs-workload.json

The finite-cohort input contract requires:
- benchmark_id = av-multiarm-rank-cs-v0, schema_version = 1;
- 2 or more unique arm IDs;
- one metric ID and alpha in (0,1);
- a non-empty unique betting fraction grid in (0,1);
- canonical sorted episode_ids;
- a permutation episode_order plus seed and order commitments;
- exactly one complete score vector per episode, in the committed order;
- every arm score finite and in [0,1].

Missing/failed runs cannot silently vanish. The upstream benchmark adapter must apply a preregistered failure score (for example zero for a higher-is-better accuracy metric), preserve a separate failure ledger and only then send a complete matrix to this rank evaluator.

## Explicit v0 limits

- Finite-cohort random-order mode only; no i.i.d. superpopulation mode yet.
- One bounded [0,1], higher-is-better scalar metric only.
- No familywise allocation across metrics/strata/policy families.
- No Pareto-frontier inference across multiple axes; compose #6771 and use simultaneous axis-wise evidence.
- The current API replays a complete score matrix and emits every look; it does not yet orchestrate model jobs or retire arms live. The paper permits optional model retirement based on prior looks; that is a planned next integration.
- It cannot prove the custodian's randomization process, data rights or seed secrecy.
- It does not support data-adaptive outer episode selection.

## Claim boundary

Passing synthetic known-answer tests validates implementation mechanics only. It is not an A0–A5 model ranking, official SPOT/DIVE/ALIVE/StreamArena score, SOTA result, measured compute saving, or independent blind evaluation.
