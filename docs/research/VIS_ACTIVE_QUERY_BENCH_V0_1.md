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
7. **Expected terminal task-loss reduction**: integrate a frozen public prior over confidence tier, sensor corruption, source-time delay and distractor returns; hypothetically update the claim belief and estimate the expected final abstention-aware decision risk for each action. If every candidate has non-positive estimated value, fall back to deterministic round-robin.
8. **Risk-threshold stop**: stop sensing when the current estimate of terminal Bayes risk is at or below a declared task-loss-unit/request threshold; otherwise select the most uncertain queried claim. This is a transparent satisficing baseline, not an optimal stopping solver. In addition to the default 0.04 operating point, a separate sensitivity report sweeps thresholds 0.01, 0.04, 0.08 and 0.12 on identical episode seeds.

The current value-of-information policy is an explicit, inexpensive approximation—not exact environment-specific expected-loss minimization. The new expected-loss policy is also approximate: it uses fixed public priors recorded in the Rust source, not true episode/family parameters. Its hypothesis enumeration is reported as a deterministic scoring-work proxy; this is not measured wall latency.

## Metrics

Primary metric is episode mean task loss. Also report answer rate, accuracy among answered episodes, mean requested observations per query claim, mean valid/relevant observations per query claim, wasted-observation rate (including distractor returns), action-allocation imbalance, logical state bytes, estimated deterministic scoring-work units per episode, and paired episode-bootstrap intervals for every active policy against round-robin. Also emit a direct paired expected-loss-policy-minus-VOI contrast; separate intervals versus round-robin do not establish whether the two selectors differ.

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

The candidate policy was developed from the preceding sample-complexity diagnosis, so this is still exploratory rather than a preregistered confirmatory study. A frozen-code independent-seed run then failed to reproduce the VOI improvement: round-robin loss was 0.1842 and VOI loss was 0.1914; paired VOI-minus-round-robin delta +0.0072, 95% CI [-0.0221, +0.0378]. This does not prove VOI is harmful, but rejects the prior result as a confirmed effect. The selector remains unchanged across those roots.

The expected-terminal-loss candidate was added after this failed replication, so it is exploratory too. Its exact-head report must include a direct paired contrast against VOI and the scoring-work proxy before interpreting whether its extra computation pays for itself. Native ActiveArena evaluation remains a separate gate.

## Expected-terminal-risk candidate result — qualified negative result

