# hydro-contextual-t0

Synthetic-first Rust evaluator for catchment-conditioned hydrology benchmark metrics and split-integrity rules.

## Run from repository root

\`\`\`sh
cargo fmt --package hydro-contextual-t0 -- --check
cargo test --locked --package hydro-contextual-t0
cargo clippy --locked --package hydro-contextual-t0 --all-targets -- -D warnings
HYDRO_CONTEXTUAL_T0_REVISION="$(git rev-parse HEAD)" \\
  cargo run --locked --package hydro-contextual-t0 -- \\
    research/hydrology/hydro-contextual-t0-v0.1.json \\
    research/hydrology/hydro-contextual-manifest-v0.1.json \\
    research/hydrology/hydro-contextual-oracle-v0.1.json \\
    artifacts/hydro-contextual-t0/receipt.json
\`\`\`

## Scope

The frozen 32-case corpus exercises:
- B0 persistence and training-only seasonal-climatology metric recomputation;
- B1 parameter-prior member visibility and invalid-member disclosure;
- B2 repeated train-only calibration-reference summary;
- B3 frozen temporal-holdout metric recomputation;
- NSE, KGE, volume bias and MAE with explicit degenerate-denominator semantics;
- contextual attainment with raw lower/model/upper values, no clipping, and explicit NOT_COMPARABLE / UNDEFINED abstentions;
- 12 prescribed split/claim leakage controls plus boundary attacks for missing-as-zero, unknown anthropogenic operations, physical-gate failure, duplicate timestamps and overlapping event windows.

The evaluator reads frozen arrays. It does not generate, calibrate, select or execute a hydrological model. It does not own hydrological mass-balance qualification; physical gate inputs are consumed as a declared result from the existing #461/#445 authorities.

## Evidence receipt

The CLI binds input, manifest, oracle and Cargo.lock SHA-256 digests and the exact implementation revision. Local runs without a valid revision are labelled PassedLocalUnpinned; CI provides the checkout SHA and emits PassedExactRevision only when all 40 rows match the separate oracle.

## Nonclaims

All v0.1 cases are synthetic. No DWS/SAWS/Highveld bytes are acquired, no catchment is calibrated, no flood/groundwater/rainfall-runoff solver is qualified, no field forecast skill is established, and no synthetic result may be represented as South African field performance.
The corpus includes all 12 named HCB-NC-01 through HCB-NC-12 leakage controls, including a separate rejection when the final test set is used to select a winning model.


### Event, low-flow and probabilistic interval metrics (cases 022–027)

The corpus now covers the process-specific metrics called for by #5338/#5356:
- named-event peak magnitude error and signed/absolute peak timing error using a frozen event window and fixed step duration; ties choose the earliest peak;
- event-specific NSE and volume bias alongside peak metrics;
- low-flow log10 error below a predeclared threshold, over strictly positive observed/predicted values only; no zero pseudocount or silent exclusion;
- probabilistic prediction-interval empirical coverage and mean width, preserving nominal coverage and coverage error;
- invalid interval bounds reject, and low-flow logarithms abstain when zero flow appears.

These are separate diagnostics, not a composite score. A four-sample interval fixture is only arithmetic conformance, not a statistical calibration claim; useful interval calibration requires many independent events and strata.

The event-peak contract also freezes the tie rule: equal maxima select the earliest time index. A separate case tests this deterministically.


Event-peak ties are resolved at the earliest index in the event window. A dedicated frozen case locks this behavior so timing metrics are deterministic.


### Proper probabilistic verification (cases 028–032)

The 44-case synthetic corpus adds:
- empirical ensemble CRPS for continuous streamflow distributions, using the finite-ensemble empirical CDF formula;
- binary threshold-event Brier score with a frozen event-definition identifier;
- aggregate Brier score plus five fixed reliability bins (including explicit empty-bin nulls);
- rejection of probabilities outside [0,1] and negative streamflow members.

CRPS and Brier are proper scores, unlike raw coverage alone. Reliability bins are descriptive, not proof of calibration; meaningful calibration requires many independent events, catchments, forecast horizons and uncertainty intervals. Event threshold, units, lead time and valid-time semantics must be frozen before scoring. All cases remain synthetic; the evaluator does not issue forecasts.


### Reference-relative probabilistic skill (cases 033–038)

The 44-case synthetic corpus adds CRPSS and Brier Skill Score against a named reference forecast. A skill score is admitted only when forecast and reference share the same evaluation population, exact ordered observation case IDs and lead time; BSS also requires the same event definition. Scores are not clipped, so negative skill remains visible. A zero/near-zero reference score returns Undefined. The comparison does not prove the reference is scientifically appropriate; the reference forecast identity and generation protocol still need independent admission for real work.


The CRPSS/BSS extension requires both `forecast_id` and `reference_forecast_id`. Cases 039–040 reject a reference with mismatched verification case IDs or a different event threshold definition. This makes persistence and train-only seasonal climatology separate, explicit reference populations rather than an unnamed baseline.


### Matched probabilistic skill comparisons (cases 033–044)

CRPSS and Brier Skill Score now compare a candidate to an explicitly named reference on identical verification cases and lead time. The comparator contract also pins evaluation population, variable, units, temporal aggregation and valid-time semantics; threshold-event comparisons pin the event definition. A mismatch is NOT_COMPARABLE, not a rescaled or partial score. A zero-loss reference is UNDEFINED. Negative skill remains negative (worse than reference) and is never clipped to zero.

Cases 041–042 seed unit and aggregation mismatch; cases 043–044 prove negative CRPSS/BSS survive unchanged. The 44-case corpus contains synthetic fixtures only and does not include a live forecast or South African station observation.
