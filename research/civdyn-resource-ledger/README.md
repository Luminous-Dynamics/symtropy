# CivDyn Resource Ledger v0.1

A small, deterministic, dependency-free resource accounting kernel for the Civilizational Systems Science programme.

## What it does

- represents water (m³), energy (kWh) and food (kg) as separate stocks;
- records each region's initial stock, exogenous production, imports, exports, essential consumption and closing stock;
- applies explicit, ordered inter-region transfers;
- supports unrestricted and reserve-preserving transfer policies;
- computes service coverage for water, energy and food separately;
- checks conservation independently for water (m³), energy (kWh) and food (kg) after every step, using separate absolute tolerances and same-unit relative scales;
- reports three native-unit conservation residuals; it never reduces m³, kWh and kg to one cross-unit maximum;
- keeps zero-demand coverage undefined rather than silently assigning a performance score.

It intentionally does **not** simulate institutions, cognition, prices, ecology or technology yet. Those must be layered on only after the stock-flow accounting is validated.

## Run

```sh
cargo test --locked --manifest-path research/civdyn-resource-ledger/Cargo.toml
cargo run --locked --manifest-path research/civdyn-resource-ledger/Cargo.toml --bin water-energy-shock-demo
cargo test --locked --manifest-path research/civdyn-resource-ledger/Cargo.toml --bin css001-water-energy-shock
cargo run --locked --manifest-path research/civdyn-resource-ledger/Cargo.toml --bin css001-water-energy-shock
```

## Scenario boundary

The binary uses a synthetic two-region water/energy shock fixture. It is an implementation demonstration, not a forecast for Johannesburg, Cape Town, South Africa or any other real region. Production, demand, inventories and transfers are deliberately fictional and identified as such in the output.

## CSS-001 cross-project benchmark

The `css001-water-energy-shock` binary compares no transfer, fixed solidarity and adaptive current-service-deficit allocation on the same synthetic two-region/four-tick water-energy-food shock. It records coverage, unmet demand, closing stocks, transfer requests/deliveries and conservation residuals. The adaptive controller is a simple deterministic rule: satisfy the receiver's current-tick service deficit only when the donor can still meet its own essential demand and retain a half-tick stock buffer. This is not a learned agent or optimal-control result.

The common scenario manifest and protocol live in the Symthaea CivBench branch: [CSS-001 protocol](https://github.com/Luminous-Dynamics/symthaea/blob/research/civbench-social-learning-core-2026-11/research/civbench-social-learning-core/CSS_001_WATER_ENERGY_SHOCK_PROTOCOL_V0_1.md) and [scenario manifest](https://github.com/Luminous-Dynamics/symthaea/blob/research/civbench-social-learning-core-2026-11/research/civbench-social-learning-core/CSS_001_WATER_ENERGY_SHOCK_MANIFEST_V0_1.json).

The analytical expectation is zero service-floor breaches for fixed/adaptive transfer and four for no transfer. It remains a hand-derived expectation until exact-head Rust tests and captured output verify it. Resource transfers must be reported separately by m³, kWh and kg; never sum them into one dimensionless volume.

## Required next step

Attach real source snapshots and units to initial stocks, production, demand, capacity, outages and environmental flow constraints. Calibrate/hindcast before interpreting scenario outputs. The companion engineering issue is Symtropy #3390.
