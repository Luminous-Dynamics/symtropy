# SAFRB lexicographic MCF transformation — tiny conformance experiment

**Status:** exploratory research artifact; not production code  
**Base planner revision:** `e8772749c8224a164d5214c4622e48d24b122f76` (SAFRB v0.3)  
**Reference library:** NetworkX 3.6.1  
**Run:** `python oracle.py`

## Question

Can the SAFRB objective

1. maximize total service;
2. lexicographically maximize service by priority tier;
3. minimize route cost after all service-tier totals are fixed,

be translated into ordinary fixed-supply minimum-cost-flow passes without using an arbitrary Big-M scalar?

## Transformation exercised

1. Compute maximum total service (F^*) on the original network.
2. Fix source supply to (-F^*) and sink demand to (+F^*), using NetworkX's demand convention (negative = supply, positive = demand).
3. For each priority tier in order, solve a min-cost-flow pass with unit cost (-1) on that tier's service gate and zero incremental costs elsewhere. This maximizes that tier's flow while total flow and previously frozen tier totals remain fixed.
4. Convert each frozen service-gate flow into an exact lower/upper bound. For the reference solvers that expose only capacities, the script applies the standard lower-bound-to-node-balance transformation:
   - residual capacity = upper - lower;
   - demand at the arc tail increases by lower;
   - demand at the arc head decreases by lower.
5. With all service-tier totals fixed, solve a final min-cost-flow pass using the original route costs.

This avoids Big-M weighting. It is an exact transformation for the bounded network class exercised here, provided lower-bound conversion and every intermediate result are independently checked.

## Experiment

- 100 deterministic seeds.
- 6 nodes and 8 directed arcs per instance.
- Two priority tiers.
- Integer capacities from 1 to 2.
- Non-negative integer route costs from 0 to 7.
- Independent exhaustive enumeration of every integral arc-flow assignment is the oracle.
- Both NetworkX 3.6.1 algorithms were run against the same instances:
  - `capacity_scaling` on a MultiDiGraph;
  - `network_simplex` on a DiGraph.

Observed result in the recorded execution:

| Implementation | Exact objective-tuple parity | Median | P95 | P99 |
|---|---:|---:|---:|---:|
| NetworkX capacity_scaling | 100/100 | 0.805 ms | 1.374 ms | 2.377 ms |
| NetworkX network_simplex | 100/100 | 0.458 ms | 0.799 ms | 1.713 ms |

The tuple compared is maximum total flow, negative service volume for each priority tier in priority order, then route cost. Tied but different optimal flow assignments are accepted. The harness also independently checks the final arc witness for exact arc coverage, lower/upper bounds, fixed total flow, and intermediate-node conservation; each previously optimized priority-gate amount must remain unchanged in the final cost pass.

## Limits — do not overclaim

This is a tiny correctness experiment for the **benchmark transformation**, not a benchmark of the Rust SAFRB planner. It does not establish solver speed superiority, field validity, physical capacity, evidence authority, service delivery, or readiness for large instances.

Not exercised:
- Rust implementation / portable primal-dual certificate;
- DIMACS core MCF instances or LEMON CostScaling / NetworkSimplex;
- OR-Tools SimpleMinCostFlow;
- lower bounds originating in the physical problem (the experiment uses lower bounds only to freeze earlier priority gates);
- parallel arcs for NetworkSimplex;
- multi-commodity, shared-resource aliases, process coupling, discrete operations or stochastic uncertainty;
- real inventory, actual asset capacities or owner-authoritative records.

Use this artifact as a small independent regression oracle for the sequential objective encoding, then extend to larger seeded and held-out instances in #7710. Never treat the experimental timings as an industry leaderboard.

## Reproduction

```sh
python -m pip install -r requirements.txt
python oracle.py
```

The fixed seed family and exact instance generation are in `oracle.py`. The result is an exploratory receipt; rerun on the target environment before making a new performance claim.
