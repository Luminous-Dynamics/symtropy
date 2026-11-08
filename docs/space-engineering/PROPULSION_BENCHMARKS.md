# Space Propulsion and Manufacturing Benchmark Charter

**Status:** research and benchmark charter; no engine or spacecraft qualification is claimed  
**Baseline date:** 2026-11-08  
**Composition:** SPACE-CAPABILITY-LEDGER-001 (#1972), BENCH-SPACE-001 (#1737), EARTH-ORBIT-CHAIN-001 (#3228), ORBITAL-LAST-MILE-001 (#3253), and PLANETARY-DEFENSE-001 (#4199)

## 1. Purpose

Build an evidence-first, mission-aware framework for evaluating propulsion technologies, the components that enable them, and the manufacturing/test systems needed to produce reliable hardware.

The programme must span:
- launch propulsion: liquid, solid, hybrid and emerging architectures at a system-comparison level;
- in-space propulsion: chemical, cold gas, electric (electrothermal, electrostatic and electromagnetic), solar-sail and other propellantless approaches;
- spacecraft power, propellant storage/feed, thermal management, guidance/navigation/control and mission operations;
- propulsion component materials, additive/subtractive/hybrid manufacture, joining, inspection, repair and qualification;
- mission-level service: accepted payload delivered to the required destination and state, with reliability, schedule, lifecycle cost and evidence.

The central question is not “which engine has the highest specific impulse?” It is:

> Which propulsion architecture delivers the required mission service with the lowest justified lifecycle burden and the highest demonstrated reliability, within the actual power, thermal, structural, navigation, manufacturing and operational envelope?

Different engine classes serve different regimes. A launch-stage chemical engine, a Hall-effect thruster, a solar sail and a nuclear-electric concept cannot be ranked by one scalar metric.

## 2. 2026-11-08 external baseline

### 2.1 Reusable liquid launch propulsion

ESA reported on 2026-11-06 that Prometheus completed several hot-fire tests at DLR's P5 test bench on 2026-10-29. The engine uses liquid oxygen and methane; the tested campaign included a combustion chamber manufactured with cold-gas-spray additive processing. ESA reported repeated restarts without a high-pressure starter system, using engine inlet pressures. Prometheus is in development, not a flight-proven reusable operational engine; its next campaign is planned for the improved Mark 2 version.

DLR's 2026-11-06 report adds an important test-sequence detail: the modernised P5 stand ran several methane/oxygen ignitions within one test day at stable operating points, with up to five ignitions in a day. Between hot runs, the stand and supply system must be shut down, drained, purged and restored to required pressure/temperature conditions. Therefore, report engine start/restart performance separately from facility turnaround, purge/reset reliability and test-sequence automation. This is ground-test evidence, not a count of successful in-flight restarts or reusable flights.

This is a strong reference for:
- LOX/methane system integration;
- repeat-start and ignition-system simplification;
- cold-gas-spray combustion-chamber manufacturing;
- test-facility adaptation and long-duration cryogenic hot-fire infrastructure;
- reusable-engine development and production-cost ambitions.

It does not by itself establish flight life, turnaround cost, fleet reliability, production rate or full-stage reusability.

Sources:
- ESA, “Prometheus reusable rocket engine fires up at improved test bench” (2026-11-06): https://www.esa.int/articles/about-us/directorates/space-transportation/future-space-transportation/prometheus-reusable-rocket-engine-fires-up-at-improved-test-bench
- ArianeGroup campaign statement: https://press.ariane.group/essai-a-feu-du-moteur-reutilisable-prometheus-au-banc-p5-du-dlr-une-nouvelle-etape-cle-de-developpement-518213/?lang=eng

### 2.2 Additive manufacturing and integrated thrust chambers

NASA's RAMPT programme reports more than 16,000 seconds of hot-fire testing and nearly 500 starts across its integrated chamber/nozzle hardware, at multiple thrust levels. It reports composite-overwrapped chamber work and a 35% mass-saving result for a particular demonstrated approach. These are programme-level results across various hardware; they must not be attributed to one engine configuration or interpreted as a universal production/flight qualification. NASA's current RAMPT page separately describes up to 40% weight savings for a specific carbon-fibre composite wrap versus a metal jacket, and lists a 35% mass-savings milestone under the programme achievements. Preserve both as configuration-specific claims with different stated baselines; do not average them or promote either to a universal thrust-chamber result.

Reference: NASA, Rapid Analysis and Manufacturing Propulsion Technology (RAMPT): https://www.nasa.gov/rapid-analysis-and-manufacturing-propulsion-technology/

NASA also describes one-piece regeneratively cooled thrust-chamber technology combining additive processes and composite overwrap. Its stated weight savings refer to the described design relative to its traditional construction baseline, not every engine or operating envelope.

Reference: NASA Technology Transfer, “A One-piece Liquid Rocket Thrust Chamber Assembly”: https://technology.nasa.gov/advancing-commercial-space/transportation_systems

### 2.3 In-space propulsion

NASA's 2026 State-of-the-Art of Small Spacecraft Technology report organizes procurable and emerging propulsion into chemical, electric and propellantless categories. The report explicitly warns that public device specifications may be incomplete, inconsistent or unverified, and that a device's maturity depends on the mission environment, duty cycle, propellant throughput, burn duration, number of cycles, radiation and interfaces.

Its propulsion chapter is an important comparison corpus, but manufacturer-published or NASA-tabulated values are not automatically independent test results.

A useful new frontier appeared in 2026: Muon Space announced that its Starlight zinc-propellant Hall-effect thruster fired in orbit on 2026-07-01 aboard its SERT-III test spacecraft, with the announcement published 2026-09-10. The company says orbital tracking confirmed a measurable thrust-induced orbit change. Treat this as a **vendor-announced on-orbit demonstration**, not as independently verified performance or life qualification until the telemetry, orbit-determination method, calibration and full burn data are independently reviewed. The published announcement is useful for a system-architecture comparison: solid metal propellant may reduce high-pressure storage burden and rare-gas supply dependence, but feed mechanism, thermal control, erosion, plume effects, total impulse, lifetime and mission fit need quantitative evidence.

Reference (vendor announcement, not independent validation): https://www.muonspace.com/muon-space-achieves-world-first-zinc-propellant-hall-effect-thruster-successfully-fires-in-space/

References:
- NASA 2026 State of the Art: https://www.nasa.gov/smallsat-institute/sst-soa/
- In-Space Propulsion chapter: https://www.nasa.gov/smallsat-institute/sst-soa/in-space_propulsion/

### 2.4 Qualification and durability

NASA-STD-5012C is a current structural design and qualification standard for spaceflight propulsion pressure vessels and pressurized systems. The relevant standard and program requirements must be selected by hardware class, applicable agency/mission and configuration. This charter does not replace qualification authority or prescribe a universal test count.

Reference: https://standards.nasa.gov/standard/NASA/NASA-STD-5012

NASA's Inspector General has also documented the importance and cost of propulsion test infrastructure, test availability, test-facility sustainment and qualification evidence:
https://oig.nasa.gov/wp-content/uploads/2024/09/ig-24-018.pdf

## 3. First executable analytic benchmark slice

Symtropy draft PR [#4220](https://github.com/Luminous-Dynamics/symtropy/pull/4220) adds a standalone Rust reference verifier with a frozen JSON case for:

- ideal rocket-equation consistency: Δv = Isp × g0 × ln(m0 / mf);
- ideal nozzle thrust consistency: F = mdot × ve + Ae × (pe − pa);
- derived specific impulse: Isp = F / (mdot × g0);
- explicit SI units, finite/positive domain checks, source records, an analytical-only claim ceiling and a SHA-256 receipt over the exact fixture bytes.

The fixture uses illustrative values and is not associated with Prometheus, NASA RAMPT or a real engine. It validates algebraic identities only. Seven Rust positive/negative tests are authored; they have **not** been reported as executed in the absence of a Rust toolchain. An independent script recomputed the frozen Δv, thrust and specific-impulse values, but that is not a substitute for the Rust test run.

The next gate is to run the verifier locally/CI on the exact PR head, then expand to frozen component-level reference cases with experimental data, measurement uncertainty and validity envelopes. Do not jump directly from this analytic slice to CFD or engine qualification.

**Important scope boundary:** analytic equations verify definitions and numerical plumbing; they do not validate combustion, feed systems, thermal management, materials, turbomachinery, fatigue, engine life or flight performance.

## 4. Paradigm shifts

### 3.1 Optimize delivered mission service, not engine headline metrics

A higher-Isp device can lose at mission level through low thrust, high power-system mass, long transfer duration, weak lifetime, propellant-management complexity or low reliability. A high-thrust engine can lose through low restart capability, thermal limits, maintenance burden or manufacturing cost.

Use the target mission's required terminal state and service window as the objective. For launch, include payload accepted into the required orbit, not simply mass at an arbitrary insertion point. For asteroid reconnaissance/interception, include navigation, arrival geometry, observation payload, communication, correction manoeuvres and independent verification.

### 3.2 Make the operating envelope part of the identity

Every benchmark result must be tagged with:
- propellant and grade/condition;
- mixture ratio or flow regime where applicable;
- chamber/inlet/outlet pressure and temperature;
- thrust/throttle point and transient phase;
- ambient or vacuum conditions and nozzle expansion assumptions;
- electrical input power and power-processing-unit configuration for electric systems;
- duty cycle, burn duration, start/stop count and total throughput;
- thermal boundary conditions, cooling, vibration and mechanical interface;
- spacecraft bus, storage, feed, attitude and mission environment;
- hardware configuration, material batch, manufacturing route and inspection status.

Performance without operating context is not comparable evidence.

### 3.3 Design for qualification and manufacture from day one

Co-design the component, process, inspection and service-life evidence:
- identify critical-to-quality characteristics;
- define material and process windows;
- quantify anisotropy, porosity, residual stress, microstructure and joining effects;
- qualify channels, walls, interfaces and pressure boundaries;
- define nondestructive inspection coverage and blind-defect controls;
- correlate process parameters to measured component properties;
- link each hardware unit to build, inspection, acceptance and test records;
- track repairs, refurbishment, scrap, yield and production learning.

A printed coupon does not qualify a chamber. A successful chamber hot-fire does not automatically qualify a full engine. An engine hot-fire does not qualify a reusable stage or fleet.

### 3.4 Qualify the test system and reset state, not just the engine

A repeated ignition is a coupled engine + facility event. Every campaign record should capture the engine hardware identity and the stand configuration, instrumentation/calibration, propellant conditioning, supply pressure/temperature, purge/drain procedure, reset duration, operator/automation sequence, facility alarms and aborts. Count successful starts per hardware unit separately from campaign-wide starts across multiple articles. Measure restart probability conditional on thermal state, prior cycle, turnaround interval and reset sequence. Include negative controls for incomplete purge, incorrect inlet conditioning, sensor drift and mismatched hardware revisions. Never infer flight reuse or fleet reliability from a one-day stand demonstration.

### 3.5 Make propellant logistics a first-class design variable

Compare total system burden rather than propellant price or Isp alone: stored propellant mass, tank/containment mass, pressurization, thermal conditioning, feed hardware, loading and ground handling, toxicology/safety controls, supply-chain concentration, residuals and end-of-life disposal. Solid-metal electric-propulsion concepts deserve a fair system-level comparison against xenon/krypton, but a lower storage burden does not establish lower total mission mass, better lifetime or lower cost until quantitative feed, thermal, erosion and flight data exist.

### 3.6

Use Symthaea's existing HAZARD-VOI-001 (#8192) and active-learning/value-of-information foundations to select the next test that most improves a consequential engineering decision. Examples include a thermal-cycle test, CT scan, flow test, hot-fire run, restart sequence, vibration test or material characterization.

The best next test is not necessarily the most precise or expensive. It is the test with the highest expected decision improvement after considering uncertainty, cost, latency, feasibility and model disagreement. Keep scientific information gain separate from economic/mission decision value.

### 3.5 Digital thread from requirements to operations

Maintain a machine-verifiable chain:

requirements → model and assumptions → design revision → material batch → manufacturing process → inspection → component test → engine/system test → environmental qualification → flight configuration → operational telemetry → maintenance and retirement.

Mycelix should preserve attestations, provenance, source disagreement and corrections. It must not turn reputation or consensus into a substitute for test evidence.

### 3.6 Multi-fidelity engineering, not a monolithic simulator

Use the appropriate solver at each fidelity:
- analytical thermodynamics and rocket-equation limits for first checks;
- 0D/1D cycle and feed-system calculations for architecture screening;
- validated CFD and combustion chemistry tools for specialist flow/combustion analysis;
- finite-element and fracture solvers for structural, thermal and fatigue problems;
- specialist electric-propulsion plasma models where necessary;
- test data and hardware-in-loop for model correlation.

Symtropy can own data contracts, systems coupling, mission-level trades, deterministic scenario orchestration and reusable numerical components where independently validated. Its current PBD/rigid-body engine must not be represented as a validated combustion CFD, reacting-flow, plasma or structural qualification solver.

Symthaea can reason about model discrepancy, propose experiments and explain assumptions, but numerical authorities remain deterministic, specialist and independently checked.

Sol-Atlas can show mission envelopes, orbital/solar-system context, ground infrastructure, test facilities, logistics and evidence layers, with model provenance and uncertainty visible.

## 4. Physics and data contracts

Use unit-aware quantities and explicit frames/time bases. Never compare dimensional values without conversion and convention checks.

### 4.1 Core performance definitions

For a rocket engine, record the applicable measured/derived quantities and conventions. Common relationships include:

- Thrust: F = mdot × ve + (pe − pa) × Ae
- Specific impulse: Isp = F / (mdot × g0)
- Effective exhaust velocity: c = Isp × g0
- Ideal rocket equation: Δv = Isp × g0 × ln(m0 / mf)
- Characteristic velocity: c* = Pc × At / mdot (under its declared chamber/nozzle convention)
- Thrust coefficient: Cf = F / (Pc × At)

These idealized relations are consistency checks, not substitutes for measured performance, real-fluid properties, combustion efficiency, nozzle losses, transient behavior or uncertainty.

For electric propulsion, preserve:
- thrust, input power and power-processing-unit losses;
- specific impulse and propellant mass flow;
- total impulse and total propellant throughput;
- thrust-to-power ratio and efficiency under a declared definition;
- discharge/cathode operation, plume interaction and operating point;
- lifetime and performance degradation over representative duty cycles.

Do not compare a high-Isp low-thrust device and a high-thrust chemical engine as if they were interchangeable products.

### 4.2 Typed model identity

Every result should identify:
- system and component configuration;
- propulsion class and operating mode;
- units and reference conditions;
- model fidelity and governing equations;
- solver, version, mesh/element or discretization details as relevant;
- convergence criteria and numerical tolerances;
- material/process batch and environmental conditions;
- uncertainty representation and data provenance;
- validity domain and unsupported regimes.

## 5. Benchmark ladder

All numeric targets below are proposed gates or reporting requirements, not claims of current performance or generic pass/fail values. Tolerances should be assigned per reference case and measurement uncertainty.

### P0 — dimensional and analytic integrity

- Unit-conversion, dimensional-consistency and sign-convention tests.
- Limiting cases for ideal rocket equation, nozzle momentum/pressure terms and energy/mass balances.
- Mass, energy and momentum conservation residuals with predeclared tolerances.
- Property-table interpolation tests and valid-domain rejection.
- Zero/invalid/non-finite inputs fail closed.
- Independent hand calculation or separate implementation for every analytic oracle.

### P1 — component models

- Injector/feed pressure-drop and flow-regime correlation against measured data.
- Cooling-channel thermal balance and wall-temperature prediction against instrumented tests.
- Nozzle flow and thrust prediction against reference measurements at matching operating points.
- Turbomachinery/pump performance maps against measured head, flow, speed and efficiency.
- Electric thruster thrust/power/Isp curves against independent thrust-stand measurements.
- Model discrepancy, sensor calibration, repeatability and uncertainty intervals reported.

### P2 — hardware and manufacturing

- Manufacturing yield and critical-dimension distributions by process and batch.
- CT/ultrasonic/radiographic defect detection with blinded seeded-defect controls.
- Pressure/leak proof and burst margins under the applicable qualification basis.
- Thermal-cycle, vibration, shock and fatigue performance by configuration.
- Process-to-property correlation and microstructure/anisotropy characterization.
- Repair/refurbishment success, inspection burden and dimensional drift.

### P3 — engine/propulsion-system endurance

- Total accumulated hot-fire or on-orbit operating time.
- Starts, restarts, throttle transitions, shutdowns and off-nominal transient coverage.
- Total propellant throughput and cycle distribution.
- Measured performance drift across the campaign.
- Failure rate and confidence bounds; explicitly report sample size.
- Service-life demonstration under representative environment and hardware configuration.
- Test-to-flight configuration equivalence and all waivers/deviations.

### P4 — mission service

- Accepted payload delivered to required terminal orbit/trajectory, not just initial insertion.
- Mission Δv, propellant, power, thermal and mass closure.
- Arrival state error and correction budget.
- Launch/mission success and availability with sample-size confidence intervals.
- Turnaround, refurbishment, maintenance and production cadence for reusable systems.
- Lifecycle cost per accepted mission service, with ground infrastructure and failures included.
- For electric propulsion, elapsed transfer time, power-system mass, duty cycle and end-of-life residual performance.

### P5 — evidence integrity

- All performance claims trace to original instrument data or explicitly qualified source.
- Frozen configuration, code, dependencies, calibration, data digests and test procedure.
- Blind reference cases and negative controls.
- Independent verifier and independent instrument/model where practical.
- Clear label: analytical, simulated, bench-tested, qualification-tested, flight-demonstrated, fleet-operational.
- No promotion of vendor nominal specifications to independently verified measurements.

## 6. Competitive comparison: how to do better than SOTA

Do not construct a single leaderboard that compares unlike systems. Build a performance envelope per mission class and show the evidence tier for each datum.

### 6.1 Chemical launch engines

The current public frontier includes reusable LOX/methane development, restart capability, additive/cold-spray chamber production, integrated thrust chambers and high-throughput hot-fire programmes.

Prometheus is a timely European development reference. NASA RAMPT provides a valuable manufacturing and repeated hot-fire reference. The correct challenge is not to claim higher thrust or lower cost from an unbuilt design. It is to establish a more complete evidence chain:

- same thrust class, propellant, pressure/temperature, ambient and duty cycle;
- mass-normalized and lifecycle-normalized comparison;
- repeated starts and transient envelopes;
- service-life and refurbishment evidence;
- material/process batch variation;
- production lead time, yield and inspection burden;
- test and flight configuration equivalence.

### 6.2 Electric propulsion

Use NASA's 2026 Small Spacecraft State-of-the-Art tables as a baseline of device families and reported manufacturer/mission data. Separately mark measured, independently verified, flight-demonstrated and manufacturer-reported values. Compare matching power, propellant, thrust, lifetime, throughput, spacecraft thermal environment and mission duty cycle.

A claimed Isp advantage that doubles transfer duration, requires an oversized power system or fails lifetime qualification may be a poor mission choice.

### 6.3 Manufacturing

Compete on repeatability and qualification throughput, not only build speed:
- number of hardware units and batches characterized;
- critical-defect probability and inspection probability of detection;
- yield at first pass and after repair;
- dimensional/process drift;
- hot-fire and thermal-cycle evidence;
- unit-to-unit variance and reliability confidence;
- traceable production records and supplier diversity.

### 6.4 System-level differentiation

The opportunity is to improve the connection between specialist physics, real measurements, design decisions, manufacturing and lifecycle operations. Do not claim to replace NASA, ESA, DLR, specialist CFD/hydrocodes, commercial propulsion suppliers or standards bodies. Instead, build an interoperable independent-evidence layer that makes engineering comparisons fair, reproducible and decision-relevant.

## 7. Initial work plan

### P0 — scope and source corpus

- Freeze reference cases for representative chemical launch engines, electric thrusters, propulsion test campaigns and additive manufacturing.
- Record exact configuration, date, source, license, measurement status, operating environment and claim ceiling.
- Reuse the historic mission evidence ledger (#1972) and independent verifier (#1737); do not create a second space-capability ledger.

### P1 — physics contract

- Typed propulsion class and operating envelope.
- Unit-safe mass/energy/power/pressure/temperature/flow/performance quantities.
- Analytic verification cases for conservation, ideal rocket equation, ideal nozzle and electric-thruster performance definitions.
- Explicit uncertainty and validity-domain behavior.

### P2 — test/qualification digital thread

- Material batch → process → inspection → test → hardware configuration.
- Batch and unit identity, instrument calibration, time series and environmental records.
- Evidence receipt schema and negative controls.
- Independent reproduction of published NASA RAMPT / ESA Prometheus summary metrics without treating them as the same hardware.

### P3 — mission-level trade studies

- Compare chemical/electric/hybrid options for the same mission requirement.
- Include power-system mass, propulsion dry mass, propellant, duty cycle, travel time, reliability, thermal, comms and end-of-life constraints.
- Integrate with #3228 and #3253 rather than creating a competing launch/logistics optimizer.

### P4 — planetary-defence integration

- Use mission propulsion and launch data to evaluate reconnaissance and kinetic-impactor delivery feasibility at a high level.
- Keep the asteroid trajectory/posterior model in #4199's separate NEO domain.
- Carry propulsion and navigation uncertainty into the intervention model.
- Require independent post-action orbit verification; a propulsion system's test success is not evidence of asteroid mitigation success.

## 8. Existing work to compose with

- Symtropy #1972 — historical space capability ledger.
- Symtropy #1737 — independent Rust corpus verifier.
- Symtropy #3228 — Earth-orbit delivery chain.
- Symtropy #3253 — orbital last-mile / transfer vehicles.
- Symtropy #4199 — NEO planetary-defence and risk assessment.
- Symtropy PR #4220 — executable, analytical-only rocket equation / thrust reference verifier (seven Rust tests authored, not yet run).
- Symthaea #8192 — natural-hazard value-of-information planning.
- Mycelix #4020 — industrial bootstrap milestone provenance.
- Mycelix #455 / #413 — qualified spatial/orbital state and spatial-truth contracts.
- Sol-Atlas #76 — ephemeris-backed solar-system catalogue.

No parallel general-purpose solver or duplicated data authority should be introduced.

## 9. Safety, qualification and non-claims

This is civil, scientific and commercial space engineering. It is not a military missile-development programme. Test and simulation results do not authorize hardware operation. Pressure systems, cryogens, combustion devices, high-voltage power systems and test stands require qualified facilities, procedures and competent safety authorities.

- No autonomous test-stand actuation from Symthaea or Mycelix consensus.
- No engine or spacecraft hardware is qualified by passing a software benchmark.
- No system reliability claim without relevant sample size, confidence bounds and configuration matching.
- No flight readiness claim based solely on hot-fire time or nominal specifications.
- All operational, regulatory and flight qualification remains with responsible agencies and certifying authorities.

## 10. Primary references

- ESA Prometheus hot-fire / cold-gas-spray chamber test (2026-11-06): https://www.esa.int/articles/about-us/directorates/space-transportation/future-space-transportation/prometheus-reusable-rocket-engine-fires-up-at-improved-test-bench
- ESA Prometheus programme: https://www.esa.int/Enabling_Support/Space_Transportation/Future_space_transportation/Prometheus
- NASA RAMPT: https://www.nasa.gov/rapid-analysis-and-manufacturing-propulsion-technology/
- NASA one-piece liquid rocket thrust chamber assembly: https://technology.nasa.gov/advancing-commercial-space/transportation_systems
- NASA 2026 Small Spacecraft State of the Art: https://www.nasa.gov/smallsat-institute/sst-soa/
- NASA 2026 in-space propulsion: https://www.nasa.gov/smallsat-institute/sst-soa/in-space_propulsion/
- DLR Prometheus multi-ignition test-sequence report (2026-11-06): https://www.dlr.de/en/latest/news/2026/prometheus-test-marks-significant-milestone-for-reusable-rocket-engines
- Muon Space Starlight zinc Hall-effect thruster on-orbit announcement (2026-09-10; vendor report): https://www.muonspace.com/muon-space-achieves-world-first-zinc-propellant-hall-effect-thruster-successfully-fires-in-space/
- NASA-STD-5012C: https://standards.nasa.gov/standard/NASA/NASA-STD-5012
- NASA OIG propulsion test programme report: https://oig.nasa.gov/wp-content/uploads/2024/09/ig-24-018.pdf

---

**First measurable milestone:** a source-frozen propulsion reference corpus plus independently checked analytic mass/energy/performance invariants. Only after that should we make higher-fidelity multiphysics or novel-engine design claims.
