# VIS-MEM-002: query-conditioned relational evidence memory v0.1

**Status:** benchmark implementation and hypotheses. Performance claims must come from an exact-head report; no SOTA or robotic-performance claim is made here.

**Validity warning (latest candidate):** an audit found that earlier generator revisions set confidence to 950 for every clean observation and 350 for every corrupted observation, leaking the corruption flag into a policy-visible field. The hazard-aware/selective metrics previously reported are not valid evidence that the policy can handle noisy sensor confidence. The current candidate removes this leakage with a probabilistic confidence channel and uses a new seed root. All earlier tables are historical results for the easier, leakage-contaminated generator and must not be used as proof for the corrected candidate.

## Why this experiment exists

The qualified VIS-ACT rendered/timing pilot found held-out mean loss of 0.0794 for bounded FIFO, 0.1927 for uniform reservoir capacity 2, and 0.0859 for fixed sweep. The reservoir-versus-fixed-sweep paired bootstrap interval for its loss increase was [0.0885, 0.1237]. The subsequent recency-protected reservoir matched FIFO at 0.0794 on that same single-target generator. That is a valuable non-win: protecting a recent direct observation did not improve the measured task.

External benchmarks motivate testing a harder capability:
- [ActiveArena](https://leeibo.github.io/ActiveArena/): interactive search, multi-object loops, multi-object decisions, and information acquisition, with ID/OOD splits.
- [RoboMME](https://proceedings.mlr.press/v306/dai26c.html): 16 manipulation tasks spanning temporal, spatial, object, and procedural memory.
- [BAVO-Bench / A-FAR](https://hcplab-sysu.github.io/BAVO-Bench/): controlled occlusion recovery, including unpredictable-time interruptions, in multi-stage manipulation.

These are external comparators, not interchangeable scoreboards. See the existing [ActiveArena reproduction track](https://github.com/Luminous-Dynamics/symtropy/issues/2332).

## External frontier reference as of 2026-10-20

A preprint posted on 2026-10-12, *Can a Frontier VLM Serve as a Robot Policy for Memory-Dependent Manipulation?*, reports a frozen GPT-6-Astra model reaching **92.1% (737/800)** on RoboMME in a single run while retaining the full episode in context and directly emitting Cartesian waypoints. The same paper lists 60.8% for the previous best leaderboard entry and 79.1% for a hierarchical GPT-6-Astra + trained-policy system. The human reference is 90.5% but uses RoboMME's privileged oracle-skill interface; the VLM emits waypoints, so those are not perfectly matched systems. The authors explicitly state that their result has not yet been validated by RoboMME maintainers and does not isolate whether history dependence comes from images, descriptions or action history. Sources: [paper/preprint](https://arxiv.org/abs/2610.17350), [RoboMME official benchmark](https://robomme.github.io/).

This is a much more demanding external reference than Symtropy's symbolic mean loss, and those numbers are **not comparable**. The practical path to competitive evidence is:
1. Reproduce the official RoboMME baseline on a named task subset and preserve the native task success checks, action interface and license.
2. Hold the vision/action model, prompt, waypoint executor, frame density and reset protocol fixed while swapping only the memory/decision controller.
3. Run history-removal counterfactuals (latest frame only, image history without textual summaries, text history without earlier images, and recent-window memory) to test if an explicit compact memory beats simply retaining full context.
4. Report all 16 task results, repeated runs/seed intervals, call count, wall latency, memory/token budget and failures. Never compare our small symbolic loss against the 92.1% success score.

BAVO-Bench adds a complementary physically grounded active-view track: it separates clean, stage-aligned occlusion and random-time occlusion over five bimanual tasks, and reports real-robot recovery rollouts. [Official project](https://hcplab-sysu.github.io/BAVO-Bench/). Symtropy's randomized occlusion work is a unit-testable precursor; it has not reproduced A-FAR or demonstrated physical camera recovery.

## Paradigm shift

Compare raw observation retention against a compact, versioned set of **query-relevant beliefs**. An observation is not intrinsically valuable: its value depends on which question must be answered, the claim it supports, when the evidence was sourced, and whether a later observation supersedes it.

Query relevance is public input, so query-keyed retention may filter irrelevant claims. It may not inspect hidden truth, future observations, evaluator labels, or seeds. Source timestamp and arrival timestamp are distinct: an older observation arriving late must not overwrite a newer source-time claim.

## v0.1 protocol

- Eight latent binary claims; each episode exposes a query over three distinct claim keys. The query asks whether at least two of the three claims currently hold.
- A 48-event evidence stream contains correlated hot-distractor bursts, independently changing claims, delayed/out-of-order source times, and occasional corrupted evidence.
- Every episode independently samples distractor frequency, claim-change rate, maximum delay, and corruption from a family-specific range. Factor values vary within each family, so high/low nuisance strata are not assigned solely by the development/held-out label.
- Policies: bounded FIFO, deterministic seeded uniform reservoir, query-keyed latest-source-time memory, a query-keyed selective policy calibrated on separate development seeds, a naive query-keyed hazard policy, and a grid-posterior query-keyed gap-aware hazard policy that scores same/different observations using their actual source-time gap.
- Capacities: 1, 2, 3, 4, and 8 evidence entries. Each policy receives the same released stream and public query at each episode/capacity pair.
- 12 seed replicates × 32 episode seeds = 384 paired episodes per family and policy/capacity.
- Loss: correct answer = 0; abstention without fresh retained evidence for all three queried claims = 0.25; incorrect answer = 1.
- Metrics: mean loss (primary), answer rate, accuracy on answered episodes, fresh query-claim coverage, mean memory entries, logical serialized-state bytes (retained evidence payload plus any auxiliary sufficient statistics), the selected selective threshold, and paired 95% bootstrap intervals for every policy's loss difference versus FIFO. Positive deltas mean worse loss. Logical bytes exclude allocator/runtime overhead and are not measured RAM.
- Selective threshold grid: minimum evidence confidence {350, 900} × maximum source age {2, 4, 7}. Selection minimizes mean decision loss on a separate 256-episode development calibration seed set; ties prefer higher confidence then lower allowed age. Those calibration seeds are disjoint from the report seeds and held-out outcomes never select thresholds.
- Fixed-seed report replay and tests cover delayed stale evidence, query-irrelevant distractors, capacity bounds, and deterministic bootstrap.

## Hypotheses and advancement gate

1. Query-keyed latest-source-time memory with capacity ≥3 should reduce held-out loss versus FIFO by preserving the three facts directly needed by the public query while filtering irrelevant repeated observations.
2. Uniform reservoir may benefit from larger capacity, but sampling can discard a recent necessary claim even when it preserves broader history.
3. If query-keyed memory does not improve the held-out paired interval versus FIFO, do not claim superiority. Inspect claim-change and delay slices, then test uncertainty-aware evidence retention rather than changing the score after seeing results.

Proposed stretch gate for a confirmatory version: at capacity 3, at least 10% relative held-out mean-loss reduction against FIFO, a paired 95% confidence interval for the loss difference strictly below zero, no answered-accuracy regression larger than 2 percentage points, and no increase in budget violations. These are hypotheses, not achieved results.

## Evaluator-only failure strata and timestamp semantics

The current candidate revision adds four overlapping evaluator-only slices at the primary capacity 3:

1. `claim_changed_after_latest_source`: at least one queried claim changed in hidden truth after the most recent source timestamp retained in the released stream.
2. `query_evidence_arrived_out_of_order`: an older source-time item for a queried claim arrived after an item with a later source timestamp.
3. `latest_query_evidence_low_confidence`: at least one queried claim's latest source-time observation is marked below the benchmark's confidence cutoff.
4. `query_observation_delay_gt_2`: at least one queried-claim observation arrived more than two ticks after its source timestamp.

These labels are generated by `Episode::evaluator_strata` and used only for report aggregation; they are not arguments to `Memory`, `observe`, or policy decision code. The report emits per-policy loss, answer rate, answered accuracy, and paired bootstrap intervals versus FIFO for each positive and negative stratum level.

Timestamp ordering is now explicitly **source tick first, arrival tick as tie-break**. Confidence is not chronology and may not cause an older report to overwrite a later arriving report with the same source timestamp. This amendment changes the ranking semantics from the initial report head, so the historical v0.1 numbers above remain evidence for that earlier revision only; current performance conclusions must use the new exact-head report after qualification.

## Historical failure-mode strata results before factorial profile amendment

The evaluator-only strata were qualified on exact source head `2f63683342d212951f73f6e0b8bb06bb3cc94bc4` by [focused run #39486092493](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39486092493). The run passed 11 tests, deterministic report replay, required-row checks, SHA-256 verification, and uploaded [`vis-relational-memory-report`](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39486092493/artifacts/12293267372). Current cross-platform CI is tracked separately.

At capacity 3 on held-out episodes:
- When none of the queried claims changed after the latest source-time evidence (77 episodes), query-keyed latest scored **0.2045** vs FIFO **0.2403**; paired delta was **-0.0357** with 95% CI **[-0.0649, -0.0065]**. The development-calibrated selective policy scored 0.2013; delta **-0.0390**, 95% CI **[-0.0584, -0.0195]**. This is evidence that query-keyed evidence can help in a subset where the relevant facts remain stable.
- When at least one queried claim changed after its latest source-time evidence (307 episodes), query-keyed latest scored **0.2630** vs FIFO **0.2541**; delta **+0.0090**, 95% CI **[-0.0041, +0.0244]**. Selective memory scored 0.2663; delta **+0.0122**, interval **[0.0000, 0.0261]**. Static retention is not robust to hidden claim changes after observation.
- With out-of-order query evidence (267 episodes), query-keyed latest scored 0.2519 vs FIFO 0.2537; delta **-0.0019**, 95% CI **[-0.0150, +0.0131]**. This does not establish a benefit for the out-of-order subset.
- Where the latest source evidence for at least one queried claim was low-confidence (77 episodes), query-keyed latest scored 0.2403 vs FIFO 0.2468; delta **-0.0065**, 95% CI **[-0.0292, +0.0195]**. The fixed selective threshold abstained on all these episodes and scored 0.2500, so this threshold was too blunt to help on this slice.
- The current `query_observation_delay_gt_2` stratum has all 384 held-out episodes positive and all 384 development episodes negative because the current family profiles set maximum delay to 2 in development and up to 5 in held-out. It is therefore confounded with the family split and cannot isolate delay as a causal factor.

## Interpretation and next hypothesis

The strongest current signal is **stability-conditioned usefulness**: query-keyed retention beats FIFO when queried facts remain valid after their latest observed source time, but does not improve the cases where a relevant fact changes unseen. The simple development-calibrated confidence/age cutoff also failed overall and on the low-confidence subgroup.

The results above motivated the current candidate revision, which now samples nuisance severities independently per episode and emits separate high/low profile strata for distractor rate, claim-flip rate, maximum delivery delay and corruption. The next exact-head report must be used for conclusions under this factorized protocol; do not assume the historical subgroup numbers persist after changing the generator. Any dynamics-aware policy must estimate risk from released history only and report its fixed memory/compute overhead as well as task loss and risk/coverage.

## Candidate protocol v0.2 — factorized nuisance sampling

The candidate update samples four nuisance dimensions independently for each episode, with the same policy-visible contract and the same source/query stream shared by all policies:

| Factor | Development values | Held-out values | High-severity stratum within family |
| --- | --- | --- | --- |
| Distractor-burst probability | 0.15, 0.25, 0.35 | 0.35, 0.50, 0.65 | Development ≥0.25; held-out ≥0.50 |
| Per-tick claim-flip probability | 0.01, 0.02, 0.03 | 0.03, 0.05, 0.07 | Development ≥0.02; held-out ≥0.05 |
| Maximum delivery delay (ticks) | 0, 1, 2 | 2, 3, 4, 5 | Development ≥1; held-out ≥4 |
| Evidence corruption probability | 0.00, 0.01, 0.02 | 0.04, 0.055, 0.07 | Development ≥0.01; held-out ≥0.055 |

The rates are sampled independently using sequential draws from the deterministic episode PRNG. The values remain evaluator-owned. Policies see only each released observation, its value/confidence, source timestamp, arrival timestamp and public query. Family-conditional high/low flags are used only to aggregate results, never by the policy.

The hazard-aware candidate is deliberately a simple baseline, not a claim of Bayesian calibration: it tracks high-confidence value transitions and elapsed source ticks per queried claim, estimates a regularized per-tick flip rate, discounts source evidence toward uncertainty as it ages, and computes the implied probability of the 3-claim majority. It answers only when expected 0/1 answer loss is lower than the benchmark's 0.25 abstention loss. Its auxiliary sufficient-statistic bytes are included in `mean_logical_state_bytes`; allocator, runtime and wall-clock costs are not.

The report now has eight overlapping evaluator-only dimensions: hidden truth change after latest evidence, observed out-of-order query evidence, low-confidence latest query evidence, any realized query-observation delay above two ticks, and high/low sampled levels of each of the four nuisance factors. This should support within-family sensitivity analysis and show policy-by-factor interactions instead of treating every nuisance shift as one OOD bundle.

**Qualification rule:** the previously recorded exact-head results refer to the earlier fixed-profile generator. The factorized generator plus hazard-aware policy are candidate changes and remain unqualified until the current exact-head tests, deterministic TSV comparison, row checks and artifact SHA succeed. Report per-factor sample counts and confidence intervals; an empty or single-level stratum is a benchmark-design failure, not evidence of robustness. The new policy must be compared on both task loss and logical state cost; its additional statistics cannot be treated as free memory.

## First factorized-profile and hazard-aware results — exploratory

The first factorized nuisance + hazard-policy report was qualified on exact source head `b4e9b1a256ab616159892820446600a4ca62c8f1` by [focused run #39487091189](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39487091189). It passed **14 tests**, deterministic output replay, schema and strata checks, SHA-256 validation, and uploaded [`vis-relational-memory-report`](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39487091189/artifacts/12294865236). These results are **exploratory**, because the policy was proposed after inspecting earlier benchmark failure strata; an independent frozen-seed confirmation is required.

Held-out, capacity 3:

| Policy | Mean loss ↓ | Answer rate ↑ | Answered accuracy ↑ | Logical state bytes | Delta vs FIFO (95% paired CI) |
| --- | ---: | ---: | ---: | ---: | --- |
| FIFO | 0.2539 | 3.65% | 64.29% | 18 | baseline |
| Uniform reservoir | 0.2500 | 0.00% | NA | 18 | -0.0039 [-0.0130, +0.0046] |
| Query-keyed latest | 0.2526 | 27.08% | 74.04% | 18 | -0.0013 [-0.0228, +0.0215] |
| Query-keyed selective | 0.2428 | 22.66% | 78.16% | 18 | -0.0111 [-0.0306, +0.0098] |
| Query-keyed hazard-aware | **0.2161** | **46.88%** | **82.22%** | 48 | **-0.0378 [-0.0625, -0.0098]** |

On this factorized synthetic held-out set, the hazard-aware candidate reduces mean loss by about **14.9% relative to FIFO** and its paired 95% interval is below zero. It also has 2.67× the reported logical state bytes (48 vs 18) because it keeps per-query transition statistics. Those bytes are a serialized logical payload estimate—not peak RAM, latency, or energy. The development-family loss difference is -0.1204 with 95% CI [-0.1452, -0.0951]; this remains a synthetic benchmark result, not evidence of general robotic or SOTA superiority.

The hard subgroup remains important: when at least one queried claim changed after its latest source-time observation (238 held-out episodes), hazard-aware mean loss was **0.2742**, vs FIFO **0.2532**; its paired delta was **+0.0210**, 95% CI **[-0.0147, +0.0620]**. That is inconclusive and not a subgroup win. In the unchanged-after-observation subgroup (146 episodes), hazard-aware loss was 0.1216 and its paired delta vs FIFO was -0.1336, 95% CI [-0.1627, -0.1079]. This supports the hypothesis that transition-aware aging helps exploit stable information, but it does not yet solve surprise changes reliably.

Several other factor slices are still inconclusive. In particular the high claim-flip-rate stratum's hazard-aware delta is -0.0223 with 95% CI [-0.0516, +0.0121], which includes zero. These subgroup analyses are exploratory, overlapping, and involve multiple comparisons; they must not be presented as independent confirmatory discoveries.

## Independent-seed replication — passed, with post-selection caveat

After selecting the hazard-aware hypothesis from earlier exploratory strata, the algorithm and factorized generator were frozen and the report seed root was changed to `0xA54F_F53A_5F1D_36F1`. This is an independent-seed replication, **not a fully preregistered confirmatory study**, because the candidate design followed inspection of earlier benchmark outputs. The resulting exact source head was `649acdd38e0fe49c13c9a0575f4cea079564ec1f`; [focused workflow #39487343659](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39487343659) passed all 14 tests, report determinism, schema/strata checks and the SHA receipt. The report artifact is [vis-relational-memory-report #12293388387](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39487343659/artifacts/12293388387). Documentation-only commit `55583e5945006da9b0bd6dc2e136bd107526168c` subsequently passed its exact-head focused PR workflow [#39487456373](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39487456373). The ordinary cross-platform CI for that documentation head was still in progress when this note was written.

Independent-seed held-out results at capacity 3:

| Policy | Mean loss ↓ | Answer rate ↑ | Answered accuracy ↑ | Logical state bytes | Delta vs FIFO (paired 95% CI) |
| --- | ---: | ---: | ---: | ---: | --- |
| FIFO | 0.2480 | 3.91% | 80.00% | 18 | baseline |
| Uniform reservoir | 0.2500 | 0.00% | NA | 18 | +0.0020 [-0.0065, +0.0091] |
| Query-keyed latest | 0.2572 | 29.43% | 72.57% | 18 | +0.0091 [-0.0124, +0.0339] |
| Query-keyed selective | 0.2572 | 29.43% | 72.57% | 18 | +0.0091 [-0.0143, +0.0326] |
| Query-keyed hazard-aware | **0.2064** | **49.74%** | **83.77%** | 48 | **-0.0417 [-0.0671, -0.0143]** |

This independent run preserves the sign and approximate size of the earlier exploratory effect: hazard-aware loss is about **16.8% lower relative to FIFO** in this synthetic held-out family, with a paired 95% interval strictly below zero. It uses 48 rather than 18 reported logical state bytes at capacity 3. This payload estimate is not peak RAM, runtime, latency or energy, and this result is not evidence of industry or robotic SOTA.

The hardest subgroup remains unresolved. In the 244 held-out episodes where at least one queried claim changed after its latest retained source-time evidence, hazard-aware policy's loss difference versus FIFO was **+0.0123**, 95% CI **[-0.0256, +0.0492]**. It is not a proven regression, but it does not demonstrate improvement in the surprise-change subgroup either. By contrast, on the 140 episodes with no queried-claim change after latest source evidence, its delta was **-0.1357**, 95% CI **[-0.1571, -0.1143]**. Out-of-order observations are another positive slice: on 239 episodes, its delta was **-0.0544**, 95% CI **[-0.0879, -0.0209]**. The high claim-flip-rate slice remains inconclusive: **-0.0266**, 95% CI **[-0.0591, +0.0079]**.

Interpretation: the independent-seed result supports a real synthetic benefit from estimating information lifetime, but the benefit is not yet robustly shown on the particular episodes where the world changes after observation. Next priorities are a third untouched seed root or a preregistered stress suite, confidence-interval-aware subgroup reporting, and external evaluation with benchmark-native task-success metrics. Do not tune against the current confirmation seeds.

## Next: truly preregistered external evaluation required

Before using the hazard result as an engineering acceptance claim, preregister the algorithm and scoring contract **before** accessing the next held-out results, choose a new seed root and factor distributions not used in earlier reports, re-run the factorized suite unchanged, and publish the new exact-head report. Then reproduce the same memory-policy delta on a native external task subset (ActiveArena) while holding model/backbone, action decoder, task seeds and environment fixed. No SOTA claim is warranted until that reproduction is completed with the benchmark-native metrics.

## Confidence-channel leakage audit and correction

**Bug found:** prior generator versions set `confidence=350` iff the synthetic value was corrupted and `confidence=950` otherwise. Since memory policies received confidence, it was a direct corruption label rather than a noisy sensor-confidence estimate. The hazard-aware policy's high-confidence transition history and the selective policy's confidence threshold could exploit this shortcut. This invalidates their earlier performance interpretation for realistic noisy confidence—even though those runs were source-head-reproducible.

**Correction in the current candidate:** draw the reported confidence tier independently of whether the value is corrupted:
- Confidence tier prior: (P(950,850,650)=(0.75,0.20,0.05)).
- Conditional corruption risk by confidence tier: (p_c = p_{base}\times(20,160,320)/63) per thousand, where (p_{base}) comes from the episode's sampled profile.
- These relative risks are normalized so the tier-weighted aggregate corruption rate is approximately the declared base rate. High-confidence events can still be corrupted, and low-confidence events can still be correct.

The report prints this model. A deterministic 100,000-draw calibration test verifies that error rates increase from high to medium to low confidence and that the aggregate remains near the profile base rate. The latest candidate uses seed root `0x510E_527F_ADE6_82D1`.

**Interpretation rule:** rerun the full report on the current exact head. If the hazard-aware advantage shrinks or disappears, report that directly; do not restore the leakage to preserve an earlier result. The candidate should only be considered for external comparison if the noisy-confidence results support it, and then it must still pass benchmark-native external evaluation.

## Next candidate: gap-aware likelihood under irregular sampling

The current branch adds issue [VIS-MEM-004](https://github.com/Luminous-Dynamics/symtropy/issues/2422) as a separate policy, `query_keyed_gap_aware`. The naive hazard estimator uses observed transition count divided by elapsed source ticks; under irregular sampling, that is not the correct likelihood. For a symmetric binary Markov process with per-tick flip probability (p) and observation gap (Delta), the probability of a changed value is:

\[
P(X_{t+\Delta}\ne X_t\mid p)=\frac{1-(1-2p)^\Delta}{2}.
\]

The new policy maintains bounded fixed-point log-likelihood scores over a small hazard grid, initialized by a documented prior. It only updates from released high-confidence observations in increasing source-time order, discounts stale evidence by the posterior expected hazard, and uses the same task loss / abstention threshold. Reported logical state bytes include its grid posterior. Negative controls check gap-one equivalence, normalization/monotonicity, stale-update rejection and state cost.

The policy was added after inspecting earlier experimental outcomes; it is therefore a candidate for exploration, not a confirmatory claim. The new confidence model, gap-aware policy and fresh seed root must pass exact-head qualification before any new metric is reported. The earlier leakage-contaminated results remain historical and cannot be used to validate the new version.

This is a symbolic evidence-stream benchmark. It isolates memory selection, source-time/provenance and decision behavior. It does not test visual perception, natural language understanding, 3D geometry, manipulation, physical safety, actual wall time, hardware memory, GPU use, or energy. Passing it is a diagnostic step—not proof of external benchmark superiority or deployment readiness.

## Corrected-confidence and gap-aware report — exploratory

The corrected-confidence, gap-aware, and evaluator-only hazard-calibration candidate passed exact-head qualification on source head `b4bf81326c6f4c282cff3a8162e1279e4ffcc964`:
- [Focused PR run #39497708756](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39497708756) passed **21 tests**, exact-head verification, byte-for-byte report replay, row/strata checks and SHA validation.
- [Report artifact #12298228043](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39497708756/artifacts/12298228043), archive SHA-256 `61a98f68adbbd93940867f7b5c7413f3516f42e6b1f939ace5cc8e7e0b0f8401`.
- The ordinary cross-platform CI for that same head was still queued at the last check; this is not a claim that the full PR CI was green.

Under the corrected, noisy-confidence generator at capacity 3:

| Policy | Mean loss ↓ | Answer rate ↑ | Answered accuracy ↑ | Logical state bytes | Hazard MAE ↓ | Hazard RMSE ↓ | Delta vs FIFO (paired 95% CI) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| FIFO | 0.2441 | 3.39% | 92.31% | 18 | N/A | N/A | baseline |
| Query-keyed latest | 0.2422 | 28.12% | 77.78% | 18 | N/A | N/A | -0.0020 [-0.0221, +0.0208] |
| Query-keyed selective | 0.2422 | 28.12% | 77.78% | 18 | N/A | N/A | -0.0020 [-0.0234, +0.0189] |
| Query-keyed hazard-aware | **0.2077** | **42.97%** | **84.85%** | 48 | **0.0191** | **0.0234** | **-0.0365 [-0.0599, -0.0124]** |
| Query-keyed gap-aware | 0.2194 | 36.20% | 83.45% | 126 | 0.0339 | 0.0487 | -0.0247 [-0.0482, -0.0013] |

The report now separates task quality from parameter-estimation quality. After each policy has acted, the evaluator compares its per-query-claim flip-hazard estimates against the hidden episode profile rate; that rate is never passed to the policy. The direct negative control proves changing only the hidden evaluator label leaves the policy decision unchanged while changing the evaluator error score. The naive hazard estimator has held-out MAE/RMSE **0.0191 / 0.0234**, while the gap-aware posterior has **0.0339 / 0.0487**.

The simpler naive hazard-aware estimator still beats FIFO on held-out loss in this corrected model. However, the more principled gap-aware posterior does **not** improve on it. The direct paired contrast is reported explicitly: **gap-aware minus naive-hazard loss = +0.0117**, 95% CI **[+0.0033, +0.0202]** on 384 held-out episodes. The development contrast is +0.0085, 95% CI [-0.0007, +0.0176]. This is a negative result for the proposed complexity increase: the gap-aware implementation has about **2.6×** the logical state cost (126 vs 48 bytes), lower answer coverage, higher hazard-estimation error, and worse held-out loss under this generator.

That does not falsify the source-time-gap likelihood math. It rejects the broader claim that adding this small posterior grid improves decision quality in this particular limited-evidence benchmark. Possible causes—coarse hazard grid, fixed prior, sparse high-confidence transitions, or imperfect observation-reliability modeling—need diagnostic evidence before choosing another algorithm. Do not keep the gap-aware policy as the preferred path just because it is more Bayesian-looking.

On held-out episodes where a queried fact changed after the latest source evidence (221 episodes), naive hazard loss was 0.2523 versus FIFO 0.2466 (delta +0.0057, 95% CI [-0.0294, +0.0441]); gap-aware loss was 0.2658 (delta +0.0192, 95% CI [-0.0136, +0.0532]). Neither establishes improvement on surprise changes, and gap-aware is no better on that subgroup. On the high maximum-delay stratum, naive hazard delta was -0.0491 [-0.0771, -0.0199] while gap-aware was -0.0315 [-0.0584, -0.0023]. These overlapping subgroup results are exploratory, not independent confirmation.

**Decision:** keep the corrected confidence channel. Keep the gap-aware implementation as a comparison baseline, but do not promote it over the simpler hazard estimator without a new frozen hypothesis and measured advantage. The next useful experiment is to audit/calibrate the observation-reliability mapping and report transition-hazard estimation error (evaluator-only), not add further state to chase the scalar loss.

## Joint latent-state / sensor-noise candidate — under qualification

A separate candidate, `query_keyed_joint_noise`, is now implemented on the branch. It is not yet a performance result.

The preceding evaluator diagnosis found that the gap-aware transition posterior both estimated hazard less accurately (held-out RMSE 0.0487 vs 0.0234 for naive hazard) and had higher task loss. The next hypothesis is that the temporal likelihood needs an explicit sensor-emission model rather than treating each observed value change as a latent state change.

For each queried claim, the joint policy keeps a bounded posterior over 7 candidate state-transition hazards × 6 candidate sensor corruption base rates. It propagates the probability of the latent binary state over source-time gaps, then updates with the released observed value and confidence tier's conditional emission error. The policy sees only observations, confidence, source/arrival timestamps and public query; it does not see the episode's true transition rate, base corruption rate, hidden truth, seed or corrupted flag. Out-of-order source timestamps do not rewind the filter. All of the additional posterior bytes are included in the policy's logical-state cost.

The report emits direct paired comparisons for:
- gap-aware minus naive hazard;
- joint-noise minus naive hazard;
- joint-noise minus gap-aware.

It also reports evaluator-only hazard MAE/RMSE after policy decisions. Tests must verify that modifying only a hidden evaluator rate changes the diagnostic score but not the decision or task loss; such labels are allowed only in evaluator-owned calculation code.

This candidate was devised after inspection of earlier corrected-confidence results. It is exploratory and must not be called confirmatory. The next exact-head run must pass the Rust tests, deterministic TSV replay, expected row/contrast checks and SHA artifact. If the joint model's task-loss interval or calibration error does not improve over the simpler hazard policy enough to justify its state cost, reject it rather than expanding the model again.

## Joint transition/noise filter results — exploratory, not an advancement

The joint filter and sensor-base-rate comparison passed focused exact-head qualification on source head `22858bcdb865386763ecce0cb751425ce857924c`; [focused run #39499054636](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39499054636) passed **24 tests**, deterministic report replay, row/strata/pairwise checks and SHA validation. [Artifact #12298383243](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39499054636/artifacts/12298383243), archive SHA-256 `baa90d9e342e4121307f0439dae7c274113b645b2e33a72f39a2b63616f06f69`. Cross-platform CI for this same head had not completed at the time of this report.

Held-out results at capacity 3:

| Policy | Mean loss ↓ | Answer rate ↑ | Answered accuracy ↑ | Logical state bytes | Hazard MAE ↓ | Hazard RMSE ↓ | Sensor-base MAE ↓ | Sensor-base RMSE ↓ | Fixed-prior sensor MAE | Fixed-prior sensor RMSE | Delta vs FIFO (paired 95% CI) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| FIFO | 0.2422 | 5.21% | 90.00% | 18 | N/A | N/A | N/A | N/A | 0.0223 | 0.0255 | baseline |
| Query-keyed latest | 0.2480 | 27.86% | 75.70% | 18 | N/A | N/A | N/A | N/A | 0.0223 | 0.0255 | +0.0059 [-0.0150, +0.0280] |
| Naive hazard-aware | **0.2227** | 40.10% | 81.82% | 48 | **0.0192** | **0.0239** | N/A | N/A | 0.0223 | 0.0255 | -0.0195 [-0.0443, +0.0059] |
| Gap-aware likelihood | 0.2279 | 31.77% | 81.97% | 126 | 0.0357 | 0.0501 | N/A | N/A | 0.0223 | 0.0255 | -0.0143 [-0.0358, +0.0085] |
| Joint transition/noise | 0.2350 | 51.82% | 77.89% | 786 | 0.0325 | 0.0456 | 0.0212 | 0.0246 | 0.0223 | 0.0255 | -0.0072 [-0.0365, +0.0254] |

The joint candidate's high coverage does not translate to better task loss or transition estimation. Against FIFO its paired interval includes zero. Against the naive hazard estimator the point difference is **+0.0124 loss**, 95% CI **[-0.0072, +0.0345]**; against the gap-aware likelihood it is **+0.0072**, 95% CI **[-0.0156, +0.0293]**. Neither direct held-out contrast demonstrates the joint model is superior. It also consumes **786 logical state bytes**, versus 48 for naive hazard, and has worse hazard MAE/RMSE than the naive estimator.

The joint sensor estimate is slightly better than the evidence-free uniform-grid prior, but the margin is small: held-out sensor-base MAE is **0.0212 vs 0.0223**, with a paired 95% interval for joint-minus-prior **[-0.0016, -0.0008]**; RMSE is **0.0246 vs 0.0255**. On development data, the paired MAE delta is -0.0034, 95% CI [-0.0038, -0.0029]. This is statistically detectable in the synthetic generator, but too small to rescue the task-level result or justify the 786-byte state overhead by itself.

On development episodes, the joint model scored 0.1081 and beat the gap-aware model (paired joint-minus-gap loss -0.0221, 95% CI [-0.0404, -0.0026]). The held-out result loses that advantage, consistent with weak generalization of the joint prior/likelihood under the stress shift. Its synthetic task result is therefore not accepted as advancement.

**Decision:** do not promote `query_keyed_joint_noise`. The model modestly improves corruption-rate estimation over its prior, but loses task-level accuracy/coverage trade-offs against the simpler hazard model and consumes much more state. Retain it as a diagnostic comparison only; any refinement needs to show task-loss improvement on a fresh, frozen generator and a clear resource-quality trade-off. The evaluator-only q-rate diagnostic is tracked in [VIS-MEM-005](https://github.com/Luminous-Dynamics/symtropy/issues/2423), and the joint-model design/claim boundary in [VIS-MEM-006](https://github.com/Luminous-Dynamics/symtropy/issues/2424).

## Sensor-base-rate identifiability diagnostic — current candidate

The joint model now reports both transition-hazard calibration and sensor-base-rate calibration. For each held-out episode, the evaluator compares the joint posterior's expected base corruption rate for each queried claim against the hidden episode profile rate **after** the decision, then averages MAE/RMSE across the three query claims. The estimate and error are not exposed to the policy.

The report also compares that inferred q rate with an evidence-free fixed prior baseline: the arithmetic mean of the six declared candidate base rates. A paired bootstrap of per-episode absolute-error differences reports whether conditioning on the observations improves q-rate estimation over the fixed prior. The baseline is shared across policies and is not selected using held-out observations or outcomes.

A dedicated negative control changes only the hidden corruption-rate label on an already-generated episode and asserts that the joint policy's decision and task loss remain identical while its evaluator-only sensor-rate error changes. Focused CI requires the sensor-MAE/RMSE columns, the prior-baseline columns, and the paired sensor-MAE comparison row.

Interpret the result carefully: q-estimation error is a model diagnostic, not task performance. If the joint posterior improves q estimation but worsens task loss, memory use and task outcomes still decide whether the candidate advances. Conversely, lower q MAE alone is not enough to justify deploying a 42-hypothesis posterior per queried claim.

## Joint parameter identifiability and sample-complexity sweep — candidate

The report now adds a controlled single-claim diagnostic independent of the decision-task generator. It simulates known two-state transition hazards and confidence-conditioned sensor base rates on the model's declared grid, with every source tick observed. For each parameter pair and observation length (8, 16, 32, 64, 128), 24 fixed-seed trajectories are generated. The estimator receives only each observation's value, confidence and source timestamp; the true hazard and corruption rate are used only after filtering to calculate estimation errors.

Each row reports hazard MAE/RMSE and sensor-base MAE/RMSE, alongside an evidence-free uniform-grid prior q MAE. This sweep is designed to distinguish limited information per claim from a biased/mis-specified joint filter. It is an **on-grid** diagnostic: it does not establish robustness to transition/sensor rates outside the model's grid or to the irregular delays/distractor bursts of the full task benchmark.

The focused workflow checks that all 210 length × hazard × sensor-rate rows are present and deterministic. If the estimation curves do not improve with more observations, that is a model correctness or likelihood-computation failure. If they improve but the 48-tick end-task results remain poor, it instead suggests the full task stream is too sparse or confounded for this parameter grid to deliver a useful decision benefit. Neither outcome alone is evidence of robotic performance.

## First joint-parameter sample-complexity measurements

The tabular-format correction and identifiability sweep passed exact-head qualification at `dedce9a7938acd4cae31e30138465347f9e86c6a`: [focused workflow #39499784246](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39499784246) passed **24 tests**, repeated report bytes exactly, validated the main and 210 calibration-row TSV schemas, checked the digest, and uploaded [artifact #12299740901](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39499784246/artifacts/12299740901). Cross-platform CI for this same source head was still in progress when recorded.

Averaged equally across all 42 on-grid hazard × sensor-rate combinations (24 trajectories per combination), estimation error falls as more observations per claim are supplied:

| Source observations per claim | Hazard MAE ↓ | Hazard RMSE ↓ | Sensor-base MAE ↓ | Sensor-base RMSE ↓ | Uniform-prior sensor MAE |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 8 | 0.0663 | 0.0769 | 0.0210 | 0.0220 | 0.0225 |
| 16 | 0.0490 | 0.0604 | 0.0194 | 0.0212 | 0.0225 |
| 32 | 0.0350 | 0.0461 | 0.0173 | 0.0198 | 0.0225 |
| 64 | 0.0237 | 0.0332 | 0.0149 | 0.0175 | 0.0225 |
| 128 | 0.0135 | 0.0205 | 0.0121 | 0.0144 | 0.0225 |

This makes the bottleneck clearer. The joint filter can learn both parameters when data are plentiful and generated on its model grid. At 8 source observations per claim—the approximate evidence volume expected under the current 48-event, three-claim stream with distractor bursts—its hazard error is still high and its sensor-rate estimate only modestly beats the prior. At 128 observations it performs much better, but that is 16 times the per-claim evidence available at the approximate task operating point.

**Research implication:** the next paradigm shift should be active evidence acquisition, not automatically making the posterior larger. The agent should choose which claim to inspect next based on how much that observation is expected to change the task decision, how quickly the claim may drift, and the sensing/action budget. This needs its own matched-budget benchmark with round-robin, random, least-recently-observed and uncertainty/value-of-information policies. These calibration curves are synthetic-only and on-grid; they do not establish external robot performance.

## Reproduce

```sh
rustc --edition=2024 --test tools/vision-relational-memory-bench.rs -o /tmp/vis-mem-tests
/tmp/vis-mem-tests
rustc --edition=2024 -O tools/vision-relational-memory-bench.rs -o /tmp/vis-mem-bench
/tmp/vis-mem-bench
```

The dedicated workflow pins Rust 1.96.0, checks out and verifies the event's exact source head, tests the benchmark, replays the report byte-for-byte, checks required rows, and uploads a SHA-256 receipt.


## First exact-head measurements

The first report was generated on source head `6c4cb3f58693036c5c2b5d5425f1bc019dcfb6f8` by [focused run #39484808095](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39484808095). The workflow passed all 8 tests, repeated the full report byte-for-byte, checked its SHA-256, and uploaded the report artifact.

At capacity 3, the paired development result is promising but the held-out result is not:

| Family / policy | Mean loss ↓ | Answer rate ↑ | Accuracy when answered ↑ | Fresh query-claim coverage ↑ |
| --- | ---: | ---: | ---: | ---: |
| Development / FIFO | 0.2467 | 0.0443 | 0.8235 | 0.5000 |
| Development / query-keyed latest | 0.1784 | 0.4844 | 0.8978 | 0.8082 |
| Held-out / FIFO | 0.2513 | 0.0156 | 0.6667 | 0.3325 |
| Held-out / query-keyed latest | 0.2513 | 0.0990 | 0.7368 | 0.5278 |

The development paired loss delta (query-keyed minus FIFO) is **-0.0684**, with 95% bootstrap interval **[-0.0885, -0.0469]**. On held-out episodes the point delta is **0.0000**, with interval **[-0.0117, 0.0130]**. The held-out result does not qualify as an improvement. Query-keyed retention increases answer coverage from 1.56% to 9.90%, but 73.68% answered accuracy is below the 75% break-even accuracy implied by this benchmark's incorrect=1 / abstain=0.25 loss. More answers alone are not a win.

Uniform reservoir does not answer any held-out episodes at capacities 1, 2, 3, 4 or 8 in this first run; at capacity 8 it also fails to improve mean loss over abstention. That is a result of this stream generator, not a general indictment of reservoir sampling. The query-keyed policy uses at most three entries even when the configured cap is 4 or 8, so those larger capacities do not create more queried-claim slots.

## Interpretation and next experiment

The central result is a **development-to-held-out collapse**: query-keyed latest-source-time memory improves the development loss but not held-out loss under higher claim drift, delays and corruption. The next experiment should isolate this generalization gap rather than tune the policy directly against held-out scores:
1. add evaluator-only strata for recent truth changes, delayed out-of-order arrivals and low-confidence/corrupted latest evidence;
2. calibrate a selective answering threshold on development episodes only, freeze it, then report held-out risk/coverage and paired loss;
3. retain the same task loss, seed pairs, memory budgets and report integrity checks.

These strata are diagnostic labels for the evaluator only; they must not enter the memory policy. A calibrated policy is accepted only if the frozen development-selected threshold improves the held-out paired loss interval without hiding answer coverage or subgroup regressions.
