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
- Development and held-out profiles change distractor frequency, claim-change rate, maximum delay, and corruption rate.
- Policies: bounded FIFO, deterministic seeded uniform reservoir, query-keyed latest-source-time memory, and a selective query-keyed decision policy whose freshness/confidence threshold is chosen on separate development-calibration seeds and then frozen for both report families.
- Capacities: 1, 2, 3, 4, and 8 evidence entries. Each policy receives the same released stream and public query at each episode/capacity pair.
- 12 seed replicates × 32 episode seeds = 384 paired episodes per family and policy/capacity.
- Loss: correct answer = 0; abstention without fresh retained evidence for all three queried claims = 0.25; incorrect answer = 1.
- Metrics: mean loss (primary), answer rate, accuracy on answered episodes, fresh query-claim coverage, mean memory entries, the selected selective threshold, and paired 95% bootstrap intervals for every policy's loss difference versus FIFO. Positive deltas mean worse loss.
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
