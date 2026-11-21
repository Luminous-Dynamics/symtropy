# SU2 history output import v0.1

## Scope and evidence ceiling

This module imports selected SU2 history columns only after the external solver receipt has been replayed and the history file's exact bytes have been checked against the receipt. Its evidence ceiling is:

\`receipt_bound_history_import_only_not_solver_convergence_or_physical_validation\`

It does not decide whether residuals converged, force coefficients stabilized, the grid is independent, the flow model is accurate, or the aircraft is airworthy.

SU2 history output is configurable. The operator must inspect the selected SU2 build/configuration (for example with that build's documented output-field listing), then declare the exact ordered header and units. Never assume that every SU2 version, solver family, or configuration emits the same columns.

## CLI

After a successful run by \`external_solver_run\`, declare the exact history schema:

\`\`\`sh
cargo run --release --locked \\
  --manifest-path tools/civil-aero-benchmark/Cargo.toml \\
  --bin su2_history_import -- \\
  --run-dir ./artifacts/dpw8-v1a-periodic-l1-a136 \\
  --history history.csv \\
  --iteration-column Iteration \\
  --header-column Iteration \\
  --header-column CL \\
  --header-column CD \\
  --header-column RMS_RES \\
  --field CL=dimensionless \\
  --field CD=dimensionless \\
  --minimum-rows 2 \\
  > ./artifacts/dpw8-v1a-periodic-l1-a136/history-import.csv
\`\`\`

Repeat \`--header-column\` for every field in the exact header and repeat \`--field NAME=UNIT\` for each numeric field to retain. All header names are case-sensitive. The iteration column must parse as an unsigned integer and strictly increase. Selected values must be finite. The declared units are recorded as operator metadata; this importer does not independently prove dimensional conventions.

The importer:
- requires a replayable external solver receipt with \`solver_id=SU2_CFD\`;
- requires successful process status and that the history file was a declared expected output;
- verifies staged inputs, logs, manifests, all outputs, and receipt digests;
- re-hashes the history bytes and requires exact agreement with the receipt;
- rejects symlinks/path traversal, schema mismatch, malformed CSV, duplicate/non-monotonic iteration counters, missing fields, non-finite selected values, and insufficient rows;
- hashes a canonical import representation binding run ID, solver executable digest, runner receipt digest, history digest, ordered schema, units, and every retained observation.

A passing import is not a passing CFD run. A future convergence classifier must independently evaluate residual histories, force/moment stability, conservation, solver exit metadata, and declared solver-specific criteria, then flow into grid convergence (Richardson/GCI) and validation uncertainty. Do not infer convergence from a completed process or from a visually flat plot.

## Test fixture

The CI integration test invokes a deterministic fake executable, captures its history as an expected output, imports it through this module, and verifies receipt replay and tamper rejection. The fixture is not SU2 and is not aerodynamic evidence.


## API construction boundary

The imported report's fields are crate-visible rather than externally constructible. Downstream consumers receive read-only accessors for the run ID, solver claim, executable/receipt/history/import digests, selected fields, observations and evidence ceiling. Only the receipt-replaying importer can construct a report through the public API. The convergence screen also recomputes the canonical import digest and checks header/field/point invariants before evaluation. This prevents ordinary external Rust callers from manufacturing a self-consistent fake report by filling public struct fields and hashing them; it does not replace receipt replay or cryptographic attestation of the solver build.
