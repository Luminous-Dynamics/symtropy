# Matched-budget experiment cost ledger

The operation ledger is an accounting interface for offline experiment-selection benchmarks, including ShapeBench adapters and future CFD campaigns. It is not a performance result.

## Record semantics

- Each row is one billable or explicitly zero-cost operation within a task / replicate / policy.
- Cost units must have a benchmark-defined mapping to actual resources. Also record wall seconds and CPU seconds.
- Failed or aborted runs retain their cost and an explicit reason; they are not dropped from the budget.
- Successful operations bind an output SHA-256. Failed operations use '-' as output hash and provide a reason code.
- Each policy under the same task / replicate uses the same declared budget cap. Actual spend can differ; report spend, remaining cap, successful operations and failures. Do not silently truncate spend or pretend unused budget is consumed.
- The audit charges evaluation, tuning, orchestration, reference-evaluator and independent-audit work to the appropriate policy. The budget definition must state whether a particular shared infrastructure cost is fixed, amortized, or attributed.
- A successful ledger audit means only the accounting entries pass consistency checks. It does not establish that the candidate or result is aerodynamically valid.

Run from the package directory:

    cargo run --locked --bin budget_audit -- data/decision_value/matched_budget_ledger.csv

The committed example is synthetic. It illustrates equal cap (5 units) and charges a nonconverged high-fidelity operation to the random policy. The overrun negative control must be rejected. No optimization win is implied by this fixture.
