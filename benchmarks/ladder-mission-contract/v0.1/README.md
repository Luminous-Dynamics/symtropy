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

## Official scoring to preserve

- An episode is a task success only if **every required subgoal** is completed.
- Suite success is computed from all episodes for all tasks in that suite using the official default episode counts.
- The paper's `overall` score pools all episodes. The `suite mean` is the unweighted mean of the three suite success rates. Report both; do not call them the same number.
- The reported `±` is the larger side of the 95% Wilson interval.
- Subgoal progress is the share of required subgoals completed. A later Relay/Revisit leg only counts if its prerequisite legs were completed in order.
- Also report horizon and category breakdowns.
- No failed episode may be dropped or selectively rerun. The leaderboard checks missing tasks and incorrect episode counts.
- Privileged systems must be marked as privileged and are not ranked alongside real-observation entries.
- State `tuned_on_ladder` truthfully if prompts, thresholds, weights or other parts of the system were tuned on LADDER scenes/tasks. Per-task prompts/settings are disallowed.
- Pin `ladder_commit`; results across different benchmark versions are not comparable.

The JSON schemas in this directory define a local *run manifest* and *derived result record*. They do not replace LADDER's official `episodes.jsonl`, `submission.yaml`, package script or validator.

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
