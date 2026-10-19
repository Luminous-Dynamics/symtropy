# MFG-DATA-002 — leave-one-tool-out transfer diagnostics v0.1

## Status

Proposed robustness increment layered on top of the frozen [MFG-DATA-001 evaluator](https://github.com/Luminous-Dynamics/symtropy/pull/2080). MFG-DATA-001's four-tool holdout and its reported result are preserved exactly. This mode is a separate diagnostic with its own report identity; it is not a replacement headline metric.

Evidence class remains `external_dataset_evaluation_unqualified_for_process_control`. Source label Vb remains in source-native units; the pinned README does not specify its physical units.

## Paradigm shift

Evaluate models as transferable engineering hypotheses with explicit failure domains, not as a single score that conceals poor transfer to a particular tool.

A row-random split would allow records from the same tool to cross between fitting and evaluation, overstating generalization. This diagnostic instead holds out one complete tool identity at a time. Every fold predeclares the same three single-feature OLS models (`AE_RMS`, `F_c_RMS`, `F_p_RMS`) plus a tool-balanced constant baseline. It does not select features or tune hyperparameters based on the fold results.

## Protocol

- Source admission remains unchanged: exact pinned upstream commit, SHA-256, byte size, row/column shape, missing-cell count, required finite fields, exact tool-ID set and duplicate-row rejection.
- For each of the 13 tools, fit the constant and each one-feature linear model on all other 12 tool identities, and test on the held-out tool only.
- Training/test tool sets must be disjoint. Each fold emits training and test identities, row counts, model coefficients, model ranking by held-out MAE, and per-model metrics.
- The aggregate report includes pooled MAE/RMSE across the 13 held-out groups, equal-tool macro MAE/RMSE, per-tool MAE/RMSE, the worst-tool MAE result, and the number of unique ranking signatures across folds.
- The fixed MFG-DATA-001 split remains emitted only by the default CLI mode. Use `--loto` for this separate report.
- Metrics are descriptive of this pinned feature table. Do not interpret folds as independent manufacturing sites or tools sampled from a known population; tool IDs are dataset group identities only.

## Acceptance targets

- Exactly 13 held-out folds, one for every observed tool ID; no silent tool omission.
- Zero tool-ID overlap between each fold's training and test sets.
- The frozen constant and feature models appear in every fold; no fold-dependent feature selection.
- Every output includes fold training/test denominators, full per-tool errors, pooled/macro metrics, worst-tool error, source digest, split/diagnostic digest, predeclared feature identities, and evidence limitations.
- Repeating with identical pinned bytes and executable produces byte-identical output.
- Source corruption (including same-length changed bytes), malformed schema, non-finite values, and missing required labels/features reject before evaluation.
- The external-data workflow validates both the unchanged MFG-DATA-001 report and the separate MFG-DATA-002 report contract.

These are implementation acceptance goals, not claims that the model generalizes or that any maintenance policy improves the physical process.

## Failure-domain interpretation

If a feature model ranks first on some held-out tools but fails badly on another, the report should make that visible. The aggregate ranking-signature count is a coarse diagnostic of rank stability, not a probability or confidence estimate. Worst-tool MAE is a red-flagging statistic, not a formal guarantee on future tools. This v0.1 does not compute prediction intervals, infer Vb units, qualify instruments, validate raw signal processing, or translate predictions into maintenance thresholds.

## Validation commands

```bash
cargo fmt --check --manifest-path tools/toolwear-eval/Cargo.toml
cargo test --locked --offline --manifest-path tools/toolwear-eval/Cargo.toml
cargo clippy --locked --offline --manifest-path tools/toolwear-eval/Cargo.toml --all-targets -- -D warnings
cargo run --locked --offline --manifest-path tools/toolwear-eval/Cargo.toml -- --data /path/to/Dataset_Machine_tool_wear.csv
cargo run --locked --offline --manifest-path tools/toolwear-eval/Cargo.toml -- --data /path/to/Dataset_Machine_tool_wear.csv --loto
```

Do not claim validation until formatting, tests, Clippy, both report-contract assertions, source-digest checks, and exact-head workflow evidence are complete.
