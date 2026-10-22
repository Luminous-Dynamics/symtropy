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


## Decision-level counterfactual audit and abstention

A 2026-10-22 literature review added two distinct audit references:
- [Counterfactual Memory Audit (CMA), arXiv:2609.27247](https://arxiv.org/abs/2609.27247): constructs histories that reconverge to an identical present, freezes the action, and scores it in matched and crossed evaluator worlds. Its report highlights that action sensitivity alone can coexist with poor paired reliability; it reports only 20/64 fully reliable audited Put Back pairs for the evaluated Mem-0 setting.
- [TANGLE, arXiv:2608.13921](https://arxiv.org/abs/2608.13921) and [STALE, arXiv:2605.06527](https://arxiv.org/abs/2605.06527): test unresolved conflict, source/state revision, premise resistance and whether the policy adapts behavior after memories change.

These preprint/benchmark reports motivate evaluation; they are not Symtropy results. Do not duplicate their assets unless use and license are separately verified.

VIS-MEM-009 already tests relevant-history sensitivity and irrelevant-history invariance. The follow-on [VIS-MEM-012 #2606 / draft PR #2607](https://github.com/Luminous-Dynamics/symtropy/pull/2607) adds an independently authored finite decision audit with six paired memory cases and eight deterministic policies. Both latent worlds share the same current-observation digest. Evaluator truth is withheld from the policy and only used after action selection for:
- matched and crossed-world utility;
- bidirectional pair reliability;
- wrong-action versus unnecessary-clarification rates;
- invariance under unrelated-history injection;
- resistance to stale late arrivals and false prompt premises;
- a distinct underdetermined state in which clarification is the utility-optimal decision.

This broadens the benchmark target from retrieval correctness to **decision correctness under uncertainty**. Across native experiments, compare on the same backbone/executor and common evaluation scenarios wherever possible. Report exact memory/write/read resource budgets and a success–utility–cost Pareto frontier. A candidate should not win just by changing more actions, clarifying everything, or hiding unsafe committed mistakes inside average task success.


## Failure memory and recovery: recurrence versus overgeneralization

The 2026-10-22 sweep identified two useful public comparators beyond generic long-horizon task success:

- **FAER** (Findings ACL 2026) defines 3,323 failure-focused episodes over 65 tasks and 83 objects. It separates failure detection, categorization, description, and correction, and reports high false-negative rates for failure detection among evaluated VLMs. The official ACL paper is at https://aclanthology.org/2026.findings-acl.948/. Its annotations/data license must be checked independently before reuse.
- **LIBERO-Recover** (arXiv:2609.05178) moves evaluation from nominal success to recovery from execution-born failure, with four levels: action retry, action adaptation, object-state recovery, and environmental recovery. The project reports that initial-task success can overstate deployment robustness; its current public repository describes 4,803 annotated failure scenes and points to ModelScope assets. Code source main was pinned during this pass at `2162b4c5875835780ddfdbe89a8151273d2fe30a`. The public run path requires a separate CUDA/Python/LIBERO environment, so a source audit alone is not an executed benchmark result.
- **BetaMem** (arXiv:2610.26554, submitted 2026-10-19) directly motivates failure-conditioned rule repair gated by independent replay. Its project page reports a separate EMem-Bench run at 38.4% mean task success and 24.9% error recurrence for Qwen3-VL-8B-Instruct. These are paper-reported scores, not Symtropy results or directly comparable to LIBERO-Recover's recovery-success metric. https://www.roboticscenter.ai/research/papers/betamem-learning-to-remember-a-changing-world-for-embodied-agents-2610

The design lesson is that minimizing recurrence alone is gameable: a policy can suppress every action, at the cost of blocking useful behavior. Measure repeat failures **and** false suppression, unrelated-task collateral, nominal action retention, recovery success, and stale-rule invalidation.

The follow-on [VIS-MEM-013 #2614 / draft PR #2615](https://github.com/Luminous-Dynamics/symtropy/pull/2615) adds an independent synthetic Rust diagnostic comparing always-retry, global action taboo, context-keyed suppression, conditional repair, and replay-gated repair. A global taboo candidate must be rejected if it regresses protected cases; a task/object/context-scoped candidate must pass a finite contract before it can be promoted. The runtime policy API does not receive evaluator-safe-action labels. The current synthetic suite qualifies only action/precondition semantics; native comparisons require pinned eligible baseline weights, assets, evaluator, seed protocol, and license review.


## First exact-head synthetic results (2026-10-22)

Two Rust mechanism diagnostics have now produced qualified reports; both remain synthetic and must not be presented as native benchmark results.

### VIS-MEM-011 transfer-under-mismatch

Corrected head `14a13547de42a67281db571e514aab69d6c742e6`; focused workflow [#39713654827](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39713654827) and general CI passed. The exact report contains the 192-case finite matrix for each policy and the named control cases. In the matrix:

| Strategy | Task success | Read of result |
|---|---:|---|
| No memory | 0.00% | Ordered task constraints are intentionally held in demonstration memory |
| Fixed trajectory replay | 3.65% | Extremely brittle under starts/edges changed |
| Nearest-demo replay | 51.04% | Recovers some changed-start utility but still follows trajectory |
| Constraint planner | 81.25% | Replans from current graph while keeping stored waypoint order |
| Guarded hybrid | 81.25% | No success gain over simpler planner on these cases |

The constraint planner improves by 30.21 percentage points over nearest-demo replay in this tiny graph. The guarded hybrid offers no additional success, although it reuses a demonstration path in some cases instead of replanning. This is useful evidence against promotion based on added architectural complexity alone. Zero illegal graph edges were reported. The artifact TSV SHA-256 is `1c310754b9cbc54c8acd0fb0ad6708f346bb9683c093fd8e3375a606c7db4d88`.

The first run exposed a waypoint-start accounting bug; the first-waypoint revisit contract was corrected and the current figures are from the regression-fixed head. These numbers are exact averages for the finite synthetic matrix, not results on a physical robot, not an external benchmark and not evidence of general SOTA.

### VIS-MEM-012 counterfactual decision audit

Qualified head `4cea6c46996c78a5b184c2b5438764b032ae7e5a`; [focused workflow #39713733432](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39713733432) and [general CI #39713733402](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39713733402) passed. In six authored paired cases, the authority-weighted resolver reached 100% warranted choice / bidirectional reliability and zero wrong actions; it clarified equal-authority irreducible conflicts. Relevant source-time recency reached 80% warranted choice, while relevant arrival-order recency reached 60%, with the latter failing the stale-arrival control. These are expected discriminations on a hand-authored contract test, not general accuracy estimates. Artifact TSV SHA-256: `9eb251efb887be26f73e291023e2f77c72fa8236163c3c3aa1cef6bd18045904`.

VIS-MEM-013 (failure-conditioned memory) is implemented in draft PR #2615 but remains awaiting exact-head qualification. Its synthetic contract deliberately tests recurrence against overgeneralization and should not be represented as passing yet.


## Belief trapping and full-cycle execution/recovery

The current research sweep adds two complementary references with a stronger focus on *goal progress* rather than simply memory quality.

### PoS: explicit beliefs and recovery from loops

[PoS (Progression of States)](https://github.com/luoyu100/PoS) source `main` was pinned on 2026-10-22 to `d6acb43013647b459fa524bc6a8f19586cf1d65e`. The public README describes a structured belief over entities/states/relations, task-relevant epistemic and achievement gaps, consistency validation and trapping-aware recovery. It provides Raw/ReAct baselines and ablations without consistency or trapping recovery across ALFWorld, LOCA-Bench, RCA-100 and ClinDiag. The repository says large datasets are not bundled and requires model/provider setup; LOCA also requires Node.js/npm plus MCP tooling, so a source audit is not an executed reproduction.

The paper reports 88.81% ALFWorld success for PoS with Qwen3.7-Plus versus 72.39% for the strongest evaluated same-backbone baseline, and 38.83% RCA-100 joint accuracy versus 26.21%. In its cited RCA-100 token analysis, PoS total token use is 1,800.13K/episode against 355.70K for Raw Trajectory—about 5.06×—even though task-agent tokens decrease. This is a useful caution: recovery quality and total decision cost need a joint frontier, and tokens must not be mistaken for latency or energy. These are published paper values, not Symtropy scores. Paper: https://arxiv.org/abs/2610.01415.

### SCOPER: evaluate execution as a closed loop

[SCOPER](https://arxiv.org/abs/2610.17657), submitted 2026-10-13, tests high-level plans coupled to visual observations, a shared low-level System 1 and controlled action failures. Its reported suite has 870 tasks across BEHAVIOR and ProcTHOR plus approximately 3,500 situated questions. The paper reports 11.1–28.6% task-success improvements from its SCOPER-harness across the evaluated settings, and gains in offline next-action accuracy on observations from 200 real-robot demonstrations. These figures describe that paper's experiment and are not directly comparable to VIS-MEM synthetic task completion.

### Symtropy experiment

The new [VIS-MEM-014 issue #2624 / draft PR #2625](https://github.com/Luminous-Dynamics/symtropy/pull/2625) adds an independently authored Rust state-machine diagnostic for static stagnation, cycles, drift, epistemic versus achievement gaps, healthy gradual progress, visible environment revision, and unsolvable/no-probe handoff.

It compares raw looping, a generic timer, a cycle monitor and a gap-aware monitor. It explicitly measures false-positive traps, trap recall/classification, steps to detection/recovery, requirement closure, no-progress transitions and correct handoff. The generic timer is an important control: it should expose the tradeoff between quick intervention and false alarms. The gap-aware policy should target evidence-seeking actions for epistemic blockage and a state-changing recovery for achievement blockage.

This finite synthetic diagnostic is an executable contract, not a replacement for ALFWorld, LOCA-Bench, RCA-100, ClinDiag or SCOPER. Native follow-on should use one of the pinned open implementations with the same model backbone and equal action/tool limits; keep raw/ReAct, full harness, no-consistency and no-trapping ablations, and report success alongside total model tokens, tool calls, false trap alarms and recoveries per episode. The implementation remains pending until its exact-head workflow passes.


## First exact-head failure-memory result (2026-10-22)

VIS-MEM-013's exact-head focused workflow and general CI passed at `dcbf53e9a844e4e931a7db6170814066a5249fe1`; see [focused run #39732567921](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39732567921). Its report is an exhaustive record of eight hand-authored contract fixtures across five policies:

| Strategy | Fixture success | Repeat-failure rate | False-suppression rate | Collateral suppression |
|---|---:|---:|---:|---:|
| Always retry | 87.5% | 12.5% | 0% | 0% |
| Global action taboo | 25.0% | 0% | 75.0% | 25.0% |
| Context-keyed suppression | 87.5% | 0% | 12.5% | 0% |
| Conditional repair | 100% | 0% | 0% | 0% |
| Replay-gated repair | 100% | 0% | 0% | 0% |

Artifact TSV SHA-256: `06f5061a1a7d5bb7f24764360be46b6f0337884d2488385b6fed5035fae0e1e0`.

The global taboo eliminates repeat failure by preventing the action even after the blocker resolves, on another task/object, or when a later validated success supersedes the prior failure. The exact task/object/context conditional repair respects that source-revision change in the registered fixtures. The 100% values are expected-contract results on an authored finite suite; this is not a general accuracy estimate, external benchmark result, or native robotics improvement.

This is a useful falsification result: recurrence alone rewards over-suppression. Symtropy should report recurrence jointly with valid-action retention and false suppression.

VIS-MEM-014 (belief trapping / progress recovery) is now implemented in draft PR #2625, with its exact-head Rust workflow still queued at last check. It adds a separate question: can the agent detect that it is stuck despite no explicit action failure, distinguish epistemic from achievement gaps, and recover without mislabeling healthy gradual progress or visible exogenous changes as traps?
