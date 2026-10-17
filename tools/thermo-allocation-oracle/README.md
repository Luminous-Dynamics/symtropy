# Thermodynamic allocation oracle: finite-grid fairness and regret v0.1

## Purpose

This dependency-free Rust tool makes the overlapping-well failure mode measurable.
It is an exhaustive finite-grid oracle for tiny instances, not a replacement for
the production allocator, not a continuous rational LP solver, and not a proof
that a floating-point implementation refines a mathematical model.

Run from the repository root:

    cargo test --manifest-path tools/thermo-allocation-oracle/Cargo.toml
    cargo run --manifest-path tools/thermo-allocation-oracle/Cargo.toml -- --audit

## Exact counterexample

| Entity | Supply / cap | Eligible wells |
|---|---:|---|
| Well 10 | 12 quanta | agents 1 and 2 |
| Well 20 | 12 quanta | agents 1 and 3 |
| Agent 1 | shared cap 9 quanta | both wells |
| Agent 2 | cap 12 quanta | well 10 only |
| Agent 3 | cap 12 quanta | well 20 only |

One quantum is exactly one third of the original amount. The original supplies
4,4 and caps 3,4,4 become integer supplies 12,12 and caps 9,12,12. The exact
joint optimum assigns 8 quanta to each agent (8/3 original units each):
well 10 sends 4 to agent 1 and 8 to agent 2; well 20 sends 4 to agent 1 and
8 to agent 3.

The objective is lexicographic max-min fairness on total accepted amount:
sort agent totals from smallest to largest and maximize the sorted vector
lexicographically. Ties are broken by the lexicographically smallest canonical
edge matrix (well IDs, then agent IDs). This is a baseline, not a claim that
raw amount is the universally correct fairness unit. Demand-normalized service,
urgency, priority classes, and utility-weighted objectives are separate policy
choices and should be reported as separate candidates.

## Compared policies and metrics

- **Exhaustive joint oracle:** enumerates every feasible integer-grid assignment
  for at most 3 wells, 4 agents, and 12 supply quanta per well.
- **Sequential shared-cap greedy:** processes each well to completion in turn,
  enforcing remaining shared caps. It is feasible but can be order-sensitive.
- **Independent-source negative control:** each well resets its local recipient
  ledger. It deliberately exposes cross-well cap violations.
- **Metrics:** sorted leximin profile, first rank of divergence from the oracle,
  L1/max-agent order sensitivity, source/cap/eligibility feasibility, and search
  node/leaf counts.

## Fixture acceptance results

| Check | Expected |
|---|---:|
| Joint oracle profile | [8, 8, 8] |
| Oracle capacity violations | 0 |
| Sequential, well order 10 then 20 | agent totals [9, 6, 9] |
| Sequential, well order 20 then 10 | agent totals [9, 9, 6] |
| Sequential permutation L1 delta | 6 quanta |
| Maximum per-agent permutation delta | 3 quanta |
| Independent-source global cap violation | 3 quanta |
| Oracle invariant to input-vector permutation | yes |

The independent-source negative control must continue to reveal its violation;
do not weaken the fixture to make it pass. These exact-grid checks are not
performance results.

## Next benchmark targets

1. Exhaust all small bipartite eligibility graphs within an explicit search
   budget; add disconnected graphs, repeated caps, asymmetric supplies, zero
   capacity/supply, and permuted inputs.
2. Compare sequential, independent, weighted max-min, and joint policies.
   Report feasibility violations separately from fairness regret and utilization.
3. Benchmark 10, 100, 1,000, 10,000, and 100,000 edges; capture p50/p95/p99,
   throughput, memory, termination, source SHA, Rust toolchain/profile, CPU,
   and deterministic workload seed.
4. Target zero source/cap/eligibility violations, zero mismatches against the
   exhaustive oracle on all enumerable cases, permutation invariance for
   policies advertised as order-invariant, and at least 95% mutation score with
   surviving mutations disclosed.
5. For approximate solvers, publish a proven regret bound when available.
   Otherwise label the result as empirical; do not call it a guarantee.

## Limitations

- Exactness is only for the finite integer grid. This does not prove existence
  or completeness of the continuous waterline finder or refine the f64 code.
- The fixture is not evidence of production performance, and no ECS integration
  is attempted.
- Spatial eligibility (including 2D-truncated positions and strict distance
  less than radius) and source debit/recipient credit accounting remain separate
  invariants that must not be weakened.
- A queued CI check is not a pass; evidence must name the exact tested head.

## Research basis

- Namyar et al., *Solving Max-Min Fair Resource Allocations Quickly on Large
  Graphs* (NSDI 2024): generalized multi-path water-filling/optimization and
  bounded-unfairness performance trade-offs. https://arxiv.org/abs/2310.09699
- Ogryczak et al., *Fair Optimization and Networks* (2014): lexicographic
  maximin fairness and constrained equitable outcomes.
  https://onlinelibrary.wiley.com/doi/10.1155/2014/612018
