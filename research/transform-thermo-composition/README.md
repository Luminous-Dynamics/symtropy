# Physical Transformation Thermodynamics Composition Reference

Dependency-free Rust 1.96 reference for checking cross-owner receipt composition.

## Purpose

This crate checks that topology/material, first-law and supported thermal-entropy receipts refer to the same exact transformation event, world revision, boundary and interval. It does not calculate a physical transformation, energy balance, entropy or exergy.

## Included

- Exact event/world/predecessor/successor revision binding.
- Shared boundary and interval binding across energy and entropy receipts.
- Owner-status consistency checks for first-law residual intervals.
- Material disposition / residual consistency without recalculating the material ledger.
- Body + bath entropy closure and second-law status consistency for the existing bounded thermal reference.
- Optional exergy closure bound to the exact entropy artifact and declared environment reference temperature.
- Evidence artifact digest syntax, unique artifact identities and evidence-class preservation.
- Vector-valued material, energy, entropy, exergy and overall dispositions.
- Fail-closed handling of incomplete, inconclusive, imbalanced, violated and not-applicable cases.

## Boundaries

This is a composition verifier, not a second energy ledger, thermal model, fracture solver, materials ledger, exergy engine or evidence authenticator. It cannot prove that an owner chose a complete physical system boundary, measured every flow, included every stored-energy domain, or applied a valid constitutive model. A valid hash-shaped string is not source authentication.

The research contract is tracked in Symtropy #4886 and PR #4887. The intended owner composition is #2209/#2238, #3594, #3597, #3577 and the legacy migration #1693.

## Local verification

```sh
cargo +1.96.0 fmt --manifest-path research/transform-thermo-composition/Cargo.toml -- --check
cargo +1.96.0 test --manifest-path research/transform-thermo-composition/Cargo.toml --locked
cargo +1.96.0 clippy --manifest-path research/transform-thermo-composition/Cargo.toml --all-targets --locked -- -D warnings
```

A passing software contract does not constitute physical validation, structural certification or authorization for construction/demolition.
