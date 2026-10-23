# Shipyard benchmark evidence auditor

A dependency-free Rust CLI for validating benchmark manifests and auditing result reports. It intentionally avoids network access and third-party crates; the same input bytes produce the same ordered diagnostics.

## Commands

Validate a manifest:

    cargo run --locked --offline --manifest-path tools/shipyard-benchmark-audit/Cargo.toml -- manifest tools/shipyard-benchmark-audit/fixtures/valid-manifest.json

Audit a report against its manifest:

    cargo run --locked --offline --manifest-path tools/shipyard-benchmark-audit/Cargo.toml -- report \
      tools/shipyard-benchmark-audit/fixtures/valid-manifest.json \
      tools/shipyard-benchmark-audit/fixtures/valid-report.json

Verify the SHA-256 of a source/configuration artifact after obtaining the actual file locally:

    cargo run --locked --offline --manifest-path tools/shipyard-benchmark-audit/Cargo.toml -- verify-digest path/to/corpus.json EXPECTED_64_HEX_SHA256

This command hashes the exact file bytes; it does not normalize JSON before hashing. It is useful to verify locally supplied source/configuration artifacts against digests pinned in the manifest.

Exit codes: 0 valid; 1 structurally or semantically invalid evidence; 2 input/usage error. Diagnostics are sorted and written in a deterministic order.

## Checks performed

- Strict JSON syntax, including duplicate object-key rejection and Unicode escape validation.
- Contract version, enums, required fields, model scope, constraints, seed and generator recipe, source case count, digests, candidate and baseline revision syntax, oracle identity, and budgets.
- SHA-256 of the exact manifest bytes, checked against the report's manifest_sha256.
- The run's source revision must match the candidate revision in the manifest.
- A case's `outcome` records the candidate's statement (`feasible-claim`, `infeasibility-proof`, `no-solution`, `timeout`, `invalid-output`, or `not-run`); it does not predeclare that statement verified. The reference outcome and verifier status are independent evidence fields.
- The report separates the count of feasible reference cases actually found from feasible reference cases missed. A heuristic that returns no solution on an oracle-feasible case is a missed case, not a false infeasibility proof.
- Duplicate case IDs, case-count completeness, per-case outcome/verifier consistency, oracle/reference agreement, and summary arithmetic.
- False-feasible and false-infeasible counts recomputed from per-case reference outcomes, rather than trusted as caller-supplied aggregates.
- Unknown, timeout, invalid-output, and not-run remain distinct outcomes. A claim of proven infeasibility requires a matching infeasibility reference; heuristic exhaustion or a timeout is not proof.
- Incomplete reports may retain unknowns and missing cases, but cannot claim to be complete while omitting the manifest's expected case count.

## Limitations

This is a contract-specific semantic auditor, not a general JSON Schema Draft 2020-12 implementation. It checks the schema files are valid JSON, but Rust-side validation logic is maintained explicitly and must be updated with schema changes. It checks source/configuration digest syntax but cannot verify artifacts not supplied to it. A correct digest proves byte identity only, not source truth, provenance authorization, representativeness, physical safety, or operational gain.

Do not label a benchmark state-of-the-art or industrially validated based solely on this tool. It validates evidence structure and internal consistency; the reference model and industrial source still require independent qualification.
