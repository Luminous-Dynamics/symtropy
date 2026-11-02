# TBM ring metrics reference evaluator v0.1

Dependency-free Rust metrics layer for [TBM-ODD-ENERGY-WEAR-001 #3305](https://github.com/Luminous-Dynamics/symtropy/issues/3305).

## Run

```sh
cargo test --locked --manifest-path benchmarks/tbm-ring-metrics/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/tbm-ring-metrics/Cargo.toml --all-targets -- -D warnings
```

## Input and output definitions

- Penetration per revolution = measured axial advance (mm) / cutterhead revolutions.
- Mechanical specific excavation energy = mechanical excavation energy (Wh) / excavated volume (L), numerically equal to kWh/m³.
- Whole-site electrical energy per accepted excavation volume = site meter energy (Wh) / accepted volume (L), numerically equal to kWh/m³.
- Cutter wear rate = radial radius loss (µm) / interval excavated volume (L), numerically equal to mm/m³.

Mechanical excavation energy and whole-site electrical meter energy are separate fields and are never substituted for one another. The site-meter boundary must be declared by the data provider; it is not inferred from installed machine ratings.

Cutter wear uses two measurements of the same cutter and a declared expanded uncertainty on the radial loss. The reported interval is a simple bounded measurement interval, not a probability distribution or remaining-life model.

## Claim ceiling

This crate only computes dimensional metrics from supplied values. It does not simulate rock breakage, predict TBM performance or cutter life, identify geology, optimize machine parameters, or authorize work. The caller remains responsible for measurement provenance, calibration, accepted-volume definitions, data-quality review and engineering interpretation.

The unit tests are synthetic hand-checkable arithmetic fixtures, not a field dataset.
