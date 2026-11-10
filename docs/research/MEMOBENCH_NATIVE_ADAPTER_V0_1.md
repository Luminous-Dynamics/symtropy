# MEMOBench native adapter and evidence contract v0.1

**Status:** typed Rust schema bridge and synthetic tests. No MEMOBench simulator, checkpoint, policy, or physical robot has been run by this change. No Luminous benchmark score is claimed.

## 1. Why this is a useful benchmark

MEMOBench is a process-level robotic-manipulation memory benchmark. Its published release contains 30 history-dependent tasks, 1,500 expert demonstrations, 4,200 checkpoint instances from 84 templates and three operation labels: Storage, Update and Compression. It evaluates 1,500 held-out simulator rollouts (50 per task), with a 1,200-policy-step horizon and replanning intervals of 5, 20, 35 and 50. The benchmark's task families cover temporal, spatial, object and procedural memory.

Sources:
- Paper, Findings of EMNLP 2026: https://aclanthology.org/2026.findings-emnlp.333/
- Public repository: https://github.com/Collab-Gen/MEMOBench
- Dataset card: https://huggingface.co/datasets/SunSeaLucky/MEMOBench

## 2. What the benchmark actually measures

The benchmark's Storage/Update/Compression rates are computed from *simulator predicate completion at ordered checkpoints*. Each checkpoint pairs a text annotation and an executable BDDL predicate, and has one memory-operation label. The evaluator does not inspect the policy's internal memory bytes or data structure directly.

The public task metadata uses a three-column operation selector. The upstream evaluation chooses `operating_type[row][memory_level]`, where:
- `0` = Storage;
- `1` = Update;
- `2` = Compression;
- `memory_level` is 0, 1 or 2.

For each episode, the evaluator checks ordered `subtask_goals` against simulator state and records whether each predicate was reached. It then groups those predicate outcomes by the selected operation label. Thus, official MSR/MUR/MCR are operation-stratified task-state checkpoint rates. They are a useful process-level diagnostic of whether the policy behaved consistently with a memory annotation; they are not a direct measurement of internal memory storage, belief revision or compression.

The new adapter preserves this distinction:
- `memobench_adapter.rs` imports the benchmark's selected operation labels and reconciles checkpoint predicate results. Its metric names explicitly retain the upstream task-state meaning.
- `memory_operations.rs` is a separate, stronger instrumented evaluation contract. Where a memory backend exposes independent readback, it can directly test semantic storage/update/compression effects. These two result families must never be merged into one number.

This avoids a common measurement error: reporting a successful simulator subtask as proof that the memory store itself contains a correctly sourced fact.

## 3. Published comparison at replan interval 5

The paper's main table reports the following best matched row at replan interval (k=5):

| Method | Average task SR | Storage MSR | Update MUR | Compression MCR |
|---|---:|---:|---:|---:|
| π0.5 base | 30.4% | 54.9% | 13.5% | 4.2% |
| Memory module | 31.9% | 58.1% | 16.5% | 7.8% |
| Semantic Memory Alignment | 31.9% | 60.8% | 17.4% | 8.1% |
| Framewise Memory Alignment | 32.7% | 59.2% | 15.4% | 7.1% |
| **Contrastive Memory Alignment** | **34.7%** | **59.8%** | **21.4%** | **11.1%** |

The task-family rows behind the contrastive result are:

| Memory family | SR | MSR | MUR | MCR |
|---|---:|---:|---:|---:|
| Temporal | 14.4% | 91.8% | 39.6% | 14.0% |
| Spatial | 1.4% | 7.6% | 1.6% | 8.3% |
| Object | 99.2% | 99.2% | N/A | N/A |
| Procedural | 23.6% | 40.4% | 23.0% | N/A |

The overall averages are macro averages over applicable task families/operations; N/A is not zero. The headline 31.9% average success number often repeated for MEMOBench refers to the strongest generic memory-module baseline, not the best row in every experiment.

