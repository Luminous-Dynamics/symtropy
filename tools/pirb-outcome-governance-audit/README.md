# PIRB v0.2 outcome-governance audit

Standalone Rust validator for the synthetic PIRB v0.2 metric-definition contract.

From the repository root:

```sh
cargo test --manifest-path tools/pirb-outcome-governance-audit/Cargo.toml
cargo clippy --locked --manifest-path tools/pirb-outcome-governance-audit/Cargo.toml --all-targets -- -D warnings
```

The crate exposes `validate_schema_document`, `validate_metric_contract`, and `validate_example_metric`. It checks Draft 2020-12 meta-schema compliance, instance conformance, duplicate governance-event IDs, chronological RFC 3339 event timestamps, event versions not exceeding the current metric version, approval-before-activation, and agreement between the declared status and final state-changing event.

The workflow also emits a deterministic JSON receipt binding the exact source SHA, schema digest, fixture digest, generated lockfile digest, Rust/Cargo versions, metric ID/version, and governance-event count. It verifies each field against the checked-out inputs and archives the receipt with those inputs. The example and all event references are synthetic. Validation is a contract-conformance result, not proof of legitimate authority, consent, accurate source evidence, causal impact, or deployment safety.
