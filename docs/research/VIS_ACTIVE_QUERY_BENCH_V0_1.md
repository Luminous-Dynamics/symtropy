# VIS-ACT-004: active evidence acquisition under a fixed sensing budget

**Status:** candidate benchmark implementation; synthetic only. Do not treat its metrics as visual-perception or robotic task-success results.

## Question

When a controller has a fixed observation budget, can it improve final decision quality by choosing which queried fact to observe next, instead of allocating observations uniformly? This is the natural next step after the VIS-MEM-002 sample-complexity sweep: joint transition/sensor estimation improves as observations increase, but high distractor rates leave far fewer useful observations per claim.

## Protocol

- Eight latent binary claims; the public task query is a fixed majority decision over claims 0, 1 and 2.
- Every episode lasts 48 ticks and permits exactly one sensing request per tick.
- At each tick, the environment pre-generates one potential response per requested query key. A response may be diverted to a non-query hot distractor, delayed in source time or corrupted; confidence is sampled independently with the same calibrated confidence-conditioned emission model used by VIS-MEM-002.
- All policies use the same deterministic episode stream and have the same 48-request budget. The agent is shown only the response for its chosen action. Unchosen potential responses, latent truth, profile rates and seed stay evaluator-side.
- Development and held-out families independently sample distractor-burst rate, per-tick claim-flip rate, maximum source delay and base corruption rate from the declared family-specific ranges.
- Final decision uses an online belief approximation and fixed abstention-aware task loss: correct = 0, abstain = 0.25, incorrect = 1.

## Policies

1. **Round-robin**: deterministic balanced allocation.
2. **Uniform random**: seeded random query-key request.
3. **Least observed**: request the claim with the fewest previous attempts.
4. **Stalest first**: request the claim whose latest retained source timestamp is oldest.
5. **Maximum uncertainty**: request the claim with highest Bernoulli uncertainty.
6. **Approximate value of information**: estimate how much the selected claim's uncertainty could change the final three-claim majority decision; then request the claim with greatest expected decision impact.

The current value-of-information policy is an explicit, inexpensive approximation—not exact environment-specific expected-loss minimization. Report it under that name, not as an optimal planner.

## Metrics

Primary metric is episode mean task loss. Also report answer rate, accuracy among answered episodes, mean requested observations per query claim, mean valid/relevant observations per query claim, wasted-observation rate (including distractor returns), action-allocation imbalance, logical state bytes, and paired episode-bootstrap intervals for every active policy against round-robin.

Frames/observation requests within the same episode are not independent statistical samples. Bootstrap the paired episodes. Report development and held-out results separately and include all policy failures.

## Leakage and fairness controls

- Hidden truth, true transition rate, true corruption rate, profile labels and generator seeds never enter the policy interface.
- Every policy acts on the same pre-generated episode/world and gets exactly 48 action opportunities.
- Relabeling only hidden evaluator state must not alter policy action sequences or task loss.
- A returned distractor counts against the same action budget; no policy receives a free retry.
- Query keys are fixed across seeds in v0.1 to make action indices directly comparable.
- Stale evidence cannot rewind a claim's retained source timestamp.
- Deterministic rerun must reproduce the report byte-for-byte; SHA-256 receipts are retained.

## First exact-head results — exploratory synthetic win

