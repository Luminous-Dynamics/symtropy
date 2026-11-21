# LADDER Mission Benchmark Contract M0 v0.1

**Tracking issue:** [BENCH-LADDER-MISSION-001 #6477](https://github.com/Luminous-Dynamics/symtropy/issues/6477)  
**External benchmark:** https://ladderrobotics.ai/research  
**Official code:** https://github.com/insait-institute/ladder  
**Pinned external code commit:** `e4e204073cc64b0dc49f9669c2951e6bc91fe0af` (main, observed 2026-11-21)

**Status:** M0 protocol snapshot and machine-readable contract. It does not execute the benchmark, import episode logs, modify LADDER, or claim a Symtropy result.

## Why this benchmark matters

LADDER isolates a major weakness hidden by short manipulation demonstrations: success degrades as the number of subgoals increases. It contains 145 tasks across three simulated suites, with 1–10 subgoals in the same environment. The paper reports 2,024 full-benchmark simulated episodes per system and real-world experiments on WidowX and Franka.

The low-level policy achieves 85.4% on one-subgoal tasks, 26.9% on two-subgoal tasks and 0.0% on five-subgoal tasks. Orchestration and verification help, but physical execution failures remain a bottleneck. In the paper, PSP-Agent obtains 40.1% pooled task success and GPT-Policy obtains 62.0%; their cost and suite breakdowns are separate metrics. The live page's higher 71.5% figure is a **LADDER-Lite-only** Gemini 4 Carbon result, not a full-benchmark result.

The core Symtropy hypothesis to test is that evidence-gated subgoal completion, calibrated recovery and capability-/resource-aware scheduling improve full-sequence reliability while reducing false-positive completion and total cost.

## Pinned benchmark contract

The snapshot in `benchmark_snapshot_v0.1.json` pins:
- official repository and exact commit;
- suite names, simulator, robot, task count and per-task episode count;
- full benchmark versus LADDER-Lite distinction;
- official code and episode-data licence boundaries;
- baseline and live leaderboard snapshot values with their scoring scope.

Full benchmark episode coverage is:

| Suite | Simulator | Robot | Tasks | Episodes per task | Expected episodes |
|---|---|---|---:|---:|---:|
| RoboLab-Long | Isaac Sim | Franka | 44 | 8 | 352 |
| SimplerEnv-Long | SAPIEN | WidowX | 54 | 24 | 1,296 |
| RobotArena-∞-Long | Genesis | WidowX | 47 | 8 | 376 |
| **Full total** | | | **145** | | **2,024** |

LADDER-Lite is 10 tasks and 112 episodes: 4 RoboLab tasks, 2 SimplerEnv tasks and 4 RobotArena tasks. It must never be pooled into the full benchmark's overall or suite-mean score.

## Official episode row — exact schema

The official `scripts/leaderboard.py` at the pinned commit packages each evaluator episode into these **seven fields only**:

```json
{
  "suite": "simplerenv_long",
  "task": "relay2e",
  "episode": 0,
  "success": false,
  "subgoals_done": 1,
  "subgoals": 2,
  "steps": 340
}
```

- `episode` is zero-based within the task.
- `success` is read from the official episode log. It must not be supplied by a Symtropy claim or evidence record.
- `subgoals_done` is derived from the evaluator's `goal_objects` and `goal_trace`; dependent Relay/Revisit legs count only when the earlier legs for the same object have been completed in the required order.
- `subgoals` must match the pinned task catalogue.
- The official checker validates task identity, subgoal counts, successful-episode completeness and default episode counts for each task in each claimed suite.

The benchmark commit belongs in `submission.yaml` as `ladder_commit`; it does **not** belong in an episode JSONL row. Symtropy-specific evidence is kept in a separate sidecar joined by `(benchmark_commit, suite, task, episode)`. The official leaderboard checker must never ingest that sidecar.

## Official scoring — preserve the implementation exactly

The pinned `scripts/leaderboard.py` function `score` makes two different uncertainty calculations:

1. **Per-suite success and pooled overall success:** Wilson 95% interval, reporting the larger distance from the point estimate to either interval endpoint.
2. **Equal-weight suite mean:** the unweighted arithmetic mean of the three suite rates. Its reported margin is **not a Wilson interval**. The official code computes:

```text
p_s = successes_s / episodes_s
mean = (p_robolab + p_simplerenv + p_robotarena) / 3
SE = sqrt(sum(p_s * (1-p_s) / n_s for each suite)) / 3
mean_margin_percentage_points = 196 * SE
```

3. **Pooled overall:** total successes divided by total episodes across all included suite rows, with its own Wilson margin.
4. **Subgoal progress:** `sum(subgoals_done)/sum(subgoals)` per suite; overall progress is the equal-weight mean of the three suite progress values.
5. **Breakdowns:** success by subgoal horizon and official task category.
6. **LADDER-Lite:** the Lite task subset receives separate scores; it is not mixed into full-benchmark task rows or used as a substitute for full coverage.

An official submission may cover one, two or all three suites. A partial entry must identify which suites it covers; do not describe it as a full-benchmark result. For full benchmark comparison, require all 145 tasks and 2,024 episodes at the default episode counts.

## Licenses and execution constraints

- Official LADDER code and Bridge policy: MIT.
- Official LADDER evaluation episodes: CC BY 4.0.
- External third-party components/assets retain their own licenses. The official evaluation guide flags some simulator assets/components as non-commercial; inspect each before use.
- API cost in the official leaderboard covers hosted VLM calls only; it excludes GPU time, simulator cost, robot energy, facility time and hardware depreciation. Keep those cost boundaries separate.

## Symtropy extension tracks (do not modify official score)

1. **Evidence-gated completion:** each accepted subgoal maps to an independently observed task event or sensor result, with object/goal identity, timestamp and uncertainty.
2. **Recovery:** categorize precondition failure, missed grasp, slip/drop, carrying timeout, misplacement, undone state, never-started and invalid plan; test recovery policy against locked faults.
3. **Capability-aware planning:** only plan actions inside the separately qualified joint/hand/actuator envelope.
4. **Resource-aware endurance:** report electrical energy at a declared measurement boundary, thermal state, cooldown, retries, action count and service burden.
5. **Graceful degradation:** controlled sensor/actuator impairments with safe halt or recovery, tracked separately from official score.

No Symtropy extension value may be inserted into the official leaderboard fields or used to redefine official success.

## Reproduction entry points

From the official pinned repository:
```bash
scripts/eval.sh --suite simplerenv_long --method privileged_verifier
scripts/eval.sh --suite simplerenv_long --task all --policy-server localhost:8000 --name my_policy
scripts/eval.sh --suite simplerenv_long --task lite --policy-server localhost:8000 --name my_policy_lite
```

For a full leaderboard entry, run all tasks in every claimed suite with default episode counts; package the logs with `scripts/leaderboard.py package` and validate with `scripts/leaderboard.py check`. The public submission must expose its code and result artifacts for independent rerun. Do not call a Lite score a full benchmark result.

## Claim ceiling

M0 pins the evaluation protocol. It proves no Symtropy performance, no physical robot qualification and no SOTA result. Do not execute physical robot tests until separate risk review and hardware supervision are in place.
