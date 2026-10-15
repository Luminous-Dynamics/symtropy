# Maritime Forecast Applicability v0.1

**Status:** implementation candidate / synthetic-only validation. This contract evaluates declared temporal applicability of a water-level prediction envelope. It does not validate actual forecast values or grant operational, engineering, berth or navigation authority.

## Why this gate exists

The generic receipt verifier checks declared fields and the exact local artifact bytes. It deliberately does not decide whether the data are recent enough or cover the time a scenario needs. Forecasts require distinct temporal anchors: model initialization, issue time, assessment/decision time, target time, and valid interval. Retrieval time is a separate event and cannot substitute for any of them.

The official Copernicus Marine global physics product describes a rolling time series plus ten days of forecasts updated daily. Its product metadata may therefore advertise coverage beyond today's date because forecast times extend into the future. An advertised temporal endpoint is not evidence of observations through that date, nor does recent retrieval prove a recent model run.
- [Copernicus Marine Global Ocean Physics Analysis and Forecast](https://data.marine.copernicus.eu/product/GLOBAL_ANALYSISFORECAST_PHY_001_024/description)
- [Dataset access and exact dataset identifiers](https://data.marine.copernicus.eu/product/GLOBAL_ANALYSISFORECAST_PHY_001_024/services)
- [Product IDs and dataset IDs are distinct](https://help.marine.copernicus.eu/en/articles/4703373-differences-between-copernicus-marine-products-and-datasets-and-their-identifiers)

This v0.1 implementation is intentionally narrowed to receipts whose product and domain are both water_level_prediction. It does not pretend that a product ID and dataset ID for every ocean model already fit the generic receipt. Broader forecast products should extend the receipt vocabulary/profile deliberately in a separately reviewed version.

## Offline verification

```sh
cargo run --locked -p maritime-data-receipt -- \
  --evaluate-forecast-applicability \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-water-level-forecast-profile.json \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-water-level-forecast-receipt.json
```

Optional third argument overrides the artifact file path, allowing real data to remain outside the repository. The CLI first verifies the exact receipt and artifact bytes, then binds the profile to the receipt ID, receipt digest, artifact digest, product, dataset ID/revision, and declared valid interval.

The time policy uses an explicit assessment time rather than the machine clock so results are deterministic and replayable. A caller supplies a product-specific maximum issue age and maximum forecast lead; the validator does not invent universal thresholds. A future operational policy must be selected and independently reviewed for the particular product and use. The provider-native product ID is separate from the internal receipt product kind and dataset ID; v0.1 cross-checks the internal product kind against the receipt and reports the provider-native ID without pretending that they are the same identifier.

## Decision semantics

- `TEMPORALLY_APPLICABLE_NOT_ADMITTED`: the declared run existed by assessment time, target is at or after assessment and inside the valid interval, issue age and target forecast lead are inside the declared bounds, and quality status is not unknown.
- `INDETERMINATE_NOT_ADMITTED`: a well-formed profile cannot establish temporal applicability—for example the run was not yet issued, the target is out of window, the issue is too old, the requested lead is too long, or quality is unknown.
- Invalid metadata or any receipt/profile/artifact binding mismatch is a hard error (exit status 2), not an indeterminate pass.

The source-age calculation is assessment time minus issue time. Forecast lead is target time minus model initialization time. Model initialization must not follow issue time; the valid interval must be ordered. Valid interval must match the time-coverage interval in the receipt.

All outcomes preserve:
- engineering admission: `NotAdmitted`;
- navigation, physical-truth and berth-availability inference authority: false;
- independent review required: true;
- execution performed: false.

A result only evaluates the declared metadata and policy bounds. It does not parse forecast rows, establish model skill, validate units/vertical datum beyond the generic receipt, authenticate the publisher, or qualify a physical decision.

## Frozen corpus

The corpus at `crates/tools/maritime-data-receipt/tests/fixtures/forecast-applicability-corpus-v0.1.json` mutates a synthetic baseline to test issue age, look-ahead, initialization chronology, target-window boundaries, forecast lead, unknown quality, mutable revisions, receipt binding, product identity and attempted authority escalation. The unit-test oracle separately recomputes UTC ordering/age/lead for the positive baseline.

## Claim ceiling

The sample values and times are invented. A temporally applicable synthetic test case is not a real forecast, and even a real source-bound prediction remains advisory and NotAdmitted until source quality, model suitability, uncertainty, independent engineering review and any required external authorization are separately established.
