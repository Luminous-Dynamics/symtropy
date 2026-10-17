# Recovery Adaptive Regret Auditor

Standalone Rust scenario-policy evaluator for `docs/tech/recovery-adaptive-regret-corpus-v0.1.json`.

## Run

```sh
cargo test --manifest-path tools/recovery-adaptive-regret-audit/Cargo.toml
cargo run --quiet --manifest-path tools/recovery-adaptive-regret-audit/Cargo.toml -- \
  docs/tech/recovery-adaptive-regret-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)" > recovery-adaptive-regret-receipt.json
```

The command evaluates each observation-constrained policy, excludes policy branches whose chosen operation is BLOCK/UNKNOWN in any scenario, calculates unweighted scenario costs and regret, computes the (total cost, worst-regret) frontier, and writes an input SHA-256 / source-commit receipt.

## Decision and evidence limits

- A policy maps visible observation classes to actions. A post-inspection branch is available only when the initial policy selected `inspect` and that scenario supplies the post-inspection observation.
- The per-scenario best feasible action is a clairvoyant lower bound. Regret includes the inspection cost; no probability is assigned to any scenario.
- Totals are unweighted synthetic scenario sums, not expected costs or forecasts.
- A passing audit checks the synthetic contract only; it grants no permission or actuation.
- Check the exact-head hosted workflow status; queued/pending is not pass.
