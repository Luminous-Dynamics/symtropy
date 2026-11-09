# BH reference fixtures

This crate defines a strict JSON exchange contract for independently produced Kerr / Teukolsky reference data.

It is deliberately not a Teukolsky solver. Its purpose is to make cross-engine comparisons well-posed before a numerical discrepancy is computed.

## Initial contract

- Pin the field sector `(s, ell, m)`, signed `a/M`, mass, and dimensionless complex `M omega`.
- Pin Fourier sign, Boyer–Lindquist radial coordinate, frequency normalization, angular-eigenvalue branch, and producer-described `In` / `Up` normalizations.
- Store complex values as explicit objects with named `re` and `im` components.
- Require finite radial samples outside the outer horizon, strictly increasing in `r/M`.
- Preserve producer version, source revision, artifact digest, timestamp, and precision.

`ReferenceFixture::validate` checks structural/domain integrity. `assess_compatibility` checks whether two fixtures describe the same pinned problem. A `Matched` compatibility result does not mean the numerical values agree.

A Fourier-sign, coordinate, branch, frequency-normalization, or boundary-normalization mismatch returns `Unmatched`; it is not treated as a solver failure.

## Scope limits

The first schema covers radial reference fixtures and a separate Kerr QNM-sequence record. It does not yet encode source orbits, fluxes, higher derivatives, arbitrary charts, branch-cut metadata, or complete sourced waveforms. These require explicit typed definitions and fixtures. Unknown fields are rejected to prevent silent schema drift.

QNM sequence records preserve sequence position independently of an optional overtone index. A prolate/oblate family label requires a multi-mode asymptotic evidence record, but the schema only checks that evidence is present and finite. It does not prove the family classification or assess whether the measured residuals are small enough. Unresolved and Ambiguous remain first-class outcomes.

Reference fixtures are data, not executable instructions. Do not load untrusted notebooks or code from a fixture payload.

## CLI

The crate builds a small command-line validator:

```sh
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- validate-radial fixture.json
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- validate-qnm qnm-sequence.json
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- compare-radial reference-a.json reference-b.json
```

`compare-radial` validates both inputs before comparing identity/convention metadata. It prints `matched` or `unmatched` and the differing fields. It deliberately does not calculate numerical errors: a later comparison layer must first define interpolation, common sample nodes, derivative normalization and uncertainty propagation.

Exit codes: `0` valid/matched, `2` malformed input or invalid fixture, `4` valid but convention-incompatible fixtures.
