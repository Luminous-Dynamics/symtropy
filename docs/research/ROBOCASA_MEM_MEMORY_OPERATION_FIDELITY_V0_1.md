# Operation-level memory fidelity contract v0.1

**Status:** dependency-free Rust evaluator-contract implementation with synthetic tests. It is not a native MEMOBench adapter, a persistent memory store, or a measured policy result.

## 1. Motivation

Most robotics evaluations score final task success or stage completion. Those metrics do not identify whether a system:
- stored a fact correctly;
- updated stale knowledge when the world changed;
- compressed memory without deleting task-critical information.

MEMOBench makes this distinction explicit with Storage, Update and Compression operation labels and separate Memory Storage Rate, Memory Update Rate and Memory Compression Rate. Its published benchmark description reports 30 history-dependent tasks, 1,500 expert demonstrations and 4,200 executable checkpoint instances from 84 templates; the strongest memory-module baseline reports 31.9% average task success. These figures are external benchmark references, not Luminous results.

RoboMME provides a complementary broader taxonomy of temporal, spatial, object and procedural memory across 16 manipulation tasks. RoboCasa-Mem then tests whether memory supports task-specific behavior-conditioned and process obligations. We should not collapse these three forms of evidence:
- operation fidelity asks whether memory changed correctly;
- benchmark task success asks whether the agent completed the task;
- behavior-conditioned/process evaluation asks whether later actions respected earlier choices and unfinished obligations.

Sources:
- MEMOBench: https://www.max-robotics.com/en/research/publications/cmu3096f900b95qb2w5bgczrb
- RoboMME: https://proceedings.mlr.press/v306/dai26c.html
- RoboCasa-Mem: https://arxiv.org/abs/2611.06475

## 2. Contract

The evaluator supplies a frozen `MemoryOperationManifest` with:
- protocol and operation-roster digests;
- policy-artifact, memory-system and evaluator digests;
- an ordered case roster;
- task, episode, independence-unit and event-sequence identities;
- the operation type (Storage, Update or Compression);
- an explicit applicability declaration;
- the source-event digest;
- an evaluator-owned expected-effect digest for every applicable case.

The agent/policy must not receive the evaluator-private expected-effect digest. It receives only the benchmark-permitted task inputs. The manifest is held by the evaluator/benchmark harness.

An independent `MemoryOperationReceipt` records:
- matching case/task/episode/independence-unit/sequence/operation identity;
- source-event digest actually used;
- observed readback/effect digest;
- before/after memory snapshot digests;
- independent evaluator receipt digest;
- memory bytes before/after and measured operation latency;
- valid-run status.

The auditor derives the result. The policy does not send an outcome label.

## 3. Result semantics

For each applicable case:

- **Correct:** run is valid, source-event lineage matches, before/after memory snapshots and evaluator receipt exist, and observed effect digest equals the evaluator's expected effect digest.
- **Incorrect:** the readback exists and is independently evidenced, but source lineage or resulting effect differs from the frozen expectation.
- **Unresolved:** no receipt, invalid run, missing readback, missing before/after snapshot or missing evaluator receipt.
- **NotApplicable:** the frozen evaluator roster explicitly marks the operation not applicable. It is excluded from the applicable denominator; a receipt for such a case is rejected.

Missing evidence is never converted into a pass or silently removed from the denominator. A wrong update is Incorrect; a timed-out update with no readback is Unresolved. This distinction is important for debugging and for fair memory-module comparisons.

## 4. Metrics

Each operation type receives a separate summary:

`fidelity_rate = correct_count / applicable_count`

`evidence_coverage = (correct_count + incorrect_count) / applicable_count`

Unresolved cases stay in the fidelity denominator but not in the evidence-coverage numerator. NotApplicable cases are counted separately and do not enter the denominator.

For Compression, the audit also reports:
- number of cases with both byte measurements;
- total bytes before and after;
- aggregate compression ratio = total bytes after / total bytes before.

A zero-byte baseline does not produce a ratio. Missing byte measurements reduce the compression-measurement denominator but do not alter the operation's correctness score. Latency has its own explicit measurement count and mean. Query identity is part of the evaluation contract: an observed readback for a different query cannot pass, and a missing query readback remains unresolved.

The module reports Storage, Update and Compression separately. It emits operation-wide micro rates and per-task rows, plus an equal-task macro fidelity/evidence-coverage rate over tasks with applicable cases. This exposes Simpson's-paradox-style masking when easy tasks are overrepresented. It does not produce a single weighted “memory score”; any aggregate must be preregistered with explicit weights and should not hide a weak operation dimension.

