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
