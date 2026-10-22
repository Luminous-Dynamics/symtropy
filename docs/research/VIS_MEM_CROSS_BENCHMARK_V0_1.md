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

The public project describes 2,554 executable episodes across passive observation, dynamic tracking, interaction-derived state, and experience generalization. Its reference system keeps scene, spatial, and event memories, updates them during interaction, then uses them for later action. This makes it the best *first research-only integration target* in this set because both code and an evaluation runner are currently present. The dataset card specifies CC BY-NC 4.0, however, and explicitly leaves upstream AI2-THOR/ProcTHOR assets subject to their own terms. Therefore a commercial Luminous deployment or commercially directed evaluation must not assume this data is available: obtain appropriate permission or use a separately cleared dataset. The public code repository carries a Mulan Permissive Software License v2 notice, which is distinct from the dataset license.

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

### Stage B — EmbodiedMemory-Bench native baseline (license-gated)

1. **Resolve permission first.** The [dataset card](https://huggingface.co/datasets/lzLiang/EmbodiedMemoryBench) labels the 2,554-episode dataset CC BY-NC 4.0; the public [code repository](https://github.com/ZJU-OmniAI/Embodied-Omni) has a separate Mulan PSL v2 license notice, and AI2-THOR/ProcTHOR assets have their own upstream terms. Do not infer the data's license from the code's license. Use the dataset only for a purpose allowed by the relevant terms; for commercial Luminous work, obtain explicit permission or use a separately cleared benchmark. This is a research execution gate, not legal advice.
2. **Freeze the evaluation set.** Re-resolve the exact repository commit and dataset revision; record all manifest/episode SHA-256 digests. The published dataset card states that the release is `split: all` and declares no train/validation split. Treat all 2,554 episodes as evaluation-only: do not tune prompts, memory policy, thresholds or weights against the same full manifest and then report it as a held-out result. Use separate synthetic/owned development cases or ask the authors for a valid development protocol.
3. Run the dependency-free demo and save the stdout/exit receipt.
4. Run one real simulator-backed smoke episode per available family and verify reset, legal action sequence, terminal predicate, failure labels, and artifact schema.
5. Reproduce an eligible reference configuration from the official runner. If a reference model requires a proprietary API that is unavailable, record this blocker and use the strongest eligible open baseline; do not claim reproducing a closed system.
6. Run memory-only ablations on paired episode IDs: full context, no persistent memory/current observation only, FIFO/recent window, raw-frame memory, scene/spatial memory, event/outcome memory, fixed hybrid, adaptive hybrid. Keep this research dataset and any derived artifacts in the license boundary approved for the run.
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

Internal Symtropy target: at least +5 absolute percentage points over the strongest eligible, locally reproduced baseline with paired 95% interval above zero, an equal or better resource Pareto point, and no concealed task-family or reactive-control regression. For a native VIS-MEM-015 port, also target at least a 50% relative reduction in harmful-memory-use rate versus always-recall, no more than a 1-point absolute regression on direct-observation/reactive controls, and false-abstention below 2% on cases where the benchmark establishes an available correct path. Publish verification/tool/model-token cost alongside success. These are prospective engineering gates, not community standards or results inferred from the synthetic fixture.

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


## Paired causal memory admission (VIS-MEM-015)

The separate [VIS-MEM-015 issue #2632 / draft PR #2633](https://github.com/Luminous-Dynamics/symtropy/pull/2633) adds a paired memory-use intervention. Existing slices ask whether a memory can transfer, resolve conflicts, repair failures or help detect traps; this slice asks the causal question: **for the same task state, did admitting the retrieved memory improve or harm the decision relative to the no-memory control?**

The current diagnostic is a dependency-free Rust program with 18 hand-authored states and eight policies (144 case-policy rows): no memory, always recall, task match, revision-only, observation-first, fresh verified evidence, a deliberately weak scope-claim gate, and transfer-validated admission. The controls include fresh useful memory, old but independently transfer-validated memory, stale wrong memory, an unvalidated task-invariance claim, irrelevant-but-current memories, contradictory live observations, and missing evidence with/without verification.

Metrics remain disaggregated: task success, wrong-action rate, memory-use rate, helpful/harmful memory use, paired success lift against the same case under no memory, verification cost, avoidable verification, abstention, and false abstention. The selector receives no evaluator safe-action label. Output replay, fixed field counts, test assertions and a SHA-256 artifact are part of the pinned Rust 1.96.0 workflow.

At last check, [focused qualification run #39739584641](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39739584641) and general CI #39739584528 were queued for exact head `2b269f775dab6caac4daff132d3e3e2f2ccd183d`; no result is claimed yet. The implementation is a mechanism diagnostic, not a native benchmark or SOTA result.

This direction follows directly from the external evaluation gap: MemTransfer reports sensitivity to changed start poses and irrelevant histories (https://arxiv.org/abs/2609.32313); MeMoREval jointly measures embodied memory utility and construction/use cost (https://arxiv.org/abs/2610.12561); SCOPER emphasizes evidence retention and progress verification over complete closed-loop task execution (https://arxiv.org/abs/2610.17657). Symtropy's longer-term test should apply these paired interventions to a licensed native benchmark, freeze the model/simulator/evaluator revisions, and report task/seed-block confidence intervals with memory and runtime cost. Synthetic rates must never be pooled with these external results.


## Fixed-size learned memory is a required comparator

A relevant recent method is [MemBodied](https://arxiv.org/abs/2609.28256), with its [official implementation and reported results](https://github.com/declare-lab/MemBodied). It uses a fixed-size associative state updated from action chunks and observed outcomes, plus an episode-start anchor. Storage stays constant in episode length at a fixed architecture/rank/camera configuration. This is a different design point from a vector store or text RAG pipeline: it learns how to read/write a bounded latent state inside the action policy.

The project reports:
- **RMBench, five memory tasks:** 50.0% mean success for MemBodied vs 16.8% for vanilla recurrent memory and 6.4% for stateless pi0.
- **LIBERO-Long:** 90.6% success vs 85.2% for stateless pi0.
- **Physical robot setting:** 16 successful trials out of 60 vs 2/60 for its pi0 baseline across three task categories.
- **Latency within the reported NativeMEM comparison:** about 129 ms per policy cycle vs 1.6 s, with lower peak GPU memory.

These figures are reported by the authors, not independently reproduced here, and they are **not comparable to MIKASA or to our synthetic Rust fixtures** without matching the benchmark, task set, policy backbone, training data, seeds, and success definition. The source repo states that benchmark assets, datasets, and trained checkpoints are not bundled. It uses separate Python/JAX policy environments; plan for that only at the external benchmark boundary, not in Symtropy's production implementation.

## Native benchmark priority and comparator matrix

For a first native memory experiment, prioritize **MIKASA-Robo-VLA Long** after environment and asset license review. Its public benchmark code repository is MIT, and the Hugging Face LeRobot dataset is also marked MIT; this is a better initial candidate for commercial-adjacent research than MEMOBench and EmbodiedMemory-Bench, whose datasets are marked CC BY-NC 4.0. An MIT dataset/repository marker does not grant rights to every simulator, submodule, model checkpoint, or dependency, so audit those terms separately. Keep the noncommercial benchmarks research-only unless additional rights are obtained.

Compare under identical MIKASA task IDs and seeds, and report the standard official 22-task Long result using 50 episodes/task and seeds `4242424242…4242424291`:
1. Reactive/current-observation policy (negative control).
2. Stateless or no-persistent-memory baseline.
3. Fixed frame/context window.
4. External episodic memory / retrieval baseline.
5. Learned fixed-state recurrent/associative baseline (MemBodied-like).
6. Explicit evidence ledger with revision-aware update and admission gate.
7. Fixed-budget hybrid: learned compact state plus a bounded evidence ledger for auditable exceptions and freshness.

For memory-only claims, freeze backbone/checkpoint, observation preprocessing, action executor, demonstration/training budget, precision, action chunk size and simulator; use paired task/seed analysis. Report training compute separately when learned recurrent memory adds parameters. Keep task-level and memory-type results, and run reactive controls to ensure memory does not regress tasks where the needed information remains visible.

Before training/evaluation, write down the evaluation split, held-out policy-selection method, dataset revisions and hashes, simulator/checkpoint provenance, licenses, and oracle-access audit. The published official seed protocol uses the same stream for every task, so confidence intervals should preserve task strata and seed blocks rather than treating all 1,100 episodes as independent samples.

This changes the comparative question from “does a bigger external store beat a small buffer?” to “which memory architecture gives the best task-success / reliability / latency / resource frontier under the same evidence and action budget?” The likely differentiator for Symtropy is not claiming one store architecture wins everywhere, but demonstrating that a hybrid can retain recurrent efficiency while exposing auditable provenance, stale-revision rejection, verification, and safe abstention when the latent state is insufficient.

**Prospective go/no-go gates (not measured results):** at least +5 absolute points of paired native macro success over the strongest eligible baseline with a paired 95% interval above zero; at least 50% relative reduction in harmful-memory actions compared with always-recall on labelled intervention cases; at most 1-point regression on reactive controls; false abstention below 2% when an independently established correct path exists; and no worse measured success/resource Pareto position. Publish both passes and regressions.


## Metamorphic invariants before native performance claims (VIS-MEM-018)

[VIS-MEM-018 issue #2650 / draft PR #2652](https://github.com/Luminous-Dynamics/symtropy/pull/2652) adds a deterministic metamorphic test harness over four bounded-storage policies. It measures whether memory semantics survive transformations that should be irrelevant—duplicate delivery, chunking, and unrelated inserts when no eviction is needed—and intentionally exposes where weaker policies change their answer: arrival-order permutation, stale events after payload eviction, and same-revision conflicting values.

The generated order test uses 16 fixed seeds over eight entities with three distinct source revisions each. The experiment distinguishes:
- **latest-arrival counterexamples:** same source facts arrive in a different order and the chosen version changes;
- **max-source-revision upsert limitation:** it rejects a stale value while a payload exists, but can accept it after the payload is evicted;
- **persistent watermark:** the source high-water mark remains after payload eviction and blocks stale revival, at additional metadata cost;
- **conflict-aware watermark:** equal-revision contradictory values are exposed as ambiguity instead of resolved by last arrival.

These are synthetic mechanism findings until the exact-head Rust workflow executes successfully. The watermark is a small version/fingerprint index, not a substitute for the full payload, and its estimated logical bytes exclude allocator/map overhead. Results must show these costs separately from fixed payload slots.

## Native MIKASA results: contract verified does not mean policy validated

[VIS-MEM-017 issue #2643 / draft PR #2646](https://github.com/Luminous-Dynamics/symtropy/pull/2646) adds a fail-closed verifier for canonical MIKASA-Robo-VLA Long result files. The public protocol fixes 22 Long task IDs, 50 episodes per task, seeds `4242424242…4242424291`, RGB observations, `pd_ee_delta_pose`, and the `success_once` per-episode main metric. It requires full per-task and per-memory-type SR along with split macro SR; per-task mean return remains diagnostic rather than a split aggregate. The official task/seed stream is correlated across tasks, so uncertainty calculations must preserve the task and seed structure.

The artifact verifier checks expected task roster, raw episode array lengths/seeds, score arithmetic, summary consistency, model/action-chunk consistency and common privileged-field keys in model config. It explicitly emits `contract_validated_not_performance_evaluated`: it cannot establish that policy code never accessed hidden observations or oracle state, which requires a separate source/runner audit.

**Pinned source review found a concrete admission blocker (2026-10-22).** The upstream commit `509b875f3d207c287497c0a897661062de928bb0` was inspected beyond the runner signature. In `baselines/ppo/ppo_memtasks.py`, `FlattenRGBDObservationWrapper.observation` copies `observation["task_cue"]` into the returned observation and removes it only if every element equals the missing-value sentinel. In `mikasa_robo_suite/vla/memory_envs/trace_shape_seq_vla.py`, the Long-roster TraceShapeSeq task sets `self.task_cue = self.shape_sequence.to(torch.int64)`. The pinned runner passes the full observation mapping to `policy.forward(obs)`. This creates a potential path for the target shape sequence to reach a policy in a task family where the canonical protocol says non-Rotate `task_cue` should be dropped. Official protocol: https://mikasarobo.github.io/evaluation_protocol.html and observation specification: https://mikasarobo.github.io/observation_space.html.

The source-audit receipt in PR #2646 now records `native_benchmark_admission=BLOCKED_PENDING_TASK_CUE_INPUT_BOUNDARY_REVIEW`, pins the wrapper and task source blobs, and requires a policy adapter that strips `task_cue` before model input or an upstream wrapper fix followed by a new source pin. This does **not** prove that every policy uses the leaked value; it means the current runner path does not establish the no-leak property. Do not publish canonical Long performance claims for an affected policy until its complete input boundary is demonstrably sanitized and tested with a negative-control canary. The result-artifact verifier validates score arithmetic and protocol metadata, not the policy's effective observation channel.

For the native comparison, include (under identical model/backbone, training allowance, seeds, wrappers and action executor): no persistent memory; frame/context window; external episodic retrieval; fixed-size learned recurrent/associative memory (MemBodied-like); revision-aware evidence ledger; and a learned-state + evidence-ledger hybrid. Keep the 22 tasks' canonical metrics intact, report resource/training costs, and preserve the ten-task Reactive split as an independent negative control so added memory is not rewarded when the answer is already visible.

Recent benchmarks independently support this lifecycle-aware approach: WorldMemArena spans 400 multi-session multimodal tasks and separates write, maintain, retrieve and use; EmbodiedMemory-Bench has 2,554 interaction episodes across four task families and describes weak dynamic-state tracking and interaction-derived updates; MemBodied reports that fixed-size learned associative memory can outperform its vanilla recurrent baseline on RMBench. These paper/project results are external references and are not pooled with Symtropy synthetic results or treated as a controlled comparison. Sources: https://arxiv.org/abs/2605.29341, https://arxiv.org/abs/2609.28236, https://arxiv.org/abs/2609.28256.


## State tracking and repeatability are separate evaluation axes

Two complementary 2026 resources widen the native evaluation plan beyond robot manipulation:

### StateMemBench — current state versus superseded state

[StateMemBench](https://arxiv.org/abs/2608.19652) provides 234 multi-session synthetic scenarios and 322 probes. Its closed-pool grading distinguishes answers reflecting current state, superseded state, and failure. This directly measures a failure that recall@k can miss: the system retrieves the relevant fact but still uses an outdated version.

The authors report current-state accuracy of 0.363 versus 0.205 for the strongest same-backbone baseline in a DeepSeek-V4-Flash setting, and 0.233 versus 0.149 for the strongest memory-system baseline in a Qwen-3.5-9B setting. A lightweight answer-time state wrapper is reported to improve several memory/retrieval backends by 32–67 absolute points on that benchmark; length/cost-matched controls attribute 15–32 points to state structure. These are paper-reported results, not Symtropy results or directly comparable robotics scores. Before integration, freeze the exact official artifacts, protocol, model/backbone, and license; the paper's public code/data availability should not be treated as commercial clearance without checking the actual repository/assets.

### STATE-Bench — production-style procedural reliability

[Microsoft STATE-Bench](https://github.com/microsoft/STATE-Bench) is MIT-licensed software and includes 450 synthetic multi-step tasks across travel, customer support, and shopping. It includes a memory/learning track with 100 prior trajectories per domain and 50 held-out tasks per domain. Its metrics include pass@1, pass^5 (the fraction of tasks succeeding in all five runs), user-experience score, and cost per task. The locked simulator and judge use GPT-5.4 in the official protocol, and the tasks are generated synthetically by LLMs; results therefore measure the published protocol and do not by themselves establish real-world enterprise deployment performance.

This is useful for testing whether a memory mechanism improves repeatability and procedure compliance rather than only factual recall. Its harness requires Python 3.12+ and provider/API setup; use it as an external evaluation adapter while retaining Rust as the production memory implementation. The agent-learning track requires leak-free separation: learn only from the provided training trajectories, expose the memory through the required retrieval hook, and never use held-out test task definitions as memory-writer inputs.

## Revised native benchmark matrix

Keep metrics separate across these settings instead of constructing a blended league table:

| Evaluation layer | Primary question | Required outcome measures |
|---|---|---|
| VIS-MEM deterministic Rust contracts | Does state/version/admission/capacity logic obey its declared invariants? | Falsification cases, stale revival, conflict visibility, capacity, deterministic receipts |
| StateMemBench | Does memory answer with the current rather than superseded state? | Current-state accuracy, superseded-state errors, failed/abstained answers, write/read/context cost |
| MIKASA-Robo-VLA Long | Does memory improve long-horizon physical task execution? | Official task-macro SR, per-task/per-memory-type SR, paired task/seed intervals, reactive-control regression |
| STATE-Bench Agent Learning Track | Does stored experience improve procedural execution and repeatability? | pass@1, pass^5, cost/task, UX score, held-out-domain/task regressions |
| WorldMemArena / EmbodiedMemory-Bench | Does multimodal memory survive write → maintain → retrieve → use through interaction? | Official task-family metrics, state-update correctness, memory/resource costs; apply separate license gates |

For every native evaluation, compare at least: no persistent memory, frame/context window, external episodic retrieval, learned fixed-size recurrent/associative memory (MemBodied-like), revision-aware state/evidence ledger, and the ledger + compact learned-state hybrid. Match model/backbone, observation/action interface, task IDs, seeds, training budget, and tool limits wherever the protocol permits. Show pass@1 and pass^5 or their native equivalents, not just average success; report write/read latency, memory footprint, model tokens, verification/tool calls, and training cost on separate axes.

The strongest research hypothesis is now narrower and more falsifiable: **a compact learned state provides low-latency working memory; a revision-aware evidence ledger preserves provenance, supersession, and conflict resolution; exact state evidence guards against over-trusting approximate similarity.** The combination only wins if it beats the strongest eligible baseline under fixed budgets without regressing direct-observation controls or inflating verification cost. That is a prospective hypothesis, not an established SOTA result.


## Production HDC memory: bounded retention as a safety and resource contract

[VIS-MEM-019 issue #2654 / draft PR #2655](https://github.com/Luminous-Dynamics/symtropy/pull/2655) extends the work into the actual Symtropy engine's HDC physics memory, not just the synthetic test harness. The public EpisodeMemory type in symtropy-hdc-physics now has opt-in bounded construction, FIFO-on-overflow, a capacity query, and a monotonic eviction counter, while its existing constructors preserve unbounded behavior for compatibility. The live feature-gated EchoMemory resource opts into 256 episodes, approximately 12.8 minutes at one 3-second episode per completed window, and records rejected writes and evictions through Bevy logging.

At the default 16,384-component bipolar representation, the vector payload alone is about 4 MiB at 256 retained episodes. The exact per-frame digest ledger and episode metadata add additional memory; this estimate is not total RSS and excludes allocator/container/string overhead. FIFO is a deliberately simple reference policy, not an optimal retention strategy. Its role is to remove unbounded growth and make eviction visible before conducting a fixed-budget policy comparison.

The focused workflow at last inspection was queued for the exact branch head. The change is therefore implementation-in-progress, not yet a verified compile/test result or a claim of native robotics performance. The required follow-up is to compare FIFO with diversity/novelty-aware admission under the same byte budget, reporting useful-recall retention, false matches, stale-state handling, query latency, and exact metadata cost.


## Diversity-aware retention is a falsifiable policy hypothesis, not a default

The bounded HDC implementation branch includes [VIS-MEM-019 draft PR #2655](https://github.com/Luminous-Dynamics/symtropy/pull/2655), with an executable retention comparison intended to run under the same pinned Rust workflow. The controlled stream contains four orthogonal 16,384-D bipolar prototypes (A/B/C/D) followed by 24 redundant copies of A. FIFO and a candidate vector-novelty admission policy are both limited to four retained episodes; an unbounded reference provides a quality/cost upper baseline. The future query class is only used during evaluation and never enters memory admission.

The expected metamorphic contract is intentionally narrow: FIFO forgets rare classes under skewed redundant writes, while novelty-aware admission preserves the four orthogonal prototypes. The experiment also reports number of similarity comparisons and exact vector payload bytes. If a real encoded episode stream does not show the same trade-off, the diversity policy should not be promoted. Because this is a constructed vector fixture, it does not establish that novelty-based retention improves real physical-state recall, and it is not an SOTA result.

Production EchoMemory remains FIFO-bounded at 256 episodes unless broader, real-episode evaluation supports a different policy. The complete cost metric must include more than the 16,384-byte vector per episode: exact per-frame digests, IDs, strings, metadata, container capacity and allocator overhead also matter. The right promotion test is a Pareto comparison of useful retrieval, false matches, stale-state resistance, retention diversity, admission CPU/comparison count, query latency, and total memory bytes under the same fixed budget.


## Similarity threshold calibration and abstention (VIS-MEM-020)

[VIS-MEM-020 issue #2657 / draft PR #2658](https://github.com/Luminous-Dynamics/symtropy/pull/2658) directly audits the live EchoMemory default similarity cutoff of 0.65. A high nearest-neighbour score is not proof that the stored episode describes the same situation, and a low threshold can convert every ambiguous query into a confident false analogy.

The calibration example uses the actual PhysicsFrameEncoder, EpisodeMemory and PhysicsWorld. It stores four reference scenario families (left/right drift, fall, head-on), then creates disjoint calibration and evaluation queries with held-out scales and two unknown dynamics families. It emits all query-to-bank similarity pairs, current-cutoff results, a 0.00–1.00 threshold sweep, coverage, useful/harmful accepted matches, false rejections, risk among accepted matches, and weighted cost. The selector has an explicit guard refusing evaluation queries; the selected cutoff is frozen before scoring the evaluation split.

The 4:1 false-match/false-rejection cost is a declared experimental preference, not a universal safety trade-off. The workflow also reports raw components so another deployment can choose a different cost. The live threshold remains 0.65 until held-out qualification and a separate review; even a qualified synthetic physical-scene encoder experiment is not a robot-task SOTA result. Next, run this calibration with real episodes collected under an instrumented simulator, add repeated-session drift and same-scene/different-task negatives, and determine whether the threshold transfers across encoder versions, scene scale, and environment changes.


## Provenance security: prevent memory laundering across the lifecycle (VIS-MEM-018)

The new [VIS-MEM-018 issue #2660 / draft PR #2661](https://github.com/Luminous-Dynamics/symtropy/pull/2661) adds an independently authored Rust security-contract diagnostic for provenance laundering. Its adversarial pattern is simple: a poisoned or revoked experience is summarized, compressed, or reflected into a new record that claims to be trusted; if lineage or revocation metadata is dropped, the new record can affect future action after the original evidence is no longer visible.

The diagnostic compares always-use and trust-label-only negative controls with root-authentication-only, taint-propagating, and lineage-plus-revocation-aware policies. Ten fixtures cover false trust claims, tainted summaries, missing parent links, a revoked root, a revoked ancestor with a clean-looking child, contradictory direct observation, a valid authenticated derivative, and cases where no trustworthy verification exists. It reports action success, wrong actions, poisoned-memory use and containment, false quarantine, revoked/missing-lineage use, verification and abstention costs, and full per-case ancestry/action records.

This is not a cryptographic implementation: a boolean `root_authenticated` in a synthetic fixture stands for an independent verifier boundary. Production must authenticate source identity/digests, preserve immutable parent links, derive taint through transformations, and apply revocation to descendants. An LLM-generated “trusted” or “verified” label cannot authenticate its own source. A future native benchmark needs explicit source-poisoning and revocation interventions, not just normal success evaluation.

The protocol is motivated by two process-oriented evaluations:
- [MEMOBench](https://github.com/Collab-Gen/MEMOBench) / [paper summary](https://www.max-robotics.com/en/research/publications/cmu3096f900b95qb2w5bgczrb) labels one of three memory operations (Storage, Update, Compression) at 4,200 executable checkpoints across 30 history-dependent manipulation tasks. It reports 31.9% average success for the strongest evaluated memory-module baseline, highlighting that high storage performance can coexist with weak update/compression. Dataset/code/annotations are marked CC BY-NC 4.0, so the benchmark is not automatically cleared for commercial use.
- [WorldMemArena](https://worldmemarena-mem.github.io/) describes 400 multi-session multimodal tasks in a lifecycle that explicitly spans write, maintain, retrieve and use. Its paper/site reports that harness-based memory can help harder regimes but raises token/latency cost and stability questions; these published results are not comparable to this synthetic suite.

### Security release targets (prospective)

For a licensed native task suite, report at minimum:
- **Poison action rate:** fraction of intervention cases where evidence with invalid/revoked ancestry influences the selected action.
- **Containment rate:** fraction of poisoned cases rejected, verified, or abstained before action; report task success alongside this so “reject everything” is not rewarded.
- **Revocation propagation:** fraction of descendants that stop influencing actions after an ancestor is revoked, stratified by lineage depth.
- **Legitimate evidence retention:** acceptance/success on valid authenticated derivatives and on reactive controls.
- **Provenance completeness:** fraction of decisions whose supporting memory chain is retrievable end to end.
- **Cost:** additional bytes per lineage edge and revocation watermark, p50/p95 validation latency, verification/tool calls, model tokens and action latency.

Release gates should be prospective and paired: reduce poison-action rate by at least 90% versus label-only/always-use on the declared security interventions, keep false quarantine under 2% on valid-evidence controls, avoid more than a 1 percentage-point regression on reactive tasks, and retain a positive task-success/resource trade-off. Those numbers are internal targets, not current results or universal standards. Never merge poison containment into one weighted score that can hide a safety regression.

The latest work separates this security slice from the fixed-budget watermark experiment: [VIS-MEM-016 #2637](https://github.com/Luminous-Dynamics/symtropy/pull/2637) studies whether freshness metadata survives payload eviction; [VIS-MEM-017 #2646](https://github.com/Luminous-Dynamics/symtropy/pull/2646) validates the native MIKASA Long evidence contract without claiming task performance. Together they provide a staged path from mechanism semantics to controlled native intervention tests.


## VIS-MEM-021: decision value, not just novelty, under hidden demand shift

[Issue #2675 / draft PR #2676](https://github.com/Luminous-Dynamics/symtropy/pull/2676) adds a separate fixed-budget retention experiment. It replays a paired online stream, freezes each policy's retained records, and only then scores a hidden 128-query future trace. Four scenarios test stationary demand, a demand phase shift, a rare high-consequence query, and a high-volume irrelevant-write burst. Seven online policies are compared over 24 seeds and budgets 3/6/9, with 2,016 per-run rows and 84 summaries. Baselines include FIFO, LRU, LFU, resident-write rarity, source-risk-only, source-risk-plus-access, and a source-risk + reacquisition-cost + access policy.

This question is narrower than the existing bounded-memory work: **do the signals available at retention time predict the value of what will be queried next?** A record can be novel but unimportant, frequent but now irrelevant, or rare while carrying high-consequence information. The selector receives only the online event stream, resident metadata, and online time; an online query event carries only its key, not the evaluator's truth revision. All seven policy snapshots are frozen before one shared future query trace is sampled and scored. A hindsight-only upper bound quantifies regret but cannot be selected by an online policy. Source risk and reacquisition cost are observable synthetic metadata; their relation to evaluation impact is a deliberate fixture assumption, not learned generalization. The synthetic 64-byte slot is accounting fiction rather than total RSS, and this experiment does not prove native performance.

This is motivated by recent external work but is not a reproduction of it:
- [Selective Memory Retention for Long-Horizon LLM Agents / TraceRetain](https://arxiv.org/abs/2606.29178) evaluates several interpretable retention signals. Its preprint reports that retention-policy differences can be hard to separate on clean ALFWorld but become clearer under a controlled 75% distractor-write stress, showing why both clean and noisy regimes are needed. These are author-reported results in a different setting, not a Symtropy baseline score.
- [OSL-MR: Learning What to Remember](https://arxiv.org/abs/2606.10616) frames retention as constrained stochastic optimization with delayed miss, reacquisition and stale-risk costs, while distinguishing online-observable features from offline supervision. This is a closer methodological comparator than recency heuristics alone. VIS-MEM-021's new source-risk/reacquisition-cost policy is an interpretable synthetic baseline, not an OSL-MR reproduction and does not use its learned supervision.
- [Remember When It Matters: Proactive Memory Agent](https://arxiv.org/abs/2607.08716) treats memory as selective action-time intervention rather than passive retrieval and reports improvements over passive/always-on alternatives in its evaluated settings. That motivates a later intervention-timing benchmark separate from eviction-only evaluation; no result should be pooled across these protocols.
- [MemGym](https://arxiv.org/abs/2605.20833) proposes memory-isolated scoring across multi-step agent regimes, separating the memory component from reasoning and tool-use ability. That separation is a useful design principle for any future Symtropy adapter.
- [MEMOBench](https://github.com/Collab-Gen/MEMOBench) separately scores Storage, Update and Compression over 4,200 checkpoints and reports only 31.9% average success for its strongest memory-module baseline. Its dataset is marked CC BY-NC 4.0, so do not silently bundle or use its data for commercial evaluation.

Release policy: no heuristic should be promoted because it wins this constructed generator alone. Require a held-out native matched-budget Pareto improvement, and publish all regime-level failures, miss/stale rates, false critical misses, decision utility, latency, selector work, and complete metadata/RSS accounting.


**Evaluation validity correction (VIS-MEM-021):** a source review found that the first experiment draft placed the online query's evaluator `truth_revision` inside the event passed to memory replay, affecting its historical-value score. That mechanism was removed before qualification: online query events now contain only the key; the historical-label-based policies were removed; and a cost-aware source-metadata policy replaced them. One future query trace is sampled only after all matched policy snapshots are frozen. The exact-head Rust workflow checks those semantics through typed interfaces, tests, schema counts, and deterministic replay. The queued workflow has not qualified the result yet.
