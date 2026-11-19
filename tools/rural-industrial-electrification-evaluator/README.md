# Rural and Industrial Electrification Evaluator v0.1

This standalone Rust tool screens whether one declared shift's energy demand fits a derated nominal battery and whether the required recharge fits the declared site-power and charging window. It is deliberately small enough to inspect independently.

## Run

From the repository root:

```sh
cargo test --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml
cargo run --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml -- \
  tools/rural-industrial-electrification-evaluator/fixtures/benchmark_v0_1.json
```

The reports are JSON on stdout. The fixture values are wholly synthetic and illustrate feasible, blocked and indeterminate cases. They are not measured farm, mine, vehicle or site data.

Run the separate industrial/mobile energy-path screen:

```sh
cargo test --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --all-targets
cargo run --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --bin energy_path_screen -- \
  tools/rural-industrial-electrification-evaluator/fixtures/energy_paths_v0_1.json
``

The energy-path screen performs a forward maximum-capacity pass and a backward source-power requirement pass. It reports the power needed to serve the declared load, instead of reporting unused route capacity as energy delivered. Thermal routes require stage temperature-drop inputs and a load-side temperature compatibility check. The conductive and wireless 90 kW / 30-minute examples are matched synthetic comparisons only; their efficiency factors are assumptions, not measured performance.

## V0.1 method

Usable pack energy:

`nominal_pack_kWh × state_of_health × temperature_capacity_fraction × (1 − reserve_fraction)`

Grid energy needed to replace declared battery-side shift consumption:

`shift_demand_kWh ÷ charger_efficiency_fraction`

Effective per-charger power under the simplified simultaneous-charge assumption:

`min(charger_power_kW, site_power_budget_kW ÷ simultaneous_chargers)`

Recharge duration is required grid energy divided by effective charging power. This is a bounding screen, not a battery electrochemistry or charge-curve model; constant power and efficiency are simplifications. No candidate is considered qualified just because this check passes.

## Status semantics

- `feasible_under_declared_screen`: every required field is present and the simple energy/charge-window checks pass.
- `not_feasible_under_declared_screen`: at least one known constraint fails.
- `indeterminate`: a required field is missing; the candidate is not rankable.
- `invalid_input`: a supplied numeric or evidence-class value violates the contract.

Unknown values never become zero or a pass. A failed known constraint can block a candidate even if another field is missing.

## Deliberate omissions

V0.1 does not calculate task energy from machine physics, traction, implement or PTO loads, payload, terrain, hydraulics, battery aging, cell thermal behaviour, charging curves, grid reinforcement, solar/storage dispatch, diesel efficiency, hydrogen pathways, costs, emissions or SMR economics. Those require explicit subsequent models and sources. The input demand must be separately estimated or measured for the exact task profile.

Do not use this tool for actual vehicle, farm, mine, industrial electrical or safety decisions.


## EV-CARRIER-REF-01 incremental carrier configuration screen

The `ev_carrier_configuration_screen` binary composes optional delivery-pod / AMR hardware over the existing `vehicle_design_screen` output. It does not reconstruct the base vehicle model.

Run from the repository root after the normal EV design screen:

```bash
cargo run --quiet --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml \
  --bin vehicle_design_screen -- \
  tools/rural-industrial-electrification-evaluator/fixtures/vehicle_designs_v0_1.json > vehicle-design-report.json

cargo run --quiet --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml \
  --bin ev_carrier_configuration_screen -- vehicle-design-report.json \
  tools/rural-industrial-electrification-evaluator/fixtures/ev_carrier_configurations_v0_1.json
```

Each carrier configuration references an existing `design_id` (initially `urban-distribution-truck` or `light-commercial-van`) and declares only additions beyond the base design report:
- installed pod / aircraft / dock / fixture mass;
- incremental cargo mass not already included in the base target payload;
- incremental vehicle traction-battery terminal energy draw;
- additional simultaneous vehicle HV-bus continuous and peak power;
- external site-meter energy and peak power;
- whether the external charging peak coincides with base vehicle charging, plus the declared charging window.

The energy ledgers are kept separate:
1. incremental draw from the vehicle traction battery, including vehicle-to-UAV conversion losses, is added to the EV mission demand and later replenished through the EV's declared charging efficiency;
2. external UAV/AMR site-meter energy is counted separately and is not added again to traction-battery demand;
3. site peak is compared with the base vehicle charging load, either summed for coincident sessions or maxed for explicitly noncoincident sessions.

External site recharge hours are only a **constant peak-power lower bound** (meter energy / declared peak kW). Passing that lower bound does not establish an actual charging curve or a successful depot schedule: for any positive external site-meter energy, the result remains `indeterminate` until a charge-curve or time-series dispatch profile is supplied. If the lower bound alone exceeds the declared window, the configuration is definitely blocked. The base depot peak is a conservative upper bound that assumes every declared simultaneous vehicle draws its full effective charger cap; it is not a measured demand peak. A coincident incremental site peak is added to that upper bound, while a proven noncoincident peak is compared with its maximum. Unknown coincidence, charger acceptance, mass, energy or power stays indeterminate. A known hard limit violation blocks the configuration even when other fields are unknown.

The first fixture intentionally has two truck-only controls with explicit zero additions and four unengineered alternatives (integrated enclosed bay, detachable pod, mobile drone-station trailer and AMR handoff) with null unknowns. The expected result is two synthetic-control passes and four indeterminate alternatives. This is intentional: no pod mass, aircraft count, charger, fire containment, structural retention, electrical load, thermal duty, site concurrency or maintenance data have yet been established.

The screen does **not** calculate axle loads, detailed CG, restraint, braking, crashworthiness, structure, thermal runaway propagation, HV isolation, EMC, cybersecurity, aircraft performance, aerodynamic energy, flight approval or economics. Those remain with the EV / road-dynamics, cargo, battery, airworthiness, site and factory qualification owners. It does not rank architecture options or authorize physical operation.