Compression correctness is semantic: the evaluator specifies required information and checks an independent readback projection. The compressed bytes may differ from the source representation and still pass if the required facts are preserved. Byte reduction is measured separately by the compression ratio; latency is also separate. A compact but semantically lossy memory is incorrect, while a semantically faithful representation that happens to grow is correct but inefficient.

## 5. Implementation and safeguards

Implementation: `experiments/robocasa-mem-bce-contract/src/memory_operations.rs` (exported by the nested crate).

The auditor:
- rejects an empty manifest, incorrect expected case count, duplicate case IDs, duplicate sequence numbers and non-monotonic case order within an episode;
- rejects malformed digests, missing expected effects for applicable cases and expected effects on not-applicable cases;
- rejects duplicate, unknown, not-applicable or identity-mismatched receipts;
- binds every observation to the exact evaluator query and frozen query-roster digest;
- preserves the independent-unit identity; multiple operation cases from one scene/seed cluster may share that unit;
- reports distinct independent-unit counts so many checkpoints from one scene/seed cannot be presented as many independent samples;
- requires an explicit complete task roster so omitted easy/hard tasks cannot silently change the population;
- derives operation outcomes from evaluator-owned and observed semantic-effect digests;
- returns each case's source/effect/snapshot/evaluator lineage alongside the derived outcome;
- reports operation-specific denominators, unresolved cases, compression ratio and latency measurement counts.

The module checks digest shape and identity equality. It does not hash or authenticate files, prove that the evaluator is independent, validate the truth of a memory snapshot, or establish the benchmark's training/evaluation split. Those responsibilities belong to the native adapter and independent evidence process.

## 6. Benchmark experiment

For MEMOBench and later RoboCasa-Mem adapters, compare on matched task/checkpoint/scene/seed sets:
1. no explicit memory;
2. raw FIFO/recency memory;
3. uniform reservoir or chunk writing;
4. task-state history;
5. object-state trajectory;
6. relational/process memory;
7. planner-guided sparse memory;
8. full Symthaea memory/value-of-information selector.

For every arm report:
- Memory Storage Rate;
- Memory Update Rate;
- Memory Compression Rate;
- task SR and stage completion separately;
- operation-level unresolved and incorrect cases;
- bytes stored, retained and retrieved;
- retrieval latency, write/update latency and total inference budget;
- downstream task outcome conditional on each operation type;
- held-out scene/seed-clustered uncertainty intervals.

Causal ablations should remove one necessary source fact, perturb irrelevant history, introduce a stale fact requiring update, and challenge compression with facts that differ in task relevance. The expected memory-effect manifest must be frozen by the evaluator before the policy runs. The policy must not observe the target digest or hidden predicate.

Do not claim that a larger memory or higher storage rate is better unless it improves retention of necessary facts, correction of stale state, compression fidelity and downstream task performance at a matched resource budget.

## 7. Claim boundary and next steps

The 26 tests in `memory_operations.rs` and 14 tests in `memobench_adapter.rs` are synthetic accounting/identity fixtures. They do not run MEMOBench, RoboMME, RoboCasa-Mem, HMAP, π0.5 or Symthaea; they do not demonstrate better memory performance or physical robotics capability.

The native MEMOBench typed schema bridge is now implemented in `memobench_adapter.rs`. It selects the upstream `operating_type[row][memory_level]` code, preserves the checkpoint/predicate/annotation digests, validates the complete task/checkpoint/episode roster, requires receipt-bound outcomes, and reports operation-stratified predicate completion separately from terminal task success. The bridge still needs a real JSON/HDF5 acquisition loader and a native simulator run; no benchmark results are claimed. Its license gate records the CC BY-NC 4.0 boundary and requires separate commercial permission plus a verification receipt for any commercial-development use.

Next:
1. implement a pinned Rust loader for the 30 public task metadata records and HDF5/rollout artifact identities without copying benchmark annotation text into this repository;
2. reproduce the published replan-step-5 contrastive-memory baseline before changing the model;
3. add one Symthaea memory mechanism per ablation (provenance-aware storage, contradiction-aware update, query-preserving compression, value-of-information retrieval);
4. run matched held-out checkpoint/task/seed comparisons and report SR/MSR/MUR/MCR separately;
5. correlate native predicate completion with instrumented semantic memory readback and RoboCasa-Mem SR/SCR without merging denominators.

The published contrastive-memory row at replan interval 5 reports 34.7% average SR, 59.8% MSR, 21.4% MUR and 11.1% MCR. The next preregistered Luminous target is +5 percentage points on each under the same protocol (39.7/64.8/26.4/16.1), with no task-family regression beyond 2 pp. The stronger MemER results are treated as privileged-supervision reference values, not matched SOTA. Full source and targets: [MEMOBENCH_NATIVE_ADAPTER_V0_1.md](MEMOBENCH_NATIVE_ADAPTER_V0_1.md).
