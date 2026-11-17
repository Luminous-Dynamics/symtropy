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

The frozen 41-case corpus exercises:
- B0 persistence and training-only seasonal-climatology metric recomputation;
- B1 parameter-prior member visibility and invalid-member disclosure;
- B2 repeated train-only calibration-reference summary;
- B3 frozen temporal-holdout metric recomputation;
- NSE, KGE, volume bias and MAE with explicit degenerate-denominator semantics;
- contextual attainment with raw lower/model/upper values, no clipping, and explicit NOT_COMPARABLE / UNDEFINED abstentions;
- 12 prescribed split/claim leakage controls plus boundary attacks for missing-as-zero, unknown anthropogenic operations, physical-gate failure, duplicate timestamps and overlapping event windows.

The evaluator reads frozen arrays. It does not generate, calibrate, select or execute a hydrological model. It does not own hydrological mass-balance qualification; physical gate inputs are consumed as a declared result from the existing #461/#445 authorities.

## Evidence receipt

The CLI binds input, manifest, oracle and Cargo.lock SHA-256 digests and the exact implementation revision. Local runs without a valid revision are labelled PassedLocalUnpinned; CI provides the checkout SHA and emits PassedExactRevision only when all 41 rows match the separate oracle.

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

The 41-case synthetic corpus adds:
- empirical ensemble CRPS for continuous streamflow distributions, using the finite-ensemble empirical CDF formula;
- binary threshold-event Brier score with a frozen event-definition identifier;
- aggregate Brier score plus five fixed reliability bins (including explicit empty-bin nulls);
- rejection of probabilities outside [0,1] and negative streamflow members.

CRPS and Brier are proper scores, unlike raw coverage alone. Reliability bins are descriptive, not proof of calibration; meaningful calibration requires many independent events, catchments, forecast horizons and uncertainty intervals. Event threshold, units, lead time and valid-time semantics must be frozen before scoring. All cases remain synthetic; the evaluator does not issue forecasts.


### Reference-relative probabilistic skill — batch-scoped (cases 033–041)

The 41-case corpus now computes **CRPSS and BSS over a frozen batch of forecast cases**, not as a single-observation ratio.

For CRPSS, compute each sample's ensemble CRPS, average across the ordered sample set, then:
`CRPSS = 1 - mean(CRPS_candidate) / mean(CRPS_reference)`

For BSS, compute the candidate/reference Brier scores across the same binary event sample set, then:
`BSS = 1 - mean(BS_candidate) / mean(BS_reference)`

Both require:
- named candidate and reference forecast IDs;
- matching target/event definition;
- matching valid-time definition, lead time and evaluation population;
- identical, unique, ordered sample IDs for candidate and reference arrays;
- explicit sample-set SHA-256 in the computed output/receipt.

Reference score zero/near-zero returns Undefined. Mismatched scope or sample identities return NotComparable. Raw candidate/reference mean scores are reported beside the skill score. These are still synthetic fixtures; the named B0 baseline is an identity placeholder, not a real hydrologic forecast.
