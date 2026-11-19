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
10. Reservoir-to-wellhead pressure / enthalpy / phase-path benchmark: include near-isenthalpic expansion, wellbore pressure drop, mass-flow sensitivity, transmissivity and wellhead state; never substitute reservoir temperature for delivered temperature.
11. Superhot well integrity, thermal cycling, chemistry, materials and sensor degradation, including subcritical phase-transition windows and corrosion sensitivity around 300–350 °C where supported by the fluid chemistry.
12. Superhot performance sensitivity: compare temperature-only ranking against a joint pressure–transmissivity–mass-flow–wellhead-exergy ranking.
13. ORC / flash / steam / indirect sCO₂ comparisons at matched source state and service boundary.
14. Fusion facility load breakdown by plasma heating/current drive, magnets, cryogenics, vacuum, tritium, cooling, control and buildings.
15. Concept-dependent fusion blanket / divertor / vessel heat streams with independently sourced temperature / pressure / coolant data.
16. Fusion heat plus an isolated thermal-store / industrial process service.
17. Combined multi-temperature heat cascade, storage dispatch and pinch constraints.
18. Geothermal source outage + grid outage + depleted storage + fusion startup / recovery attempt.
19. Shared substation / cooling / water / wellfield / controls common-cause failure.
20. South African low-enthalpy evidence case; superhot resource values remain unknown until source-backed.
21. Multi-service thermal co-product case: geothermal heat to thermal desalination and/or absorption cooling, geothermal electricity to RO / pumping, and future source-bounded fusion heat only where temperature grade permits. Match water quality, cooling duty, total energy, brine/residual streams and isolated thermal boundaries; facility cooling is not a substitute for fusion cryogenic refrigeration.
22. Published superhot reservoir–wellbore reproduction: reproduce the IDDP-1 calibration points and pressure / temperature / permeability-thickness sweep from Scott (2026), preserving the paper's calibration status, steady-state assumptions, property formulation, casing geometry, wellhead pressure and cycle selection. Do not call the IDDP-1 calibration points blind validation.
23. Wellhead phase / casing velocity qualification: compare single-phase density-based bulk velocity with the exact casing inner diameter; separately evaluate two-phase conditions using a qualified multiphase model. Include 9⅝-inch and 13⅜-inch casing scenarios from the paper and the cited 50 m/s erosional-velocity reference only as a source-specific screening comparator, not a universal limit.
24. Wellhead-pressure trade-off: compare lower wellhead pressure / higher flow and higher pressure / single-phase preservation against electrical output, casing velocity, scaling risk and surface-cycle compatibility. Keep economic and lifecycle outcomes separate from steady-state power.
22. Literature-reproduction case for geothermal polygeneration: reproduce a published ORC/flash + absorption refrigeration + Kalina + humidification-dehumidification desalination configuration. The 2024 Energy paper reports model outputs of 342.3 kW electricity, 301.7 kW cooling and 0.233 kg/s freshwater at 31.52% exergy efficiency; these are simulation outputs, not measurements. Reproduction is blocked until the complete source inputs, fluid properties, state points, boundary and component assumptions are frozen.
23. Superhot wellhead state / casing envelope: vary wellhead pressure, phase state, casing diameter and mass flow together; measure net power against single-phase margin, erosion velocity, silica scaling and corrosion exposure. The Scott (2026) calibrated steady-state model identifies >15 MPa wellhead pressure as a single-phase-preserving lever in some cases, while its 9 5/8-inch casing cases exceed a 50 m/s erosional-velocity reference above roughly 30 MPa reservoir pressure for 450–550 °C. These are model-specific bounds, not general design limits.
24. Transient well-integrity case: simulate circulation interruption, shut-in, restart, cold-water injection and production ramp across phase transitions; track local pressure/temperature/chemistry, cement/casing/elastomer degradation and subcritical exposure near 300–350 °C. Do not infer lifetime from steady-state peak-temperature qualification.

## Published superhot calibration anchor

See [iddp1-calibration-anchor.json](fixtures/iddp1-calibration-anchor.json). It records the approximate IDDP-1 calibration values and observed/modelled wellhead-temperature ranges reported by Scott (2026). The ranges overlap, but the source excerpt does not provide pointwise residuals or raw time series in this fixture. Therefore it is a source-pinned calibration anchor, not a blind holdout and not a completed quantitative reproduction.

Required next evidence: source tables/figure points with a declared digitization method and uncertainty, or rights-cleared raw IDDP-1 traces. The model reproduction must then preserve exact fluid-property implementation, wellbore geometry, wellhead-pressure boundary, phase/cycle selection, and source digest.

## First analytic fixture

See [analytic-geothermal-buffer.json](fixtures/analytic-geothermal-buffer.json) and [source-allocation-heat-cascade.json](fixtures/source-allocation-heat-cascade.json). The second fixture proves a single source interval can be split across useful services and curtailment only once; its over-allocation and under-allocation cases must remain visibly unbalanced.

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

## Temperature-grade reference cases

For absorption cooling, use equipment-specific heat-source maps. As a reference envelope, ASHRAE's equipment chapter reports:
- Single-effect indirect-fired water/LiBr: typically 115–132 °C for steam-heated units; some larger waste-heat units can use lower-grade heat (reported as low as 88 °C).
- Double-effect indirect-fired water/LiBr: typical hot-fluid input 160–188 °C.

These are equipment examples, not universal thermodynamic limits or guaranteed operation at every ambient/cooling-water condition. The benchmark must pin the exact chiller model, generator inlet/outlet conditions, cooling-water temperature, chilled-water duty, electrical parasitics, COP basis and measured heat-source map. Source: [ASHRAE Handbook, Chapter 18, Absorption Equipment](https://handbook.ashrae.org/handbooks/R18/SI/R18_Ch18/r18_ch18_si.aspx).

For desalination, compare RO and MED/HDH only at an identical feed, product-water quality, recovery, availability and residual/brine boundary. The 2026 desalination review finds modern seawater RO close to its practical thermodynamic minimum, and points to whole-train system losses, high-recovery configurations, pretreatment/cleaning/post-treatment and brine handling as major remaining levers ([Wang et al., 2026](https://doi.org/10.1038/s44359-026-00221-x)). Do not award an architecture efficiency gain by moving energy or brine treatment outside the system boundary.

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
