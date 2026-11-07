# Scan-to-BIM Admission Oracle

Standalone Rust package for the finite synthetic contract in #4086. It is deliberately separate from the built-asset defect/repair oracle in #4062.

## Run

```sh
cargo fmt --manifest-path tools/scan-bim-admission-oracle/Cargo.toml -- --check
cargo test --manifest-path tools/scan-bim-admission-oracle/Cargo.toml
cargo clippy --manifest-path tools/scan-bim-admission-oracle/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/scan-bim-admission-oracle/Cargo.toml
```

The CLI emits a deterministic JSON report with the fixture byte digest and case-by-case expected/actual dispositions. The manifest also declares grouped training/evaluation identities and a candidate-input allowlist; every case must bind to a declared held-out group, the oracle rejects overlapping groups and obvious ground-truth/reference-bound input markers. This is a contract-level guard, not a substitute for an independent data custodian enforcing access to held-out labels and geometry.

## Scope

The v0.1 oracle evaluates a small, declared admission contract over candidate geometry summaries, element identity/class, visibility, uncertainty, evidence lineage, topology and revision-bound modify proposals. It does not compute geometric deviation from raw point clouds: the deviation and uncertainty are typed upstream measurements supplied to the decision oracle and must be independently qualified by their producing pipeline. It also rejects duplicate IDs/edges/targets, unknown schema fields, stale model revisions, incomplete visibility, unsupported identity changes and uncertainty intervals crossing the declared tolerance. It does not register point clouds, fit geometry, infer semantic labels, write IFC, certify surveys or authorize physical work.

Expected labels are used only to compare the oracle output after candidate evaluation. The evaluator does not read expected outcomes as inference inputs.

## Claim ceiling

A passing suite proves only agreement with 23 hand-authored synthetic fixtures and their explicit rules. It does not establish real scan accuracy, sensor performance, generalization, building safety, structural adequacy, code compliance, legal survey status, or operational productivity.
