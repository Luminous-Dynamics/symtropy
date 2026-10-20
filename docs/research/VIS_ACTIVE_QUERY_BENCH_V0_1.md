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

## Acceptance gate

A policy advances only if its held-out paired mean-loss interval against round-robin is strictly below zero, the declared action budget is unchanged, and no meaningful coverage/accuracy or subgroup regression is concealed by averaging. The value-of-information policy is the main hypothesis, but the benchmark must report all six policies. A synthetic win is only a reason to proceed to native ActiveArena controller ablation—not evidence of SOTA or physical-robot readiness.

## Reproduce

```sh
rustc --edition=2024 --test tools/vision-active-query-bench.rs -o /tmp/active-query-tests
/tmp/active-query-tests
rustc --edition=2024 -O tools/vision-active-query-bench.rs -o /tmp/active-query-bench
/tmp/active-query-bench
```
