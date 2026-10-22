# VIS-MEM-010: cross-benchmark memory routing and embodied transfer (v0.1)

**Status:** research and execution contract. No Symtropy robotics result or SOTA claim is implied by this document.

**Task:** [VIS-MEM-010 #2590](https://github.com/Luminous-Dynamics/symtropy/issues/2590)  
**Causal use:** [VIS-MEM-009 #2583](https://github.com/Luminous-Dynamics/symtropy/issues/2583)  
**Interference:** [VIS-MEM-007 #2552](https://github.com/Luminous-Dynamics/symtropy/issues/2552)  
**Lifecycle semantics:** [VIS-MEM-008 #2572](https://github.com/Luminous-Dynamics/symtropy/issues/2572)

## Executive hypothesis

The next useful architectural change is **task-conditioned memory routing under a fixed end-to-end budget**, with separate state for scene/spatial relations, events/action outcomes, and selected perceptual evidence. The system must learn which modality to preserve without seeing a future request, revise world state when evidence changes, and use memory when the correct action depends on history—while staying invariant to irrelevant history.

No single benchmark captures all of this. The v0.1 plan uses three complementary protocols and preserves their separate native scores:

- **EmbodiedMemory-Bench:** cross-task memory built and updated from interactive history, with four failure-oriented families.
- **MIKASA-Robo-VLA:** within-episode hidden cues and long-horizon action, with a canonical 22-task Long split.
- **MeMoREval:** experience followed by a later request with no new observations during execution, focusing on memory modality and write/read cost. Full simulator/evaluator code is not yet released at the pinned snapshot, so it is a protocol target, not an executable gate.

## Research snapshots checked on 2026-10-22

| Resource | Exact source snapshot | Protocol facts | Execution status |
|---|---|---|---|
| [Embodied-Omni / embodied_memory](https://github.com/ZJU-OmniAI/Embodied-Omni/tree/main/embodied_memory) | `77a35e57aa17420cd7a69fae6e777c2eb1b5838b` | 2,554 episodes / 1,118 scenes; Passive Observation 1,036, Dynamic Tracking 1,052, Interaction Failure 263, Experience Generalization 203 | Code and evaluation runner present; simulator dependencies still required |
| [EmbodiedMemory-Bench data](https://huggingface.co/datasets/lzLiang/EmbodiedMemoryBench) | Revision recorded in current README: `30fa72950fcb0143316dc55e8f9339ce0a95413d` | Frozen full manifest `full2554.jsonl` | Re-resolve revision and verify download digests before execution |
| [MIKASA-Robo-VLA](https://github.com/CognitiveAISystems/MIKASA-Robo) | `509b875f3d207c287497c0a897661062de928bb0` | 90 tasks; 38 Short, 30 Medium, 22 Long; 50 episodes/task on canonical seeds | Code, canonical runner, tasks, and data pipelines available |
| [MeMoREval](https://github.com/jimwang418/MeMoREval) | `698894e0f9d0a0d802a607a9624048aa4841ff36` | 53 tasks / four indoor environments; experience phase then request/action phase with no new observations | Current README says simulator, memory backends, agent, and grader code will follow peer review; task specs and scenes are available |

These are code provenance pins, not guarantees that data, dependencies, weights, or evaluation conditions are licensed, complete, or compatible. Recheck them and record actual fetched file/artifact SHA-256 digests at execution time.

## Why these benchmarks complement each other

### 1. EmbodiedMemory-Bench: cross-task memory and feedback

The public project describes 2,554 executable episodes across passive observation, dynamic tracking, interaction-derived state, and experience generalization. Its reference system keeps scene, spatial, and event memories, updates them during interaction, then uses them for later action. This makes it the best *first integration target* in this set because both code and an evaluation runner are currently present.

The project page's paper-results table reports Gemini-3-Flash full-context at 64.2 average success. For the GPT-5.4-mini backbone, the table reports full context at 35.3 and EMem at 58.9 (+23.6 absolute points). These are external project-reported results, not Symtropy results and not a matched comparison to MIKASA or RoboMME. Closed-model availability, prompts, runtime settings, and evaluator versions must be checked before attempting exact reproduction.

### 2. MIKASA-Robo-VLA: standardized long-horizon action

The [canonical protocol](https://mikasarobo.github.io/evaluation_protocol.html) specifies:
- Long split: 22 tasks with horizons from 602 through 2,160 steps.
- 50 episodes/task, seeds `4242424242` through `4242424291`.
- RGB observation, `pd_ee_delta_pose`, normalized dense reward, official wrapper stack, and `success_once` as the main per-episode metric.
- Required outputs: full per-task SR, macro-averaged split SR, and per-memory-type SR.

The task/seed stream is identical across tasks, so the task-by-seed cells should not be treated as mutually independent. Use paired task/seed comparisons and an uncertainty calculation that preserves task strata and seed blocks.

The ten reactive/control tasks are negative controls: a memory module should not degrade tasks that can be solved without hidden historical cues. The canonical policy must not receive `oracle_info` or the RL-side `task_cue`; using either disqualifies the run.

### 3. MeMoREval: memory modality when the future request is unknown

[MeMoREval](https://arxiv.org/abs/2610.12561) describes 53 tasks in four simulated indoor environments. During experience, the agent does not know the later request; at request time it answers or plans with retained memory but receives no new observations to compensate for lost details. It compares textual records/descriptions, raw camera frames, scene graphs, and combinations, alongside write-time and read-time inference cost.

The paper reports that frames plus scene graphs have the best aggregate success among its tested modalities. Its repository README at the pinned snapshot says task specifications, scenes, and a robot asset are available, but simulator/backends/agent/grading code will be released after peer review. Therefore:
- Task JSON/schema and scene manifests can be audited now.
- No native MeMoREval performance run should be claimed until the exact simulator, memory backends, task runner, and grader have been released and pinned.
- Do not build a substitute score and call it the official MeMoREval metric.

## Memory architecture hypothesis

Maintain a bounded evidence portfolio with four logical record kinds:

1. **Perceptual evidence:** selected RGB/key frames where fine visual attributes may be needed later.
2. **Scene/spatial state:** entity identity, room/location, relationships, and current state.
3. **Events/outcomes:** actions attempted, visible successes/failures, feedback, corrections, and temporal order.
4. **Reusable experience:** a correction or procedure only after its preconditions and observed outcome are recorded; generalization must not turn one unverified event into a universal rule.

Every record includes source/session identity, event/source time, arrival time, provenance ID, confidence that is not an oracle correctness label, validity, and supersession/contradiction status. Unknown future requests must not leak into admission or compression.

Use a fixed hybrid of these kinds as the first comparison. Only then introduce a task-conditioned router that changes their allocation and retrieval order. Otherwise a router win cannot be separated from simply adding a new memory modality.

## Stage order

### Stage A — qualify mechanism semantics in Rust

Use VIS-MEM-008 to test:
- source-time inversion and stale-arrival rejection;
- update and supersession links;
- compression under a fixed budget;
- provenance-preserving citations;
- deterministic receipts and evaluator-hidden truth;
- logical bytes versus separately measured process memory.

A pass qualifies the synthetic semantics only. It does not qualify a robot policy.

### Stage B — EmbodiedMemory-Bench native baseline

1. Pin the exact repository commit and dataset revision, download manifests/assets, and record hashes.
2. Run the dependency-free demo and save the stdout/exit receipt.
3. Run one real simulator-backed smoke episode per available family and verify reset, legal action sequence, terminal predicate, failure labels, and artifact schema.
4. Reproduce an eligible reference configuration from the official runner. If a reference model requires a proprietary API that is unavailable, record this blocker and use the strongest eligible open baseline; do not claim reproducing a closed system.
5. Run memory-only ablations on paired episode IDs: full context, no persistent memory/current observation only, FIFO/recent window, raw-frame memory, scene/spatial memory, event/outcome memory, fixed hybrid, adaptive hybrid.

### Stage C — MIKASA long split

1. Start with a single-task/one-episode smoke test using the official wrapper.
2. Run a named pilot subset to find integration failures, label it exploratory, and do not mix it with canonical numbers.
3. Run the full 22-task × 50-seed Long protocol for canonical comparison.
4. Report exact native SR and per-memory-type SR. Keep reactive controls separate and show whether memory causes a regression there.

### Stage D — MeMoREval when code is released

First run the official smoke test and reproduce one memory modality. Then compare modality choices with identical experience sequences, future requests, model/runtime where matched, and fixed declared write/read budgets. Until the official code and grader are pinned, keep this phase at task-spec audit only.

## Controlled ablations and causal interventions

For every eligible benchmark, keep the backbone/checkpoint, prompts, preprocessing, inference precision, action executor, task IDs, episode seeds, and environment fixed for memory-only claims.

- **Storage:** remove a critical earlier observation while keeping all other context.
- **Update:** alter only the source-time order or the visible success/failure of a prior action.
- **Compression:** compare the full event ledger against a bounded projection and inspect which evaluator predicates survive.
- **Relevant-history swap:** substitute a different past fact while current observation/state/instruction remain identical.
- **Irrelevant-session insertion:** add unrelated sessions but preserve all facts that the task requires.
- **Stale-history substitution:** inject an older superseded record.
- **Memory knockout:** remove long-term memory while keeping current observation identical.
- **Reactive negative control:** run tasks where relevant information remains visible now.

The evaluator uses hidden outcomes only after the policy acts. No ground-truth labels, future requests, unqueried observations, or counterfactual success may enter the policy-facing writer/router.

## Metrics

Always preserve the benchmark's own score definitions. Add separately named diagnostics:

- native per-task, per-family/type, and aggregate success;
- paired task/seed deltas and appropriate stratified confidence intervals;
- required-sensitivity and irrelevant-invariance rates;
- stale rejection, revision correctness, contradiction handling, and action-outcome update correctness;
- write-time and read-time inference costs, memory/context tokens, serialized bytes and index overhead;
- measured p50/p95 wall latency, CPU, RAM, VRAM where accessible;
- success/resource Pareto frontier;
- failed, reset, crashed, incomplete, and missing episodes.

Do not equate serialized bytes with model tokens, deterministic operation counts with milliseconds, or a synthetic memory score with native task success.

## External competitive bar

Current public references must be compared on their own protocols:
- EmbodiedMemory-Bench's project page reports EMem's 58.9 against matched GPT-5.4-mini full context at 35.3, a +23.6-point gain in its own reported setting.
- MIKASA's canonical metric is per-split task-macro SR over its task table and fixed seeds.
- MeMoREval emphasizes the success/cost trade-off among memory modalities with no observation reacquisition.

Internal Symtropy target: at least +5 absolute percentage points over the strongest eligible, locally reproduced baseline with paired 95% interval above zero, an equal or better resource Pareto point, and no concealed task-family or reactive-control regression. This is an engineering target, not a general community standard.

## Falsification

Keep the simplest policy if the adaptive router:
- wins only by increasing memory, tokens, compute, or action opportunity;
- loses materially on one family despite an aggregate win;
- reacts to irrelevant history or fails to use relevant history;
- silently rewinds current state after an older observation arrives;
- passes synthetic tests but fails native task-success checkpoints;
- cannot replicate with frozen code and an independent evaluation seed plan.

## Licenses and data governance

The inspected Embodied-Omni code repository declares MulanPSL-2.0; the MeMoREval code repository declares Apache-2.0. Dataset, scene, robot-model, checkpoint, and third-party simulator licenses are separate. Verify their terms before redistribution or commercial use; do not infer dataset permission from the code repository license.


## New transfer-mismatch comparator: MemTransfer

The research sweep on 2026-10-22 found [MemTransfer](https://arxiv.org/abs/2609.32313), submitted 2026-09-26. It compares six memory representations and a working-memory baseline under a shared frozen VLM on 100 warehouse-navigation cases across ten task types. It tests starting-pose changes, route blocking, and history quantity/task relevance.

Paper-reported numbers indicate an important limitation:
- With one demonstration/task, Full-context scores 95.3% and Episodic 100.0% at the demonstrated start, but each loses roughly 48–49 percentage points at a new test start.
- After route blocking, reported success retention relative to the unblocked setting varies: Summary 39.3%, Working memory 44.8%, Episodic 56.1%, and Full-context 58.3%.
- Replacing half of the relevant histories with experience from other tasks reduces success for both trajectory-memory variants.

This is a preprint result on a small warehouse benchmark, not a general SOTA ranking, and not a Symtropy result. During this research pass an official runnable MemTransfer implementation/checkpoint was not verified, so the paper currently acts as a research comparator rather than a native reproduction target.

### Design implication: store constraints, not only routes

A stored trajectory is evidence of what worked from one start state and one route configuration. It is not a certificate that the same motion path remains executable from a new pose or after a route becomes unavailable. A reusable memory should separate:
- task constraints and required waypoint/goal order;
- the trajectory observed during the demonstration;
- current map/start/route availability;
- evidence about whether the old trajectory is applicable.

The new [VIS-MEM-011 task #2593](https://github.com/Luminous-Dynamics/symtropy/issues/2593) and [PR #2594](https://github.com/Luminous-Dynamics/symtropy/pull/2594) introduce an independently authored Rust graph diagnostic. It compares trajectory replay, nearest-demo replay, constraint-aware replanning, and a guarded hybrid under matched/shifted starts, route changes, distractor demonstrations, and disconnected goals.

A successful synthetic run will only validate the graph/memory semantics. Native MemTransfer claims require the official model, assets, environment and grader to be pinned and executed; otherwise use an eligible public navigation benchmark and label it separately.


## Recurrent fixed-state comparator with executable source: MemBodied

The 2026-10-22 research sweep verified an official public implementation for [MemBodied](https://github.com/declare-lab/MemBodied). Main resolved to `e268134a9e5c4b99a4607b92f466f25d784b3f26`; repository license is Apache-2.0, with separately licensed vendored/dependency components recorded in `THIRD_PARTY_NOTICES.md`.

MemBodied combines a learned fixed-size associative state updated after action outcomes with a fixed episode-initial scene anchor. The policy reads this state during action generation; its memory footprint does not grow with episode length at a fixed rank/configuration. This is an important competing design to Symtropy's external provenance-bearing event ledger and task-conditioned retrieval.

The official README/project reports:
- RMBench five-task mean: MemBodied 50.0%, compared with 6.4% for stateless π₀ and 16.8% for vanilla recurrent memory.
- π₀.5 RMBench comparison: 48.0% for the MemBodied configuration versus 12.4% for the π₀.5 baseline in that report.
- LIBERO-Long: 90.6% versus 85.2% for stateless π₀.
- Three physical-robot tasks: 16/60 successful trials versus 2/60 for the stateless baseline.
- Same-A100 project-reported inference measurements: 129.2 ms mean policy cycle for MemBodied versus 1,593.7 ms for NativeMEM, with peak GPU allocation 18.61 GiB versus 20.57 GiB. The project says the latency comparison pools four tasks after excluding warm-up queries.

These are the official project's paper/README results, not Symtropy measurements or a general leaderboard ranking. They do not imply every memory task is solved, and RMBench, LIBERO-Long, physical trials, MIKASA, RoboMME, EMem-Bench and MemTransfer remain different protocols.

The code is available, but the README explicitly says trained checkpoints and benchmark datasets are not bundled. Reproduction therefore requires selecting the precise backend/config, obtaining the approved dataset/assets, training or obtaining an eligible checkpoint, recording compute and code/data provenance, and executing the native grader. Do not call a source-code audit a result reproduction.

### What Symtropy should compare

Once the synthetic lifecycle/intervention tests qualify, a focused apples-to-apples experiment should compare:
1. no memory;
2. fixed-state recurrent memory plus first-scene anchor (MemBodied if checkpoint and environment are available);
3. bounded external episodic retrieval (matching-session baseline);
4. provenance-aware relational/event store;
5. fixed hybrid combining the strongest simple options;
6. task-conditioned router.

Hold the eligible backbone and action executor fixed wherever the implementation allows. If learned memory requires retraining weights, report training compute and do not label the result a memory-only inference ablation unless all other training/evaluation factors match.

The architectural research question becomes: **can a small recurrent summary serve as a fast working state while an external, provenance-rich store handles rare retrieval, contradiction resolution and transfer?** Test recurrent-only, external-only, and combination at matched inference/resource budgets. A hybrid is worthwhile only if it improves the success/cost frontier or repairs clearly identified failure slices without obscuring regressions.
