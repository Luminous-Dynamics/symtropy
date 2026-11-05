# WPT asset-health contract validator

**Status:** executable schema-subset and evidence-invariant validator  
**Parent:** [WPT-ASSET-HEALTH-001 #3726](https://github.com/Luminous-Dynamics/symtropy/issues/3726)  
**Benchmark artifacts:** `research/wpt-asset-health-v0/`

## Run

`cargo test --locked -p wpt-asset-health-contract`

## What is checked

- Every observation property and nested spatial point against the subset of JSON Schema keywords actually used by the checked-in contract.
- Required properties, nullable types, enumerations, numeric bounds, lowercase SHA-256 syntax, UTC timestamp shape, item schemas, unique array items and closed-object additional-property rules.
- Evidence-specific invariants: synthetic records carry SYNTHETIC_ONLY; measured records carry instrument identity, calibration receipts and a raw artifact digest; published-measured records carry a source locator; every observation explicitly classifies its thermal stimulus.
- All spatial strain, curvature, temperature and heat-flux points use the asset's declared coordinate frame. A nonempty spatial profile requires a declared asset frame.
- Scenario case identities are unique and evidence classes are recognized.

The checked-in sample is deliberately synthetic and is not industry measurement data.

## Important limitation

This crate implements only the exact JSON Schema subset used by this contract. It is not a general-purpose JSON Schema 2020-12 validator, a standards-certified schema engine, or a condition/RUL estimator. Its date-time check accepts the contract's required UTC YYYY-MM-DDTHH:MM:SSZ shape; it is not a general RFC 3339 parser. A future schema keyword requires an explicit validator implementation and a negative-control test before use.

The cross-field rules are evidence hygiene, not physical safety rules. No condition estimate authorizes energization or overrides local protection, FOD, exposure, stability, identity or metrology gates.
