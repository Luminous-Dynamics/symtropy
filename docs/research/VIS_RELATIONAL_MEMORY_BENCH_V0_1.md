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
- Policies: bounded FIFO, deterministic seeded uniform reservoir, and query-keyed latest-source-time memory.
- Capacities: 1, 2, 3, 4, and 8 evidence entries. Each policy receives the same released stream and public query at each episode/capacity pair.
- 12 seed replicates × 32 episode seeds = 384 paired episodes per family and policy/capacity.
- Loss: correct answer = 0; abstention without fresh retained evidence for all three queried claims = 0.25; incorrect answer = 1.
- Metrics: mean loss (primary), answer rate, accuracy on answered episodes, fresh query-claim coverage, mean memory entries, and paired 95% bootstrap intervals for every policy's loss difference versus FIFO. Positive deltas mean worse loss.
- Fixed-seed report replay and tests cover delayed stale evidence, query-irrelevant distractors, capacity bounds, and deterministic bootstrap.

## Hypotheses and advancement gate

1. Query-keyed latest-source-time memory with capacity ≥3 should reduce held-out loss versus FIFO by preserving the three facts directly needed by the public query while filtering irrelevant repeated observations.
2. Uniform reservoir may benefit from larger capacity, but sampling can discard a recent necessary claim even when it preserves broader history.
3. If query-keyed memory does not improve the held-out paired interval versus FIFO, do not claim superiority. Inspect claim-change and delay slices, then test uncertainty-aware evidence retention rather than changing the score after seeing results.

Proposed stretch gate for a confirmatory version: at capacity 3, at least 10% relative held-out mean-loss reduction against FIFO, a paired 95% confidence interval for the loss difference strictly below zero, no answered-accuracy regression larger than 2 percentage points, and no increase in budget violations. These are hypotheses, not achieved results.

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
