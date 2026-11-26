# Self-pair traversal comparison — 2026-11-26

## Reproduction identity

- Repository revision: `c58fbc303084ffba05900ad52b90ca9a94b8660c`
- Workflow run: [45861156185](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45861156185)
- Raw artifact (CSV + hardware/compiler manifest): [broadphase-tournament-45861156185-1](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45861156185/artifacts/15137139997)
- Rust/Cargo: 1.96.0
- Host: GitHub-hosted Azure VM, AMD EPYC 7763 64-Core Processor, 4 logical CPUs assigned
- Release campaign: 128 declared proxies, 40 temporal frames, 2 repetitions, six scene families, D=2/3/4
- Rows: 108 = 6 families × 3 dimensions × 6 backends
- Correctness: 0 false positives, 0 false negatives and 0 mismatching frames in all 108 rows

The oracle is precomputed before backend timing. Each backend times per-frame update/query/candidate generation and current-envelope refinement; parity comparison occurs after its timer stops. Campaign wall time additionally includes orchestration and verification, so the per-frame p50/p95 samples and summed timed-frame duration are the primary backend measurements.

## Paired traversal result

Both query methods use the same incremental-tree implementation, insertion/update/removal sequence and balancing policy. The per-leaf control issues one general tree query per proxy. The self-pair method starts one root-root traversal and decomposes diagonal branches into left-left, left-right and right-right.

| Tree policy | Per-leaf timed sum | Self-pair timed sum | Reduction | Workloads improved |
|---|---:|---:|---:|---:|
| Unbalanced | 121.723 ms | 86.580 ms | **28.87%** | 18 / 18 |
| Height-rotation balanced | 130.727 ms | 93.249 ms | **28.67%** | 18 / 18 |

The per-frame p95 sums over the 18 family/dimension workloads fell by 31.05% for the unbalanced tree and 28.48% for the balanced tree.

| Tree policy | AABB pair tests, per-leaf | AABB pair tests, self-pair | Reduction |
|---|---:|---:|---:|
| Unbalanced | 5,124,646 | 1,144,914 | **77.66%** |
| Height-rotation balanced | 4,674,778 | 1,168,566 | **75.00%** |

Node visits also fell by 68.50% (unbalanced) and 54.33% (balanced). Pair populations were identical to the independent brute-force AABB oracle; the traversal change did not drop contacts/candidates.

## Whole-portfolio context

| Backend | Sum of timed frame work |
|---|---:|
| Sweep-and-prune | 14.344 ms |
| Brute force | 15.430 ms |
| Self-pair, unbalanced tree | 86.580 ms |
| Self-pair, balanced tree | 93.249 ms |
| Per-leaf, unbalanced tree | 121.723 ms |
| Per-leaf, balanced tree | 130.727 ms |

The self-pair implementation is a substantial improvement over its own per-leaf predecessor, but remains approximately 6.04× (unbalanced) and 6.50× (balanced) slower than sweep-and-prune in this particular workload/corpus. Do not describe it as the fastest broadphase or as an industry/SOTA win.

## Interpretation and next hypotheses

1. The original tree's repeated full traversals were a material cost. A single self-pair walk removes most of those AABB tests.
2. Balancing still costs rotations (141,134 total in each balanced run) and has higher total time than the unbalanced tree in this corpus. It may still have value for tail height/latency or adversarial churn; that needs larger/held-out sequences and p99/height correlation.
3. SAP remains the strongest CPU AABB backend in this test. Next likely opportunities: a stateful SAP implementation (maintain endpoint order), an adaptive axis/algorithm selector using only past/current observations, and a tree traversal priority based on pair-potential/surface area rather than height alone.
4. Benchmark the persistent tree on workloads with high locality, static-heavy bodies and clustered temporal updates at larger populations (1k, 10k, 100k) before deciding whether to tune it further or leave it as a specialty backend.
5. These are current-frame AABB overlap workloads, not shape-level CCD, contact manifold, solver, GPU or full-tick measurements. Use [#7593](https://github.com/Luminous-Dynamics/symtropy/issues/7593) for the external NYU analytic-TOI lane and do not merge those denominators into this tournament.

This is one reproducible internal runner campaign. Repeat across at least two hardware classes and multiple seeds before considering a backend policy change.
