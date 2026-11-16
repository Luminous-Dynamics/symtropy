# PressureForge Analytical Reference Protocol v0.1

**Date:** 2026-11-16  
**Status:** Analytical software reference only; no pressure-system qualification

## 1. Purpose

This protocol defines the narrow reference problem implemented in `kernel/src/pressure.rs`: ideal-gas pressure bookkeeping for one gas species in a rigid, isothermal, well-mixed compartment with prescribed gas mass transfers.

The reference is useful for checking units, mass conservation, and pressure response to an already-known mass change. It does **not** estimate the mass transfer from a leak, predict structural integrity, or authorize a pressure boundary.

## 2. Governing relation

For one ideal gas species:

`p V = n R_u T`, with `n = m/M`.

Therefore:

`p = (m/M) R_u T / V`

where:
- `p` is absolute pressure in Pa;
- `V` is fixed compartment volume in m³;
- `m` is gas inventory in kg;
- `M` is molar mass in kg/mol;
- `R_u = 8.31446261815324 J/(mol K)`;
- `T` is uniform gas temperature in K.

For a known mass transfer over an interval:

`m_next = m_current + m_in - m_out`

The transfer is rejected if any term is negative/non-finite or if `m_out > m_current + m_in`.

Pressure is then recomputed from the new inventory under the same fixed-volume, fixed-temperature and unchanged-composition assumptions.

## 3. Implemented comparison

The module can compare the predicted pressure with one observed absolute pressure measurement:

`r_p = p_observed - p_predicted`

The current classification uses a user-supplied absolute tolerance and a z-score times the observed instrument standard uncertainty. Missing observed uncertainty yields `Unquantified`.

Important: the current comparator does not propagate uncertainty from volume, temperature, gas inventory, gas composition or model form. It is therefore a narrow residual calculator, not a probabilistic safety assessment.

## 4. Assumptions and exclusions

The reference assumes:
- one ideal-gas species;
- rigid, fixed-volume compartment;
- uniform and constant temperature;
- well-mixed gas;
- no phase changes, adsorption, condensation or chemical reaction;
- prescribed mass inflow/outflow values.

It does not implement:
- orifice flow, leak conductance, choked flow or transitional/rarefied gas flow;
- pressure-dependent leak rates;
- multi-species gas mixtures or partial pressures;
- heat transfer or thermal transients;
- flexible wall behaviour, stress, fatigue, buckling or fracture;
- rock-mass interaction or pressure-loaded geological voids;
- sensors, actuators, life-support controllers or emergency authorization.

No result from this model qualifies a real pressure vessel, habitat, lava tube, cave or other enclosure.

## 5. Analytical tests

The module's unit tests cover:
1. One-mole ideal-gas pressure identity.
2. Ten-percent known mass loss gives a ten-percent pressure reduction under the model assumptions.
3. Outflow greater than available inventory is rejected.
4. Missing observed pressure uncertainty produces `Unquantified`.
5. A seeded observation deviation outside tolerance is detected.

These are proposed S1 reference cases; the Rust tests have been authored but not executed in the current environment because Rust tooling is unavailable.

## 6. GeoBioBench PressureBench extension

When the model is expanded, the benchmark should progress in separate evidence tiers:

- **S0:** deterministic unit cases, malformed values, numerical overflow, mass-transfer conservation, and injected sensor anomalies.
- **S1:** independent ideal-gas calculations and independently implemented state updates.
- **S2:** instrumented, uncrewed pressure compartment with calibrated pressure, temperature and mass/flow measurements.
- **S3:** repeated pressure and thermal cycles in an analogue environment, with independent structural instrumentation.

Required metrics:
- pressure prediction residual by operating regime;
- uncertainty calibration when the model includes parameter uncertainty;
- pressure decay per unit measured mass loss;
- leak detection latency and false-alarm rate when leak-flow inference is added;
- mass-balance residual;
- thermal-cycle and pressure-cycle degradation;
- isolation and recovery performance only after the corresponding hardware exists.

A simulator cannot demonstrate leak detection or isolation merely because it is supplied with the true mass loss. Flow inference, sensor detection, and physical isolation are separate capabilities and must receive separate benchmarks.

## 7. Next engineering steps

1. Add uncertainty for compartment volume, temperature and gas inventory, with explicit covariance.
2. Add mixture gas species and partial-pressure bookkeeping.
3. Add time-dependent energy and temperature states.
4. Introduce a validated leak-flow model only after choosing the intended continuum, transitional or rarefied-flow regime and comparing with an independent analytical or experimental reference.
5. Couple the pressure boundary to structural and geomechanical models only after independent validation of both sides of the interface.
6. Test pressure control and isolation on an uncrewed, independently instrumented rig before any occupied or extraterrestrial application.

The key safety rule remains: rock may provide shielding and spatial enclosure, but it is not presumed to be a pressure vessel.
