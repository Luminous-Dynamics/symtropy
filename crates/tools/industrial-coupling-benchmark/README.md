# Industrial Coupling Benchmark

A small, dependency-free Rust reference evaluator for a deliberately narrow part of INDUSTRIAL-COUPLING-001: economic and physical screening of heat exchanged between two facilities.

## Status and claim ceiling

- This is a **synthetic, discrete, constant-temperature screening model**.
- It is not a process-synthesis tool, heat-exchanger-network solver, dynamic simulator, plant controller, or field-performance study.
- A verifier pass certifies only the modeled constraints and accounting in this crate.
- The source and sink temperatures are represented by one constant temperature each. A real process needs temperature profiles and pinch/heat-transfer analysis.
- The optimizer enumerates integer-kWh inputs, making the optimum exact only over this finite discrete model. It refuses a search domain above the configured bound rather than silently performing unbounded work.
- This crate uses no external dependencies and is its own Cargo workspace so it can be tested without building the full Symtropy workspace.

## What it models

An industrial source facility has surplus heat. A sink facility has useful heat demand. A possible link has limited capacity, conversion loss, a variable operating cost, and a fixed coordination cost. Transferring heat is permitted only when the simplified temperature-grade screen passes.

For each feasible integer-kWh transfer quantity, the evaluator calculates:

- heat delivered and transfer loss;
- avoided utility expense at the sink;
- source heat opportunity cost;
- real variable and fixed exchange costs;
- net network surplus;
- an explicitly declared split of positive network surplus between the two participants, after satisfying each participant's minimum reservation benefit.

Internal participant allocation is not added again as a social cost. The independent verifier recomputes the ledger and rescans the finite candidate domain to reject tampering, infeasibility, or non-optimal results. Zero exchange is always the baseline and is retained when the link is incompatible or destroys value.

All costs use **milli-credits** per stated unit; these are synthetic accounting units, not USD or any actual tariff.

## Finite-scenario robust evaluation

The crate also exposes `evaluate_robust` and `verify_robust_plan`. They select one **common transfer quantity** that must be feasible and meet both facilities' participation floors in every supplied scenario. The primary objective maximizes worst-case network surplus; ties between positive worst-case plans prefer larger total surplus and then a smaller exchange. A zero-transfer baseline wins ties at zero, avoiding coordination with no robust benefit.

The robust evaluator rejects empty or duplicate scenario sets and applies explicit bounds of 64 scenarios and 2,000,000 scenario evaluations. It independently rescans the feasible integer decision domain rather than trusting the optimizer's reported aggregate. The CLI includes a two-scenario synthetic case whose conservative result is 44,200 milli-credits worst-case surplus at 42 kWh input.

This is **finite-set robust optimization**, not a probability-weighted expected-value model or an adaptive recourse policy. All listed scenarios must admit the same transfer amount. Future work can compare adaptive policies, calibrated distributions, CVaR/regret objectives, and recourse against this conservative baseline.


## Run offline

From this directory:

```sh
cargo test --offline --manifest-path Cargo.toml
cargo run --offline --manifest-path Cargo.toml
```

Or from the repository root:

```sh
cargo test --offline --manifest-path crates/tools/industrial-coupling-benchmark/Cargo.toml
cargo run --offline --manifest-path crates/tools/industrial-coupling-benchmark/Cargo.toml
```

Expected demo properties:

- `IC-001-POSITIVE-SYNTHETIC`: 45 kWh input, 40 kWh delivered, 5 kWh loss, 85,500 milli-credits network surplus under the stipulated assumptions.
- `IC-001-NEGATIVE-GRADE-SYNTHETIC`: zero exchange because the source temperature does not meet the sink requirement plus minimum approach.
- `IC-001-NEGATIVE-ECONOMICS-SYNTHETIC`: zero exchange because the transfer is more expensive than the avoided utility expense.
- `IC-001-TWO-SCENARIO-SYNTHETIC`: the common robust decision transfers 42 kWh and reports 44,200 milli-credits worst-case surplus across the nominal and conservative fixtures.

These are exact arithmetic outputs for the authored synthetic fixtures, not evidence of real-world energy savings.

## Verification gates implemented

- scenario identifier, efficiency and allocation-share validation;
- strict source, sink and link capacity checks;
- explicit temperature-grade precondition;
- integer energy balance: input = delivered + transfer loss;
- recomputed cost and net-surplus accounting;
- participant allocation sums exactly to positive surplus and meets both reservation floors;
- independent finite-domain scan for optimality;
- checked arithmetic with overflow rejection;
- search bound fail-closed behavior;
- negative controls for grade incompatibility, uneconomic exchange, zero capacity and unmet participation floors, plus tamper tests.
- finite-scenario adverse-condition fallback and robust aggregate tamper tests.

## Next steps

1. Add more hand-checkable fixtures and property-based conservation checks.
2. Reproduce a pinned, public multi-period process-integration reference case in a supported subset before claiming parity.
3. Extend into time-indexed storage, variable temperature streams, demand windows and outage scenarios only after the appropriate process models are independently qualified.
4. Compare independent and joint optimization at matched information and compute budgets.
5. Attach versioned Sol-Atlas capacity/material evidence and Mycelix execution/provenance records through explicit adapters—without pulling a distributed runtime into the offline reference evaluator.
6. Add Symthaea-guided candidate exploration only as an ablation/tournament arm against strong conventional optimization.
