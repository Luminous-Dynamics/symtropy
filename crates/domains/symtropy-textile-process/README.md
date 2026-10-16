# symtropy-textile-process

Dependency-free primitives for the first Textile Process Optimization Benchmark slice.

This crate is intentionally not a factory simulator or process controller. It evaluates already-declared candidate objective values and evidence-bearing hard gates. It does not predict quality, calculate energy/water use, authenticate evidence, perform lifecycle assessment, schedule actual process operations, or control equipment.

## Run locally

```sh
cargo test -p symtropy-textile-process
cargo run -p symtropy-textile-process --example textile_tradeoff
```

The example uses synthetic values only. It demonstrates that two feasible candidates can remain on a multi-objective Pareto frontier while a numerically attractive candidate is excluded after a hard environmental-admission failure.

## Core contracts

- The caller declares at least two objectives with a stable ID, exact unit string, integer scale, and minimize/maximize direction.
- Objective values use integer ticks. Unit/scale mismatches are rejected; no implicit unit conversions or floating-point tie-breaking occur.
- Every candidate must supply at least one hard gate. Gate outcomes are evidence-bearing `Pass`, evidence-bearing `Fail`, or `Unknown` with a non-empty reason.
- Any failed gate blocks a candidate. If no gate fails but at least one is unknown, the candidate is incomplete. Neither can enter the Pareto frontier.
- Feasible candidates need a value and evidence reference for every declared objective.
- Input IDs, gate IDs, objective IDs, units, scales and value dimensions are validated. Results are sorted deterministically and each dominated candidate receives a lexicographically deterministic dominance witness.
- `MassBalance` checks exact integer-milligram closure at one declared process boundary with mutually exclusive material fates and checked arithmetic.

Evidence references are identifiers, not authentication. The calling system must verify source provenance, validity, calibration, independence and domain qualification before any real-world admission. The evaluator intentionally cannot turn untrusted IDs into proof.

## Not yet implemented

A process-operation model, resource-constrained schedule simulator, incumbent dispatch baselines, factory-data connectors, objective uncertainty propagation, lifecycle-impact calculator and Mycelix evidence receipt are future steps. The authored unit tests include a seeded independent exhaustive Pareto oracle, but no test execution or CI pass is claimed until exact-head checks finish.
