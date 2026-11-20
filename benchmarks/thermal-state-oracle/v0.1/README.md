# Thermal State Oracle v0.1

**Status:** M1 analytic reference implementation with synthetic known-answer tests. No physical actuator model is identified or qualified.

**Parent:** [THERMAL-STATE-ORACLE-001 #6392](https://github.com/Luminous-Dynamics/symtropy/issues/6392)  
**Thermal experiment contract:** [ACTUATOR-THERMAL-MISSION-001 #6155](https://github.com/Luminous-Dynamics/symtropy/issues/6155)  
**Thermal mission record/schema:** [PR #6157](https://github.com/Luminous-Dynamics/symtropy/pull/6157)

## Purpose

The thermal mission record describes what an experiment must observe and preserve. This crate supplies an executable analytic baseline for replaying known heat-input histories, retaining state across partial cooldown, and checking numerical semantics.

The paradigm shift is **thermal history as state**, not a cold-start torque-time lookup. Two missions with identical current torque or heat input can have different remaining thermal headroom because their initial temperatures and preceding load/cooldown sequences differ.

This model is intentionally only a one-node reference. It is useful for unit tests, analytic cross-checks and falsifying mission-replay code. It must not be selected as the physical model of an actuator merely because it is easy to fit.

## Governing equation and exact step

For one lumped thermal node:

[
C_{th}\frac{dT}{dt}=P_{loss}-G_{th}(T-T_{amb})
]

where (T) is node temperature in °C, (T_{amb}) is the declared ambient boundary in °C, (C_{th}>0) is thermal capacitance in J/K, (G_{th}\geq0) is conductance to the boundary in W/K, and (P_{loss}\geq0) is constant dissipated heat input in W over the segment.

For (G_{th}>0) and constant input over (Delta t):

[
T_{k+1}=T_{amb}+(T_k-T_{amb})e^{-G_{th}\Delta t/C_{th}}+
\frac{P_{loss}}{G_{th}}\left(1-e^{-G_{th}\Delta t/C_{th}}\right)
]

For the adiabatic limit (G_{th}=0):

[
T_{k+1}=T_k+\frac{P_{loss}}{C_{th}}\Delta t
]

The implementation uses an expm1-based small-exponent form to reduce cancellation. Piecewise-constant mission segments are composed by carrying each segment's final temperature into the next segment without resetting state.

For constant input and (G_{th}>0), equilibrium is:

[
T_\infty=T_{amb}+\frac{P_{loss}}{G_{th}}
]

When the initial temperature is below a threshold and equilibrium is strictly above it, modelled first-crossing time is:

[
t_{lim}=\frac{C_{th}}{G_{th}}
\ln\left(\frac{T_\infty-T_0}{T_\infty-T_{lim}}\right)
]

If equilibrium is at or below the threshold, there is no finite-time crossing under that constant input. For (G_{th}=0), crossing time follows the linear heating equation. An initial temperature at or above the limit is reported as crossing at (t=0).

## API

- ThermalRcModel::new(C_th, G_th) validates model parameters.
- advance(T_initial, T_ambient, P_loss, duration) advances one constant-input interval.
- time_to_limit(T_initial, T_ambient, P_loss, T_limit) returns a modelled crossing time or None.
- replay(&ThermalMission) carries state through uniquely identified load/cooldown segments and reports final/peak temperature and first threshold crossing.

The model reports a threshold crossing but does not stop replay, apply derating, trip a driver, or infer sensor/protection latency. remained_below_limit compares the mathematical trajectory with the supplied threshold only.

## Required interpretation boundary

**Heat-loss power is an explicit model input.** This crate does not infer it from commanded torque, output mechanical power, bus voltage/current, motor constants, efficiency or regenerative operation. A separate source-qualified electrical/mechanical loss model is needed to derive heat input from actuator operation.

The one-node temperature is not automatically winding, stator, case, gearbox or driver temperature. The reference has no spatial gradients, multiple thermal masses, temperature-dependent winding resistance, saturation, active cooling dynamics, contact resistance, protection hysteresis or sensor delay.

The QDD100 beta 3 points in the thermal mission profile are vendor-source, zero-speed observations with missing initial temperatures, sensor/threshold details and raw traces. They are not sufficient to identify this model. No parameter fit to those points is performed here.

## Synthetic fixtures and regressions

thermal_rc_reference_fixtures_v0.1.json provides machine-readable known-answer cases for cooling over one time constant, heating halfway to equilibrium, the zero-conductance adiabatic limit, and cold-start/hot-start threshold times under identical constant input.

The Rust corpus additionally checks partition invariance, repeated load with partial cooldown, no finite crossing below equilibrium, immediate crossing, monotonic heating, invalid inputs and duplicate segment identifiers.

## Validation ladder

- **M1 (this crate):** exact analytic one-node reference; synthetic known-answer and metamorphic tests.
- **M2 (future):** compare one-node, multi-node and nonlinear model classes only when observed channels, excitation and identifiability support them. Use frozen held-out mission profiles and publish residuals and interval coverage.
- **M3 (future):** independently calibrated, synchronized measurements of electrical input, mechanical output, and distinct winding/stator/case/driver temperatures.
- **M4 (future):** repeated integrated joint/robot missions with declared initial state, cooling boundary, task success, derating and interruptions.
- **M5 (future):** aged/remounted/repaired articles and explicit requalification.

A more complex model is not automatically better. It must predict held-out traces and distinguish thermal states better than the simpler reference under comparable identification budgets.

## Reproduction

From the repository root:

```sh
cargo fmt --manifest-path benchmarks/thermal-state-oracle/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/thermal-state-oracle/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/thermal-state-oracle/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

The package is dependency-free and declares its own Cargo workspace boundary. These commands are executed by the package-scoped GitHub Actions workflow.

## Claim ceiling and safety

This crate is an analytic software reference using synthetic inputs. It is not an actuator rating, thermal estimator, safety control, hardware qualification, or evidence of any commercial product's behavior. A passing software test or CI run does not authorize energized tests, deliberate thermal-limit trips, fabrication or human-contact trials. Physical testing remains subject to a separately approved guarded rig, qualified operators, interlocks, manufacturer limits and hazard analysis.
