# SOC-Dependent EV Charging Curve Screen v0.1

Programme: [EV-CHARGE-CURVE-001 #2116](https://github.com/Luminous-Dynamics/symtropy/issues/2116).

## Purpose

A charger nameplate and a vehicle's maximum charge acceptance do not define the whole session. The battery may accept different power at different states of charge; health and temperature also affect the usable energy and accepted power. The v0.1 screen integrates a **piecewise-constant SOC acceptance curve** analytically, with separate site/port limits and declared static temperature factors.

This is a bounded next layer after the constant-power fleet queue simulator. It is intentionally a standalone model before integration into the event scheduler.

## Run

    cargo test --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --all-targets
    cargo run --quiet --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --bin charging_curve_screen -- tools/rural-industrial-electrification-evaluator/fixtures/charging_curve_v0_1.json

## Definitions and boundaries

- `nominal_pack_capacity_kwh`: reference capacity.
- `state_of_health_fraction`: capacity retained due to aging.
- `temperature_capacity_fraction`: static available-capacity fraction.
- `temperature_acceptance_fraction`: static multiplier applied to the reference power curve.
- `initial_soc_fraction` and `target_soc_fraction`: fractions of the effective pack capacity after SOH/temperature capacity derating. Target must be greater than initial and at most 1.
- `acceptance_curve`: contiguous [0,1] SOC bands with a maximum **battery-side charge acceptance** in kW.
- `charging_efficiency_fraction`: battery-side energy divided by site-input energy (constant for v0.1).
- `port_input_power_kw` and `site_input_power_budget_kw`: both measured at the site-input boundary.

Effective pack energy is

    effective_capacity_kwh = nominal_pack_capacity_kwh × state_of_health_fraction × temperature_capacity_fraction

For each overlapped SOC band:

    band_battery_energy_kwh = effective_capacity_kwh × (band_soc_end − band_soc_start)
    external_battery_side_limit_kw = min(port_input_power_kw, site_input_power_budget_kw) × charging_efficiency_fraction
    temperature_derated_curve_kw = reference_curve_kw × temperature_acceptance_fraction
    effective_battery_charge_power_kw = min(external_battery_side_limit_kw, temperature_derated_curve_kw)
    band_duration_minutes = 60 × band_battery_energy_kwh / effective_battery_charge_power_kw
    band_site_input_kwh = band_battery_energy_kwh / charging_efficiency_fraction
    band_conversion_loss_kwh = band_site_input_kwh − band_battery_energy_kwh

The curve is piecewise constant within each SOC band. The screen reports the limiting side—battery acceptance curve or site/port cap—for every band. It does not average a taper curve into one arbitrary power value.

## Required properties

- Curves must be ordered, non-overlapping, contiguous and cover SOC [0,1].
- Missing, gap, overlap, out-of-domain and non-finite values fail validation.
- All reported SOC intervals are within the declared session range.
- Battery energy sums to effective capacity × (target SOC − initial SOC).
- Site energy reconciles to battery-side energy plus conversion loss within a declared numerical tolerance.
- Lower charge acceptance or a tighter site/port cap cannot shorten the computed session for the same energy request.
- Lower SOH or temperature capacity changes energy demand explicitly; it is not silently represented as charge efficiency.
- Lower temperature acceptance cannot increase accepted power.

## Current synthetic fixture panel

1. Passenger EV high-power mid-session taper.
2. Light commercial van with managed depot/port limit.
3. City bus opportunity charge with static acceptance derating.
4. Heavy truck high-power session with a static cold-acceptance factor.
5. Rural service EV under a weak site import cap.

These are synthetic curves and parameters chosen to exercise behavior. They are not manufacturer charging curves, field measurements, predictions of actual charging times, or a recommended battery choice.

## What the model excludes

No electrochemical model, cell-level voltage/current, SOC-dependent charging efficiency, lithium plating, coolant or cell thermal dynamics, current limits by cell voltage, cell imbalance, charger communication, cable temperature, battery aging during each session, stochastic site power, charger faults, queueing, or charge-session preconditioning is modeled. Static temperature acceptance and capacity factors are declared inputs, not temperature predictions.

The next integration layer will let the queue simulator recompute each active session's acceptance when it crosses an SOC band. A later physical model can replace the static derating with instrumented cell/pack charge curves and dynamic thermal/aging state.

## Research references

- IEA, *Ultra-fast charging batteries* (Global EV Outlook 2026): https://www.iea.org/reports/ultra-fast-charging-batteries
- IEA, *Electric vehicle charging* (Global EV Outlook 2026): https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-charging-chap-6-and-10
- Battery thermal management under high-load electric mine-truck conditions (2026): https://www.sciencedirect.com/science/article/pii/S1290072925008622
- SAE J3271 MCS interface, cable cooling/handling, controls and interoperability report: https://saemobilus.sae.org/standards/j3271_202503-sae-megawatt-charging-system-electric-vehicles

## Claim ceiling

The screen is model verification over synthetic inputs. Passing this arithmetic does not certify a charging curve, battery pack, thermal envelope, charger or physical vehicle. No production, safety, warranty or deployment claim is made.