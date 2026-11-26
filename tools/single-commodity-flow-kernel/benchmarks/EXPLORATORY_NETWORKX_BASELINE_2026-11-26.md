# Exploratory NetworkX max-flow baseline — 2026-11-26

**Status:** local exploratory measurement; not a Rust-kernel benchmark, SOTA claim, or qualification result.

## Environment

- OS/kernel: Linux x86_64, kernel 6.18.57
- CPU exposed to the container: 3 vCPUs, AMD EPYC 9V74
- Memory visible: approximately 5.8 GiB total, 4.7 GiB available at measurement
- Python: 3.13.5
- NetworkX: 3.6.1
- OR-Tools: not installed
- Rust toolchain: `cargo`, `rustc` and `rustfmt` unavailable

These are container-visible resources, not a bare-metal hardware qualification.

## Method

Run `python3 tools/single-commodity-flow-kernel/benchmarks/networkx_baseline.py`.

All graphs are directed simple graphs, with integer capacities uniformly sampled from 1 to 1000 and a serial source-to-sink backbone. A fixed seed is derived as `20261126 + node_count + len(kind)`. The sparse generator uses edge probability `4/n`; dense uses `12/n`; layered uses adjacent-layer probability 0.12 plus the serial backbone. Parallel arcs are not represented by NetworkX `DiGraph` and are excluded from this comparison.

Algorithms and repetitions:
- `preflow_push`: 5 runs per case
- `dinitz`: 5 runs per case
- `edmonds_karp`: 3 runs per case

The script asserts exact max-flow value agreement across algorithms. It reports median, minimum and maximum wall-clock time. These are small-sample exploratory timings, not p95/p99 claims.

## Results

| Graph | Nodes | Edges | Preflow-push median | Dinitz median | Edmonds-Karp median | Flow |
|---|---:|---:|---:|---:|---:|---:|
| Sparse | 100 | 475 | 2.383 ms | 1.479 ms | 1.044 ms | 1,208 |
| Sparse | 400 | 1,955 | 8.215 ms | 17.607 ms | 5.330 ms | 2,762 |
| Sparse | 1,000 | 5,075 | 33.149 ms | 18.682 ms | 10.603 ms | 590 |
| Dense | 400 | 5,127 | 12.819 ms | 23.165 ms | 10.936 ms | 3,380 |
| Dense | 1,000 | 12,745 | 41.581 ms | 91.871 ms | 33.165 ms | 7,699 |
| Layered | 1,000 | 11,830 | 35.376 ms | 80.623 ms | 144.290 ms | 4,334 |

All three algorithms agreed on all six max-flow values. Separate independent minimum-cut checks on the reverse-residual rerouting and joint-vs-individual counterexamples are included in the research notes.

## Interpretation

1. There is no universally fastest algorithm across these small cases. Dinitz led some sparse cases, while Edmonds-Karp had the lowest medians on several other generated cases. The layered case favored preflow-push.
2. Maximum and median values show noticeable run-to-run outliers in some cases. Container scheduling and small sample sizes limit the strength of any timing conclusion.
3. Edmonds-Karp is useful as a small-instance cross-check, not a default scale choice.
4. These numbers are a baseline for future same-host comparisons. They do not show whether the custom Rust Dinic implementation is faster or slower.
5. Next comparison must add OR-Tools `SimpleMaxFlow`, run the new Rust kernel and independent certificate verifier on identical graph files, pin exact versions, increase repetitions, include warm-up, and report p50/p95/p99 plus peak resident memory.
6. Parallel-edge cases require a separate identity-preserving benchmark track. NetworkX max-flow does not accept `MultiDiGraph`; do not coalesce parallel arcs and then compare edge-identity witnesses.

## Reproduction and limits

The script is research-only and uses NetworkX as an independent baseline. It is not imported by production crates and adds no runtime dependency to Symtropy. The exact generator and run counts are encoded in the script; the table above records one observed run on the environment listed above.
