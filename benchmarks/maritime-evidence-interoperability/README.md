# Maritime evidence interchange auditor

Standalone offline Rust 1.96 reference auditor for the synthetic conformance corpus defined by MARITIME-EVIDENCE-INTEROP-001.

## Run from the repository root

```sh
cargo fmt --manifest-path benchmarks/maritime-evidence-interoperability/Cargo.toml -- --check
cargo test --locked --all-targets --manifest-path benchmarks/maritime-evidence-interoperability/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-evidence-interoperability/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path benchmarks/maritime-evidence-interoperability/Cargo.toml --bin maritime_evidence_interchange_audit -- benchmarks/maritime/maritime-evidence-interchange-conformance-v0.1.json --output /tmp/maritime-evidence-interchange-report.json
```

The binary checks the exact corpus digest, fixture count/identity, record shape/field allowlists, asset/configuration identity, native channel identity, source payload byte length and SHA-256, RFC3339 event/receipt times, event time against the corpus-pinned evaluation clock, quality/value-state semantics, derived/reconstructed/imputed lineage, reference integrity, ambiguous mapping abstention, duplicate source sequence, and diagnostic assertion authority/claim ceiling.

The report is machine-readable and records per-fixture expected versus actual disposition plus non-compensatory hard gates. The offline binary does not fetch content references: records whose payload bytes are remote are rejected as incomplete for byte-level integrity verification. It does not validate digital signatures, prove source identity, interpret vendor fault semantics, diagnose machinery, connect to a vessel, or authorize maintenance/ODD/mode changes.

## Test boundary

The frozen corpus is synthetic. A passing report means only that the reference auditor reproduces this versioned contract corpus. It does not establish ISO conformity, physical diagnostic accuracy, class approval, shipboard effectiveness, cybersecurity certification, safety or permission to operate.