The seven-policy implementation passed focused exact-head qualification at `c0618126d6b7c8e9e973deafcd53eaabf92aa82e`: [run #39509119447](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39509119447) passed 24 memory tests and 11 active-query tests, deterministic report replay, main/stratum/pairwise schema checks and SHA verification. Combined report artifact: [#12304139446](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39509119447/artifacts/12304139446). General CI was still running at the exact-head check; this records focused benchmark qualification, not a fully green PR check suite.

Held-out results on the fresh active benchmark root, 384 paired episodes, budget 48:

| Policy | Mean loss ↓ | Answer rate | Answered accuracy | Estimated scoring work/episode | Delta vs round-robin (paired 95% CI) |
| --- | ---: | ---: | ---: | ---: | --- |
| Round-robin | **0.1842** | 50.26% | 88.08% | 48 | baseline |
| Approximate VOI | 0.1914 | 66.15% | 83.86% | 144 | +0.0072 [-0.0221, +0.0378] |
| Expected terminal task-loss reduction | 0.2207 | 59.64% | 79.91% | 5,328 | +0.0365 [+0.0046, +0.0697] |

The expected-risk candidate is significantly worse than round-robin on this held-out generator. Its direct paired contrast versus approximate VOI is +0.0293 loss, 95% CI [-0.0065, +0.0645], which does not establish the two selectors differ from each other; nevertheless, it fails the advancement gate because its point loss is worse and it does not beat round-robin. Its scoring-work proxy is 37× the VOI policy's and 111× round-robin's. The added theoretical complexity worsened task outcome in this setting.

This is consistent with a mismatch between the simplified internal belief/transition model and the generator's actual delay, corruption, and distractor process. The experiment does not establish which mismatch causes the regression. **Reject this expected-risk implementation as the preferred policy**; retain it as a benchmark baseline until the model is independently diagnosed. Do not respond by increasing grid size or adding more unvalidated parameters.

The independent-seed VOI replication also remains inconclusive: VOI loss 0.1914 vs round-robin 0.1842, paired delta +0.0072, 95% CI [-0.0221, +0.0378]. Thus neither active selector currently has replicated evidence of improvement over round-robin. The first-root VOI win and this fresh-root failure must both remain in the research record.

The next question is resource-sensitive: what is the full task-loss versus sensing-budget frontier, and when should an agent stop spending observations? See [VIS-ACT-006](https://github.com/Luminous-Dynamics/symtropy/issues/2431). Native ActiveArena task-success testing is still separate.

## Acceptance gate

A policy advances only if its held-out paired mean-loss interval against round-robin is strictly below zero, the declared action budget is unchanged, and no meaningful coverage/accuracy or subgroup regression is concealed by averaging. The expected-risk candidate must additionally beat the approximate VOI policy on a direct paired contrast to justify its extra computation. Report all eight policies and their resource-quality trade-offs, including the stopping policy's actual number of requests consumed. A synthetic win is only a reason to proceed to native ActiveArena controller ablation—not evidence of SOTA or physical-robot readiness.

## Sensing-budget frontier — full-horizon schedule

The benchmark sweeps request caps of **8, 16, 24, 32 and 48** for every acquisition policy. The latent world always evolves through tick 47, and the target is evaluated at tick 47. A budget of B requests is evenly scheduled across the full horizon at ticks `floor(slot * 48 / B)`; for B=48, this is one request per tick. Smaller budgets therefore do not accidentally become short prefixes followed by long blind tails. A stopping policy may end earlier, in which case no more potential observations are revealed and the world still evolves to tick 47.

The report emits a separate `BUDGET` table with mean task loss, answer rate, answered accuracy, requested/relevant observations per query claim, wasted-observation rate, logical state bytes, estimated scoring-work units and paired 95% episode-bootstrap loss delta vs round-robin. Rows use the same episode seeds at every budget and policy priors are unchanged. This exposes the loss-versus-budget frontier without collapsing sensing costs into an arbitrary scalar.

The first qualified frontier report at head `7a2fcd0b59ec6c15efe2b90da612dfbd86035f42` scheduled each smaller budget only over the first B ticks, then left the agent blind as the world continued. Although code/report integrity gates passed, this measured a *front-loaded prefix/early-stop* regime rather than a fair full-horizon budget allocation. Held-out loss was 0.25 at budgets 8, 16 and 24 because the policy almost always abstained at the stale terminal belief; do **not** use those rows to claim how observation count alone affects quality. The current implementation supersedes them by spreading sensing opportunities across the entire horizon. The full-horizon table must pass a new exact-head run before reporting its results.

The risk-threshold stop baseline uses a declared task-risk threshold of 0.04 as its default operating point. A separate sensitivity table sweeps thresholds 0.01, 0.04, 0.08 and 0.12 over identical seeds to expose threshold fragility. This is a transparent satisficing policy, not a claimed optimal stopping solver.

The budget-frontier schema is 80 data rows (2 families × 5 budgets × 8 policies), 16 columns. The stop-cost sensitivity schema is 8 data rows (2 families × 4 thresholds), 14 columns. Both require byte-for-byte report replay, row/column validation and SHA verification.

## Full-horizon budget report — exploratory result

The corrected full-horizon scheduler passed exact-head qualification at `8e27f5a569dedc7f8f79c377481ff16f25d09db5`: [run #39510256320](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39510256320) passed **24 memory tests + 15 active-query tests**, repeated-report byte comparison, 80 budget rows/8 cost-sensitivity rows, schema checks, and both SHA validations. Combined artifact: [#12305231544](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39510256320/artifacts/12305231544). The general CI workflow was still pending at the qualification check, so this is focused benchmark qualification rather than a claim that every PR check was green.

Held-out loss at each maximum request budget (384 matched episodes per policy):

| Max requests | Round-robin loss ↓ | Approx. VOI loss ↓ | VOI − RR loss (paired 95% CI) | Risk-threshold stop loss ↓ | Mean requests used by stop |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 8 | 0.2500 | 0.2500 | 0.0000 [-0.0091, +0.0104] | 0.2565 | 8.000 |
| 16 | 0.2500 | 0.2565 | +0.0065 [-0.0163, +0.0312] | 0.2350 | 16.000 |
| 24 | 0.2318 | 0.2298 | -0.0020 [-0.0299, +0.0247] | 0.2357 | 24.000 |
| 32 | 0.2331 | 0.2311 | -0.0020 [-0.0345, +0.0312] | 0.2311 | 32.000 |
| 48 | 0.1842 | 0.1914 | +0.0072 [-0.0208, +0.0378] | 0.2103 | 47.969 |

The active policy's paired interval against round-robin includes zero at every budget in this seed set. The answer rate/accuracy frontier also warns against optimizing only for answer coverage: at budget 16, VOI answers more often but has higher mean loss than round-robin. More requests help, but the simple active allocation heuristic does not yet demonstrate a stable advantage over round-robin on the new seed root.

The **risk-threshold stop baseline is not a successful stopping method** in this implementation. With threshold 0.04, it uses 47.969 of 48 requests on average in held-out episodes, so it saves virtually no sensing budget while its task loss is worse than round-robin. It frequently fails to push its own terminal Bayes-risk estimate below the stop threshold. The cost-sensitivity sweep reinforces that conclusion:

| Stop cost (task-loss units/request) | Held-out mean requests used | Held-out loss | Delta vs round-robin (paired 95% CI) |
| ---: | ---: | ---: | --- |
| 0.010 | 48.000 | 0.2103 | +0.0260 [-0.0059, +0.0605] |
| 0.040 | 47.969 | 0.2103 | +0.0260 [-0.0072, +0.0592] |
| 0.080 | 47.781 | 0.2096 | +0.0254 [-0.0065, +0.0618] |
| 0.120 | 47.492 | 0.2083 | +0.0241 [-0.0091, +0.0579] |

These are exploratory intervals, with no supported claim that stopping beats round-robin. Raising the cost threshold slightly reduces requests, but not enough to justify the loss trade-off. Do not tune further thresholds on this held-out report; the next stopping candidate should use a calibrated **marginal value of another observation** and a frozen development-only cost calibration.

The initial budget report at head `7a2fcd0b59ec6c15efe2b90da612dfbd86035f42` used a front-loaded prefix then a long blind tail. That report is superseded for general budget comparisons; see the protocol correction above. The new report spreads low budgets across the horizon, while preserving the world transition process through the terminal tick.



## Reproduce

```sh
rustc --edition=2024 --test tools/vision-active-query-bench.rs -o /tmp/active-query-tests
/tmp/active-query-tests
rustc --edition=2024 -O tools/vision-active-query-bench.rs -o /tmp/active-query-bench
/tmp/active-query-bench
```

## VIS-ACT-008: development-calibrated marginal value of sensing

**Status:** new candidate implementation; exact-head qualification and numeric results pending. This section makes no performance claim.

### Hypothesis and design

The absolute-risk stop rule has not worked: it nearly always spends the full budget. The new policy instead estimates the *expected reduction in terminal decision loss from one more requested observation* and compares the largest candidate value with an explicit request cost. Its calibration objective is:

`J = mean(task loss + c × requests used)`

where `c` is task-loss units per request. The fixed development grid is `[0, 0.00025, 0.0005, 0.001, 0.0015, 0.002, 0.003]`. The chosen value is the grid point minimizing the mean objective on development episodes only. The chosen threshold is then reported on the held-out family; held-out outcomes never select the threshold. The full threshold-by-family sensitivity table is diagnostic and must not be used to retune it.

This is deliberately a **myopic counterfactual estimator**, not an optimal stopping proof. It reuses the existing frozen confidence, delay, distractor and hazard priors. The previous expected-task-loss policy's negative result is a reason for skepticism, not a reason to presume this stopping adaptation works.

### Avoiding a symmetric-prior trap

A majority decision can have complementary information value: with all three claims unknown, the modeled marginal decision-loss reduction of observing only one claim may be exactly zero because the other two remain at 0.5. A naive marginal stop policy would stop before acquiring any evidence. This implementation therefore enforces an explicit exploration floor: request until at least one **relevant** observation has been acquired for each query claim before applying the marginal-value stopping rule. It first tries claims not yet requested, then retries the least-requested claim still missing relevant evidence; this prevents repeated distractors from starving other claims. A regression test locks this behavior. Distractor responses do not satisfy the floor. This is a declared heuristic that will be ablated later, not a guarantee that three observations are sufficient.

### Qualification and rejection rules

The exact-head Rust workflow checks deterministic replay, all policy invariance tests, the nine-policy report schema, all 90 budget-frontier rows, risk-stop cost sensitivity, the marginal-cost sensitivity table, the development calibration row, and SHA-256 report integrity.

The marginal report includes:
- raw paired task-loss delta versus round-robin, to show quality/coverage trade-offs;
- paired cost-adjusted objective delta versus round-robin, where round-robin pays for all 48 requests; and
- mean deterministic scoring-work units per episode, so extra evaluation complexity is visible even when requests fall. This remains an operation-count proxy, not a runtime measurement.

The candidate advances only if the **development-selected** threshold has (a) a held-out cost-adjusted objective delta whose paired 95% interval is entirely below zero and (b) a raw task-loss degradation whose paired 95% upper bound is no greater than +0.01 absolute. The cost grid is an explicit synthetic preference, not a measured sensor-cost calibration. Also report answer rate, requests used and per-episode uncertainty; lower spend is not a win if quality fails the non-inferiority gate. Reject the policy as preferred if either gate fails. Estimated scoring-work units are a deterministic operation-count proxy, not measured wall-clock latency, GPU time, RAM, energy or robotics efficiency.

### External benchmark boundary

This remains symbolic and synthetic. It cannot be ranked numerically against ActiveArena task success or RoboMME scores. After this gate, the next external gate remains a controller-only ablation on a pinned ActiveArena baseline: fixed model/checkpoint, prompts, preprocessing, executor and committed ID/OOD seed order; report task-macro success, per-task/category results, paired uncertainty, failures, latency, VRAM/RAM and observation counts. A +5 percentage-point OOD improvement with paired evidence and no more than a 3-point ID regression is an internal target, not a community standard.

## Qualified negative result: development-calibrated marginal stop

At source head `90b901972c4493c6ad6c61eb64561700fd11f120`, the focused benchmark and cross-platform CI passed. The artifact was downloaded and inspected: [workflow run #39520072684](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39520072684), [artifact #12310402449](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39520072684/artifacts/12310402449).

The development-only objective selected request cost `0.00000`, effectively disabling cost-based stopping. On 384 held-out episodes, that setting produced loss `0.2305`, mean requests `47.086`, and paired raw-loss delta vs round-robin `+0.0462`. The paired interval in the sensitivity row was `[+0.0137, +0.0788]`; the separately seeded calibration-row interval was `[+0.0150, +0.0820]`. The held-out result clearly fails the predeclared objective-improvement gate and the raw-loss non-inferiority gate. At higher request costs the policy usually abstains and loses too much task quality; do not choose a higher cost from the held-out grid.

**Decision: reject `marginal_value_stop` as the preferred policy.** This also indicates that threshold selection is not the central failure: the underlying estimated value/action policy is not improving held-out decisions.

## VIS-ACT-009: Monte Carlo expected-value calibration and ranking

**Status:** qualified at source head `8efbceffef306c2d85d50bb491b3dbd7384b636f`; the focused report workflow passed and artifact [#12339984694](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39589593835/artifacts/12339984694) was inspected. This qualification applies to that exact source head; later edits require fresh qualification.

The question is whether `expected_task_loss_reduction_scores` predicts decision-risk reduction under the simulator's actual observation process. The first-pass single-sample ranking was too noisy: it compared predictions to one potential outcome, making top-action agreement sensitive to ties and random corruption. The revised diagnostic estimates each candidate's **conditional expected** realized belief-risk change by drawing 64 evaluator-only candidate observations from the episode's hidden sensor profile at each of 12 fixed ticks across the 48-tick horizon.

For each sampled pre-action information state:
- The running reference controller is `ExpectedTaskLossReduction`; only its selected pre-generated observation changes its state.
- For each of the three query candidates, the evaluator uses a separately seeded Monte Carlo sample from the actual hidden episode profile (delay range, distractor frequency and corruption rate), applies each draw to a cloned state, and averages the resulting terminal belief-risk drops.
- Those hidden-profile draws and all counterfactual outcomes are evaluator-only; they are never passed to policy selection or the reference trajectory.
- The model's three predicted values are compared with three Monte Carlo estimated expected values for calibration, top-action agreement, pairwise ranking concordance and regret versus the Monte Carlo-best action.
- Repeated outcomes are aggregated within episode; bootstrap intervals resample episodes rather than treating correlated candidate draws as independent.

The report emits three tables:
- `VALUE_CALIBRATION`: two rows, development and held-out, showing mean predicted value versus Monte Carlo expected value, paired episode-level error interval and mean absolute gap.
- `VALUE_REALIZED_STRATA`: realized belief-risk drop by relevant/distractor outcome, source delay and confidence. These are descriptive conditional outcome strata; they do **not** compare an unconditional score against a conditional outcome as though they were the same estimand. Empty strata are explicitly `NA`, not zero.
- `VALUE_RANKING`: top-action agreement, pairwise ranking concordance and nonnegative Monte Carlo one-step regret. The Monte Carlo-best action is an evaluator-only reference under the known synthetic generator, not a policy available at deployment.

This remains a synthetic model audit, not task-loss improvement, real-world confidence calibration, or evidence that any active policy should be preferred. A lower average calibration gap alone is not sufficient: the action-ranking metrics must also show useful separation, and final task loss must still beat round-robin in a held-out policy experiment. If predictions are calibrated in magnitude but do not rank candidate value, investigate decision-relevant complementarity or scoring ties; if predictions are biased under held-out profiles, fix the sensor/delay/hazard model before another policy sweep.

The previous single-sample value report should be treated as a noisy exploratory diagnostic and is superseded by this Monte Carlo protocol, not pooled with it.

## VIS-ACT-010: online adaptation of observable nuisance priors

**Status:** policy, tests and diagnostic report qualified at source head `8efbceffef306c2d85d50bb491b3dbd7384b636f`; focused workflow [#39589593835](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39589593835) passed. General CI was still queued for that head when last checked. Later edits require fresh exact-head qualification.

### Why this experiment is justified

The qualified VIS-ACT-009 Monte Carlo audit found mean value estimates that were badly low on development episodes yet only slightly biased in aggregate on held-out episodes. More importantly, value-score pairwise concordance was only about 0.526 in both domains. The simulator exposes a large, known domain shift in nuisance distributions: development delays are at most two ticks with distractor rates 0.15–0.35, while held-out delays can reach five ticks with distractor rates 0.35–0.65. The current scorer nevertheless uses one fixed delay prior and one fixed distractor rate.

This motivates a narrow, testable adaptation—not another tuned task-value heuristic.

### Policy

The added `online_adaptive_value` policy uses the same expected terminal belief-risk scorer as the fixed-prior policy, but adapts only nuisance factors directly observable from its own released observations:
- a Dirichlet-smoothed histogram of source-arrival delay, initialized from the declared public delay prior with six pseudo-observations; records whose source time is clipped to tick zero are treated as censored and excluded from exact-delay counts
- a Beta-smoothed distractor-frequency estimate, initialized around the public distractor prior with four pseudo-observations;
- no hidden profile, latent truth, unchosen observation, evaluator label, or future data is exposed to the policy.

The conditional sensor-corruption prior remains fixed. A controller generally cannot identify sensor corruption from a single unlabelled stream without additional assumptions or independent ground-truth feedback; pretending it can infer that quantity would create another source of silent leakage. Confidence-tier frequencies also remain fixed in this experiment, since they are generated independently in the current simulator.

The policy uses the fixed-prior scorer unchanged as the direct control. Both receive identical episodes and the same fixed sensing budgets. Reports include paired policy-loss contrasts, the five-point budget frontier, observed state/work proxies, and a unit test that verifies priors shift in the expected direction after observed distractors/delays. The source also retains hidden-truth/profile relabelling tests to detect leakage.

### Hypothesis and rejection gate

**Hypothesis:** online nuisance adaptation improves decision quality under shifted delay/distractor distributions relative to the fixed-prior expected-value selector, without consulting hidden metadata or increasing the sensing budget.

Primary comparison: `online_adaptive_value - expected_task_loss_reduction` using paired episode-loss differences and 95% episode-bootstrap intervals, separately for development and held-out families. Round-robin remains the reference floor, and each policy's paired difference versus round-robin remains reported.

Do not promote the adaptive scorer merely because it beats the fixed scorer. It must improve the held-out comparison, show no unsupported win on development that collapses under shift, and have its changes verified by the nuisance-prior tests. If the held-out interval includes zero or the adaptive policy is worse than round-robin, retain it as a diagnostic baseline and move to a better-calibrated belief model or direct external benchmark—not more threshold tuning.

### What this does not claim

This only learns nuisance rates that are observable from released data. It does not solve sensor-noise identifiability, establish the causal correctness of any observation model, or prove a robotics benefit. Synthetic mean loss and estimated scoring-work units must remain separate from native ActiveArena/RoboMME success and measured hardware performance.

### Online nuisance estimates are separately audited

The follow-on `NUISANCE_ADAPTATION` table evaluates the adaptive policy's final estimates against the simulator profile **after each run**. It reports per-episode delay-distribution MAE, distractor-rate absolute error and signed bias, plus exact-delay sample counts and intervals bootstrapped over episodes. The true profile is used only by the evaluator for scoring; neither policy action nor prior update sees it. Delay samples clipped at source tick zero remain censored and do not enter the exact-delay histogram. This extra table prevents a task-loss difference from being misattributed to successful adaptation when the rate estimates themselves have not converged.

### Qualified result — observable-prior adaptation (384 episodes per family)

The exact-head artifact reports:

| Held-out policy | Mean task loss ↓ | Answer rate | Accuracy when answered | Scoring-work units/episode |
|---|---:|---:|---:|---:|
| Round-robin | 0.1842 | 0.5026 | 0.8808 | 48 |
| Fixed expected-loss scorer | 0.2207 | 0.5964 | 0.7991 | 5,328 |
| Online-adaptive value | 0.1908 | 0.6120 | 0.8468 | 5,712 |

The adaptive policy is **significantly better than the failed fixed expected-loss scorer** on held-out episodes: paired `adaptive - fixed` loss delta **−0.0299**, 95% CI **[−0.0475, −0.0137]**. This is evidence that adapting observable delay/distractor priors helps relative to that poorly calibrated baseline.

It does **not** yet beat round-robin: paired loss delta **+0.0065**, 95% CI **[−0.0228, +0.0384]**, which includes zero. Development delta versus round-robin was **−0.0124** (95% CI **[−0.0371, +0.0130]**), also inconclusive. Keep the adaptive policy as a promising diagnostic candidate, not the preferred policy, until the held-out comparison beats round-robin under a predeclared gate.

The nuisance-estimator audit showed:
- Delay-distribution MAE: **0.0368** held out (95% CI **[0.0355, 0.0380]**), with an average of 45.331 uncensored delay samples from 48 responses per episode.
- Distractor-rate absolute error: **0.0533** held out (95% CI **[0.0493, 0.0573]**); signed bias **−0.0068** (95% CI **[−0.0135, −0.0005]**).

The policy uses 5,712 deterministic scoring-work units per episode, versus 48 for round-robin—about 119 times the operation-count proxy. This is not measured wall-clock cost, but it makes the resource trade-off visible. The current candidate has more coverage and lower answered accuracy than round-robin, and no significant overall task-loss improvement, so efficiency and decision quality still need work.

**Decision:** retain online adaptation as a useful comparison, reject the claim that it beats the simple baseline, and investigate uncertainty-aware fallback/compute-efficient scoring next. Do not tune on held-out episodes.