The paper also reports a MemER reference with average SR/MSR/MUR/MCR of 56.4/65.4/45.2/70.3. It receives checkpoint-based keyframe and subtask supervision, so it is a **privileged-supervision reference**, not a matched SOTA comparator. The oracle-memory decomposition likewise appends ground-truth memory text to the prompt. These are useful diagnostics of the remaining headroom, not fair baselines for an ordinary deployed memory system.

Source for the exact row values, task-family metrics, and privileged-reference caveat: https://www.thelatestinai.com/papers/2609.07047 (Tables 2, 4 and 10; paper arXiv:2609.07047).

## 4. Rust implementation

Implementation: `experiments/robocasa-mem-bce-contract/src/memobench_adapter.rs`, exported from the nested crate.

The adapter provides:
- a typed three-level memory selector and operation-code mapping;
- per-checkpoint hashes for the selected memory-description annotation and evaluator-private subtask goal;
- a frozen benchmark commit, evaluation protocol, policy, memory system and evaluator identity;
- code, dataset, asset and third-party license identities and digests;
- an explicit use-purpose attestation; commercial development requires both a separate permission digest and a separate verification-receipt digest;
- an exact task-ID roster, per-task checkpoint count and exact episode count per task;
- checkpoint outcomes bound to task, episode, checkpoint index, memory level, seed, selected annotation, subtask predicate, episode artifact and evaluator receipt;
- separate terminal episode-success outcomes;
- micro operation rates, per-task rates, equal-task macro rates, and distinct unresolved counts.

The adapter preserves the source's ordered checkpoint indexing. A missing or duplicated task, checkpoint, episode, result or receipt cannot silently reduce the denominator. A predicate explicitly reported false is a genuine NotCompleted result. A missing result, malformed run record, or missing trajectory/evaluator receipt is Unresolved. Official rates are withheld for a stratum with unresolved outcomes instead of treating missing artifacts as successes or silently converting infrastructure indeterminacy to a predicate miss.

The adapter does **not** download, parse, or vendor the benchmark dataset itself. The caller supplies parsed annotations and externally computed digests. A separate loader is still required for the public JSON/HDF5 and simulator artifacts. The adapter checks digest shape and identity equality; it does not authenticate files, verify evaluator independence, or establish the truth of a rightsholder permission.

## 5. Licensing boundary

The public MEMOBench repository identifies code, task definitions and annotations under CC BY-NC 4.0. The Hugging Face dataset card also identifies the dataset as CC BY-NC 4.0; its distribution includes the benchmark data and object/scene assets. Third-party code such as LIBERO remains under its own license.

This is non-commercial research material. A commercial Luminous deployment must not use MEMOBench code, task annotations, demonstrations or assets on the assumption that the public research license permits it. Separate permission from the relevant rightsholders and independent review are required. A license digest in this adapter is evidence identity, not a legal opinion or a proof that the stated use is authorized.

The adapter deliberately stores hashes rather than copying the benchmark's task text, demonstration data, BDDL predicates or asset files into this repository.

## 6. How to beat the matched baseline

Do not claim improvement by comparing different replan intervals, memory levels, task subsets, seeds, checkpoints or success definitions. First reproduce the published contrastive-alignment row at (k=5) on all 30 tasks and 1,500 held-out rollouts. Then open a separately preregistered held-out condition.

Proposed Luminous target at the same (k=5) protocol:

| Metric | Published matched best | Proposed target | Required gain |
|---|---:|---:|---:|
| Average task SR | 34.7% | **≥39.7%** | +5.0 pp |
| Storage MSR | 59.8% | **≥64.8%** | +5.0 pp |
| Update MUR | 21.4% | **≥26.4%** | +5.0 pp |
| Compression MCR | 11.1% | **≥16.1%** | +5.0 pp |

These are prospective targets, not predictions or results. Guardrails:
- no task-family SR, MSR, MUR or MCR regression greater than 2 percentage points against the reproduced matched baseline;
- preserve N/A for operations that do not apply; do not fill missing operations with zeros;
- use at least three independent training/checkpoint seeds, the same 30 tasks, same 50 evaluation episodes per task, same replan interval and same policy/action wrapper;
- report all per-task results and macro/micro aggregates, plus task/seed-clustered uncertainty intervals;
- disclose memory bytes, write/update/retrieval latency, VLA calls, token/context budget, wall-clock time and GPU/CPU energy;
- retain all videos, simulator traces, checkpoint receipts and exact artifact digests.

