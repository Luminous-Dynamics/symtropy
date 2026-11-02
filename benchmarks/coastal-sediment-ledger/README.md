# Coastal sediment ledger reference evaluator v0.1

Standalone, dependency-free Rust reference evaluator for
[Symtropy #3279](https://github.com/Luminous-Dynamics/symtropy/issues/3279).

## Run

```sh
cargo test --locked --manifest-path benchmarks/coastal-sediment-ledger/Cargo.toml
```

## Contract

- Transfers account **dry-solid mass in integer grams**. Wet dredged/slurry
  volume is a distinct type; no implicit density or solids-fraction conversion
  exists.
- Lots have stable IDs, grain classes, contamination states and optional
  characterization evidence.
- Every transfer is sequential, lot-bound and atomic. A store cannot transfer
  more mass than it holds; transfer IDs are unique.
- Boundary inflows/outflows and internal stock are accounted separately.
- The conservation oracle is `external inflow = external outflow + internal stock`.
- Placement eligibility is a narrow synthetic rule check; it is not a permit,
  environmental approval, retention forecast or real material-suitability
  determination.

## Claim ceiling

This crate models accounting only. It does not compute bed shear, settling,
cohesive flocculation, waves, currents, shoaling, scour, shoreline change or a
real dredging quantity. Field claims require calibrated survey data and
specialist models with declared validity envelopes. Unknown or contaminated
material cannot silently become eligible for placement.
