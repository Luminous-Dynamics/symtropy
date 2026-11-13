# PUMP-SYSTEM-SERVICE-BENCH-001 — Benchmark Protocol v0.1

**Tracking issue:** [Symtropy #4959](https://github.com/Luminous-Dynamics/symtropy/issues/4959)  
**Status:** proposed research contract; no pump/system tested  
**Scope:** supply, transfer, booster, wastewater and flood-duty pump systems.

## 1. Research question

Can a complete pumping system deliver the required pressure, flow and service continuity at the actual delivery boundary with less lifetime energy, cost and maintenance burden under variable demand and disturbances?

The object being benchmarked is not just a pump. It is the interacting system: pump + motor + drive + suction/discharge piping + fittings + valves + storage + controls + instrumentation + maintenance strategy.

## 2. Boundary with existing Luminous hydro work

The existing Symtropy #4386/#4397/#4398/#4401/#4420 work owns pressure-energy recovery / pump-as-turbine component maps, network PAT co-optimization, EPANET differential testing, PAT uncertainty and PAT cassette DFM.

This benchmark is for **pump-mode water service**. PAT generation data cannot be reused as evidence of pump-mode efficiency or safe bidirectional transitions. We will reuse qualified hydraulic solvers and accounting, not create another network solver or resource ledger.

## 3. Paradigm shift — from pump efficiency to delivered service

Compare strategies by:
- required flow and pressure at the delivery node;
- service continuity and unmet demand;
- wire-to-water efficiency and energy over the full duty trace;
- pump operating region, starts/stops and actual duty point;
- NPSH / suction constraints and transient envelope;
- clogging, solids, abrasion, wear and maintenance;
- lifecycle cost and repairability;
- performance through power, sensor, network and equipment faults.

A pump with high peak efficiency but poor service availability is not a better system. A controller that saves energy by allowing pressure or delivery to fall below the required service envelope is infeasible.

## 4. Duty profiles — do not pool unlike services

1. Household/building booster and pressure management.
2. Community borehole/source transfer with elevated storage.
3. Municipal distribution and multi-pump booster stations.
4. Wastewater lift and declared solids-handling service.
5. Stormwater/flood drainage, high volume and low head.
6. Remote / solar-direct / intermittent-power water supply.
7. Special process duties only with a separate water-property and material-compatibility profile.

Candidate pump families include end-suction and multistage centrifugal, submersible, vertical-turbine, axial/mixed-flow, non-clog solids-handling and appropriate positive-displacement pumps. Compare only within a matched duty envelope; do not pool abrasive wastewater and clean potable-water pump results.

## 5. Industry baselines

- ANSI/HI 14.6-2022 provides repeatable rotodynamic pump acceptance-test procedures and grades, covering hydraulic performance, NPSH and hydrostatic pressure tests. Freeze the applicable grade and purchase/test tolerance for each candidate: <https://webstore.ansi.org/standards/hi/ansihi142022>.
- Hydraulic Institute application / piping / installation / operation guidance should be checked at the exact edition and duty scope before test-plan freeze: <https://webstore.ansi.org/sdo/HI>.
- DOE Pump Systems / PSAT / MEASUR material is a system-assessment comparator for matching pump, drive, controls, piping and actual service: <https://www.energy.gov/cmei/ito/pump-systems>.
- OWA-EPANET 2.3.5 is the distribution-network hydraulic reference where applicable. The feature coverage and fallback rules in Symtropy #4398 remain binding; a steady-state network model does not qualify transient water hammer or pump mechanical performance.

Vendor curve data are useful only when exact revision, test point, speed, impeller, liquid properties and acceptance evidence are available. Catalog curves do not replace acceptance testing of the installed system.

## 6. Modular bench architecture

Build a closed-loop, instrumented and serviceable test rig with:
- source reservoir and controllable suction condition;
- interchangeable pump, motor and variable-frequency-drive modules;
- configurable suction/discharge pipe sections, fittings, valves and restrictions;
- calibrated flow, suction pressure, discharge pressure and differential-pressure channels;
- electrical power/energy measurements with a declared meter boundary;
- rotational speed and shaft torque where feasible;
- vibration, bearing/casing temperature and acoustics where appropriate;
- reservoir level, leak/spill detection and bypass measurement;
- configurable downstream storage, demand trace and backpressure;
- physical pressure protection, guards, isolation and local manual stop.

The rig is a design candidate only. Pressure rating, pipe dimensions, electrical design, guards, lifting, wet-side protection and installation require competent engineering review before procurement or energization.

## 7. Known-answer physical relations

For a declared incompressible-water operating point:

`P_hydraulic = rho * g * Q * H`

`eta_wire_to_water = P_hydraulic / P_electrical`

`e_specific = P_electrical / Q = rho * g * H / eta`

where Q is in m³/s, H is total dynamic head in metres, power is in watts, and rho/g are the declared fluid density and gravitational acceleration. Convert to kWh/m³ only with an explicit time/volume basis. All reported efficiency values must include the exact electrical and hydraulic boundaries.

For a simple turbulent system approximation:

`H_system(Q) = H_static + K * Q^2`

The pump duty point is the intersection of pump and system curves for the actual speed/impeller/configuration. This approximation is not a substitute for a validated transient or network model; friction, viscosity, fittings, control valves, tanks and fluid properties must be represented for the relevant regime.

## 8. Required scenarios

1. Factory / acceptance duty point and declared pump curve.
2. Full permitted flow range, including off-BEP operation.
3. Variable demand and changing system curve.
4. Fixed-speed throttling versus VFD speed control.
5. Correctly sized single pump versus staged parallel pumps.
6. Pumping to storage and gravity distribution versus direct pressure maintenance.
7. Suction head / NPSHa degradation and cavitation margin.
8. Fouled strainer, partial blockage and increasing system resistance.
9. Solids-bearing wastewater with declared particle/solids profile.
10. Abrasive sediment and wear for pumps designed for that duty.
11. Power outage, voltage/frequency envelope and safe restart.
12. Failed standby pump, common-cause power failure and controller reset.
13. Leak, valve stuck, low source level, loss of prime and zero-flow condition.
14. Sensor bias, drift, stuck, stale, missing or contradictory flow/pressure readings.
15. Rapid starts/stops and valve transitions; require a validated transient model and adequate instrument bandwidth.
16. Communications loss and manual/local fallback.
17. Progressive wear and maintenance / replacement cycle.

## 9. Required observables

- Q, suction/discharge pressure, total dynamic head, fluid temperature and density basis.
- Pump, motor, drive and system input power; speed and torque where measured.
- Wire-to-water efficiency and time-integrated electrical kWh per delivered m³ at declared head/pressure.
- BEP and preferred operating-region exposure, starts/stops, runtime and minimum-flow violations.
- NPSHa/NPSHr inputs and margin under the named method.
- Vibration, temperature, acoustic emission, wear, clogging, corrosion, seal leakage and solids passage.
- Critical-node delivery pressure/flow, storage trajectory, demand delivered/unmet and outage recovery time.
- Energy, water, chemicals, consumables, operator labour, parts, downtime and full lifecycle cost.
- Instrument, calibration, uncertainty, pump-curve and configuration revision for each result.

## 10. Fair comparison protocol

Freeze the same:
- service flow/pressure envelope at the actual endpoint;
- system curve, pipe length/material/diameter/roughness and fittings;
- source/suction conditions, fluid properties and permitted solids content;
- demand, power-quality and disturbance traces;
- installation, priming, controls, speed/impeller and storage strategy;
- instrumentation, measurement method, calibration, sample rate and uncertainty;
- reliability/maintenance boundary and lifecycle horizon.

Compare four baselines:
A. Current installed configuration.
B. Correctly selected commercial reference pump.
C. System-optimized commercial reference (suitable VFD, staging, pipe/valve changes or storage).
D. Luminous candidate.

This prevents comparing an oversized/throttled incumbent to a well-tuned candidate and attributing all gains to a new pump. Separate hardware, controls, network redesign and maintenance contributions through ablations.

## 11. Benchmark tiers

| Tier | Work | Allowed conclusion |
|---|---|---|
| T0 | Analytic pump/system curves, unit arithmetic, energy and service-envelope known answers | Formula and contract correctness |
| T1 | Clean-water component + system acceptance bench | Measured performance within the exact bench envelope |
| T2 | Endurance, solids/fouling, power and fault-injection test | Qualified robustness within the tested envelope |
| T3 | Supervised site pilot with matched pre/post pressure, flow and power data | Site-specific operating performance |
| T4 | Replicated sites, pump families, operators, water qualities and seasons | Broader transfer only within replicated scope |

## 12. Candidate targets — proposed, not achieved

### Hard gates
- 100% of kWh/m³ results include measured flow, head, electrical boundary and actual delivery endpoint.
- Every schedule respects minimum pressure, delivered demand, storage reserve, pump operating envelope and NPSH/suction requirements, or abstains.
- Missing/stale/conflicting inputs never become zero or a feasible schedule by default.
- No water-hammer, surge or cavitation compliance claim from an unvalidated steady-state model.
- No energy objective can compromise declared drinking-water service, wastewater conveyance or flood-drainage capacity.

### Improvement objectives
- Two matched commercial/reference systems per duty family where data permit.
- Investigate >=10% lower time-integrated wire-to-water energy against a correctly selected and tuned reference on at least one varying-demand duty trace, without service or reliability regression.
- Investigate >=20% lower lifecycle cost in at least one profile through right-sizing, serviceable modules, local repair, staging or control improvements.
- Demonstrate no increase in unplanned failure and maintenance burden over the defined endurance window.
- Report a Pareto frontier over energy, cost, service, reliability, maintainability, noise and resilience.

These are aspirational test objectives, not expected results or universal limits.

## 13. First execution increments

1. Freeze typed pump duty profiles and exact service endpoints.
2. Create analytic T0 cases for single and parallel pumps, system curves, affinity-law transformations, wire-to-water energy and invalid/missing data.
3. Create the independent service-floor evaluator and fault-injection corpus.
4. Map measurements to Mycelix #6025/#6029 and H1 #3239; do not invent new generic pressure/flow instrument semantics.
5. Complete a safety and instrumentation design review for a clean-water T1 bench.
6. Test one commercial pump and one optimized commercial configuration before designing a custom impeller or motor.
7. Integrate network dispatch with #4397/#4398 only after exact hydraulic feature coverage and uncertainty are qualified.

## 14. Explicit nonclaims

No pump curve, energy saving, reliability, NPSH, transient safety, service continuity, regulatory compliance or commercial readiness is demonstrated by this protocol. No physical testing has yet been performed.

 
## 15. New control comparator: model-free ENC under sparse sensing

A 1 October 2026 *Water Research* paper introduces model-free Exact Nonlinearity Cancellation (ENC) control for water distribution pumps (Villacres, Putri & Moazeni, DOI [10.1016/j.watres.2026.126295](https://doi.org/10.1016/j.watres.2026.126295)). It reports 200 measured samples and a 90% computation reduction against its data-driven MPC comparator in the studied small WDS. Crucially, the article states that every regulated tank level and junction pressure was directly measured; it identifies sparse/partial sensing as the major next step. Data are available on request, so exact reproduction is not yet claimed.

New child work [#5018](https://github.com/Luminous-Dynamics/symtropy/issues/5018) and its 15-case [scenario pack](./pump-control-robustness/PUMP_CONTROL_ROBUSTNESS_V0_1.md) place ENC beside rule/PI, hydraulic MPC, DeePC and qualified-estimator hybrids under identical measurement and disturbance budgets.

Report service-qualified coverage, pressure and reserve shortfall duration, demand delivered, energy at matched delivery conditions, fault detection, estimator interval coverage, abstention, data budget and worst-case execution time. A lower compute or energy score is invalid if it violates a hard service floor or acts on unobservable state. The synthetic scenario values are not utility setpoints or engineering limits.
