# Failure-aware rollout fixture

These files are deterministic **synthetic-only** fixtures for the
`nanodrone_failure_aware_rollout` command-line smoke test. They contain no physical
flight data and must never be used to report model performance.

The fixture deliberately includes one `non_finite` baseline outcome at the 1.0 s
horizon. The evaluator must retain that rollout in the denominator with the declared
position-error penalty, while the paired candidate keeps a finite prediction.

The workflow hashes the exact receipt and model-manifest bytes at runtime, runs the
Rust CLI with two synthetic training seeds and two registered evaluation flights, then
checks the emitted JSON schema, grid counts, penalty rate, and RMSE direction.
