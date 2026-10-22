# Shipyard benchmark auditor

Rust-first semantic auditor for the benchmark manifest/report contract in the sibling `shipyard-benchmark-contract` directory.

## Usage

```sh
cargo run --locked --manifest-path tools/shipyard-benchmark-auditor/Cargo.toml -- \
  benchmark-manifest.json run-report.json
```

Exit code is 0 only when the manifest/report pair passes the contract, cross-field checks, provenance linkage and aggregate recomputation. Exit code 1 means evidence is invalid; exit code 2 means usage or file IO failed. A machine-readable JSON summary is printed to stdout for valid reports, and diagnostics to stderr for invalid reports.

The auditor:
- checks schema JSON syntax and a narrow contract-consistency invariant for required fields (it does not implement every keyword in JSON Schema Draft 2020-12);
- enforces required/allowed fields, enumerations, identifiers, hashes, metric definitions and generated-corpus metadata;
- checks that the report pins the exact manifest bytes and that the run commit matches the candidate revision;
- recomputes counts and false-feasible / false-infeasible cases from per-case records and independent reference outcomes;
- rejects duplicate case IDs, oracle-result lies, summary drift, timeout-as-infeasibility, and incomplete reports marked complete;
- preserves unknown, timeout, invalid-output and not-run states rather than silently mapping them to infeasibility or zero.

A passing audit establishes internal consistency only. It does not establish the truth or completeness of an industrial data source, the physical adequacy of the model, operational safety, or a state-of-the-art result. Full Draft 2020-12 validation through a dedicated schema library remains separate follow-up work; do not describe the structural schema check here as a standards-complete validator.

## Conformance suite

`tests/conformance.rs` uses frozen positive and negative JSON fixtures. The report fixture's manifest hash is refreshed in-memory for each test so the same fixture can test the other semantic defect without copying a stale digest. The dedicated workflow pins Rust 1.96.0, fetches locked dependencies, and then performs format, offline test and Clippy checks.
