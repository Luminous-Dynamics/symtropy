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

The tool compares two explicit leximin objectives over the *same feasible
allocation graph*:

- **Absolute amount:** sort accepted amounts from smallest to largest and
  maximize that vector lexicographically. This produces totals [8, 8, 8].
- **Capacity-normalized amount:** compare exact fractions amount/cap, sort them
  from smallest to largest, and maximize that ratio vector lexicographically
  using integer cross-multiplication (no floating-point division). This
  produces totals [7, 9, 8], with sorted normalized profile [2/3, 3/4, 7/9].

Ties within an objective are broken by the lexicographically smallest canonical
edge matrix (well IDs, then agent IDs). The normalized objective is appropriate
only when each cap really represents the recipient's demand/service target. If
the cap is merely a physical safety ceiling, it must not be silently reused as
need; add an explicit need/weight field and compare that policy separately.
Neither objective is universally correct. The point is to make the policy
choice visible and benchmark its consequences instead of hiding it inside the
allocator.

## Compared policies and metrics

- **Exhaustive joint oracle:** enumerates every feasible integer-grid assignment
  for at most 3 wells, 4 agents, and 12 supply quanta per well.
- **Sequential shared-cap greedy:** processes each well to completion in turn,
  enforcing remaining shared caps. It is feasible but can be order-sensitive.
- **Independent-source negative control:** each well resets its local recipient
  ledger. It deliberately exposes cross-well cap violations.
- **Policy metrics:** sorted absolute profile, exact-rational capacity-normalized
  profile, first rank of divergence from the selected objective's oracle,
  L1/max-agent order sensitivity, aggregate cap violation units, and search
  node/leaf counts.

## Fixture acceptance results

| Check | Expected |
|---|---:|
| Absolute-amount oracle totals/profile | [8, 8, 8] |
| Capacity-normalized oracle totals by agent ID | [7, 9, 8] |
| Capacity-normalized sorted exact-ratio profile | [2/3, 3/4, 7/9] |
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
