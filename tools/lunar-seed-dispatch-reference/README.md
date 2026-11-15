# Lunar Seed-Base Dispatch Reference v0.1

A dependency-free Rust discrete-interval dispatch and accounting reference for a synthetic lunar seed-base. It compares a greedy policy with a forecast-reserve policy over the same generation forecast, critical habitat loads, crop-light window, battery limits and one indivisible ISRU batch.

## Run

```sh
cargo test
cargo run --release
cargo run --release -- --input fixtures/synthetic-lunar-seed-base-reserve-v0-1.input
```

## Frozen synthetic case

- 24 one-hour intervals.
- 40,000 Wh generation in intervals 0–18, zero in intervals 19–23.
- 20,000 Wh survival-critical demand in every interval.
- 5,000 Wh farm-light demand in intervals 0–15 only (16-hour declared light window).
- 120,000 Wh battery capacity and initial SOC.
- 95% charge and discharge efficiencies, expressed in basis points.
- 50,000 Wh internal discharge limit and 30,000 Wh charging input limit per interval.
- 5-hour forecast-reserve horizon.
- One 60,000 Wh indivisible ISRU batch at interval 18; synthetic 100,000 g feed is split into 5,000 g product and 95,000 g residue.

This intentionally creates a counterexample. Immediately before the five-hour generation gap, the greedy controller spends battery energy to execute the productive batch. The forecast-reserve controller defers that batch because it would violate the stored-energy reserve needed for the next five critical-load intervals.

## Accounting semantics

- Energy uses integer Wh per one-hour interval; instantaneous power is not inferred beyond the one-hour average.
- Battery charge/discharge efficiency is expressed in integer basis points.
- Battery SOC is the stored-side inventory; bus output and bus charge input are recorded separately.
- Critical habitat service is prioritized ahead of farm lighting; farm lighting is only requested inside the declared light window; an indivisible process batch runs only if its complete energy can be supplied.
- The reserve estimator covers the next N intervals of critical load against the stated generation forecast. It does not forecast crop lighting, process loads or stochastic faults.
- Every interval independently reconciles generation + battery bus output against served loads, batch energy, charge input and curtailment.
- Battery SOC change is independently recomputed from internal withdrawal and stored charge.
- An executed batch must close its synthetic feed/product/residue mass ledger. A deferred batch consumes no feed.

## Results the fixture is designed to test

- Greedy: executes the ISRU batch and subsequently incurs unserved critical energy during the forecast generation gap.
- Forecast-reserve: defers the batch, maintains declared critical service and serves the declared farm-light schedule.
- Both traces must close energy, SOC and executed-batch mass balances exactly.

These are expected synthetic behavior tests, not evidence that a real lunar settlement can operate on a 40 kWe source. The generation profile is a scenario input inspired by NASA's developing 40 kWe-class Fission Surface Power concept, not a supplied or qualified reactor. The farm-light profile is a synthetic load derived for control testing; no crop physiology or farm yield is modeled.

## Composition

- [#1970 LUNAR-SEED-UTILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/1970) owns this dispatch/control reference.
- [#3985 PHOTON-DELIVERY-001](https://github.com/Luminous-Dynamics/symtropy/issues/3985) owns source-to-canopy DLI and LED/optical photon calculations; this tool only consumes a declared interval lighting load.
- [#3957 CEA-FULL-BOUNDARY-001](https://github.com/Luminous-Dynamics/symtropy/issues/3957) owns whole-facility energy, thermal, water and accepted-food comparisons.
- [#3952 CAVERN-BIOLOOP-001](https://github.com/Luminous-Dynamics/symtropy/issues/3952) owns habitat/farm integration and no-resupply semantics.

## Evidence ceiling

Synthetic discrete dispatch and conservation accounting only. No reactor, battery, crop physiology, life-support closure, lunar-site, ISRU yield, safety or flight qualification is implied.
