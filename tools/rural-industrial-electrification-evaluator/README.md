# Rural and Industrial Electrification Evaluator v0.1

This standalone Rust tool screens whether one declared shift's energy demand fits a derated nominal battery and whether the required recharge fits the declared site-power and charging window. It is deliberately small enough to inspect independently.

## Run

From the repository root:

```sh
cargo test --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml
cargo run --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml -- \
  tools/rural-industrial-electrification-evaluator/fixtures/benchmark_v0_1.json
```

The report is JSON on stdout. The fixture values are wholly synthetic and illustrate both feasible and blocked/indeterminate cases. They are not measured farm, mine, vehicle or site data.

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
