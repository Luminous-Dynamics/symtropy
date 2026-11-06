# Symtropy Fragment Transport Bench

Standalone Rust validator and descriptive evaluator for measured fragmentation-to-transport windows.

## Scope

The crate validates:
- common task, material, site, machine, installed transport, energy and resource-ledger references;
- synchronized positive-duration measurement windows;
- independently measured dry-solid mass values and calibration/lineage references;
- ordered, contiguous, non-overlapping particle-size bins whose mass reconciles to generated mass within stated uncertainty;
- generated/recovered/residual/rejected mass balance;
- exactly one capacity interval for capture, convey, separate and discharge;
- measured capacity evidence bound to the exact installed transport configuration;
- robust comparison of generation-rate and stage-capacity intervals.

It reports per-window mass-balance status, PSD closure, measured output rate, extraction efficiency and possible/definite stage capacity constraints.

## Important boundaries

- Factory/nameplate capacity cannot support an installed-path capacity claim.
- Nested cumulative size thresholds are not bins and cannot be summed.
- Missing/unaccounted mass is never silently zero.
- The uncertainty check uses a conservative sum of stated uncertainty magnitudes; it is not a probabilistic coverage claim.
- “No capacity bottleneck evidenced” is not equivalent to “breakage limited”.
- The crate does not simulate CFD/DEM, classify real blockages, issue operating setpoints or command equipment.

## Run

    cargo test --manifest-path crates/domains/symtropy-fragment-transport-bench/Cargo.toml

The bundled fixture is entirely synthetic. It does not validate an actual mine, hose line, blower or cutting system.
