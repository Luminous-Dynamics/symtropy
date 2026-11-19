# Marine Forecast Skill and Mission Decision Scorecard

Standalone Rust scorecard auditor for Symtropy #6220. It consumes scorecard records whose replay-input provenance and grouped holdouts are established by the forecast replay admission work in #6209 / PR #6215.

## Commands

```sh
cargo fmt --manifest-path tools/marine-forecast-scorecard/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-forecast-scorecard/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-forecast-scorecard/Cargo.toml
cargo run --manifest-path tools/marine-forecast-scorecard/Cargo.toml -- audit-scorecard tools/marine-forecast-scorecard/fixtures/synthetic-scorecard-0001.json
```

## What this increment computes

Forecast skill and downstream decision value remain separate scorecards.

- Deterministic scalar forecast diagnostics by variable and lead: bias, MAE and RMSE.
- Empirical ensemble CRPS using the finite ensemble members included with each case.
- Threshold-event Brier scores plus reliability-bin counts, mean forecast probability and observed frequency.
- Paired mission outcomes using one shared matching-context identity for both candidate and baseline: purchased energy per occupied berth-day, mission-regret proxy, CVaR95 regret, hard-gate violations, service-floor breaches, comfort breaches and plan changes.
- SHA-256 identity for the complete scorecard input, source artifacts, first-availability receipts, dataset, split manifest, transforms and paired mission context.

The validator rejects future-available forecasts, issue/lead/valid-time inconsistencies, forecast/verifier time mismatch, source self-verification, cross-fold voyage/weather-event reuse, missing or malformed digests, duplicated samples, unpaired cases, and candidate hard-gate/service/comfort regressions.

## Interpretive limits

- The fixture is synthetic and must remain `SYNTHETIC_ONLY`; its scores are software known-answer examples, not evidence of marine forecast performance.
- The engine does not download GRIB/NetCDF, decode grids, spatially collocate station data, judge observation representativeness, authenticate publishers, fit calibration transforms, compute vector-wind direction error, or authorize route/navigation actions.
- Ensemble CRPS is descriptive for the supplied sample members. Brier scores and reliability bins are event-specific. The fixture does not establish calibration from nine cases.
- CVaR95 is descriptive and reports the number of tail samples used. With small data its tail estimate is unstable; it is not a risk guarantee.
- No confidence intervals, significance tests, or statistical-superiority claims are produced. Before such claims, preregister power analysis and use paired whole-voyage/weather-event block resampling.
- A valid audit is not a physical truth, seaworthiness, safety certification or regulatory approval.

## Research boundary

The next paradigm is to connect forecast *information* to decisions without collapsing them. Score forecasts by variable, lead and threshold; separately evaluate mission decisions under an information-matched baseline. Keep hard constraints non-compensatory. Treat reanalysis as evaluator context, in-situ sources as spatially limited checks, and perfect foresight as a non-deployable lower-bound reference. A better aggregate weather score does not automatically imply better sailing or liveaboard outcomes.
