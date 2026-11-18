# Pump/Valve Acceptance Profile Mapping — Stage 0

**Owner:** Symtropy [PUMP-VALVE-ACCEPTANCE-MAPPING-001 #5987](https://github.com/Luminous-Dynamics/symtropy/issues/5987)  
**Composes:** pump wet-bench #4959, material-accounted routes #5875, component digital thread #3331  
**Status:** dependency-free Rust applicability classifier; frozen public-metadata snapshot checked 18 November 2026.

## Scope

The evaluator decides only whether a declared product/test/facility/contract combination fits a frozen public-scope profile for:
- ISO 9906:2012 rotodynamic pump hydraulic acceptance;
- ANSI/HI 14.6-2022 rotodynamic pump hydraulic, NPSH and hydrostatic acceptance;
- ANSI/HI 11.6-2022 submersible pump mechanical/electrical integrity;
- ISO 5208:2015 metallic valve pressure-boundary/closure tests;
- ISO 5208 Amendment 1 draft (must not be used as a published requirement);
- API 598 11th edition 2023 valve inspection/pressure tests for its stated valve and seat families, with other valve types only by agreement.

It deliberately separates the submersible mechanical/electrical scope in HI 11.6 from hydraulic/NPSH/hydrostatic work in HI 14.6 and vibration in HI 9.6.4. It also models API 598's seat-family scope instead of treating every valve as interchangeable.

## Dispositions

- APPLICABLE — declared profile fields fit the frozen public-scope metadata.
- NOT_APPLICABLE — product/test/fluid/facility/seat lies outside the declared scope.
- PROFILE_INCOMPLETE — purchaser grade, fitting boundary, product-standard link, facility or seat information is missing.
- EVIDENCE_INCOMPLETE — evidence such as calibration/test record is missing or stale.
- STALE_OR_SUPERSEDED — selected profile is draft or withdrawn.
- UNSUPPORTED_CLAIM — a test-applicability profile is used to claim product release or pressure-test safety, regardless of whether the selected test standard expressly excludes safety.

An APPLICABLE result is not a test pass, conformity finding, safety case, certification, or production release.

## Frozen corpus

- 6 standard profiles.
- 31 synthetic applicability scenarios.
- Separate 31-row expected-disposition file.
- 21 Rust unit tests.
- Expected disposition totals: 11 APPLICABLE, 8 NOT_APPLICABLE, 7 PROFILE_INCOMPLETE, 1 EVIDENCE_INCOMPLETE, 1 STALE_OR_SUPERSEDED and 3 UNSUPPORTED_CLAIM.

The public metadata fields are linked in the profile fixture and the research note. No copyrighted normative clauses, numerical test limits or acceptance tolerances are encoded.

## Run

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo run --locked -- --report
```

The focused workflow pins Rust 1.96.0 and archives the report plus exact fixture/source hashes.

## Claim ceiling

This does not establish the current SABS adoption status, South African legal applicability, normative conformity, facility accreditation, measurement-system capability, pressure-test safety, pump/valve performance, product quality, or release authority. Verify exact governing standard, product type, edition, contract, purchaser grade and competent authority before any physical test.
