# HAPS Robust Qualification 001 — persistence, thermal closure and fleet service

**Programme:** AeroWeft / AeroCommons  
**Status:** research benchmark specification v0.1; synthetic-first  
**Parent architecture:** [AERO-HAPS-ENERGY-NETWORK-001 #5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885)  
**Benchmark work:** [AERO-HAPS-ROBUST-QUALIFICATION-001 #5910](https://github.com/Luminous-Dynamics/symtropy/issues/5910)  
**Claim ceiling:** offline research benchmark. No airworthiness, flight, power-beaming or operational claim.

---

## 1. Qualification objective

The question is not simply whether a battery state of charge rises over a typical day. It is whether a configuration-identified aircraft can deliver a declared sensing, communications or relay service over a declared geographic and seasonal envelope while maintaining energy, thermal, structural, reserve, recovery and maintenance constraints—and whether a fleet can keep delivering that service while individual aircraft are unavailable.

Keep three claims separate:

1. **Sortie endurance:** elapsed time for one airframe to remain airborne.
2. **Platform persistence:** repeatable operation over an environmental and seasonal envelope, including degraded modes and recovery.
3. **Service availability:** continuity delivered by a fleet with handover, inspection, maintenance, spares and common-cause risks.

A multi-month design proposition is not a multi-month measured flight. A continuous regional service is not proof that one airframe flew for that same duration. A HAPS is an electrically propelled aircraft with glide capability, not an unpowered glider that can hold station without propulsion energy.

## 2. Industry and research baseline

These are source-attributed anchors, not interchangeable performance claims.

| Programme / study | Reported evidence | Comparison boundary |
|---|---|---|
| Airbus / AALTO Zephyr | 67 days, 6 hours and 52 minutes continuous stratospheric flight, completed 28 April 2025; supersedes the earlier 64-day 2022 result | Strong public fixed-wing stratospheric endurance benchmark; one flight, not a fleet availability distribution |
| BAE / Prismatic PHASA-35 | Stratospheric trials in 2023 and 2024 above 66,000 ft; solar generation by day and battery operation at night | Altitude trials; months-long operation is a design proposition, not a reported months-long flight |
| BAE / ARIA PHASA-35 programme | September 2026 programme to demonstrate ground-to-HAPS power beaming | Ground transmitter to airborne HAPS receiver; not HAPS-to-UAV transfer |
| U.S. Navy / Skydweller | Publicly reported 73-hour solar-powered unmanned flight in 2025 | Multi-day solar-flight comparator, not a stratospheric HAPS endurance record |
| Sceye ST1 buoyant LTA HAPS | Approximately 30-day, nearly 30,000 km stratospheric round trip in Aug–Sep 2026; more than seven days over Japan; SoftBank reported station-keeping radius as small as 5 km during the Japan test | Buoyant free-balloon/LTA platform; not a 30-day station-keeping claim and not comparable to a fixed-wing continuous sortie record |
| Sceye SE2 buoyant HAPS endurance campaign | More than 12 days and 88+ hours at selected areas, station-seeking radius reported as low as 1 km, April 2026 | Separate SE2 campaign; do not merge with ST1 flight data/configuration |
| Dianovský et al., Drones 2026 | Curved-wing PV model: 6.0% RMS power error and 3.5% daily-energy agreement against ground charging measurements over three clear-sky days spanning a 90-day calendar range | PV-model validation; not a 90-day flight |
| ACS Energy Letters 2026 battery study | Cell energy sweep from 100–1,000 Wh/kg; storage-to-solar-collection transition near 500 Wh/kg in the studied cases; predicted battery fractions of 30–40% of takeoff mass | Model-specific result, not a universal threshold or installed-pack result |
| Reach Power / PowerLight | Vendor-reported 10-hour remotely powered UAV flight (October 2026) and in-flight laser power delivery to a fielded UAS (April 2026) | Ground-to-air power delivery; not an airborne HAPS transmitter demonstration |
| Optical WPT research | Outdoor optical transfer reported at 15% over 1 km under atmospheric turbulence | Link experiment, not a moving-aircraft HAPS-to-UAV mission |

Primary references:

- [AALTO's 67-day Zephyr record, 1 May 2025](https://www.aaltohaps.com/zephyr-sets-world-record-for-longest-continuous-flight-flying-67-days-in-stratosphere/)
- [BAE Systems PHASA-35](https://www.baesystems.com/en-us/product/phasa-35)
- [BAE Systems / ARIA power-beaming programme, 15 September 2026](https://www.baesystems.com/en-uk/article/PHASA-35-selected-for-ARIA-backed-power-beaming-project-)
- [U.S. Navy / Skydweller demonstration, 5 December 2025](https://www.usff.navy.mil/Press-Room/News-Stories/Article/4354688/navy-demonstrates-multi-day-solar-uas-flight/)
- [Dianovský et al., 4 July 2026](https://doi.org/10.3390/drones10070510)
- [ACS Energy Letters, 21 August 2026](https://doi.org/10.1021/acsenergylett.6c01616)
- [Reach Power, 28 October 2026](https://reachpower.com/news/wireless-powered-drone-10-hour-flight/)
- [PowerLight, 20 April 2026](https://powerlighttech.com/press-release-powerlight-achieves-industry-first-wireless-power-beaming-to-a-fielded-military-uas-in-flight/)
- [Ochiai et al., optical WPT research](https://doi.org/10.1049/ell2.70379)
- [Solar-UAV energy systems review, April 2026](https://doi.org/10.1016/j.nexres.2026.101404)
- [Near-space motor heat-pipe cooling study, November 2026](https://doi.org/10.1016/j.ast.2026.113202)

The 2026 review identifies PV, hybrid storage, thermal regulation, power electronics and autonomous energy control as coupled design variables. The November 2026 motor-cooling study treats heat rejection as a specific near-space design problem. These motivate explicit coupling; neither establishes one universal cooling solution.

## 3. Paradigm shifts

### 3.0 Compare lift classes explicitly

The HAPS tournament must contain at least three distinct platform families:

- **Heavier-than-air fixed wing:** aerodynamic lift; propulsion is required for station-keeping, and the 67-day Zephyr result is the current public fixed-wing endurance comparator.
- **Buoyant free balloon:** buoyancy supports the vehicle; the wind/altitude-control strategy and ability to remain over a service region must be measured rather than inferred from an airship.
- **Buoyant airship:** buoyancy provides lift but propulsion may be available for wind-relative positioning; do not transfer its control/energy assumptions to a free balloon.

Sceye's 2026 ST1 mission is described by Sceye's press release as an unmanned free balloon and by SoftBank as a lighter-than-air HAPS. It completed an approximately 30-day trans-Pacific round trip and was tested over Japan for more than seven days; SoftBank reports a station-keeping radius as small as 5 km. A separate Sceye SE2 Endurance Program report describes a 12-day flight and 88+ hours at selected areas with a station-seeking radius as low as 1 km. Keep the ST1 and SE2 flights separate. [Sceye ST1 return announcement](https://www.prnewswire.com/news-releases/sceyes-stratospheric-platform-re-enters-united-states-after-record-breaking-roundtrip-stratospheric-flight-between-us-and-japan-302873007.html); [SoftBank trial report](https://www.softbank.jp/en/corp/news/press/sbkk/2026/20260902_01/); [Sceye SE2 report](https://sceye.com/press-releases/sceye-completes-historic-12-day-6400-mile-stratospheric-flight-advancing-a-new-layer-of-infrastructure-for-humanity/).

Sceye's product page states 425 Wh/kg for its advanced lithium-sulphur battery, and a months-to-a-year design proposition. Record these as manufacturer claims. They are not independently validated installed-pack figures or demonstrated year-long endurance. [Sceye platform page](https://sceye.com/platform/).

The buoyant platform trade is not "balloons are better" or "fixed wing is better." IEEE Spectrum reports buoyant platforms can carry more solar area, batteries and payload, while fixed-wing platforms have so far flown longer and are easier to steer. Convert these qualitative claims into matched metrics: payload kg and kW, service-area-hours, station-keeping error/energy, solar area, wind envelope, repositioning, launch/recovery burden and lifecycle cost. [IEEE Spectrum, 2026](https://spectrum.ieee.org/high-altitude-platform-station).

### 3.1 Optimize service continuity, not the record flight

Treat an aircraft as one node in a service system. Include launch/recovery, inspection, maintenance, battery replacement, weather holds, spares, handover and common-cause outages. Measure delivered service-hours and coverage continuity separately from sortie duration.

### 3.2 Treat a HAPS as an energy-constrained microgrid

Model generation, storage, prioritized loads, reserve, optional export and waste heat. External demand is a revocable service request: admit it only when the platform can preserve its own flight, thermal and reserve constraints under the declared forecast uncertainty.

### 3.3 Add exergy and thermal closure to the Wh ledger

Electrical energy is high-grade work potential. Conversion losses become heat and may require mass, surface area, conductive paths, heat pipes, radiators or active cooling. A modest energy loss can still create a local hot spot or cooling-mass penalty.

For heat transferred at absolute temperature T against a declared environmental reference T0, the associated heat exergy may be represented as:

\[
B_Q = Q\left(1 - \frac{T_0}{T}\right)
\]

This is not a substitute for a thermal model. It emphasizes that energy quantity alone does not describe energy quality or work potential.

For each motor, inverter, battery, transmitter and receiver, account for heat generation, heat capacity, conduction, radiation, convection assumptions, temperature gradients, derating, thermal-control power and limits. Do not assume rarefied air provides sea-level convective cooling or that cold ambient conditions eliminate local thermal problems.

### 3.4 Co-design energy, structure and aerodynamics

PV, batteries, transmitters, receivers, shielding and heat rejection change installed mass, stiffness, deformation, drag, wiring and maintenance access. Optimize the installed system, not component-level efficiency in isolation. Solar collection is also heading- and wing-geometry-dependent; a flat-plate approximation is a weak default.

### 3.5 Schedule in four dimensions

Solar geometry, latitude, season, altitude, wind layers, coverage footprints and service demand all vary with time. The energy-optimal trajectory may not be coverage-optimal or safest. Preserve these as separate objectives and constraints.

### 3.6 Treat export as a separate aircraft mission

HAPS-to-UAV transfer adds transmitter/receiver mass, alignment, acquisition, interruption, cooling, drag and safety constraints. It must be compared against ground-direct power beaming, battery-only and docking on a matched mission.

## 4. Benchmark scenario matrix

| ID | Scenario | Essential stressors | Primary question |
|---|---|---|---|
| H0 | Reference diurnal closure | Declared 24-hour sun, loads, battery, power limits and reserve | Does the ledger close? |
| H1 | Consecutive-day sequence | Multi-day changing solar and loads | Does SoC recover after each night? |
| H2 | Seasonal/geographic holdout | Solstice/equinox, low solar elevation, altitude and latitude | What is the valid operating envelope? |
| H3 | Wind-layer station-keeping | Adverse winds, track error, loiter and coverage | What is the true propulsion energy? |
| H4 | Battery end-of-life | Usable capacity loss, impedance, power fade, imbalance and cold | Does reserve still hold? |
| H5 | Thermal extremes | Solar heating, battery and motor heat, rarefied-air cooling, payload peaks | Can thermal limits be maintained? |
| H6 | Payload duty cycles | Continuous low power and burst communications/imaging/compute | Can payload service be delivered without eroding flight reserve? |
| H7 | Component degradation | PV-string loss, shading, soiling, cell mismatch and drive derating | Can degradation be detected and absorbed? |
| H8 | Export admission | Receiver demand, conversion, pointing, interruption and heat rejection | Is there useful export at the receiver meter? |
| H9 | Fleet/node failure | Node loss, maintenance, recovery, spares, grounding and handover | Does the service survive an unavailable aircraft? |
| H10 | Lift-class comparison | Fixed-wing versus buoyant free balloon versus propelled airship; payload, solar area, wind field, altitude control and launch/recovery | Which lift class delivers the same coverage service with the lowest robust system burden? |

Preserve weather autocorrelation, synoptic regime, altitude correlations and geography. Do not shuffle hourly irradiance and wind independently. Hold out entire weather campaigns, seasons or locations. Monte Carlo seeds are not independent weather campaigns.

Use model-specific time scales and demonstrate numerical time-step convergence for solar/battery dispatch, battery/motor thermal transients, wind-relative station-keeping, beam acquisition/reacquisition and fleet handover. Do not force one sample rate across all physics.

## Machine-readable cases and arithmetic controls

The versioned [HAPS energy case schema](schemas/haps_energy_case_v1.schema.json) defines a provenance-aware time-series contract. Two fixtures pin down both the positive and negative control:

- [Synthetic day/night surplus-export case](examples/synthetic_haps_daynight_surplus_case_v1.json): 14,076.72 Wh solar generation; 5,040 Wh own-load demand; 4,444.444... Wh battery charge input; 2,520 Wh battery discharge output; 1,500 Wh source-side export; 1,200 Wh receiver-useful energy; 3,200 Wh ending SoC; no own-load deficit; the daily cycle closes.
- [Synthetic rejection of battery-funded export](examples/synthetic_haps_reject_battery_funded_export_v1.json): 1,200 Wh own-load demand served from 1,333.333... Wh stored-energy withdrawal; 2,400 Wh requested receiver energy is unserved; zero source export and zero receiver energy; 1,666.667 Wh ending SoC remains above the 1,000 Wh reserve but below initial SoC, so the daily cycle and export-service admission both fail.

Both fixtures are synthetic arithmetic oracles, not aircraft profiles or measured performance. The interval contiguity, horizon sum, initial SoC/reserve bounds and ledger equations are semantic invariants in addition to JSON Schema validation. The current Symthaea Rust screen in PR #12674 evaluates an exact 24-hour profile; the case schema can represent longer horizons, but no multi-day execution implementation or validation is claimed yet.

## 5. Qualification gates

| Gate | Evidence | Permitted claim |
|---|---|---|
| Q0 — identity | Aircraft/configuration/mission/environment IDs, unit contracts and input digests | Inputs are identified |
| Q1 — arithmetic oracle | Independent energy fixtures and negative controls | Accounting matches declared arithmetic |
| Q2 — owner-model replay | Solar, wind, battery, thermal and aero outputs with applicability receipts | Model result within declared owner domains |
| Q3 — independent comparison | Matched baselines and held-out data/conditions | Comparative model evidence |
| Q4 — component bench | Calibrated PV, battery, motor/inverter, thermal and receiver measurements | Component result within tested conditions |
| Q5 — integrated ground emulator | Integrated energy/thermal path, receiver emulator, interruption/reacquisition and fault injection | Ground-integrated research result |
| Q6 — controlled flight campaign | Separately approved test plan, instrumentation, safety, recovery and authority | Flight-test result for exact configuration/envelope |
| Q7 — service campaign | Observed operation, maintenance, downtime, handovers and delivered service | Service-availability result for observed period |

Evidence does not automatically promote between gates. Keep NOT_COVERED, OUT_OF_DOMAIN, INDETERMINATE, FAILED and NOT_RUN distinct.

## 6. Metrics and definitions

Report a vector. Preserve raw measurements, distributions, uncertainty and missing strata. Never hide a hard failure inside a weighted score.

**Energy and persistence:** PV generation; own-load demand; battery charge input and stored energy; discharge withdrawal and delivered energy; curtailment and export; minimum SoC; reserve margin; own-load unserved Wh; end-of-day/horizon SoC; ledger residual; lower-tail energy margin by held-out weather/season regime; consecutive days within reserve.

**Thermal and exergy:** peak/percentile temperatures; component gradients; time outside limits; heat generated and rejected by component; thermal-control power and mass; heat per useful receiver Wh; exergy destroyed by conversion stages where supported; residual against calibrated thermal measurements.

**Installed-system/aerostructural:** mass of PV, battery, transfer equipment, shielding and cooling; wing loading, drag, stiffness/deformation, aeroelastic margin, control/trim effects, inspection burden and payload lost to the export package.

**Network/fleet:** service-hours, coverage continuity, payload throughput, node/fleet availability over 30/90/365-day windows, maintenance, weather-grounding, spares, handover gap, request-service fraction and p50/p95/p99 delay.

**Evidence quality:** out-of-domain and NOT_COVERED rates, prediction-interval coverage/width, false-feasible rate, hard-constraint violations, independent sites/seasons/weather campaigns, source rights, calibration and reproducibility.

## 7. Matched baselines

Physical/platform baselines:
1. Solar-electric HAPS without export.
2. Solar-electric HAPS with ground-to-HAPS power support.
3. Ground-to-UAV power beaming.
4. HAPS-to-UAV transfer hypothesis.
5. Battery-only UAV plus ground charging.
6. Docking, conductive/inductive charging and pack exchange.
7. One HAPS versus a rotating fleet.
8. Fixed altitude/loiter versus wind/solar/coverage co-optimized trajectory.

Design/decision baselines:
1. Analytical lower bounds and first-principles sizing.
2. Sequential discipline-by-discipline workflow.
3. Existing owner models composed without adaptive analysis selection.
4. OpenMDAO gradient-based MDO where differentiability and applicability permit.
5. NSGA-II / NSGA-III or another documented evolutionary optimizer.
6. Bayesian/multi-fidelity optimization with calibrated surrogate uncertainty.
7. Robust-feasibility/minimax-regret selection when probability assumptions are unavailable.
8. Symthaea decision-aware analysis/fidelity selection.

Freeze mission, geography, weather sequence, payload, reserve, model versions, input digests, compute/test budget and meter boundaries. A method must not receive a better surrogate, more solver calls, broader operating domain or more independent evidence than its comparator. Publish seeds, solver failures, aborted runs, budgets and Pareto fronts.

## 8. Research targets and staged milestones

These are goals, not certification criteria or performance guarantees.

**Immediate software/model goals**
- Independent oracle agreement within the declared numerical tolerance on every synthetic energy case.
- Zero reserve/own-load/thermal false-feasible outcomes in the frozen benchmark corpus.
- Zero double-counting across PV, battery, transmitter, receiver and useful-load meter planes.
- Unsupported conditions return NOT_COVERED / INDETERMINATE, never assumed zero loss.
- 100% source-to-useful-load traceability for any energy-export claim.

**Model-validation goals**
- First reproduce the 2026 curved-wing PV study's validation boundary and published 6.0% RMS / 3.5% daily-energy figures.
- Then seek less than 3% RMS power error and less than 2% daily-energy error on a new independently instrumented, multi-heading, multi-temperature and multi-season holdout. These are stretch goals and only count with an independently frozen test set.
- Calibrate battery and thermal models against measured cells/packs at relevant pressure/temperature; do not infer pack behavior from cell data.
- Report station-keeping energy prediction error by altitude/wind regime before claiming trajectory optimization superiority.

**Persistence milestones**
- Seven consecutive measured days with independent energy/reserve accounting before a 30-day campaign.
- Thirty measured days before planning a 90-day campaign.
- A verified 90-day stratospheric fixed-wing mission would exceed the current public 67-day Zephyr flight duration. Valid comparison requires matched claim boundaries, complete data, independently reviewed instrumentation and applicable record verification.
- Report 30/90/365-day fleet availability independently of any individual flight record.

**Energy-export milestones**
- Start with non-energized receiver emulation and measured mass/drag/thermal integration.
- Close the source-to-useful-load ledger on a stationary bench before moving-target work.
- Establish acquisition/reacquisition and interruption handling before any separately authorized energized flight test.
- Beat ground-direct transfer only on matched receiver, range, power, availability, safety, infrastructure and lifecycle boundaries.

## 9. Thermodynamic, lifecycle and falsification contract

Account for solar input; electrical work to propulsion, avionics, payload, storage and transfer; battery, motor/inverter, transmitter and receiver losses; thermal-management mass and power; radiative/conductive/convective assumptions; degradation and replacement; and launch/recovery operations.

Lifecycle boundaries must include aircraft production/repair, battery replacements, ground stations, energy source, fleet spares and delivered service. Solar input alone is not a lifecycle-emissions result.

Reject or mark INDETERMINATE when the energy ledger fails to close; a sequence breaches reserve; thermal limits or cooling mass break aircraft closure; wind/solar conditions break station-keeping or coverage; receiver mass/drag/heat erases claimed benefit; the transfer link cannot maintain useful receiver power; or the service relies on an unmodelled single node.

## 10. Ownership and non-duplication

- [Symthaea #7343](https://github.com/Luminous-Dynamics/symthaea/issues/7343): solar HAPS daily energy and persistent-platform owner; [draft PR #12674](https://github.com/Luminous-Dynamics/symthaea/pull/12674) adds a bounded 24-hour surplus-only ledger screen.
- [Symthaea #7646](https://github.com/Luminous-Dynamics/symthaea/issues/7646): aircraft mission energy and reserve.
- [Symtropy #5853 / PR #5856](https://github.com/Luminous-Dynamics/symtropy/pull/5856): whole-aircraft portfolio/MDO.
- [Symtropy #5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885): HAPS energy network and architecture comparison.
- Symtropy #3885 / #4286: aircraft/energy co-design and replenishment.
- Symtropy #3458 / #3448 / #4080 / #4115 / #4157: optical physics, empirical WPT boundaries, session lifecycle, observability and ground-air scheduling.
- Mycelix #3638: component, configuration, inspection and maintenance provenance.

This document defines benchmark contracts only. It must not create duplicate discipline solvers or self-authorize engineering results.

## 11. Claim ceiling

This supports research specification, source synthesis and offline benchmarking only. It authorizes no laser/RF energization, battery experiment, public-airspace activity, flight-critical control change, airworthiness claim, certification claim or operating permission. Every physical test requires its own reviewed safety, instrumentation, recovery and authority basis.
