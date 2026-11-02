# CivDyn Resource Ledger v0.1

A small, deterministic, dependency-free resource accounting kernel for the Civilizational Systems Science programme.

## What it does

- represents water (m³), energy (kWh) and food (kg) as separate stocks;
- records each region's initial stock, exogenous production, imports, exports, essential consumption and closing stock;
- applies explicit, ordered inter-region transfers;
- supports unrestricted and reserve-preserving transfer policies;
- computes service coverage for water, energy and food separately;
- checks conservation for every region/resource after every step;
- keeps zero-demand coverage undefined rather than silently assigning a performance score.

It intentionally does **not** simulate institutions, cognition, prices, ecology or technology yet. Those must be layered on only after the stock-flow accounting is validated.

## Run

```sh
cargo test --locked --manifest-path research/civdyn-resource-ledger/Cargo.toml
cargo run --locked --manifest-path research/civdyn-resource-ledger/Cargo.toml --bin water-energy-shock-demo
```

## Scenario boundary

The binary uses a synthetic two-region water/energy shock fixture. It is an implementation demonstration, not a forecast for Johannesburg, Cape Town, South Africa or any other real region. Production, demand, inventories and transfers are deliberately fictional and identified as such in the output.

## Required next step

Attach real source snapshots and units to initial stocks, production, demand, capacity, outages and environmental flow constraints. Calibrate/hindcast before interpreting scenario outputs. The companion engineering issue is Symtropy #3390.
