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

The first schema does not claim to encode every BHPT product. Source orbits, fluxes, higher derivatives, arbitrary charts, branch-cut metadata, and QNM pole records will be added only with explicit typed definitions and fixtures. Unknown fields are rejected to prevent silent schema drift.

Reference fixtures are data, not executable instructions. Do not load untrusted notebooks or code from a fixture payload.
