# AV Pareto Resource Frontier v0

**Issue:** #6770. Composes with #6750 (all-action EVSI), #6555 (multi-round executor), #6492 (response windows/silence), #6249 (foreground/background isolation), #6183 (AV evidence plane), and #6754 (blind custody).

## Objective

Compare matched AV policies as a quality–resource frontier, not one weighted leaderboard score. Accuracy, response-window rate, silence compliance, regret, latency, energy, audio bytes and video frames remain separate axes. Hard safety/deadline/resource caps are applied before Pareto analysis.

## Paradigm

From "which aggregate score is highest?" to: **which candidates are non-dominated under an explicit operating envelope, what does each frontier point cost, and where does it fail?** A battery-powered wearable, low-latency assistant, high-throughput video service and privacy-constrained robot can rationally choose different points on the same measured frontier.

## Match identity and attribution

Each run shares protocol, cohort/split, model, runtime, hardware and input-envelope digests. A policy-only ablation changes only the policy source digest; end-to-end comparisons may change model/backbone but must be labelled system-level. Different cohorts or hardware envelopes are rejected, not normalized after the fact.

## Metric/provenance contract

Each metric declares a stable ID, unit, direction, kind and tolerance. Initial axes include safe-decision accuracy, response-window rate, silence compliance, regret, p95 latency, energy, audio bytes and video frames. Real runs should also report p50/p99, tokens, model calls, memory, network bytes, failure counts and evidence violations.

Statuses are `MEASURED`, `DERIVED_FROM_MEASUREMENT`, `SYNTHETIC`, `ESTIMATED`, `NOT_MEASURED` or `NOT_APPLICABLE`. A measured frontier requires a measurement method and evidence digest. Missing metrics never become zero; estimated energy cannot qualify a measured energy axis. Synthetic frontiers remain separate from empirical claims.

## Pareto semantics

A dominates B only when A is no worse on every required metric (within preregistered tolerance) and strictly better on at least one. Ties remain ties. Hard caps (e.g. minimum accuracy, maximum p95 latency, measured energy/bandwidth or zero-tolerance safety violations) make a run ineligible even if it has higher accuracy. Optional resource budget slices produce their own frontiers. No weighted aggregate is calculated.

## Synthetic known-answer fixture

Four hand-authored rows exercise distinct outcomes: always-on is best on accuracy but fails the energy cap; adaptive EVSI and minimal early-stop are both non-dominated trade-offs; a weak adaptive policy is dominated. These are evaluator mechanics, not model results or physical measurements.

## Local execution

```sh
python3 tools/av-modality-necessity-evaluator/resource_frontier.py \
  --bundle research/av-pareto-resource-frontier-v0/fixtures/synthetic-comparison-v1.json \
  --output /tmp/av-pareto-frontier-scorecard.json
python3 -m unittest discover -s tools/av-modality-necessity-evaluator -p "test_resource_frontier.py" -v
```

## External benchmark baselines and targets

Keep official metrics separate; do not combine or rank unlike tasks.

- **SPOT-Bench (NeurIPS 2026 E&D):** published 1 Hz Ctrl-SPOT T-F1@5 is 20.9 overall (ABD 22.2, PNR 19.5). Reproduce the official protocol, then target 23 and 25 without worsening dead-time false positives. https://dibschat.github.io/SPOT-Bench/
- **DIVE (Oct 2026):** 525 continuous AV samples over six tasks; leading reported overall response-window rate is 53.77% at 0.831 s mean reaction latency. Reproduce per-task WRR; target 60% overall and improved proactive intervention at matched latency. https://arxiv.org/abs/2610.28405
- **InteractionBench (Oct 2026):** 1,060 interactions over 812 videos; the paper reports Qwen3-VL-8B timing accuracy 77.8 but silence compliance 10.9, and no open-weight system clears a third of near-miss suites. Target at least 50% near-miss success while retaining 90% silence compliance and 75% timing accuracy on the exact protocol. https://arxiv.org/abs/2610.05775
- **ALIVE (2026):** reports 52.8 FSI-Bench EDP and 76.0 StreamingBench. Reproduce both official protocols; target 60+ EDP without regression on streaming quality. https://lijinzhao30.github.io/ALIVE/
- **Active Visual Perception review (15 Oct 2026):** recommends separating acquired evidence, downstream task impact and action/system costs, with matched action spaces and resource envelopes. https://doi.org/10.3390/s26206544

External scores above are published reference points, not scores on this synthetic frontier. They use different tasks, splits and metrics.

## Execution order

1. Pass the synthetic schema, dominance, provenance and hard-cap negatives.
2. Bind #6753 EVSI calibration/regret and #6497 timing/silence metrics into matched run manifests.
3. Compare always-on, fixed cadence, random budget-matched, EIG, myopic EVSI, deadline-aware EVSI, limited-lookahead, AVP-style and Symthaea policies under identical streams.
4. Instrument p50/p95/p99, energy, peak memory, bytes/frames/tokens and all failed/timeout attempts.
5. Repeat held-out seeds; only then report confidence intervals and empirical frontiers.
6. Use #6754 for independent confirmatory custody.

A synthetic pass validates only deterministic evaluator mechanics. It does not prove real efficiency, safety, privacy or SOTA superiority.
