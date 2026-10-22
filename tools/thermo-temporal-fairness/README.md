# Thermodynamic temporal fairness evaluator v0.1

A single-tick fairness metric can hide systematic disadvantage over time. This
standalone, dependency-free Rust crate evaluates allocation traces across a
finite horizon. It is a measurement tool, not an allocator, source/cap
feasibility checker, or proof of optimality.

## Paradigm shift: fairness has both slot and horizon semantics

A one-tick sorted service profile cannot say whether deprivation is temporary,
rotating, or permanent. For example, two policies can each give one of two
eligible recipients all available supply per tick. Their worst per-tick
service ratio is zero in both cases. Yet rotating service can produce equal
horizon service, while repeatedly serving the same recipient leaves the other
with a zero horizon ratio and a two-tick starvation streak.

This evaluator reports those dimensions separately rather than folding them
into one scalar:

- **Horizon service profile:** sorted exact fractions of total allocation over
  eligible demand across the trace. Ticks where an agent is ineligible do not
  count toward its allocatable-demand denominator.
- **Worst eligible slot ratio:** minimum allocation/demand fraction among
  eligible agent-tick pairs with positive demand.
- **Starvation exposure:** per-agent starved tick count and maximum consecutive
  eligible-demand ticks receiving zero service.
- **Demand coverage:** total demand/allocation/unmet demand and separate
  eligible-demand, eligible-allocation, and eligible-unmet totals per agent.
- **Input validity:** reject duplicate agents, jagged time series, allocations
  exceeding demand, and positive allocation while ineligible.
- **Determinism:** canonical report order independent of input agent order;
  rational ordering uses a continued-fraction comparator rather than
  multiplication, avoiding overflow in ratio comparison.

An agent with zero eligible demand is excluded from horizon ratios; it is not
silently assigned a 0/0 value. Demand during an ineligible tick remains visible
in total unmet demand but does not falsely look like allocatable demand when
comparing the allocation policy. A tick with no eligibility or no positive
eligible demand breaks a starvation streak. This semantics is explicit and testable.

## Run

From repository root:

    cargo fmt --manifest-path tools/thermo-temporal-fairness/Cargo.toml -- --check
    cargo test --manifest-path tools/thermo-temporal-fairness/Cargo.toml
    cargo run --manifest-path tools/thermo-temporal-fairness/Cargo.toml -- --audit tools/thermo-temporal-fairness/fixtures/alternating.tsv
    cargo run --manifest-path tools/thermo-temporal-fairness/Cargo.toml -- --audit tools/thermo-temporal-fairness/fixtures/monopoly.tsv

Trace format is tab-separated UTF-8:
- header: THERMO-TEMPORAL-TRACE<TAB>1
- one CASE row
- one OBS row per tick and agent with fields: tick, agent_id, demand, eligible, allocated
- ticks are zero-based and dense; every agent has exactly one observation at every tick.
- eligible is exactly true or false; all numeric values are unsigned 64-bit integers.

The CLI exits 0 for a valid trace, 1 for semantic trace violations, and 2 for
file or format errors. The CLI rejects sparse traces and more than 1,000,000 ticks.
It does not infer source supplies, graph eligibility, or an allocation oracle's optimality from a time series.

## Acceptance targets

These are proposed qualification targets, not current measured results.

1. Zero accepted invalid traces for allocation-above-demand and ineligible positive-credit controls.
2. Exact agreement between per-agent totals and independent recomputation for every corpus trace.
3. Zero output changes from irrelevant agent-order permutations.
4. The alternating fixture must report horizon service profile [1/2, 1/2].
5. The monopoly negative control must report [0/1, 1/1], two starved ticks, and a maximum two-tick starvation streak for the disadvantaged agent.
6. Add generated demand patterns with bursty/intermittent eligibility, asymmetric demand, repeated recipients, and horizon lengths from 1 to 100,000 ticks.
7. Measure p50/p95/p99 evaluator latency and memory separately from allocator runtime. Include trace SHA, agent count, horizon length, machine and toolchain.
8. Report slot fairness and horizon fairness separately. Do not convert either into a universal fairness claim without application-level assumptions.

## Suggested research use

Feed the same recorded decisions from sequential and joint allocation policies into this evaluator. Then compare it alongside the exact small-instance fairness-regret oracle and the independent allocation-certificate checker. The oracle answers whether a tick's decision matches the reference optimum in an enumerable case, the certificate checker answers whether an allocation was feasible and accounted for, and this evaluator answers who experiences repeated deprivation across time.

This separation is aligned with research that distinguishes per-slot fairness from horizon fairness in dynamic resource allocation, and with systems such as Karma that introduce allocation memory for dynamic demands. Those publications motivate evaluation dimensions; they do not imply that the same policies or guarantees apply unchanged to Symtropy.

References:
- Si Salem, Iosifidis & Neglia, Enabling Long-term Fairness in Dynamic Resource Allocation (SIGMETRICS Performance Evaluation Review, 2023): https://doi.org/10.1145/3606376.3593541
- Vuppalapati et al., Karma: Resource Allocation for Dynamic Demands (OSDI 2023): https://www.usenix.org/conference/osdi23/presentation/vuppalapati
- Namyar et al., Solving Max-Min Fair Resource Allocations Quickly on Large Graphs (NSDI 2024): https://www.usenix.org/conference/nsdi24/presentation/namyar-solving
