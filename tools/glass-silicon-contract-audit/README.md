# Glass / Silicon Contract Audit

A small Rust reference verifier (dependency versions pinned and lockfile included) for the GLASS-SILICON-001 benchmark.

It audits the **evidence contract** for the checked-in synthetic benchmark corpus and checks elemental mass-balance arithmetic on an explicitly declared boundary. It currently defines 37 Rust unit tests; their count is not a statement that they have been executed. It is not a glass-property predictor, thermochemical solver, furnace digital twin, lifecycle assessor, product certification tool, or hardware controller.

## Commands

From the repository root:

```sh
cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-corpus \
  docs/tech/glass-silicon-process-benchmark-v0.1.schema.json \
  docs/tech/glass-silicon-process-benchmark-v0.1.json
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-decision-value \
  tools/glass-silicon-contract-audit/fixtures/decision-value-pool.synthetic.json
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- check-mass \
  tools/glass-silicon-contract-audit/fixtures/mass-balanced.synthetic.json

# Run only after permitted local acquisition of the source files; this command does not download or admit them.
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- hash-artifacts \
  docs/tech/glass-silicon-process-benchmark-v0.1.json \
  scale-zenodo-18430657-v1 /path/to/lawfully-acquired-files
```

Exit codes: 0 = checks pass at the declared contract/arithmetic level; 2 = failed contract/arithmetic; 3 = unresolved or insufficient evidence; 64 = command/usage error. Reports are deterministic JSON with sorted element maps and stable finding codes. Run the CLI with `--locked`; Cargo.lock pins the standalone verifier's resolved dependency graph.

## Local artifact hash receipt

`hash-artifacts` takes a corpus manifest, artifact ID and caller-supplied local directory. It rejects path traversal and symlinks escaping the supplied directory, computes SHA-256 for each listed file, and compares the locally computed MD5 with the legacy checksum published in the source manifest. If a SHA-256 has already been pinned, it compares that too.

The command never downloads data, changes the corpus manifest, resolves a license, or marks an artifact admitted. A successful report uses `HASHED_PENDING_ADMISSION` and lists the computed SHA-256 values for review. Update the manifest only after exact artifact identity and permitted use have been independently reviewed. MD5 is only legacy checksum corroboration; it is not secure authentication. Never claim reproducibility from a public record listing alone.

## Mass ledger contract

Inputs and outputs are elemental masses in the *same declared unit*. Inventory change is final minus initial boundary inventory. For each element:

`residual = input - output - inventory_change`

The record must declare a complete stream census and element-specific tolerances accompanied by per-element justification metadata; justification text is not independently verified by this tool. Missing elements, incomplete census, unknown evidence status, or missing tolerance yield `UNRESOLVED`; a negative input/output quantity or residual outside tolerance yields `FAIL`. Negative inventory change is permitted because it can represent inventory drawdown.

The result proves only that the supplied arithmetic fits the supplied contract. It cannot prove that the stream census is physically complete, that an instrument is calibrated, or that a material was produced. Those require independent evidence.

## Claim ceiling

The checked-in fixtures are synthetic software tests. They are not plant data, recipes, measured energy consumption, qualified glass, or silicon production results. Full JSON Schema validation, source-data reproduction, and physical-process validation remain separate gates.


## Synthetic decision-value pool

The `audit-decision-value` command evaluates a fully enumerated **synthetic** scenario/action pool. It computes the best robust action before measurement, the post-measurement worst-case regret for each observation partition, gross regret reduction, eligibility under separate cost and duration caps, and a Pareto frontier. Its exact one-step oracle is evaluator-side and chooses only among the enumerated budget-feasible measurements.

The fixture includes an informative measurement, an uninformative measurement and an informative-but-over-budget measurement. Unit mutation tests reject a missing cost, producer-access leakage declarations, feasibility-control bypasses, and incomplete observation maps. The producer-access values are a declared contract, **not proof that a real producer process is isolated**. All utility values, scenarios, costs and feasibility controls in this fixture are synthetic; this command is not a materials model, physical value-of-information estimate, experiment authorization, or evidence of industrial gains.

Exit codes retain the tool convention: 0 = PASS at the declared contract/arithmetic level; 2 = FAIL; 3 = UNRESOLVED; 64 = usage error.
