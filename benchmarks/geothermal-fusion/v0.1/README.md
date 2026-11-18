# Geothermal–Fusion Integration Benchmark v0.1

**Tracking:** [GEOFUSION-INTEGRATION-001 / Symtropy #6142](https://github.com/Luminous-Dynamics/symtropy/issues/6142)  
**Status:** analytic synthetic fixture and proposed benchmark family; not a validated physical model.

## Benchmark question

For the same declared electricity, heat, reliability and process-service obligations, does a geothermal-supported, storage-enabled, multi-source energy architecture outperform a fair grid-only / non-hybrid alternative across the complete source-to-useful-service boundary?

The benchmark must not reward a candidate for:
- dropping a parasitic load or storage loss;
- counting the same heat in multiple paths;
- delivering heat below the required temperature grade;
- smoothing a pulse with unbounded or lossless storage;
- treating plasma Q as fusion plant net electric power;
- treating a model output as a measured resource or plant;
- omitting common-cause failures or resource decline.

## Evidence classes

Use one explicit class per input/output:
- `synthetic`: generated for arithmetic or adversarial testing;
- `published_design`: source-reported design parameter, not operating performance;
- `vendor_reported`: supplier / project claim with source and date;
- `measured`: identified physical measurement with calibration and custody;
- `calibrated_model`: model calibrated against identified data;
- `independently_validated_model`: matched validation with independent assessor;
- `audited_operation`: operational evidence with declared meter and audit boundaries.

Evidence class is not a confidence score. A newer or more precise value does not automatically have stronger provenance.

## v0.1 architecture cases

1. Grid-only fusion research campus (reference electrical supply; synthetic or metered demand profile).
2. Geothermal-only conversion plant (direct heat, ORC / binary / flash / steam as applicable).
3. Geothermal net electricity serving steady facility auxiliaries.
4. Geothermal with electrical buffer serving a short-duration demand pulse.
5. Geothermal + electrical storage + grid support / export constraint.
6. Geothermal direct heat cascade with minimum approach temperatures and temperature-grade demand.
7. Geothermal + heat pump versus direct heat / resistive electric heating.
8. EGS chronological decline, pump power, reinjection and downtime.
9. Superhot geothermal state-aware fluid properties above 375 °C and 22 MPa.
10. Superhot well integrity, thermal cycling, chemistry, materials and sensor degradation.
11. ORC / flash / steam / indirect sCO₂ comparisons at matched source state and service boundary.
12. Fusion facility load breakdown by plasma heating/current drive, magnets, cryogenics, vacuum, tritium, cooling, control and buildings.
13. Concept-dependent fusion blanket / divertor / vessel heat streams with independently sourced temperature / pressure / coolant data.
14. Fusion heat plus an isolated thermal-store / industrial process service.
15. Combined multi-temperature heat cascade, storage dispatch and pinch constraints.
16. Geothermal source outage + grid outage + depleted storage + fusion startup / recovery attempt.
17. Shared substation / cooling / water / wellfield / controls common-cause failure.
18. South African low-enthalpy evidence case; superhot resource values remain unknown until source-backed.

## First analytic fixture

See [analytic-geothermal-buffer.json](fixtures/analytic-geothermal-buffer.json).

This fixture is synthetic and deliberately avoids the supercritical water property regime. It uses declared duties rather than assuming a constant heat capacity. It checks:
- geothermal thermal split into direct process heat, power-cycle input and a declared unrecovered/loss term;
- power-cycle first-law split into gross electricity and rejected heat;
- gross geothermal electric output less parasitics = net bus supply;
- electrical bus balance with facility base load, a ten-minute pulse, and charge/discharge losses;
- the difference between energy (MWh) and power (MW).

Expected values, using the frozen inputs:

| Quantity | Expected |
|---|---:|
| Geothermal thermal energy extracted over 70 min | 116.666667 MWhth |
| Direct useful heat delivered over 70 min | 21.000000 MWhth |
| ORC gross electricity over 70 min | 11.666667 MWhe |
| ORC rejected heat over 70 min | 70.000000 MWhth |
| Geothermal net electricity over 70 min | 9.333333 MWhe |
| Facility electricity service | 8.333333 MWhe |
| Net storage state increase | 0.847368 MWh |
| Storage conversion losses | 0.152632 MWh |
| Electrical bus balance residual | <= 1e-9 MWh in this arithmetic fixture |

The facility demand trace is a hypothetical synthetic profile, **not** a measured fusion device profile or a statement about a typical tokamak. The example does not include any fusion heat source, plasma heating physics, cryogenic plant model or net fusion generation.

## Required metrics

Report, without collapsing into a single score:
- geothermal thermal extraction, direct-use heat and net electrical output;
- gross / parasitic / net power at each declared electrical boundary;
- heat delivered by temperature band and unmet demand;
- electrical and thermal storage state, power limits, charge/discharge efficiency, standing losses and degradation;
- net imports/exports and facility unserved energy by interval;
- exergy destruction / exergy efficiency only with a pinned reference environment and property model;
- reservoir temperature, pressure, flow, injectivity/productivity and decline only where supported by data;
- water use / reinjection chemistry / seismic observations / environmental limits;
- CAPEX, OPEX, LCOE, levelized cost of heat and lifecycle replacements with matched assumptions;
- forecast calibration, uncertainty intervals, common-cause fault resilience and evidence-class counts.

## Comparison rules

- Compare identical time series, weather, outage assumptions, service quality and facility load envelopes.
- Baselines: grid-only; geothermal-only; geothermal + grid; geothermal + grid + storage; hybrid thermal/electrical cascade; no-build where applicable.
- Use NREL SAM / GETEM and GEOPHIRES-X for matched geothermal subproblems; PROCESS for matched fusion system-code cases; OpenGeoSys TH2M or another validated reservoir solver for coupled subsurface subproblems.
- Pin code version, inputs, outputs, property databases, solver tolerances and hardware for every run.
- Compare like-for-like model fidelity. Do not compare a simplified 0D Symtropy ledger to a high-fidelity solver and call runtime or accuracy superiority.
- Use paired uncertainty intervals and the full Pareto vector. There is no universal best architecture.

## v0.1 acceptance gates (targets, not achievements)

1. Every source, conversion, store and load has a typed identity, unit, boundary and evidence class.
2. Every source and conversion stage closes against an independent hand calculation.
3. First-law normalized residual <= 0.1% on exact analytic fixtures.
4. Exergy residual <= 0.5% only when reference environment and property model are pinned; otherwise report `EXERGY_NOT_EVALUATED`.
5. Adversarial cases catch source double counting, unmetered parasitics, heat-pump electricity omission, impossible heat-exchanger approach, storage state overflow/underflow, pulse-power violation and missing property data.
6. Direct geothermal-heat-to-D–T-plasma path is rejected as physically invalid.
7. Missing covariance, resource measurements or calibration remain UNKNOWN; never become zero or independent by default.
8. Identical frozen inputs and deterministic model versions produce byte-stable receipts.
9. Synthetic, published-design, measured and audited evidence cannot be silently promoted between classes.
10. No hardware actuation or plant safety authority is implied.

Stretch goals such as a 10% net-purchased-electricity reduction, 15% lower exergy destruction, or 20% lower critical-load unserved energy are hypotheses. They become scoreable only after a preregistered matched case, full metering boundary, independent replay and uncertainty analysis exist.
