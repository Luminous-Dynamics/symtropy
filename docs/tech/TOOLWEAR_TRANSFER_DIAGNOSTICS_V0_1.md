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

## Current research context and fair comparison ladder

A 2025 study by Goodarzi, Schütze and Schneider, [*Domain shifts in industrial condition monitoring: a comparative analysis of automated machine learning models*](https://doi.org/10.5194/jsss-14-119-2025), evaluates leave-one-group-out transfer over seven public datasets. It reports that strong random K-fold results can drop substantially under machine-to-machine domain shift and that lower-capacity, more interpretable models can be competitive. This supports making grouped evaluation and worst-tool diagnostics primary, rather than treating row-random validation as a credible transfer score.

A January 2026 paper on [physics-guided unsupervised domain adaptation for tool-wear prediction](https://doi.org/10.1016/j.aei.2025.103883) proposes subspace alignment, pseudo-label regularization and physics-guided adjustment under novel machining conditions. This is a potential later comparator, but the current pinned dataset lacks the source-specified physical units and instrument/calibration metadata needed to validate physics consistency or a wear threshold. Do not claim to reproduce that paper or compare method names alone. If adding an adaptation baseline later, preregister exactly which *unlabeled* target-tool data it may access; held-out target labels must remain sealed.

[PDMBench](https://github.com/zshuai8/PDMBench) is a broader predictive-maintenance benchmark platform spanning multiple datasets and model families. It is useful as a future external reference, but a cross-domain transfer comparison is valid only when task definition, target, input modality, split, adaptation budget, and metric aggregation are matched. The current Vicomtech CSV is a fixed feature table and cannot directly establish parity with raw-signal or run-to-failure benchmarks.

### Comparison ladder (future milestones)

1. **Integrity and leakage:** immutable source, exact train/test tool identities, no group overlap, deterministic replay, shuffle/corruption controls.
2. **Strong transparent baselines:** the frozen tool-balanced constant and one-feature linear models, then predeclared regularized multivariate regression and a simple non-linear baseline where the available features support a fair comparison.
3. **Transfer and failure-domain metrics:** leave-one-tool-out macro MAE/RMSE, pooled metrics, worst-tool error, fold rank changes, plus interval coverage and out-of-domain detection after those capabilities are implemented.
4. **Fair state-of-the-art comparison:** only after source licensing/schema/provenance and task equivalence are verified; compare to a published baseline using its documented split, and report the original published result separately from our reproduction.
5. **Physical utility:** measure whether a validated model improves maintenance decisions, conforming output, escaped-defect rates, downtime and energy per conforming unit in a controlled trial. Prediction score alone cannot establish industrial value.

Proposed stretch gate for the model-comparison stage: beat the strongest preregistered transparent baseline on **macro-tool MAE without materially worsening worst-tool error**, and report uncertainty across tool groups. Numeric improvement thresholds should be chosen after baseline variance and measurement quality are known; this v0.1 structural/transfer harness does not yet implement intervals or a physical utility trial.

## First exact-head dataset result

The focused GitHub Actions run [#39315118040](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39315118040) passed on code head `a4bc812f7caa316d84baaea64a7783ba9bc38b0b`. It verified the pinned source bytes (3,346,122 bytes; SHA-256 `0199bc69cae5e74be3d668244f708da78450c669be641d254b5ca6aa712dda56`), ran the Rust tests/Clippy, validated the fixed MFG-DATA-001 report and the separate 13-fold report, and uploaded both artifacts.

The generated MFG-DATA-002 report was 22,806 bytes with SHA-256 `b607a2c95eafcd0bf5469f379b6bd48c5a74614d64697861573377d28e06280a`. The companion unchanged MFG-DATA-001 report was 3,247 bytes with SHA-256 `1c82242d40859997cedabcf07e14bdefc754b3795cd7d6cc716ba299b93baa34`.

| Predeclared model | LOTO pooled MAE | LOTO pooled RMSE | Macro-tool MAE | Macro-tool RMSE | Worst held-out tool (MAE) |
|---|---:|---:|---:|---:|---:|
| Tool-balanced constant | 82.285 | 118.276 | 80.007 | 105.266 | 4 (126.346) |
| OLS on `AE_RMS` | 81.467 | 126.223 | 77.210 | 104.763 | 10 (165.325) |
| OLS on `F_c_RMS` | 37.942 | 51.654 | 37.722 | 46.752 | 4 (66.092) |
| OLS on `F_p_RMS` | 61.953 | 82.436 | 66.220 | 79.499 | 9 (104.337) |

The `F_c_RMS` one-feature baseline had the lowest held-out MAE on 10 of 13 tools; `AE_RMS` led on 2, and `F_p_RMS` on 1. There were seven distinct model-ranking signatures across folds, confirming that ranking depends on the held-out tool. Against the declared constant baseline, `F_c_RMS` reduced LOTO pooled MAE by about 53.9% and pooled RMSE by about 56.4% in this dataset/protocol.

These values are source-native Vb errors; physical units are not specified by the pinned upstream README. The large per-tool changes and seven ranking signatures are precisely why a single fixed holdout or pooled score is insufficient. The result does not imply the best feature model is universally best, that the tool IDs represent independent factories, or that maintenance, safety, yield, energy savings, or process control would improve.

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