Only after the matched reproduction should Symthaea be described as outperforming the benchmark baseline. A gain on the privileged MemER row does not count as a matched SOTA improvement.

### Paired inference and cluster bootstrap

The instrumented-memory comparator in Symtropy PR #4566 requires both memory arms to use the same protocol, task/checkpoint/query roster, policy checkpoint, evaluator, embodiment/action/resource-control manifest and independent-unit roster. Only the memory-system identity changes. It reports per-unit paired deltas, then a deterministic task-stratified cluster bootstrap resamples independent scene/seed units within each task and equally weights task means. Repeated checkpoints from one scene/seed are never resampled as separate independent observations.

Recommended first preregistration: at least 20 truly independent scene/seed units per contributing task, 10,000 replicates, 95% percentile interval and a fixed RNG seed. The published 50 episodes/task are not automatically 50 independent units; scene reuse, initial-state reuse, demonstrations, simulator resets and seed construction must be audited first. If any task falls below the declared minimum, the adapter reports Underpowered and withholds bounds. These percentile intervals are not p-values and do not guarantee nominal coverage.

## 7. Paradigm shifts for Symthaea

The research opportunity is not to add an ever-larger text summary. It is to make memory a queryable, versioned and evidence-carrying state system:

1. **Event-sourced relational memory.** Store source events and object/state relations with provenance, time, confidence and scope. Derive compact views from immutable history; do not destructively overwrite the only record.
2. **Contradiction-aware belief revision.** On an update, retain the old belief, source, new evidence, invalidation reason and replacement relation. Stale or contradictory facts become explicit states, not silently blended prose.
3. **Query-preserving compression.** Compress against preregistered future queries and critical facts. Test the compressed representation by asking held-out evaluator queries; separate semantic preservation from byte reduction.
4. **Value-of-information retrieval.** Retrieve or re-observe only when expected decision improvement justifies latency and resource cost. Measure requests, retrieved bytes/tokens, useful retrievals, and unnecessary retrievals separately.
5. **Epistemic safety boundary.** Treat observation, inference, user instruction, simulator oracle and external memory as different trust classes. Untrusted content must not silently acquire policy authority. Unknown critical state triggers re-observation or abstention.
6. **Memory as a challengeable hypothesis.** Every critical memory claim has a source, timestamp, freshness rule and testable predicate. Symtropy challenges memory state; the independent simulator/evaluator or instrument decides whether the predicate is true.

Ablations should compare π0.5/base, generic memory module, semantic/contrastive/framewise memory alignment, FIFO/recency, uniform reservoir, task-state history, object-state trajectories, event-sourced relational memory, contradiction-aware revision, query-preserving compression and the complete Symthaea selector. Use the same backbone and action interface whenever possible.

## 8. Next work sequence

1. Finish the typed-adapter review and exact-head Rust formatting/tests/strict Clippy; no pass is claimed until the workflow completes.
2. Build a pinned Rust loader for the public annotation JSON and a separate HDF5/artifact acquisition adapter; keep license-restricted files outside the general-purpose production repository.
3. Reproduce the published (k=5) contrastive-alignment row on the exact 30-task/1,500-episode protocol.
4. Add Symthaea memory arms one mechanism at a time: provenance-aware storage, contradiction-aware update, query-preserving compression, then value-of-information retrieval.
5. Add stale-fact, source-conflict, irrelevant-history, adversarial-memory and missing-receipt controls.
6. Relate official task-state MSR/MUR/MCR to independent instrumented memory readback and final SR, without merging their denominators.
7. Add SafeVLA-Bench safety specifications and NIST/ASTM-aligned contact/grasp measurements before moving to supervised physical trials.

The strategy is to outperform by making memory more faithful, updating beliefs correctly, preserving the information that matters under compression, and demonstrating the downstream task gain with comparable resources and reproducible evidence.
