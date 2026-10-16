# symtropy-textile-process

Dependency-free primitives for the first Textile Process Optimization Benchmark slice.

This crate is intentionally not a factory simulator or process controller. It evaluates already-declared candidate objective values and evidence-bearing hard gates. It does not predict quality, calculate environmental impacts from first principles, authenticate evidence, perform lifecycle assessment, or control equipment. The scheduling module produces planning outputs from caller-supplied durations, resource quantities, and evidence; it is not a factory simulator or equipment controller. Job readiness gates are preconditions; finished-product condition and quality acceptance must be measured separately and are never inferred from a planned finish. Each schedule also declares one shared time-unit ID alongside the energy and water unit IDs.

## Robust stress evaluation

The stress module evaluates explicit processing-time delays, input-release delays,
and pre-horizon machine-availability shocks against both the bounded exact solver
and all four dispatch rules. The result preserves each fixture reference,
exact feasibility outcome, concrete heuristic plan (if found), profile/service
identifiers, and per-objective gaps. This is an equally enumerated stress corpus,
not an estimate of real-world disruption probabilities. Full protocol and
proposed acceptance targets: docs/ops/TEXTILE_SCHEDULE_ROBUSTNESS_BENCHMARK_V0_1.md.

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


## Scheduling

This crate includes a bounded single-stage scheduler with capability checks,
release/due windows, evidence-backed sequence-dependent setup costs, hard
energy/water budgets, deterministic dispatch baselines, and an exhaustive
Pareto oracle for tiny instances. Benchmark contract: docs/ops/TEXTILE_SCHEDULER_BENCHMARK_V0_1.md.
