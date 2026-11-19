# Member-Coherent Ensemble Route Risk

A separate executable known-answer control for Symtropy #6233. It intentionally contrasts complete per-member voyage paths with independently resampled edge marginals.

## Command

```sh
cargo test --manifest-path tools/marine-forecast-scorecard/Cargo.toml
cargo run --manifest-path tools/marine-forecast-scorecard/Cargo.toml --bin member-coherent-risk -- audit-member-risk tools/marine-forecast-scorecard/fixtures/synthetic-member-coherent-risk-0001.json
```

## What it checks

- Forecast source/cycle, artifact digest and first-availability receipt bind the ensemble.
- Every route candidate preserves the exact declared ensemble member set and source member digests.
- Every member path preserves route edge identity and order, with one finite non-negative loss per edge.
- Whole-route loss is summed for each member before member-coherent CVaR95 is computed.
- An intentionally wrong diagnostic independently resamples member losses edge by edge, destroying dependence. It computes the resulting CVaR only to show whether this invalid shortcut changes route ranking.

The fixture contains two routes and two synthetic members:
- Route A has member paths [10, 0] and [0, 10], so each coherent voyage totals 10; independent edge resampling creates a 20-tail.
- Route B has member paths [8, 8] and [0, 0], so coherent CVaR95 is 16; independent resampling also has a 16-tail.
- Thus coherent route risk selects A (10 vs 16), while independent edge resampling selects B (16 vs 20).

This is a software known-answer demonstration of dependence destruction, not a real sea-state model or risk guarantee. The independent-edge comparator is an explicit negative control, never an approved estimator. Small-ensemble CVaR95 has very coarse tail resolution; operational research needs calibrated ensemble size, event validation, member perturbation provenance, and whole-voyage out-of-sample evidence.
