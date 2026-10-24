# VIS-BENCH-001: external benchmark matrix for active relational memory

**Status:** research and execution contract. No external robotics result has been produced by Symtropy as of 2026-10-21. Synthetic results must not be described as robotics success, SOTA, or an industry comparison.

**Research update (2026-10-21):** ActiveArena's official results table confirms ActiveArena-OFT at 62.97% ID / 47.60% OOD task-macro success (35 tasks), and RoboMME's current leaderboard reports 16 tasks / 800 episodes, with PonderPounce-9B at 60.83 overall among listed model entries and Human Performance at 90.50. These are separate benchmark scales and must not be compared numerically to each other or to the synthetic loss. See the [ActiveArena results](https://leeibo.github.io/ActiveArena/) and [RoboMME leaderboard](https://robomme.github.io/leaderboard.html).

## Core paradigm shift

Treat memory as a task-conditioned allocation of *decision-relevant information*, not a contest to store the most frames or the largest latent state. The controller should decide:

1. **What to retain:** relational facts, object identity/location, event counts, procedural demonstrations, and perceptual frames have different utility.
2. **When to refresh:** source time, arrival time, reliability, occlusion, and estimated world-change hazard affect whether old evidence remains trustworthy.
3. **When/where to observe next:** acquire the view or fact that is expected to change a consequential decision, subject to a declared cost.
4. **When to stop:** stop only when calibrated marginal decision value is no greater than the real observation/action cost, while protecting against myopic information complementarities.
5. **What to prove:** isolate memory, active sensing, and controller interactions with paired ablations and external evaluators.

RoboMME's published analyses support testing task-conditioned representations rather than assuming one memory form dominates: its 16-task suite spans temporal, spatial, object, and procedural memory, and its reported findings identify different winners for different task demands. ActiveArena provides the corresponding active-perception test with multi-round evidence acquisition. See the official [RoboMME page](https://robomme.github.io/) and [ActiveArena page](https://leeibo.github.io/ActiveArena/).

## Benchmark matrix

| Benchmark | Native focus and protocol | Role for Symtropy | What counts as evidence |
|---|---|---|---|
| Local VIS-MEM / VIS-ACT synthetic diagnostics | Controlled evidence streams, fixed seed roots, delayed/stale/corrupted observations, distractors, paired episode comparisons | Fast mechanistic falsification; verify source-time semantics, leakage resistance, determinism, budget accounting and policy behavior | Exact-head Rust tests and deterministic report artifacts only. Never equate synthetic loss or work-unit proxies with robot success, milliseconds, VRAM, energy or SOTA. |
| [ActiveArena](https://leeibo.github.io/ActiveArena/) | 35 tasks in 5 categories; controllable views; fixed ID/OOD protocol; 18-dimensional actions; task-macro success | Primary external target for active evidence acquisition and memory-under-viewpoint-change | Pin upstream evaluator, simulator, checkpoint, assets, prompts and seed lists; reproduce baseline first. Official page reports ActiveArena-OFT at 62.97% ID / 47.60% OOD task-macro success and ActiveArena-Plan at 60.97% / 45.54%. These are literature references, not Symtropy results. |
| [RoboMME](https://robomme.github.io/) | 16 manipulation tasks in four suites (Counting, Permanence, Reference, Imitation); 800 test episodes (50/task); published memory ablations on a shared π0.5 backbone | Primary external target for relational/temporal memory representation and integration ablations | Match the eligible backbone/checkpoint and evaluator; report task and suite macro success, not pooled frames. The leaderboard last reported as updated 2026-09-29 shows PonderPounce-9B at 60.83 overall among listed non-oracle entries and Human Performance at 90.50; oracle subgoal predictors must not be described as deployable SOTA. Official study highlights task-dependent memory utility and memory/compute trade-offs. |
| [BAVO-Bench](https://github.com/HCPLab-SYSU/BAVO-Bench) / [paper](https://arxiv.org/abs/2609.37292) | Bimanual active vision with controlled external occlusion and viewpoint recovery; paper released in September 2026 | Promising secondary stress test for recovery after task-relevant visual evidence disappears | The project repository's current roadmap still lists dataset and code release as future items. Track it as a research target, but don't make it an executable pass/fail gate until the actual evaluator, data and code are released and pinned. |
| [RoboCerebra](https://robocerebra.github.io/) | Long-horizon manipulation; official project page describes 100 task variants and 1,000 annotated trajectories, with evaluation over 600 rollouts | Later test of memory persistence, hierarchical planning and recovery from disturbances | Separate planner/controller ablations, preserve official anchor-aligned evaluation, and report long-horizon completion/failure modes; do not compare its metrics numerically with ActiveArena or RoboMME. |

### Important published reference points

The [ActiveArena paper/project page](https://leeibo.github.io/ActiveArena/) reports task-macro ID/OOD success of **62.97% / 47.60%** for ActiveArena-OFT and **60.97% / 45.54%** for ActiveArena-Plan, averaged over 35 tasks. OOD weak spots include Multi-Object Loop (33.33%), Multi-Object Decision (27.20%), and Interactive Information Acquisition (35.33%). Use these as external context; reproduce the baseline locally before claiming head-to-head superiority.

The [RoboMME paper](https://proceedings.mlr.press/v306/dai26c.html) studies 14 memory-augmented VLA variants on a shared π0.5 backbone across 16 tasks. Its published conclusion that memory representations are task-dependent motivates a **hybrid, task-conditioned memory policy**, not a claim that Symtropy's current symbolic store already beats perceptual or recurrent VLA memory.

## Required controlled ablations

Run the same seed list and base controller through:
1. no persistent memory / current observation context only;
2. bounded FIFO or recent-window history;
3. uniform frame/evidence sampling;
4. query-keyed relational memory with explicit source and arrival times;
5. hazard-aware freshness and reliability;
6. task-conditioned hybrid memory (symbolic relations + selected perceptual snippets);
7. active evidence/view selection without changing the memory representation;
8. the combined memory + active-selection controller, only after individual effects are measured.

For a valid controller-only ablation, keep model checkpoint/backbone, prompt, image preprocessing, action head/decoder, action executor, precision, simulator revision and ordered seeds fixed. If any one changes, label it a separate experiment rather than attributing the result to memory.

## Metrics and statistical contract

### Primary outcomes
- Native benchmark success, computed exactly by each evaluator.
- Task-macro mean; per-task and per-suite/category success; ID and OOD kept separate where defined.
- Paired episode-level deltas using the same task/seed across methods, with bootstrap intervals clustered/stratified by task (and scene when available).
- Failure rate, reset/action-interface errors, incomplete rollouts, and missing episodes; failures are not silently discarded.

### Resource and information efficiency
- Number of observation requests/views and fraction that contribute relevant evidence.
- Success-versus-observation-budget frontier; do not optimize coverage or answer rate alone.
- Measured end-to-end latency (median/p95), controller CPU time, GPU utilization/VRAM, system RAM, and hardware/driver/software versions.
- Calibrated confidence/reliability, stale-evidence errors, and recovery success after occlusion or world change.

Work-unit counters and logical-state bytes from the synthetic Rust bench are diagnostic proxies only. A hardware claim requires wall-clock/device measurement under the same rollout and inference configuration.

## Internal advancement gates (not community standards)

- **Synthetic stop/value candidate:** the threshold is selected only on development episodes. Advance only if the selected threshold's held-out paired 95% interval for cost-adjusted objective delta versus round-robin is entirely below zero, while publishing raw task-loss delta and requests used. Otherwise reject it as the preferred policy.
- **RoboMME memory ablation:** first reproduce a named eligible baseline with the official evaluator, checkpoint and test seeds. Target at least +5 absolute percentage points in task-macro success with paired evidence; additionally disclose suite-level regressions rather than hiding them in the average.
- **ActiveArena active-perception ablation:** first reproduce the strongest eligible baseline locally. Target at least +5 absolute percentage points OOD task-macro success, with paired 95% interval above zero and no more than 3 percentage points absolute ID regression. This is an ambitious project target, not an established SOTA threshold.
- **Replication:** freeze code/config/checkpoint/seed digests, publish full result tables and failed/missing episode counts, and re-run the chosen final configuration on a separate seed or benchmark when the official protocol allows it. Never tune on the final held-out test.

## Execution order

1. Finish exact-head qualification of the current synthetic candidate; use its results to reject or retain policies, not as external performance claims.
2. Reproduce a small but official baseline subset and one actual end-to-end episode for the chosen external benchmark. A launcher dry run is not an actual task execution.
3. Prioritize RoboMME for memory-component ablations and ActiveArena for the view-selection controller; keep the datasets' native metrics separate.
4. Add hybrid memory and active selection only after single-factor ablations identify a measurable mechanism.
5. Add BAVO-Bench occlusion-recovery and RoboCerebra long-horizon evaluation after confirming their exact release/test protocols.

The first external milestone is a reproducible baseline plus an honest controller-only ablation, not a premature SOTA headline.
