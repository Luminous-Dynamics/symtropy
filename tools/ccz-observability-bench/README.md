# CCZ Environmental Observability Benchmark

Standalone Rust verifier for a frozen **synthetic** corpus of evidence-adequacy and claim-boundary cases. It evaluates symbolic evidence states; it does not simulate the ocean or predict ecological outcomes.

From the repository root, with Rust installed:

```sh
cargo test -p ccz-observability-bench --locked
cargo run --quiet -p ccz-observability-bench --locked -- \
  --corpus docs/tech/CCZ_ENVIRONMENTAL_OBSERVABILITY_CORPUS_V0_1.json \
  --source-commit "$(git rev-parse HEAD)" \
  --receipt target/ccz-observability-receipt.json
```

The receipt binds the exact corpus bytes by SHA-256, the source commit, suite/schema version, case count, and each expected/actual result. It is deterministic for identical corpus bytes, source commit, and tool version. Failed frozen cases return a nonzero exit code.

## Separate outputs, never one safety score

- Evidence integrity: whether source-snapshot identity and sample identities are consistent.
- Comparability: whether baseline, control, method, detection limits, calibration and coverage support the declared question.
- Impact result: positive evidence or a scoped non-detection statement. It never emits “no harm.”
- Recovery result: separately records whether recovery was assessed or remains unestablished.
- Cumulative exposure: tracks completeness of declared sound, light and sediment exposure channels separately from the local impact result.

## Claim ceiling

All fixture identifiers and evidence states are synthetic. This benchmark is not a plume model, ecology model, environmental-impact assessment, regulator approval, operational safety case, authorization to deploy a vehicle, or extraction planner. It contains no universal numeric “safe plume” or biodiversity threshold. Physical models require external qualification against measured/reference data before their outputs can be relied on.
