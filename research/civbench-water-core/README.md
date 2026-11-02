# CivBench Water Core

This is the first executable reference component for CivBench: a small, headless, dependency-free Rust kernel for one settlement's water stock and allocation over one simulation tick.

It is intentionally not a civilization simulator. It exists to make one physical/accounting subsystem explicit and testable before introducing populations, institutions, ecology, or cognitive agents.

## Units and semantics

All quantities are integer litres for one discrete simulation tick:
- stored water is a stock at the beginning of the tick;
- inflow is water entering during the tick;
- household and agriculture demand are requested flows during the tick;
- spill is inflow/stock that cannot fit in storage;
- delivered water is consumed during the tick;
- the remainder is the next tick's stored stock.

The accounting identity is:

    stock_before + inflow = stock_after + spill + household_delivered + agriculture_delivered

The kernel rejects initial stock above capacity and integer overflow. It never silently clamps invalid initial state.

## Allocation priority

The caller must choose either HouseholdFirst or AgricultureFirst. This is an explicit policy parameter, not a claim about the correct moral or political ordering. The kernel reports delivered and unmet demand separately.

## What is deliberately omitted

No rainfall generation, aquifer dynamics, evaporation, contamination, conveyance loss, ecological minimum flow, inter-settlement transfer, pricing, legal rights, population growth, or historical calibration is implemented. Those require separate models and evidence. Do not infer them from this kernel.

## Run

    cargo test --manifest-path research/civbench-water-core/Cargo.toml

The crate has no external dependencies and does not require Bevy, a renderer, a network service, or a model runner.
