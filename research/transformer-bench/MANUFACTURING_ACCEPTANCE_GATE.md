# Manufacturing route and metrology acceptance gate

**Status:** research validator with synthetic fixtures. It does not authorize product release or establish standards compliance.

## Purpose

The manufacturing ledger already checks product/component genealogy and event chronology. It did not answer the next engineering question: did this exact serial follow its approved route, did every required measurement exist, was the instrument calibration valid at measurement time, and did the full measurement uncertainty interval fit inside the approved engineering limits?

The new manufacturing-acceptance binary adds a fail-closed decision-support screen over those inputs.

## Contract

Run:

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin manufacturing-acceptance -- \
  research/transformer-bench/fixtures/manufacturing-ledger-synthetic.csv \
  research/transformer-bench/fixtures/manufacturing-product-route-synthetic.csv \
  research/transformer-bench/fixtures/manufacturing-route-plan-synthetic.csv \
  research/transformer-bench/fixtures/manufacturing-test-plan-synthetic.csv \
  research/transformer-bench/fixtures/manufacturing-calibrations-synthetic.csv \
  research/transformer-bench/fixtures/manufacturing-measurements-synthetic.csv \
  /tmp/manufacturing-acceptance-report.csv
```

Inputs are strict CSV with canonical UTC timestamps and exact headers. Quoted CSV fields are rejected rather than silently misparsed. The tool checks:

1. Every event-bearing product has one product-to-route assignment and matching family/serial identity.
2. Every required route step has a matching product operation, component type and event type with the required disposition.
3. A failed test is resolved only by a later rework on the same component followed by a later passing repeat of the same test operation.
4. Every required test-plan measurand exists for each component of its required type.
5. Test plan and measurement units agree; measured standard uncertainty does not exceed the declared maximum.
6. Calibration status is CURRENT, instrument and calibration class match the test plan, and the measurement timestamp lies inside the calibration validity interval.
7. A measurement passes only when its complete declared coverage interval, value ± coverage_factor × standard_uncertainty, lies inside the approved lower/upper limits.

The three interval outcomes are PASS, FAIL and INDETERMINATE. Partial overlap with a limit is not rounded into a pass. Ineligible calibration/evidence, missing measurements and unresolved failures block eligibility for authorized review.

## Interpretation

The report field eligible_for_authorized_review is only a deterministic screen result. It is not a conformity certificate, QA sign-off, statutory approval, or authority to ship/energize. Final release must remain tied to the applicable purchaser-approved quality plan, product-specific standards, validated instruments, actual evidence-file verification, and an accountable release authority.

The route plan currently supports one route revision per assigned product. The route rows are completeness requirements; global precedence/dependency graphs for parallel factory work are not yet modeled. Required measurement checks apply one required measurand to every component of the declared type in the product genealogy.

SHA-256 fields are syntax-checked only. The binary does not read/hash artifact bytes, verify signatures, certify source provenance or validate that a route/test-plan digest identifies a controlled approved document. The existing evidence-receipt tool must be used for artifact-byte integrity; authorization and provenance need their own contracts.

## Synthetic fixture and tests

The transformer fixture includes a failed winding-resistance test followed by rework and a passing retest. The measurement table includes core stack height, winding resistance, tank leak rate and no-load loss; every result is inside the uncertainty-guarded limits with a synthetic current calibration certificate.

Unit tests cover guard-band pass/fail/indeterminate classification, calibration status/instrument/class/time checks, required rework followed by a later passing repeat, SHA-256 syntax and strict UTC/leap-day validation. Fixtures are synthetic plumbing tests, not physical data or standards tolerances.

## Next research step

Add multi-revision approved route manifests with immutable manifest content receipts; calibration uncertainty/certificate verification; MSA/Gage R&R evidence; route precedence and rework/NCR disposition logic; signed human authorization; and explicit product-specific standards/purchaser inspection plans. Then connect only the already-eligible measurement candidates to Symthaea's heteroscedastic EIG selector. A model must never create eligibility or relax an acceptance limit.

## Industry alignment

NISTIR 8620 (2026) identifies interoperability, verification/validation/uncertainty quantification and cybersecurity as persistent digital-twin challenges. ISO 23247-6:2026 defines integrated, unified and federated digital-twin composition for manufacturing. This implementation is a narrow, inspectable contract slice; it does not claim ISO conformity.

- NISTIR 8620: https://doi.org/10.6028/NIST.IR.8620
- ISO 23247-6:2026: https://www.iso.org/standard/94635.html
- IEC 60076 transformer test requirements must be applied using the relevant current edition, product type and approved purchaser specification; this generic fixture does not implement those test tolerances.
