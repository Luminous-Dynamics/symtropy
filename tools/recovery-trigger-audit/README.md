# Recovery Trigger Auditor

Standalone Rust reference evaluator for `docs/tech/recovery-trigger-corpus-v0.1.json`.

## Run

```sh
cargo test --manifest-path tools/recovery-trigger-audit/Cargo.toml
cargo run --quiet --manifest-path tools/recovery-trigger-audit/Cargo.toml -- \
  docs/tech/recovery-trigger-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)" > recovery-trigger-receipt.json
```

The receipt records the exact corpus SHA-256 and source commit, each case's prior/resulting state, outcome/reason code, profile identity, and review-only action ceiling.

## Limits

- The 12 cases are synthetic boundary fixtures, not field measurements or calibrated trigger thresholds.
- Criteria-version mismatch forces reassessment.
- Unknown/stale/incomplete/unit-incompatible data never clears a trigger or authorizes an operation.
- The evaluator does not actuate hardware or grant permission.
- A green audit checks fixture logic only; check the exact PR head for hosted compilation/tests.
