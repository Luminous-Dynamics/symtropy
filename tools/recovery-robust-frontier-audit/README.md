# Recovery Robust Frontier Auditor

Standalone Rust reference for the narrow interval-separated Pareto dominance theorem defined in `docs/tech/RECOVERY_ROBUST_FRONTIER_V0_1.md`.

## Run

```sh
cargo test --manifest-path tools/recovery-robust-frontier-audit/Cargo.toml
cargo run --quiet --manifest-path tools/recovery-robust-frontier-audit/Cargo.toml -- \
  docs/tech/recovery-robust-frontier-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)" > recovery-robust-frontier-receipt.json
```

The executable emits a machine-readable receipt with input SHA-256, source commit, feasible plan set, excluded plans and reasons, frontier, strict dominance certificates and structured errors. It exits nonzero if the corpus fails its contract.

## The theorem is intentionally conservative

For all-minimize integer-valued objective ranges A dominates B only when A's upper bound is no larger than B's lower bound for every objective and at least one margin is positive. Overlapping ranges do not establish dominance. Mandatory gates run first; a plan with any `BLOCK` or `UNKNOWN` gate is ineligible regardless of its objective values.

## Evidence boundaries

- The six-plan corpus is synthetic and tests logic only.
- This crate is an isolated workspace; it does not pull private crates or authorize physical operations.
- A local/static check is not hosted CI success. Take status from the exact-head run.
- No empirical optimizer quality, real timber recovery, ecological safety or legal permission is claimed.
