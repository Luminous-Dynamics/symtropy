# WPT predictive uncertainty benchmark v1

This benchmark evaluates interval coverage and sharpness from frozen held-out observations. It intentionally accepts only `split: "test"` records; calibration-set construction is upstream and must be frozen independently.

## Run

```sh
cargo test --manifest-path tools/wpt-predictive-uncertainty/Cargo.toml
cargo run --manifest-path tools/wpt-predictive-uncertainty/Cargo.toml -- benchmarks/wpt-predictive-uncertainty/v1/known-truth.jsonl
```

## Semantics

- A record is one prediction/observation pair for one declared physical quantity, unit, model, calibration set, test set, trajectory group and operating regime.
- Coverage is inclusive: lower <= observed <= upper.
- Sharpness is mean interval width in the record's declared unit. Do not compare widths across unlike quantities or units.
- The mean Winkler interval score penalizes misses in addition to width; lower is better. For nominal coverage `1 - alpha`, a miss below/above the interval incurs `2 / alpha` times its distance beyond the relevant bound.
- Reports stratify by regime. Aggregate coverage must never conceal a weak regime. Reports also count unique trajectory groups so repeated points from one trajectory are visible rather than silently treated as independent trials.
- One report is restricted to one model digest, calibration-set digest, test-set digest, physical quantity kind and unit. Mixed experiments must be evaluated separately.
- Model/calibration/test digest fields must be 64-character lowercase SHA-256 hex strings. The evaluator checks digest syntax only; it does not retrieve or authenticate the referenced bytes.
- This v1 evaluator requires one nominal coverage level per report.
- It rejects calibration records, duplicate IDs, mixed model/calibration/test/quantity/unit metadata, invalid intervals and non-finite values.

## Interpretation limits

The output is descriptive empirical held-out performance, not a distribution-free guarantee. Conformal prediction coverage depends on assumptions such as exchangeability; time-correlated trajectories, hardware changes and distribution shift require dedicated evaluation. OOD detection is not a safety certificate. An unsupported regime must be rejected or handled by a separately qualified fallback.

The initial fixture is synthetic known-truth data only. It is not experimental WPT evidence, a standards conformance result, or permission to energize hardware.

## Interval-aware control gate

The library also exposes `evaluate_decision_gate` for hard operating-envelope constraints. Every predictive interval must lie completely on the permitted side of its threshold: minimum constraints require `lower >= minimum`; maximum constraints require `upper <= maximum`. A threshold-overlapping interval returns `NOT_EVALUABLE`; a definitely out-of-bounds interval returns `INHIBIT`. Empty, malformed, non-finite and duplicate constraints are rejected.

`ELIGIBLE` means only that the supplied interval constraints pass. It is not an energization authorization and does not replace independent identity, positioning, foreign-object, exposure, thermal, communications-freshness or hardware protection interlocks. The action for unresolved intervals is conservative derating or ramp-down, not optimistic continuation.

The `evaluate_decision_gate_for_regime` API additionally requires the current operating regime to belong to an explicit, non-empty, unique qualified-regime set. Unsupported conditions produce `NOT_EVALUABLE` even when their supplied intervals appear feasible; a definite physical constraint breach still dominates and produces `INHIBIT`. Regime qualification must be built from held-out evidence and a frozen operating-domain definition, not inferred from a model's point prediction.
