# Glass / Silicon Contract Audit

A small Rust reference verifier (dependency versions pinned and lockfile included) for the GLASS-SILICON-001 benchmark.

It audits the **evidence contract** for the checked-in synthetic benchmark corpus and checks elemental mass-balance arithmetic on an explicitly declared boundary. It currently defines 20 Rust unit tests; their count is not a statement that they have been executed. It is not a glass-property predictor, thermochemical solver, furnace digital twin, lifecycle assessor, product certification tool, or hardware controller.

## Commands

From the repository root:

```sh
cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-corpus \
  docs/tech/glass-silicon-process-benchmark-v0.1.schema.json \
  docs/tech/glass-silicon-process-benchmark-v0.1.json
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- check-mass \
  tools/glass-silicon-contract-audit/fixtures/mass-balanced.synthetic.json
```

Exit codes: 0 = checks pass at the declared contract/arithmetic level; 2 = failed contract/arithmetic; 3 = unresolved or insufficient evidence; 64 = command/usage error. Reports are deterministic JSON with sorted element maps and stable finding codes. Run the CLI with `--locked`; Cargo.lock pins the standalone verifier's resolved dependency graph.

## Mass ledger contract

Inputs and outputs are elemental masses in the *same declared unit*. Inventory change is final minus initial boundary inventory. For each element:

`residual = input - output - inventory_change`

The record must declare a complete stream census and element-specific tolerances accompanied by per-element justification metadata; justification text is not independently verified by this tool. Missing elements, incomplete census, unknown evidence status, or missing tolerance yield `UNRESOLVED`; a negative input/output quantity or residual outside tolerance yields `FAIL`. Negative inventory change is permitted because it can represent inventory drawdown.

The result proves only that the supplied arithmetic fits the supplied contract. It cannot prove that the stream census is physically complete, that an instrument is calibrated, or that a material was produced. Those require independent evidence.

## Claim ceiling

The checked-in fixtures are synthetic software tests. They are not plant data, recipes, measured energy consumption, qualified glass, or silicon production results. Full JSON Schema validation, source-data reproduction, and physical-process validation remain separate gates.
