# WPT asset-health contract validator

**Status:** executable schema-subset and evidence-invariant validator  
**Parent:** [WPT-ASSET-HEALTH-001 #3726](https://github.com/Luminous-Dynamics/symtropy/issues/3726)  
**Benchmark artifacts:** `research/wpt-asset-health-v0/`

## Run

`cargo test --locked -p wpt-asset-health-contract`

## What is checked

- Every observation property and nested spatial point against the subset of JSON Schema keywords actually used by the checked-in contract.
- Required properties, nullable types, enumerations, numeric bounds, lowercase SHA-256 syntax, UTC timestamp shape, item schemas and closed-object additional-property rules.
- Evidence-specific invariants: synthetic records carry SYNTHETIC_ONLY; measured records carry instrument identity, calibration receipts and a raw artifact digest.
- All spatial strain, curvature, temperature and heat-flux points use the asset's declared coordinate frame. A nonempty spatial profile requires a declared asset frame.
- Scenario case identities are unique and evidence classes are recognized.

The checked-in observation and lifecycle samples are deliberately synthetic and are not industry measurement data.

The asset contract currently has 26 Rust unit tests, including the original schema-subset checks, duplicate `uniqueItems` rejection, unsupported-keyword fail-closed behaviour, impossible calendar-date rejection, ambient min/max consistency and lifecycle outcome/split tests.

## Important limitation

This crate implements only the exact JSON Schema subset used by this contract. It is not a general-purpose JSON Schema 2020-12 validator, a standards-certified schema engine, or a condition/RUL estimator. Its date-time check accepts the contract's required UTC YYYY-MM-DDTHH:MM:SSZ shape; it is not a general RFC 3339 parser. A future schema keyword requires an explicit validator implementation and a negative-control test before use.

The cross-field rules are evidence hygiene, not physical safety rules. No condition estimate authorizes energization or overrides local protection, FOD, exposure, stability, identity or metrology gates.


## Time-to-event and censored-asset validation

The validator also checks `asset-lifecycle-outcome.schema.json` using `validate_lifecycle_outcomes`. The survival contract records one first-event outcome per physical asset and distinguishes:

- `TARGET_FAILURE`: a declared target failure mode and event time;
- `RIGHT_CENSORED`: observation ended before the target event, with a censor reason and no event code;
- `COMPETING_EVENT`: a distinct event such as planned replacement or road reconstruction that prevents observing the target event.

The evaluator recomputes duration from fixed-format UTC start/end timestamps and rejects mismatches. It rejects duplicate assets for this first-event estimand and rejects leakage of asset, segment, material-lot, installation, route or environmental-campaign groups across TRAIN/CALIBRATION/TEST splits. Repeated observations are not independent survival subjects.

The included lifecycle fixture is synthetic and contains one target failure, one right-censored record and one competing event. It is not an observed pavement-lifetime dataset. The custom date parser deliberately supports only fixed UTC `YYYY-MM-DDTHH:MM:SSZ` timestamps with seconds 00–59; it is not a general RFC 3339 parser.


Lifecycle evidence provenance is also evidence-class aware: MEASURED outcomes require one or more source-record IDs and a raw artifact SHA-256 digest; PUBLISHED_MEASURED outcomes require source-record IDs and a non-empty source locator. A date and event code alone cannot promote an outcome into measured evidence. The schema-subset integer predicate follows JSON Schema's mathematical semantics and accepts integral values such as `1.0`, while rejecting fractional values such as `1.5`.
