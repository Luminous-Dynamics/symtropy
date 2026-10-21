# VIS-MEM-007: task-conditioned memory under cross-session interference (v0.1)

**Status:** research/experimental contract. No Symtropy native robotics result is claimed by this document.

**Linked task:** [#2552](https://github.com/Luminous-Dynamics/symtropy/issues/2552)  
**Related architecture work:** [VIS-MEM-003 #2485](https://github.com/Luminous-Dynamics/symtropy/issues/2485)  
**Related active-observation work:** [VIS-ACT-011 #2520](https://github.com/Luminous-Dynamics/symtropy/issues/2520)

## Executive hypothesis

A robot's memory should be a *bounded, provenance-bearing state that can be retrieved, revised, contradicted, and retired*, rather than a growing transcript or a similarity-ranked pile of frames. What matters is not how much history is stored, but whether the correct past evidence remains accessible and decision-relevant after unrelated sessions, world changes, and contradictory observations.

The architectural hypothesis is that a **task-conditioned memory broker** can beat fixed FIFO, uniform sampling, and matching-session retrieval at the same memory and compute budgets by combining:

1. typed relational/event records for stable entities, quantities, commitments, and procedural steps;
2. a capped perceptual cache for details that symbolic records cannot preserve;
3. explicit source time, arrival time, provenance, confidence, and contradiction links;
4. task-conditional retrieval and eviction, with conservative handling of uncertain identity or stale evidence;
5. deterministic fallback retrieval when a learned/heuristic ranking cannot separate candidates confidently.

This is a hypothesis, not a result. We should first reproduce the simpler baselines and only then test the broker.

## Why this benchmark now

The synthetic active-query value model has shown weak action ranking (pairwise concordance around 0.527 against a tie-adjusted reference of 0.5), and development-calibrated stopping was worse than round-robin on held-out episodes. More tuning of the same expected-value estimate is not justified until its ranking becomes useful.

The external test that most directly challenges memory architecture is cross-session interference. RoboMME-Interference inserts unrelated sessions between a relevant lesson and a later query, with conditions no-history, k0, k1, k3, and k7. The published project reports that several perceptual-memory methods lose much of their gain as distractors accumulate, while matching-session retrieval retains more performance in its tested setting. This is a strong problem definition, not proof that a proposed Symtropy approach will win.

External benchmark references:
- **RoboMME** (ICML 2026): 16 manipulation tasks across temporal, spatial, object-reference, and procedural memory; official results use 800 test episodes. [Paper](https://proceedings.mlr.press/v306/dai26c.html) · [official benchmark](https://github.com/RoboMME/robomme_benchmark)
- **RoboMME-Interference**: cross-session relevance under no-history / k0 / k1 / k3 / k7 conditions. [Paper](https://arxiv.org/abs/2606.22338) · [code and canonical rollout table](https://github.com/SoumilRathi/robomme-interference)
- **ActiveArena**: separate primary target for active view choice and ID/OOD task execution, not interchangeable with the memory benchmark. [Official code](https://github.com/leeibo/ActiveArena)
- **RoboMemArena**: 26 manipulation tasks with adapter-based evaluation and native CSR/TSR metrics. The authors announced a material dataset/evaluator update on June 20, 2026, so pin the matching code, data, and evaluator together before using it. [Official repository](https://github.com/OpenHelix-Team/RoboMemArena)

At preparation time, the public source snapshots checked were RoboMME `bf6fd29841a63bc1c2e58ef7fe222961388fd35d`, RoboMME-Interference `3536f63bbd483654cdd50ef86a656572a03fb72d`, ActiveArena `1bf2c77b8d3ab978fc5b4599a69d017f593edbe8`, and RoboMemArena `14a89476cf0c03ef7b72b8269a21c67db4295bfa`. These are research snapshots, not endorsements or a claim that they are the correct evaluation pins for a later run. Re-resolve and record code/data/checkpoint digests when starting an actual reproduction.

## The paradigm shift

### 1. From history length to evidence access under interference

A larger context window or memory bank is not intrinsically better. The relevant question is whether a system can still retrieve the right evidence after unrelated experiences are inserted. Treat the number and semantic similarity of distractor sessions as explicit stress axes.

### 2. From transcript memory to provenance-bearing state

A memory item must distinguish:
- **claim/content**: what is believed or remembered;
- **source**: which observation/session or actor supplied it;
- **time**: source/event time and arrival/recording time;
- **confidence**: reported or inferred reliability, not an oracle correctness bit;
- **validity**: timeless, time-sensitive, or explicitly expired;
- **relations**: entity, task, location, event, causal/temporal dependence, and contradiction edges;
- **status**: current, superseded, disputed, or unresolved.

Contradictory evidence must not be silently averaged into a confident fact. In particular, entity identity ambiguity and unidentifiable sensor corruption must remain explicit uncertainty.

### 3. From one memory representation to a fixed-budget portfolio

Use one declared total budget split across relational/event state and a perceptual cache. Test fixed splits before testing adaptive routing. Report actual serialized bytes, native token cost where relevant, retrieval/write counts, and measured resource use. Never call unlike bytes and tokens equivalent.

### 4. From ranking score to selective authority

A weak ranking model should not control all memory access. A candidate broker may use its preferred retrieval only when selection is stable across a frozen nuisance set and otherwise invoke a robust comparator, such as matching-session retrieval or a recency-bounded baseline. The fallback must be specified before held-out evaluation. No extra oracle information or post-hoc evaluator label may enter the acting policy.

### 5. From average success to reliability curves

Clean-task success can hide catastrophic degradation after task switches, lookalike distractors, delayed evidence, or old contradictions. Report raw task success separately for every interference level and stress family. A composite scalar is secondary and cannot override a failed slice.

## Experimental design

### Stage A — freeze and reproduce

Before changing Symtropy's controller:
1. Select a named eligible baseline and pin evaluator, code commit, weights/checkpoint, environment/assets, prompts, preprocessing, action executor, precision, ordered seeds, and dataset split.
2. Verify data licenses/availability and record SHA-256 digests for downloaded artifacts.
3. Run one real episode end-to-end, including inference, action execution, and the official success checker. A launcher dry run or CSV analysis is not baseline reproduction.
4. Reproduce an official subset before spending compute on the full suite. Count crashes and incomplete episodes under the official protocol; report missing episodes separately.

Do not call a partial subset a full benchmark reproduction.

### Stage B — memory-only ablations

Keep backbone/checkpoint, action policy, decoder/executor, observation preprocessing, prompts, inference precision, and environment revision fixed. Compare in this order:

| Variant | Purpose |
| --- | --- |
| M0: no persistent memory | Establish the no-history reference. |
| M1: FIFO/recent window | Establish the low-complexity recency baseline. |
| M2: uniform reservoir/session sampling | Test coverage without query guidance. |
| M3: matching-session retrieval | Strong simple retrieval baseline under interference. |
| M4: source-time-aware relational/event store | Test provenance and typed temporal state. |
| M5: fixed hybrid (M4 + capped perceptual cache) | Test representation complementarity without a learned router. |
| M6: task-conditioned hybrid broker | Test adaptive admission, update, retrieval, and eviction. |

Run the clean RoboMME protocol and the RoboMME-Interference protocol as **separate benchmark reports**. For interference, preserve the official no-history, k0, k1, k3, and k7 conditions and seeds. Do not mix these task-macro means into one leaderboard score.

After Stage B, ActiveArena may test active view selection, but only as a new factor. Do not change the memory policy and view-selection policy in the same first experiment.

### Stage C — shift and failure stress

In a separate diagnostic suite, introduce:
- semantically similar but irrelevant sessions;
- repeated names/objects with different identities;
- contradictory observations at later source times;
- delayed/out-of-order evidence;
- task/user switches;
- long gaps that make old state stale;
- distractor bursts and skewed session frequency;
- a relevant lesson that is old but uniquely informative.

Stress tests must be generated from fixed declared distributions and published seed roots. Hidden truth and labels are used only by the evaluator to stratify scores after decisions are complete. The policy itself may observe only runtime-available task and evidence fields.

## Budget and fairness contract

Every compared policy gets the same total limits:
- memory bytes per episode/session (and native token accounting for token-based models);
- number of policy queries or retrieval operations, when applicable;
- action budget, inference configuration, and wall-clock measurement conditions.

Because record formats differ, report both serialized payload bytes and any extra index/vector-store overhead if measured. Logical-state estimates are not measured RAM. Work units are not measured latency. Do not let the adaptive policy silently run more retrieval calls, use a larger context, or retain a larger bank.

Keep active sensing, planning, training, and backbone changes disabled in the memory-only comparison. Any later combined system requires a factorial or sequential ablation that can identify each factor's contribution.

## Metrics and definitions

### Primary outcome

- Official native task success and task-macro mean.
- Per-task results, suite/category results, and every interference condition.
- Paired task/seed deltas versus the strongest eligible baseline reproduced locally; bootstrap confidence intervals stratified by task and clustered by episode.

### Interference benefit

Let (S_k) be task-macro success at interference level (k), and (S_N) be the matched no-persistent-history baseline. Define the **memory benefit** as (B_k = S_k - S_N). Report (B_k) in percentage points at each condition.

When (B_{k0}) is positive and sufficiently far from zero, report **benefit retention** (R_k = B_k/B_{k0}). If the denominator is non-positive or near zero, mark retention undefined rather than allowing a ratio to mislead. Always report raw (S_k) and (B_k) alongside retention. A policy with low clean success cannot win by retaining a near-zero benefit perfectly.

Also report the area under the *memory-benefit curve* over the ordered interference conditions as a secondary summary, with the values at k0/k1/k3/k7 always visible. Do not replace worst-condition performance with the area.

### Retrieval and safety diagnostics

- Relevant-session retrieval precision/recall where relevance labels are provided by the benchmark.
- Irrelevant-memory exposure and retrieval latency.
- Contradiction handling, stale-fact error rate, entity-confusion rate, and recovery after a task switch.
- Memory entries admitted/updated/evicted, actual bytes/tokens, read/write counts, and measured median/p95 latency, CPU, RAM, and VRAM.
- All failed, reset, crashed, incomplete, and missing episodes.

Any memory deletion or counterfactual usefulness audit is **evaluator-only**. It cannot provide labels, rewards, or unselected observations to the acting policy.

## Statistical and provenance controls

- Pair methods on the same ordered task/seed episodes.
- Bootstrap at the episode level, stratified by task; never treat frames or steps in an episode as independent samples.
- Freeze policy code and development tuning before the final evaluation seeds are opened.
- Report raw counts, effect sizes, intervals, missingness, and all task-level failures.
- Preserve command lines/configuration, artifact digests, checkpoint and dataset hashes, environment versions, and exact source SHA.
- A CI result applies only to the exact tested commit. A historical pass is not qualification of a new branch head.
- Exploratory candidate selection must be labelled exploratory. A new seed root is an independent-seed replication, not a fully preregistered confirmatory study when the candidate was selected from earlier results.

## Advancement and falsification gates

**Internal ambition (not an industry standard):** improve task-macro success by at least 5 absolute percentage points over the strongest eligible baseline reproduced locally, with the paired 95% interval above zero and a better success/resource Pareto point at the same budget.

Also require:
1. no hidden task, suite, or k7 collapse hidden by aggregate metrics;
2. no material ID/clean-task regression unless it is explicitly part of a predeclared trade-off;
3. improved interference benefit over matching-session retrieval, not merely over FIFO;
4. no gain attributable solely to more memory, more compute, a different checkpoint, or different action execution;
5. a distinct native robotics result before making any real-world/SOTA claim.

Reject the broker if its gains vanish under independent frozen seeds, if it fails to beat the simple retrieval baseline, or if a supposedly better aggregate depends on excluding failures. If M3 is competitive at lower cost, prefer M3; complexity must earn its place.

## Expected deliverables

1. A pinned baseline-parity record with one complete real rollout.
2. A deterministic report schema and episode-level results for every memory variant.
3. A clean-task report and a separate interference report.
4. Paired task/seed confidence intervals, cost/success frontiers, all failure counts, and reproducibility receipts.
5. Only after the controlled memory-only comparison: a separate ActiveArena adapter/controller experiment.

## Explicit non-claims

This research contract does not claim that Symtropy has reproduced RoboMME, RoboMME-Interference, ActiveArena, or RoboMemArena; it does not claim that the proposed broker improves success; and it does not convert synthetic loss or operation-count proxies into robot success, milliseconds, energy, RAM, VRAM, or industry/SOTA standing.


## Industry / SOTA comparison without false equivalence

A vendor capability announcement, a synthetic diagnostic, and an official benchmark score are different evidence classes.

- **ActiveArena** is the best public reference for view selection under manipulation: the official table reports ActiveArena-OFT at 62.97% ID and 47.60% OOD task-macro success over 35 tasks. This is a published external baseline, not a Symtropy result. Exact parity requires the eligible checkpoint, official evaluator, preprocessing, action head/executor, scene split, and ordered seeds. Source: https://leeibo.github.io/ActiveArena/
- **RoboMME** is the relevant open memory-representation comparison: 16 manipulation tasks across four memory suites and 800 test episodes under the published protocol. Use its own task-macro and suite metrics. Source: https://proceedings.mlr.press/v306/dai26c.html
- **RoboMME-Interference** adds the more deployment-realistic question of whether memory survives unrelated sessions. It should be reported separately from RoboMME clean-task scores. Its project reports that matching-session retrieval is more resistant to k7 interference than frame-sampling variants in its tested configuration. This is a motivating external result, not a guarantee about a new architecture. Source: https://github.com/SoumilRathi/robomme-interference
- **RoboMemArena** broadens the memory setting to 26 manipulation tasks and publishes CSR (final goal success) and TSR (all-stage success). Its maintainers flag the June 20, 2026 dataset refresh as important; reproduce only after pinning matching assets, code, and scoring logic. Source: https://github.com/OpenHelix-Team/RoboMemArena
- **Commercial foundation-model systems** are important context but generally cannot be compared by headline claims alone. For example, Google DeepMind's July 30, 2026 Gemini Robotics ER 2 announcement emphasizes real-time spatial reasoning, orchestration, and multi-robot collaboration, but those feature statements are not a directly comparable cross-session-memory score. Source: https://blog.google/innovation-and-ai/models-and-research/google-deepmind/gemini-robotics-er-2/

### What would justify a “better than SOTA” claim?

1. Name the benchmark, task split, checkpoint, and exact eligible baseline; distinguish published reference numbers from runs reproduced locally.
2. Reproduce baseline parity, then run a controller-only memory ablation with identical observations, backbone, executor, inference precision, seeds, and memory/compute budgets.
3. On ActiveArena, aim internally for at least +5 absolute percentage points OOD task-macro success over the strongest eligible locally reproduced baseline, with a paired 95% interval above zero and no more than a 3-point absolute ID regression. These are project gates, not universal community standards.
4. On the interference benchmark, report the full no-history/k0/k1/k3/k7 curve, raw success and memory benefit for each level, and paired uncertainty. A k7 gain must not be bought by degrading k0/clean-task success or by increasing memory/compute.
5. Report measured p50/p95 end-to-end latency, CPU, RAM, VRAM, memory bytes/tokens, failures, and the success/resource Pareto frontier. Synthetic operation counts are not a substitute.
6. Publish failures and per-task regressions. A higher aggregate is not enough if memory becomes brittle for one task family or under contradictions and task switches.

**SOTA boundary:** the ActiveArena and RoboMME numbers are not numerically comparable to each other, to RoboMemArena, or to any synthetic VIS loss. Without a matching official protocol and reproduced baseline, the right claim is “new research candidate” rather than “industry-beating.”


## Process-level diagnosis: separate *what failed* from final task failure

**New external benchmark discovered during the 2026-10-21 research pass:** [MEMOBench](https://github.com/Collab-Gen/MEMOBench), paper [arXiv:2609.07047](https://arxiv.org/abs/2609.07047). The upstream README at code snapshot `490b58ffcbd4ac042d6d1daf218b4fbd2827f4cb` describes 30 history-dependent manipulation tasks and 4,200 executable checkpoints annotated for three operations: **Storage, Update, Compression**. It separately reports final task success and per-operation rates, plus failure categories such as storage failure, stale memory, compression collapse, reference drift, and execution failure. Its reported strongest memory-module baseline reaches 31.9% average task success, and the authors report that high storage performance can coexist with weak updating and compression. Those are the paper's reported results, not Symtropy results.

This changes the diagnostic plan: a final task failure alone cannot tell whether the system failed to write the needed information, failed to revise it after the world changed, discarded decision-critical information while compressing, retrieved the wrong referent, or executed the action incorrectly.

### Lifecycle-gated memory hypothesis

Treat each memory operation as a state transition with an auditable result rather than an unobservable side effect:

- **Storage:** append a provenance-bearing observation/event and record which downstream-required fields it preserves. Do not mark storage successful merely because a write call returned.
- **Update:** when new evidence changes or contradicts an earlier belief, create a revision/supersession edge and preserve the earlier provenance. A delayed observation with older source time must not silently overwrite newer state.
- **Compression:** compact multiple events only if the compressed representation preserves every evaluator-declared decision-relevant predicate for subsequent checkpoints. Compression ratio alone is not success.
- **Retrieval/use:** log which version/source informed a decision and whether it was stale, disputed, superseded, or identity-ambiguous. Do not let evaluator labels or future outcomes influence the controller.
- **Execution:** track the action outcome separately from memory state so memory correctness is not blamed for a controller/executor failure, and vice versa.

The proposed engineering primitive is an append-only event/provenance log plus a bounded current-state projection. Compression may replace the projection, but it must not forge provenance or erase unresolved contradictions. For each write/update/compression, emit a deterministic receipt containing operation kind, prior-state digest, resulting-state digest, changed record IDs, retained/retired source IDs, byte delta, and a reason code. Receipts are diagnostic evidence; they are not rewards and do not expose hidden truth.

### Stage 0 experiment before the interference cross-product

1. Pin MEMOBench source, dataset/assets version, LIBERO/environment revision, policy/checkpoint, OpenPI protocol, evaluator, and seeds; record hashes. The repository README declares **CC BY-NC 4.0** for MEMOBench. Treat it as non-commercial research data unless licensing counsel confirms a use is permitted; do not bundle its assets/annotations into a commercial product or redistribute them absent permission. Third-party dependencies retain their own licenses.
2. Run one true end-to-end checkpointed episode and verify the native success checker plus the operation-level outputs. Record simulator setup and missing/failure counts.
3. Reproduce one named memory baseline on a small declared subset, then the official protocol if resources allow. A source/CSV inspection is not a reproduction.
4. Compare the same controller through storage/update/compression variants while holding backbone, policy, action decoder/executor, precision, prompt, and seeds fixed.
5. Only after the clean process-level baseline is stable, cross a **small predeclared subset** of operation types with interference conditions. Do not relabel this hybrid subset as the original MEMOBench or RoboMME leaderboard protocol.

### Additional process metrics

- Native MEMOBench: task success, Memory Storage Rate (MSR), Memory Update Rate (MUR), and Memory Compression Rate (MCR), with their official checkpoint predicates and native aggregation preserved.
- **Revision correctness:** fraction of changed-world checkpoints where the live projection reflects the newest valid source-time evidence and retains a link to superseded evidence.
- **Stale-use rate:** decisions whose cited evidence was superseded/expired at decision time, reported separately from delayed-but-still-valid evidence.
- **Compression retention:** fraction of evaluator-declared decision-critical predicates preserved after compression; report bytes saved as a separate resource result.
- **Provenance integrity:** every answerable current claim traces to source observations; no hidden evaluator labels are present in controller state.
- **Failure attribution:** classify storage miss, update miss, compression loss, wrong retrieval/reference, model/controller failure, executor failure, and missing evaluator outcome.
- **Cross-session degradation:** on the separate RoboMME-Interference protocol, report raw success and memory benefit at no-history/k0/k1/k3/k7; do not merge its scores with MEMOBench MSR/MUR/MCR.

### Falsification criteria

Reject the lifecycle-gated design if receipts can pass while native predicates fail, if compressed states lose required facts more often than the fixed-summary baseline, if updates cannot resolve source-time/identity contradictions, or if extra audit state breaks the fixed memory budget. If the audit trail is larger than the saved representation, report that overhead rather than excluding it. If the controller task score improves while one lifecycle operation regresses materially, keep that trade-off explicit and do not claim general memory reliability.

### Research snapshot / constraints

The MEMOBench main branch resolved to `490b58ffcbd4ac042d6d1daf218b4fbd2827f4cb` on 2026-10-21; the README described dataset/asset downloads and an OpenPI websocket policy server. Re-resolve all artifacts when execution begins. The benchmark is separate from RoboMME and RoboMME-Interference; its operation-specific rates and its 31.9% task-success result are not numerically interchangeable with their published success metrics.


## Stronger architecture comparator discovered: HyMeS

Research sweep update (2026-10-21): the preprint [Skills in Weights, Memory in Code: Hybrid Learning for Memory-Dependent Robot Manipulation](https://arxiv.org/abs/2608.09410) reports **HyMeS**, an architecture that separates motor-skill learning in model weights from memory-strategy learning in executable code. A frozen Markovian VLA is steered by state-conditioned constraints, and stage progression is updated using both proprioceptive signals and multi-frame VLM verification. The paper uses a 3-of-5 completion-query vote before advancing a stage.

On the authors' corrected **12-task / 160-episode RoboMemArena protocol** (not the full 26-task benchmark), the paper reports:
- π₀.₅: 52.5% Cumulative Success Rate (CSR), 41.3% Task Success Rate (TSR).
- PrediMem, reevaluated on the same corrected subset: 61.7% CSR, 45.6% TSR.
- HyMeS: 66.2% CSR, 60.1% TSR.
- On three SO-101 real-world tasks / 35 trials total, the paper reports TSR rising from 25.7% for the matched fine-tuned π₀.₅ policy to 57.1% with HyMeS.

These are **paper-reported preprint results**, not a Symtropy reproduction or an independently validated industry score. They must not be compared to RoboMemArena's 26-task leaderboard numbers without matching the protocol. The paper also notes remaining failures from motor execution and stage-verification errors, and inference latency/cost from periodic VLM queries and coding-agent use. During this research pass, a public HyMeS implementation/checkpoint repository was not found, so local reproducibility and licensing remain unverified.

### What this changes in the Symtropy hypothesis

The strongest design challenge is no longer just "better retrieval" or "better compression". HyMeS suggests the control boundary between **memory, executable intent, verification, and action** may matter more than the storage representation by itself.

The Symtropy-specific hypothesis should be a *provenance-gated executable memory controller*:
1. Keep the evidence ledger and memory-state transitions auditable and bounded.
2. Compile only verified, non-ambiguous state into a typed task-state machine or action constraint; unresolved or stale evidence cannot advance a task stage.
3. Require a predeclared, calibrated completion rule using independently sourced signals when available, with a safe no-advance/fallback state when signals disagree.
4. Record which evidence versions authorize a transition, the verifier votes, and the resulting state digest.
5. Keep memory strategy and action executor independently ablatable. Do not attribute an improvement to memory if the policy weights, action chunks, completion verification, or replanning interval also changed.

This adds an important split to the test plan:
- The Rust VIS-MEM-008 lifecycle model checks source-time monotonicity, provenance links, fixed-budget compression, and deterministic receipts cheaply.
- A later native RoboMemArena comparison should reproduce the exact 12-task HyMeS protocol **only if its checkpoint/code/dependencies can be pinned and legally used**; otherwise use an eligible public baseline, label the HyMeS paper as literature context, and preserve its distinct protocol.
- Compare memory-only effects first; test stage verification as a separate factor; test their interaction only in a small predeclared factorial design.

**New advancement criterion:** a memory system must improve both the correctness of its internal lifecycle operations and the external decision/task outcome. Passing the lifecycle microbench is necessary evidence of semantics, not sufficient evidence of robotic competence.


## New current SOTA-style comparator: Mem4Act and causal memory use

The research sweep on 2026-10-22 found the [Mem4Act preprint](https://arxiv.org/abs/2610.22323), submitted 2026-10-15, plus its [official project page](https://handsomeyun.github.io/Mem4Act/). It reports a fixed **512-latent-token** memory interface and:
- **RoboMME:** 49.88% across 16 tasks; the paper reports a +5.37 percentage-point gain over FrameSamp+Modul (44.51%). Mem4Act leads overall and on Counting, Reference, and Imitation; MemER is stronger in Permanence, which is an important reminder not to hide suite-specific losses.
- **RMBench:** 61.0% over nine dual-arm tasks, with one jointly trained policy; the paper reports 41.3% on its four longer-history M(n) tasks versus 14.8% for NativeMEM in its comparison, but notes that NativeMEM uses task-specific policies while Mem4Act is jointly trained. This is not a perfectly matched training setup.
- **Four real-world task setups:** 81.5% average success in the preprint.

These are paper-reported preprint results, not Symtropy results, an official universal leaderboard, or independent reproduction. The project page labels code and dataset as “coming soon” during this check, so no code/checkpoint parity is currently established. The results must be rerun with a matched evaluator and configuration when artifacts are released.

### Architectural lesson: memory must be used, not merely stored

Mem4Act explicitly studies *memory utilization*: it swaps the historical prefix with another rollout from the same task while keeping the current observation, proprioception, and instruction correct. Its reported two-stage policy shows a larger success drop under history swap, which the authors interpret as stronger reliance on history. This suggests adding an intervention diagnostic to Symtropy, while guarding against the opposite failure: an agent that is over-sensitive to irrelevant or stale history.

Define two complementary tests over the same current observation and task instruction:

1. **Relevant-memory intervention:** substitute a matched counterfactual history that changes a task-critical past fact while leaving current input fixed. The chosen action or task state should change when, and only when, the fact changes the correct decision.
2. **Irrelevant-memory intervention:** add or replace unrelated history while preserving all task-relevant facts. The decision should stay invariant within a declared tolerance.
3. **Stale/contradictory-memory intervention:** replace a current valid source revision with an older or superseded record. A provenance-aware controller must reject the stale substitution or reduce authority rather than silently changing to an obsolete answer.
4. **Memory knockout:** remove persistent memory while retaining the exact current observation. Measure the success delta on tasks whose answer mathematically requires historical information; do not reward dependence on memory for tasks solvable from the current view.
5. **Memory-reliance precision:** report both (a) required sensitivity to relevant changes and (b) invariance to irrelevant changes. A single “memory reliance” number is insufficient because brittle over-reliance can also produce a large history-swap performance drop.

These interventions belong in the evaluator, offline counterfactual runs or controlled replay—not as oracle labels fed to the acting policy. Keep the policy state and current observation identical except for the declared history intervention. Use episode-level paired measures and task-stratified confidence intervals.

### Consequence for the Symtropy experimental sequence

Once the Rust lifecycle microbench is exact-head qualified:
1. Add deterministic relevant/irrelevant/stale-history intervention tests to the synthetic memory evaluator.
2. Use them to select memory semantics and admission policy without claiming native control gains.
3. On the first eligible VLA baseline, reproduce RoboMME and run a history-channel ablation with the same checkpoint, prompt, image processing, executor and seeds.
4. Add memory intervention and interference only as explicitly separate factors, then combine them in a small predeclared matrix.

Mem4Act is the reference for learned fixed-size latent memory and causal utilization; HyMeS is the reference for executable memory strategy and stage verification; MEMOBench is the reference for process-level storage/update/compression. They should be compared on their native protocols, not ranked using a fabricated combined score.
