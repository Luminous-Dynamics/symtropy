# Habitat Revalidation Scale Benchmark v0.1

**Status:** proposed benchmark contract; no scale measurements have been collected.
**Scope:** synthetic, deterministic planning workloads for the Rust habitat revalidation prototypes.
**Claim ceiling:** no permit, inspection, construction, safety, compliance, or real-world performance authority.

## 1. Why this benchmark exists

The existing joint-target planner establishes a small exhaustive correctness reference. It is not yet evidence that search or validation scales. This contract separates three questions that are often conflated:

1. **Semantic correctness:** are dependencies, evidence states, gates, and target scopes interpreted correctly?
2. **Search epistemics:** does the report distinguish a complete search from a budget-limited partial frontier?
3. **Operational scalability:** how do parse/validation cost, search work, memory, and frontier size grow as the graph and task library grow?

Use this suite with [HABITAT-REVALIDATION-005 #1869](https://github.com/Luminous-Dynamics/symtropy/issues/1869). Do not report performance wins until the benchmark runner emits results tied to an exact commit and frozen workload digest.

## 2. Paradigm shift: benchmark claims, not just elapsed time

A planner can be fast and still be wrong. A benchmark report must therefore contain both resource metrics and epistemic status.

Every result record must include:

- repository commit SHA, dirty-tree state, Rust toolchain, optimization profile, OS/kernel, CPU model/core count, memory limit, and command line;
- benchmark suite and generator versions, workload profile, seed, input byte length, SHA-256 of the exact serialized workload, and planner configuration;
- wall-clock time by phase (decode, structural validation, reachability/reduction, search, report serialization), peak RSS, expanded states, generated successors, duplicate-state hits, dominance comparisons, retained frontier size, and budget exhaustion;
- planner status, frontier completeness, and the explicit reason for any incomplete result;
- feasibility checks for every returned plan, including dependency ordering, gate satisfaction, target coverage, unique task counting, and recomputed objective values.

Do not treat a queued or missing CI run as measurement evidence. Do not mix numbers from different hardware into one latency percentile.

## 3. Frozen workload matrix

The adjacent machine-readable file **fixtures/HABITAT_REVALIDATION_SCALE_BENCHMARK_V0_1.json** defines the v0.1 profile names and parameters. The [contract workflow](https://github.com/Luminous-Dynamics/symtropy/blob/research/whole-property-habitat-benchmark-v0-1/.github/workflows/habitat-revalidation-scale-contract.yml) checks the matrix shape and wording about unmeasured targets; it does not run the search algorithm or produce performance results. The deterministic materializer in [tools/habitat-revalidation-scale](https://github.com/Luminous-Dynamics/symtropy/tree/research/whole-property-habitat-benchmark-v0-1/tools/habitat-revalidation-scale) now emits exact workload bytes and a sidecar SHA-256 receipt pinned to the matrix digest, seed, generator version, and source commit. Its [dedicated workflow](https://github.com/Luminous-Dynamics/symtropy/blob/research/whole-property-habitat-benchmark-v0-1/.github/workflows/habitat-revalidation-scale-generator.yml) regenerates every frozen profile twice and compares bytes. Neither workflow runs the downstream revalidation search or produces performance results.

The profiles intentionally stress different failure modes:

- **micro_oracle:** small enough for an independent brute-force oracle; the full Pareto frontier must match exactly.
- **deep_chain:** long dependency chains reveal repeated closure work and dependency-order mistakes.
- **shared_portfolio:** multiple targets share reviews and inspections; shared work must be counted once.
- **wide_fanout:** many available tasks and gates stress branching and state budgets.
- **large_mostly_irrelevant:** large evidence universe with a small reachable repair cone measures indexing and pruning rather than rewarding irrelevant graph bulk.
- **dominance_stress:** many trade-offs and dominated alternatives expose Pareto-maintenance cost.
- **budget_exhaustion:** a deliberately tight expansion budget verifies truthful partial results before and after a feasible candidate is found.

A profile may be marked SEARCH_INCOMPLETE by design. The expected result is not always a completed frontier; it is always a truthful status and a valid, explicitly provisional known-plan set.

## 4. Correctness and fail-closed gates

Release gates for any future scalable algorithm:

1. **Oracle agreement:** 100% exact Pareto-frontier agreement on every completed micro-oracle case, including generated permutations of equivalent input ordering.
2. **No false certainty:** zero reports of OPTIMAL_EXACT or NO_VALID_PLAN_PROVEN when search ended because of a state/time/memory budget.
3. **Plan soundness:** 100% of returned plans pass an independent feasibility checker; a partial candidate is not a proof that the frontier is complete.
4. **Determinism:** same bytes, implementation commit, toolchain, and config produce byte-identical canonical reports, excluding explicitly segregated timing fields.
5. **Adversarial validation:** malformed or contradictory graphs, unknown fields, missing gates, dependency bypasses, cycles, duplicate IDs, invalid-validity claims, arithmetic overflow, and zero/one-state budgets fail closed.
6. **No false negative:** on exhaustive small instances, every feasible plan is accounted for in the final nondominated frontier; any budget-truncated case must be labelled incomplete.

Never weaken the micro-oracle, invalid-input cases, or negative controls to improve scale numbers. Run both the reference exhaustive implementation and the candidate algorithm on the same small corpus.

## 5. Measurements and reporting

For each profile, run 5 warm-ups and at least 30 measured trials per workload/configuration for p50/p95, and retain every raw trial. Report median (p50), p95, minimum/maximum, and sample count. Report p99 as a formal percentile only with at least 1,000 measured trials for that exact workload/configuration; otherwise mark p99 unavailable, report the observed maximum, and state the sample count. Do not imply that the maximum from 30 runs is a reliable p99 estimate. For small deterministic cases, also report work counters because wall-time noise may dominate.

Report at least:

- latency per phase and end-to-end;
- peak resident memory and serialized input/report sizes;
- expanded/generated/pruned/duplicate states and dominance comparisons;
- reachable versus total artifacts/tasks;
- returned plan count, frontier count, and whether the frontier is complete;
- oracle agreement and independent plan-validator results;
- incomplete-result rate by budget/profile;
- worst-case latency and memory, not only averages.

Pin CPU governor/affinity where practical and disclose any constraints. Report both debug-check builds and release builds only when they are clearly separated. The first run establishes a baseline; it is not an improvement claim.

## 6. Proposed engineering targets (not measured facts)

Use the following as provisional targets for the first native release-build baseline on a documented developer-class machine; adjust only after publishing baseline data and explaining the trade-off.

| Workload / property | Initial target |
|---|---|
| Small exhaustive oracle suite | 100% exact frontier agreement |
| Invalid/contradictory input corpus | 100% rejected or explicitly represented as non-actionable; never silently accepted |
| Budget exhaustion | 100% correctly reported as SEARCH_INCOMPLETE |
| 10,000-artifact graph, at most 64 tasks | p95 structural decode + validation ≤ 1.0 s |
| Search capped at 100,000 expanded states | p95 search wall-time ≤ 2.0 s; budget stopping must remain truthful |
| Memory under the 100,000-state cap | ≤ 512 MiB peak RSS |
| Repeatability | 100/100 fixed-seed runs produce the same canonical non-timing report |
| Independent plan feasibility checker | 100% agreement on every emitted candidate |

These are **engineering targets to test**, not results already obtained. Treat performance thresholds as provisional until the baseline is run on specified hardware. In particular, 64-task instances can remain combinatorially hard; hitting the cap is acceptable if and only if the report says the search is incomplete.

## 7. Do not conflate planning objectives

The current portfolio prototype uses invented additive cost and duration units and serial task-order enumeration. Those values are not construction estimates or calendar makespan. A realistic schedule requires explicit task precedence, overlap, crew/equipment/resource constraints, procurement lead times, calendars, and risk/uncertainty assumptions.

Keep the current additive micro-benchmark for algorithm regression. Add realistic scheduling as a separately versioned workload and model, with independent feasibility checks. Do not silently reinterpret legacy duration values as elapsed project time.

Likewise, a task's existence is not evidence that work ran, and a model-generated recommendation is not an inspection, engineering sign-off, code approval, or proof of physical conformity.

## 8. Scale-out sequence

1. Freeze profile definitions and generate exact workload bytes + SHA-256 receipts.
2. Run the independent oracle against all small generated cases.
3. Record baseline phase timings, peak memory, state counts, and incomplete rates.
4. Add candidate search/indexing optimizations behind the same report contract.
5. Differential-test every completed result against the oracle and independently validate every candidate plan.
6. Publish before/after results only at matching workload digests and comparable hardware/toolchains.
7. Expand artifact and task scales only after the previous tier's correctness gates hold.

This benchmark contract is intentionally conservative: a truthful partial answer is more useful than a fast, unauditable assertion of optimality.

## 9. Materialized workload inputs and provenance receipts

The standalone Rust crate `tools/habitat-revalidation-scale` materializes each named profile to canonical pretty-printed JSON and emits a separate receipt. The receipt contains the exact workload SHA-256 and byte length, exact profile-matrix SHA-256, generator version, source commit, profile ID, seed, and materialized counts. The CLI fails closed when the profile is unknown, the matrix includes an unknown top-level/profile field, resource dimensions exceed generator limits, or the source commit is not an exact 40-character hexadecimal SHA.

Every output graph is checked for duplicate IDs, unknown or repeated dependencies, dependency cycles, task/output dependency mismatch, undeclared gates, target reachability, and agreement between the generated objective alternatives and a quadratic pairwise Pareto oracle. The objective-alternative probe deliberately includes nondominated trade-offs and dominated controls.

This is a reproducibility milestone, not a search benchmark. It does **not** execute the current portfolio planner, solve a construction schedule, or claim that search complexity scales with artifact count. The generator workflow is designed to verify byte-for-byte repeatability and receipts on the seven frozen profiles. Generated files live in the workflow's temporary directory unless separately retained; the workflow is not a performance-results publication mechanism.
