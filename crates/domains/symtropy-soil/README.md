# symtropy-soil

A dependency-free Rust reference kernel for auditable soil-water accounting.

## Scope

This crate implements a single-step water-storage accounting identity over a control volume. It accepts fluxes supplied by the caller and reports the resulting storage and numerical balance residual.

It does **not** model or predict infiltration, evapotranspiration, runoff, drainage, hydraulic properties, sorption, crop response, biochar performance, contaminant risk, or regulatory status. Those require independent models, measurements, calibration, and review.

## Run focused tests

From the Symtropy workspace root:

```sh
cargo test -p symtropy-soil
```

The initial suite covers zero flux, inflows/outflows, exact depletion, over-capacity conditions, invalid inputs, explicit unit conversion, overflow, deterministic replay, and multi-step accounting. These are synthetic software-invariant tests, not soil observations or efficacy benchmarks.

## Accounting identity

All quantities are equivalent millimeters over the same control area and time step:

```text
final_storage = initial_storage + precipitation + irrigation
                - evapotranspiration - runoff - drainage
```

The kernel does not silently clamp storage or invent a missing outflow. If storage would be negative, or exceed the configured capacity without the excess being accounted for as runoff/drainage, it returns an error.

## Research boundary

Passing unit tests demonstrates only that the reference accounting code obeys its implemented contract. It is not evidence of agricultural prediction accuracy, product safety, fertilizer efficacy, or authorization to apply any material.

Related research contract: [Mycelix Terra Preta Protocol and Benchmark PR #5513](https://github.com/Luminous-Dynamics/mycelix/pull/5513).
