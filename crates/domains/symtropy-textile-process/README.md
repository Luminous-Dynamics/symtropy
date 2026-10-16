# symtropy-textile-process

Dependency-free primitives for the first Textile Process Optimization Benchmark slice.

This crate is intentionally not a factory simulator or process controller. It
validates declared candidate values and evidence-bearing hard gates. It does not
predict quality, calculate environmental impacts from first principles,
authenticate evidence, or perform lifecycle assessment. The scheduling module
produces plans from caller-supplied durations, resource quantities, and evidence.
Job readiness gates are preconditions; finished-product condition and quality
acceptance must be measured separately and are never inferred from a planned
finish. Each schedule declares a shared time-unit ID alongside energy and water
unit IDs.

## Robust stress evaluation

The stress module evaluates processing-time delays, input-release delays,
utility-budget curtailment, and pre-horizon machine unavailability against the
bounded exact solver and all four dispatch rules. The result preserves each fixture reference,
exact feasibility outcome, concrete heuristic plan (if found), profile/service
identifiers, and per-objective gaps. This is an equally enumerated stress corpus,
not an estimate of real-world disruption probabilities. Full protocol and
proposed acceptance targets: docs/ops/TEXTILE_SCHEDULE_ROBUSTNESS_BENCHMARK_V0_1.md.

## Run locally

```sh
cargo test -p symtropy-textile-process
cargo run -p symtropy-textile-process --example textile_tradeoff
```

The example uses synthetic values only. It demonstrates that feasible candidates
can remain on a multi-objective Pareto frontier while a numerically attractive
candidate is excluded after a hard environmental-admission failure.

## Demand and surplus evaluation

The demand module compares five policies on the same scenario: made-to-order,
preorder plus explicit buffer, order-driven small batch, forecast-driven batch,
and reuse-existing-inventory-first. It keeps production, accepted/rejected
output, fulfilled demand, stockout, unsold new output and resource inputs as
separate integer ledgers. It uses order/forecast data to make the plan and
realized demand only for outcome accounting.

Policy results are stratified by scenario, garment profile and functional unit;
the module does not pool unlike service units. The paired fixture and benchmark
contract live at docs/ops/fixtures/textile_demand_surplus_v0_1/ and
docs/ops/TEXTILE_DEMAND_SURPLUS_BENCHMARK_V0_1.md. The JSON fixture is not yet
loaded by Rust. Inventory-first is a counterfactual until physical issue/transfer
receipts prove use; yield is a conservative integer-floor approximation, not QA
evidence.

## Core contracts

- The caller declares at least two objectives with a stable ID, exact unit string,
  integer scale, and minimize/maximize direction.
- Objective values use integer ticks. Unit/scale mismatches are rejected; no
  implicit unit conversions or floating-point tie-breaking occur.
- Every candidate must supply at least one hard gate. Gate outcomes are
  evidence-bearing `Pass`, evidence-bearing `Fail`, or `Unknown` with a
  non-empty reason.
- Any failed gate blocks a candidate. If no gate fails but at least one is
  unknown, the candidate is incomplete. Neither can enter the Pareto frontier.
- Feasible candidates need a value and evidence reference for every declared
  objective.
- Input IDs, gate IDs, objective IDs, units, scales and value dimensions are
  validated. Results are sorted deterministically; each dominated candidate
  receives a lexicographically deterministic dominance witness.
- `MassBalance` checks exact integer-milligram closure at one declared process
  boundary with mutually exclusive material fates and checked arithmetic.

Evidence references are identifiers, not authentication. The calling system
must verify source provenance, validity, calibration, independence and domain
qualification before any real-world admission. The evaluator intentionally
cannot turn untrusted IDs into proof.

## Not yet implemented

Multi-stage process operations, time-indexed utility constraints, operator/skill
calendars, mid-operation breakdowns, factory-data connectors, validated
uncertainty models, a lifecycle-impact calculator and Mycelix evidence receipts
remain future work. Rust tests are authored, including an independent exhaustive
Pareto oracle and an independent tiny stress oracle, but no Rust test execution,
rustfmt/Clippy pass, or CI pass is claimed.


## Scheduling

This crate includes a bounded single-stage scheduler with capability checks,
release/due/hard-finish windows, evidence-backed sequence-dependent setups, hard
energy/water budgets, four deterministic dispatch rules (EarliestDue, LowestSetup,
LowestEnergy, LeastSlack), and an exhaustive Pareto oracle for tiny instances.
The robustness harness evaluates processing, input-release, utility-budget and
machine-availability shocks against the exact reference. Contracts: docs/ops/TEXTILE_SCHEDULER_BENCHMARK_V0_1.md
and docs/ops/TEXTILE_SCHEDULE_ROBUSTNESS_BENCHMARK_V0_1.md.