The active-query implementation passed exact-head qualification at source head `a2a9d7b400fc2d05bc8f8970adee3611ee6499f5`; [focused run #39500610452](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39500610452) passed the existing memory suite (**24 tests**) and active-query suite (**8 tests**), deterministic replays, TSV checks and both report SHA checks. Combined artifact: [#12300016250](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39500610452/artifacts/12300016250), archive SHA-256 `7fd22ad7ad823b1b2fa13b768a6e246daea2ec4873bd3527940eac2a42e03f59`.

Held-out family, 384 paired episodes per policy, fixed 48-request budget:

| Policy | Mean loss ↓ | Answer rate ↑ | Accuracy when answered ↑ | Relevant observations per query claim | Wasted observation rate | Allocation imbalance | Logical state bytes | Loss delta vs round-robin (paired 95% CI) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Round-robin | 0.2227 | 58.85% | 79.65% | 8.109 | 49.32% | 0.000 | 96 | baseline |
| Uniform random | 0.2292 | 48.96% | 79.26% | 8.023 | 49.86% | 6.680 | 96 | +0.0065 [-0.0228, +0.0358] |
| Least observed | 0.2227 | 58.85% | 79.65% | 8.109 | 49.32% | 0.000 | 96 | +0.0000 [0.0000, 0.0000] |
| Stalest first | 0.2077 | 62.76% | 81.74% | 8.083 | 49.48% | 6.169 | 96 | -0.0150 [-0.0469, +0.0176] |
| Maximum uncertainty | 0.1953 | 65.62% | 83.33% | 8.043 | 49.73% | 10.682 | 96 | -0.0273 [-0.0612, +0.0046] |
| **Approx. value of information** | **0.1868** | **67.97%** | **84.29%** | **8.109** | **49.32%** | **13.807** | **96** | **-0.0358 [-0.0677, -0.0046]** |

The approximate value-of-information selector reduces the primary mean loss by **0.0358 absolute**, about **16.1% relative** to round-robin, with a paired episode-bootstrap interval strictly below zero. It answers more often and has better accuracy among answers, while requesting exactly the same 48 observations and returning the same average number of relevant observations (8.109 per query claim). The gain therefore comes from *allocation* of useful evidence, not extra sensing. The selector also has more uneven allocation (imbalance 13.807 versus 0), which is consistent with focusing on claims with higher decision impact.

Development result: round-robin loss 0.0977; value-of-information loss 0.0540; paired delta -0.0436, 95% CI [-0.0684, -0.0195].

This is an encouraging result for this controlled generator, not SOTA evidence. The candidate arose from the sample-complexity finding and the six-policy comparison was not preregistered before the broader line of memory research. Freeze the selector, action budget, confidence model and seed protocol, then run a new untouched seed root before claiming confirmatory evidence. Also evaluate slices by recent hidden claim change, delay and high corruption, because average loss may conceal weak handling of surprise changes. Then reproduce a native ActiveArena baseline before testing the memory/acquisition controller swap. Synthetic loss must not be compared to ActiveArena task-success percentage.

## Policy-independent failure-strata result — exact-head qualification

The current active-acquisition implementation was qualified on exact source head `f4b3f78bcf94449b34d8794ef00f171aae24635b`. [Focused run #39501614409](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39501614409) passed **24 memory tests + 9 active-query tests**, report replay, main/report-stratum schema checks and both SHA receipts. Combined artifact: [#12299762990](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39501614409/artifacts/12299762990), archive SHA-256 `47da11887831b9a2cbb514c788f126caf59b5a67eeb01682d64a7488fe7a263a`. Cross-platform CI for this same head was still pending at the recorded check.

The subgroup logic was corrected before this run: all labels are computed from policy-independent hidden world state, the episode profile and the full **pre-generated potential observation tensor**, after each policy has completed. No subgroup label or unchosen potential observation enters the policy API. The paired comparisons now use the same episodes for every policy within a given stratum.

Held-out subgroup diagnostics for the primary approximate value-of-information selector versus round-robin (all intervals are paired episode-bootstrap 95% CIs):

| Episode stratum | Episodes | VOI mean loss | Delta vs round-robin (95% CI) |
| --- | ---: | ---: | --- |
| World query claim changed in final 8 ticks | 264 | 0.2320 | -0.0398 [-0.0814, +0.0019] |
| No query claim changed in final 8 ticks | 120 | 0.0875 | -0.0271 [-0.0729, +0.0167] |
| Potential query evidence arrived out of order | 375 | 0.1847 | -0.0393 [-0.0727, -0.0073] |
| No potential query evidence arrived out of order | 9 | 0.2778 | +0.1111 [-0.0833, +0.3056] |
| Latest potential query evidence low confidence | 231 | 0.1861 | -0.0119 [-0.0509, +0.0271] |
| Latest potential query evidence not low confidence | 153 | 0.1879 | -0.0719 [-0.1242, -0.0212] |
| Any potential query observation delay >2 | 276 | 0.2038 | -0.0317 [-0.0716, +0.0091] |
| No potential query observation delay >2 | 108 | 0.1435 | -0.0463 [-0.0949, +0.0046] |
| High claim-flip profile | 247 | 0.2014 | -0.0304 [-0.0698, +0.0071] |
| Lower claim-flip profile | 137 | 0.1606 | -0.0456 [-0.1022, +0.0146] |
| High corruption profile | 246 | 0.1717 | -0.0691 [-0.1108, -0.0305] |
| Lower corruption profile | 138 | 0.2138 | +0.0236 [-0.0290, +0.0779] |
| High distractor profile | 251 | 0.2042 | -0.0229 [-0.0618, +0.0179] |
| Lower distractor profile | 133 | 0.1541 | -0.0602 [-0.1184, -0.0075] |

Interpretation:
- The active-acquisition policy's aggregate held-out advantage survives policy-independent stratification, but evidence for improvement in the **world-change-in-final-8-ticks** subgroup remains inconclusive: its interval slightly crosses zero.
- It does not demonstrate a benefit on all high-stress dimensions: claim-flip and high-distractor subgroup intervals include zero, as do delay strata. High-corruption and low-distractor strata show stronger effects in this run, but all subgroup results are overlapping and multiple comparisons are exploratory.
- Only nine held-out episodes are in the “no out-of-order potential observation” slice; do not interpret its noisy estimate.
- The strata describe potential observation support independent of selected actions, not exactly which evidence a policy consumed. Per-policy realized sensing efficiency is reported separately in the main table.

The candidate policy was developed from the preceding sample-complexity diagnosis, so this is still exploratory rather than a preregistered confirmatory study. Freeze the code/scoring/loss and replicate on a new seed root before promoting the finding; native ActiveArena evaluation remains a separate gate.

## Acceptance gate

A policy advances only if its held-out paired mean-loss interval against round-robin is strictly below zero, the declared action budget is unchanged, and no meaningful coverage/accuracy or subgroup regression is concealed by averaging. The value-of-information policy is the main hypothesis, but the benchmark must report all six policies. A synthetic win is only a reason to proceed to native ActiveArena controller ablation—not evidence of SOTA or physical-robot readiness.

## Reproduce

```sh
rustc --edition=2024 --test tools/vision-active-query-bench.rs -o /tmp/active-query-tests
/tmp/active-query-tests
rustc --edition=2024 -O tools/vision-active-query-bench.rs -o /tmp/active-query-bench
/tmp/active-query-bench
```
