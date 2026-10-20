# VIS-MEM-002: query-conditioned relational evidence memory v0.1

**Status:** benchmark implementation and hypotheses. Performance claims must come from an exact-head report; no SOTA or robotic-performance claim is made here.

## Why this experiment exists

The qualified VIS-ACT rendered/timing pilot found held-out mean loss of 0.0794 for bounded FIFO, 0.1927 for uniform reservoir capacity 2, and 0.0859 for fixed sweep. The reservoir-versus-fixed-sweep paired bootstrap interval for its loss increase was [0.0885, 0.1237]. The subsequent recency-protected reservoir matched FIFO at 0.0794 on that same single-target generator. That is a valuable non-win: protecting a recent direct observation did not improve the measured task.

External benchmarks motivate testing a harder capability:
- [ActiveArena](https://leeibo.github.io/ActiveArena/): interactive search, multi-object loops, multi-object decisions, and information acquisition, with ID/OOD splits.
- [RoboMME](https://proceedings.mlr.press/v306/dai26c.html): 16 manipulation tasks spanning temporal, spatial, object, and procedural memory.
- [BAVO-Bench / A-FAR](https://hcplab-sysu.github.io/BAVO-Bench/): controlled occlusion recovery, including unpredictable-time interruptions, in multi-stage manipulation.

These are external comparators, not interchangeable scoreboards. See the existing [ActiveArena reproduction track](https://github.com/Luminous-Dynamics/symtropy/issues/2332).

## Paradigm shift

Compare raw observation retention against a compact, versioned set of **query-relevant beliefs**. An observation is not intrinsically valuable: its value depends on which question must be answered, the claim it supports, when the evidence was sourced, and whether a later observation supersedes it.

Query relevance is public input, so query-keyed retention may filter irrelevant claims. It may not inspect hidden truth, future observations, evaluator labels, or seeds. Source timestamp and arrival timestamp are distinct: an older observation arriving late must not overwrite a newer source-time claim.

## v0.1 protocol

- Eight latent binary claims; each episode exposes a query over three distinct claim keys. The query asks whether at least two of the three claims currently hold.
- A 48-event evidence stream contains correlated hot-distractor bursts, independently changing claims, delayed/out-of-order source times, and occasional corrupted evidence.
- Every episode independently samples distractor frequency, claim-change rate, maximum delay, and corruption from a family-specific range. Factor values vary within each family, so high/low nuisance strata are not assigned solely by the development/held-out label.
- Policies: bounded FIFO, deterministic seeded uniform reservoir, query-keyed latest-source-time memory, a query-keyed selective policy calibrated on separate development seeds, and a query-keyed hazard-aware policy that estimates per-claim transition hazard from released high-confidence evidence.
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

## Limits

This is a symbolic evidence-stream benchmark. It isolates memory selection, source-time/provenance and decision behavior. It does not test visual perception, natural language understanding, 3D geometry, manipulation, physical safety, actual wall time, hardware memory, GPU use, or energy. Passing it is a diagnostic step—not proof of external benchmark superiority or deployment readiness.

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
