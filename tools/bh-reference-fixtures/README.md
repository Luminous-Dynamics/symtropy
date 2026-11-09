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
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- compare-radial reference-a.json reference-b.json 1e-14 1e-9
```

`compare-radial` validates both inputs, then compares only when problem identity, conventions, radial sample nodes and per-channel coverage match. It performs no interpolation or normalization fitting. It reports per-channel maximum absolute error, maximum tolerance-scaled error, sample count and count of out-of-tolerance values. The explicit absolute and relative tolerances are required CLI arguments.

Statuses are `unmatched` (problem/convention mismatch), `unresolved` (different radial nodes or data coverage), `numerically_agreeing`, and `numerically_disagreeing`.

Exit codes: `0` numerical agreement, `2` malformed input/invalid fixture or tolerance, `4` problem or convention mismatch, `5` numerical disagreement, `6` comparison unresolved.
