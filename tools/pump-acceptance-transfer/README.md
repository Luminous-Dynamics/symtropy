# Pump Acceptance-to-Service Transferability — Stage 0

**Owner:** [PUMP-ACCEPTANCE-TO-SERVICE-TRANSFER-001 #6008](https://github.com/Luminous-Dynamics/symtropy/issues/6008)  
**Composes:** pump-system service benchmark #4959, clean-water test-cell basis #4994, product-standard applicability #5987, MTConnect/QIF interoperation #3025, and composed manufacturing twin VVUQ #3121.  
**Status:** dependency-free Rust synthetic evaluator; no physical product/test claim.

## Paradigm

A factory acceptance test does not automatically establish installed pump-service capability. This evaluator requires an exact standard-applicability receipt, product serial/configuration, test boundary, fluid, speed, calibration and measured Q-H curve coverage. It separately requires manufacturer NPSHr and site/system NPSHa evidence under an application-specific margin profile.

It never equates:
- factory **NPSH3** test result;
- manufacturer-supplied **NPSHr**;
- site/system-specific **NPSHa**.

No speed-law, fluid correction, curve extrapolation, boundary correction or fuzzy identity join is inferred.

## Frozen Stage-0 corpus

- 31 synthetic acceptance-to-service scenarios.
- 93 Q-H test-point interval rows.
- 31 independently authored expected dispositions.
- 30 authored Rust unit tests.
- Dependency-free Rust library and deterministic CLI.
- 22-column evidence-carrying TSV report.
- Focused Rust 1.96.0 workflow with locked build/tests/Clippy, report-line assertion, SHA-256 manifest and 30-day evidence artifact.

Expected dispositions:

| Disposition | Cases |
|---|---:|
| TRANSFERABLE_WITHIN_DECLARED_ENVELOPE | 2 |
| IDENTITY_MISMATCH | 4 |
| OUT_OF_DOMAIN | 4 |
| NOT_TRANSFERABLE | 4 |
| EVIDENCE_INCOMPLETE | 7 |
| PROFILE_INCOMPLETE | 7 |
| INDETERMINATE | 2 |
| INSUFFICIENT_NPSH_MARGIN | 1 |

## Q-H logic

Only bracketed, source-bound test points are admitted. Interpolation is linear in the supplied flow/head fields and uses outward conservative integer rounding:
- lower head bound rounds down;
- upper head bound rounds up;
- no extrapolation;
- repeated, unordered or invalid curve points do not qualify.

The evaluator compares the conservative measured pump-head interval with the declared required-head interval. If the intervals overlap without proving adequate head, it returns INDETERMINATE.

The v0.1 unit profile is explicitly `SI_MLPS_MM`: flow in mL/s and head/NPSH in mm. No implicit conversion is allowed.

## NPSH logic

The factory NPSH3 source, manufacturer NPSHr curve source and site NPSHa source must be distinct and independently identified. The manufacturer serial/configuration/speed must match the tested pump. Site NPSHa must have a source and method reference. A separately versioned application margin profile supplies the required margin.

The conservative comparison is:

`NPSHa_low >= NPSHr_high + margin_required` → NPSH evidence clears this declared check.

`NPSHa_high < NPSHr_low + margin_required` → INSUFFICIENT_NPSH_MARGIN.

Otherwise → INDETERMINATE.

This mathematical check does not select or endorse the required margin. That value must come from an applicable, externally authorized application profile.

## Run

```sh
cargo fmt --manifest-path tools/pump-acceptance-transfer/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/pump-acceptance-transfer/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/pump-acceptance-transfer/Cargo.toml --all-targets
cargo run --locked --manifest-path tools/pump-acceptance-transfer/Cargo.toml -- --report
```

## Claim ceiling

The synthetic evaluator does not select a pump, calculate NPSHa, create a hydraulic network model, qualify a test facility, validate real pump curves, establish cavitation immunity, verify transient safety, approve pressure/electrical design, certify a product, authorize installation, or prove delivered service. Only a source-pinned T1/T2/T3 campaign with competent facility/engineering review and independent measurement evidence can advance those claims.
