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

The current suite includes 17 synthetic tests covering zero flux, inflows/outflows, exact depletion, over-capacity conditions, invalid inputs, millimeter/centimeter/liter-per-area/rate-time conversions, overflow, deterministic replay, multi-step accounting, and a fixed-seed 10,000-case conservation sweep. These are synthetic software-invariant tests, not soil observations or efficacy benchmarks.

## Calibrated constitutive relationship

The crate now also contains an opt-in van Genuchten-Mualem relationship for a measured soil, with explicitly stated units and guarded parameter domains:

- water-retention curve: van Genuchten (1980), https://doi.org/10.2136/sssaj1980.03615995004400050002x
- unsaturated hydraulic conductivity closure: Mualem (1976), https://doi.org/10.1029/WR012i003p00513

This evaluates a curve from supplied parameters; it does not estimate those parameters or infer how biochar changes them. Parameter fitting requires measured retention data, and predictions need validation against independent observations. The model is not a Richards-equation solver and does not calculate transient infiltration/drainage by itself.

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
