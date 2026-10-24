# Shipyard paired dispatch benchmark

This crate creates a deterministic synthetic corpus for matched evaluation of
fixed-order, earliest-due-date, critical-ratio, and least-slack dispatch rules.
Every method receives the same episode's policy input. The independent audit
receives the full effective-outage truth separately.

## Quick start

    cargo test --locked --offline --manifest-path tools/shipyard-dispatch-benchmark/Cargo.toml
    cargo run --locked --offline --manifest-path tools/shipyard-dispatch-benchmark/Cargo.toml -- \
      --episodes 64 --seed 20261024 --out /tmp/shipyard-dispatch-corpus \
      --policy-revision <exact-source-revision> --evaluator-revision <exact-source-revision>

The episode count must be a multiple of eight so every scenario family gets
development and held-out episodes. The runner writes a frozen corpus manifest,
exact-byte initial-state and disturbance-tape artifacts, a JSONL result ledger,
content-addressed candidate/audit sidecars, and a paired comparison summary.
Every output has an explicit SHA-256 or is referenced by a digest-bearing ledger
record. The budget profile is descriptive: this harness does not enforce a
wall-clock or compute-unit budget and must not be used to compare runtime
claims.

## Scenario families

- nominal: no declared outage.
- known resource outage: an outage is visible at the decision tick and must
  be avoided prospectively.
- hidden surprise: evaluator-side outage truth is not in the policy input.
  Auditing may reject a plan that could not have anticipated the event; this
  deliberately measures realized failure rate, not clairvoyant policy skill.
- frozen commitment conflict: an observed outage intersects immutable
  pre-existing work. The policy must preserve the commitment and report the
  conflict rather than silently repair history.

Within each family, the generator splits deterministic seeds into development
and held-out partitions. These are generated synthetic instances, not
industrial data. Do not tune on held-out episodes, and do not interpret sample
differences as proof of generalization to live yards.

## Paired analysis

Comparisons use the same episode, initial-state digest, full disturbance-tape
digest, and budget profile. The summary reports every terminal-status count
separately, then reports paired metric deltas only on episodes where both
methods pass the abstract independent audit. It includes eligible-pair counts,
wins/ties/losses, median delta, and signed sum of deltas. Ineligible episodes
remain in the coverage and status counts; they are not silently converted into
zero-cost outcomes.

Lower makespan, total tardiness, and occupied resource ticks are descriptive
objectives—not a blended score. Abstract feasible means only that the current
unary-resource/interval/release/duration/precedence model passed.
Geometry, polygon occupancy, transfer paths, crane kinematics, load charts,
causality replay for multi-step policies, and physical safety are not modeled
by this harness. A complete result-ledger terminal status remains incomplete
until those declared independent gates and the runtime budget receipt are
executed by a broader runner.

## Published industrial baseline

Before claiming improvement over industry-facing literature, follow
[PUBLISHED-BASELINE-REPRODUCTION.md](PUBLISHED-BASELINE-REPRODUCTION.md).
It pins the published 2026 irregular-block ABC/ALNS Case9_50_x reference values,
the author's declared model assumptions, and the evidence gates required for a
reproduction. Machine-readable aggregate-only targets are in
`fixtures/published-case9-50-anchor-v0.json`; they are not raw run outputs and
are explicitly marked unreproduced. This repository has not reproduced the
paper's results yet.

## What would justify an SOTA claim

1. Reconstruct a published problem with the original geometry, constraints,
   objective denominator, disturbances, and run protocol before comparing
   point estimates.
2. Add the reported ABC/ALNS and GA/DE comparators—or reproducible equivalent
   implementations—under matched compute budgets and identical event tapes.
3. Retain exact solver status/bounds for small cases and report objective gaps.
4. Publish paired per-episode differences, uncertainty, hard-gate failure
   rates, tail behavior and complete exclusions.
5. Validate first against historical episodes, then shadow an APS/MES workflow,
   then compare controlled live outcomes with a site safety owner. Synthetic
   results alone cannot satisfy this evidence ladder.
