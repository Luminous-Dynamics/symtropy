# Pump/Valve Acceptance Profile Mapping v0.1

**Owner:** Symtropy [PUMP-VALVE-ACCEPTANCE-MAPPING-001 #5987](https://github.com/Luminous-Dynamics/symtropy/issues/5987)  
**Composes:** [PUMP-SYSTEM-SERVICE-BENCH-001 #4959](https://github.com/Luminous-Dynamics/symtropy/issues/4959), [MATERIAL-ACCOUNTED-ROUTES-001 #5875](https://github.com/Luminous-Dynamics/symtropy/issues/5875), and [COMPONENT-DIGITAL-THREAD-001 #3331](https://github.com/Luminous-Dynamics/symtropy/issues/3331).

This is a synthetic applicability classifier for a frozen, public-metadata snapshot checked on 18 November 2026. It does not encode normative test thresholds or reproduce copyrighted standard clauses. It does not establish SABS adoption, product conformity, pressure-test safety, certification or release.

## Mapped standard profiles

| Profile | Public status snapshot | Product/test scope | Applicability limits |
|---|---|---|---|
| ISO 9906:2012 Ed.2 | Published; ISO catalogue stage 90.60 / under review | Rotodynamic centrifugal, mixed-flow and axial pumps; hydraulic performance; liquids behaving as clean cold water | Customer acceptance at pump test facilities; grade and boundary must be pinned; not every pump family or process fluid |
| ANSI/HI 14.6-2022 | Most recent listed HI edition | Rotodynamic hydraulic performance, NPSH and hydrostatic testing | Test-facility acceptance, clear-water behavior; fittings require prior agreement; specialized types may have separate standards |
| ANSI/HI 11.6-2022 | Current published HI edition | Rotodynamic submersible mechanical/electrical integrity | Does not own hydraulic performance/NPSH/hydrostatic (moved to HI 14.6) or vibration (HI 9.6.4) |
| ISO 5208:2015 Ed.4 | Confirmed in 2025; remains current | Metallic industrial valve pressure-boundary integrity and closure tightness | Must be used with applicable valve product standard; explicitly excludes pressure-test safety |
| ISO 5208 Amd.1 draft | Under development | Not a published acceptance basis | Cannot be selected as a published requirement |
| API 598 11th Ed. (Feb. 2023) | Listed current | Gate, globe, plug, ball, check and butterfly valves with its defined seat families | Supplements API standards that reference it; other types only by purchaser/manufacturer agreement |

### Primary sources

- ISO 9906:2012: https://www.iso.org/standard/41202.html
- ANSI/HI 14.6-2022: https://webstore.ansi.org/standards/hi/ANSIHI142022
- ANSI/HI 11.6-2022: https://webstore.ansi.org/standards/hi/ANSIHI112022
- Hydraulic Institute catalogue: https://www.pumps.org/what-we-do/standards/
- ISO 5208:2015: https://www.iso.org/standard/65111.html
- ISO/TC 153 catalogue (including Amendment 1 status): https://www.iso.org/committee/53156/x/catalogue/
- API 2025 publications catalogue (API 598 11th edition): https://www.api.org/-/media/files/publications/2025-catalog/2025_pubs%20catalog.pdf

### South African procurement evidence

Project specifications and tender documents cite SANS/ISO 9906 in particular pump-procurement contexts:
- O. R. Tambo District Municipality pump specification: https://ortambodm.gov.za/wp-content/uploads/2025/05/GIBB003-REV-1-1-General-Pumping-Equipment.pdf
- eTenders project specification references ISO 9906:2012: https://www.etenders.gov.za/home/Download/?blobName=69afdd81-db56-4b78-b5bd-bb5e65bbadb5.pdf&downloadedFileName=C3.4.2.1+-+Particular+Specifications+WPE+Rev+1.pdf
- Eskom 2024 refurbishment evaluation references ISO 9906 Grade 2B test-facility evidence for its centrifugal pump scope: https://www.etenders.gov.za/home/Download/?blobName=1ae87bd3-241c-4ff7-9922-da8e3e059c26.pdf&downloadedFileName=Technical+Evaluation+Strategy+for+Refurbishment+of+Vertical+Sandfilter+and+Anion+Supply+Pumps.pdf

These are evidence of project-specific procurement practice, not proof of universal South African applicability, the current SABS catalogue adoption status, or one mandatory grade. The applicable tender/contract and SABS record must be checked for each project.

## Evaluator semantics

Pressure-test safety is an unsupported claim under every mapped test profile. The classifier never interprets the absence of an explicit exclusion as permission to infer safe pressurization.

The classifier is also temporally scoped. Each standard profile binds `catalog_snapshot_date`; each scenario binds `assessment_as_of`. A mismatch returns EVIDENCE_INCOMPLETE, because a current public catalogue snapshot cannot prove historical standard status. Purchaser agreement, fitting agreement and product-standard presence flags must be paired with exact reference identifiers. This verifies declared binding consistency only; it does not authenticate the referenced contract or standard record.

The classifier reports only:
- `APPLICABLE`: declared metadata passes this finite scope profile;
- `NOT_APPLICABLE`: product/test/fluid/facility lies outside this profile;
- `PROFILE_INCOMPLETE`: required grade, fitting agreement, product-standard link or test basis is missing;
- `EVIDENCE_INCOMPLETE`: required evidence is missing/stale (for example calibration);
- `STALE_OR_SUPERSEDED`: selected profile is draft or withdrawn;
- `UNSUPPORTED_CLAIM`: applicability is being used to claim pressure-test safety or product release, under any mapped standard.

It does not compute conformance, test tolerances, uncertainty, pressure values or product release. A repeated source URL is not test evidence; the exact unit, revision, facility, measurement, calibration and external authority must be independently bound.

## Run

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo run --locked -- --report
```

The synthetic fixture corpus contains 35 cases with a separate expected-disposition file, including missing agreement references, missing product-standard references and historical assessments without a matching catalogue snapshot. The QUALIFIED_TEST_FACILITY prerequisite is imposed by this local profile; it is not presented as normative text from the referenced standards. All standard-status dates and source links are frozen; they must be refreshed against the official catalogue before a real contract or physical test.
