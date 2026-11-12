# Electric Aviation Generation 4 — Mission-First Engineering and Evidence Benchmark RFC

**Status:** Research / engineering RFC; not an aircraft design approval, flight release, or certification basis  
**Date:** 2026-11-12  
**Owner:** Luminous Dynamics  
**Working principle:** Every performance claim must be tied to a mission definition, model version, measurement boundary, uncertainty statement, and reproducible evidence.

## Executive thesis

Generation 4 should not be treated as another eVTOL concept or as a battery-and-motor upgrade. The opportunity is to build an **integrated electric-aviation engineering system** that co-designs vehicle topology, propulsion, energy storage, thermal management, reserves, charging infrastructure, operations, and safety evidence.

The central shift is from component-first optimization to **mission-first, evidence-gated co-design**:

> Optimize the aircraft and its energy ecosystem against a declared mission, while making every assumption, constraint, failure mode, and verification result traceable.

This RFC establishes the first research architecture, benchmark taxonomy, external comparison points, and staged development gates. It deliberately does not add a new full-scale Symthaea flight-control platform. The public multirotor code now includes a trained-controller transfer path and dedicated cognition-causality / Red-tier non-interference tests. Their presence is not proof of a passing exact-head run (the inspected public commit returned no CI statuses), and even a passing microquad acceptance test would not validate full-scale aircraft dynamics. Full-scale fixed-wing/eVTOL integration is therefore gated on (1) reproducible exact-head acceptance evidence for the current controller path, (2) a separately qualified aerodynamic/propulsion/energy model for the new aircraft regime, and (3) HIL, fault-injection, and independent safety-boundary evidence.

## 1. What exists, and what does not

### Existing foundations

- Symtropy is a generic N-dimensional rigid-body physics engine with collision, constraint, replay, and Bevy integration. Its published README reports 827 tests and 68 research experiments. This is useful infrastructure, but those counts are not evidence of validated aircraft aerodynamics or propulsion.
- The public Symthaea `symthaea-multirotor` crate is a quadrotor controller. Its actuator type is explicitly described against a Crazyflie 2 layout, with 0.6 N total maximum thrust and approximately 0.265 N hover thrust. It is not a full-scale eVTOL, fixed-wing, or passenger-aircraft model.
- The broader Symthaea platform roadmap distinguishes multirotor, eVTOL, and fixed-wing regimes. Keep those models separate: transition flight, passenger reserves, lift/drag, stall, and cruise are not extensions of a quadrotor hover model.

References:
- [Symtropy README](https://github.com/Luminous-Dynamics/symtropy/blob/main/README.md)
- [Symthaea multirotor actuator and flight types](https://github.com/Luminous-Dynamics/symthaea/blob/main/crates/domains/symthaea-multirotor/src/types.rs)
- [Symthaea multirotor architecture](https://github.com/Luminous-Dynamics/symthaea/blob/main/crates/domains/symthaea-multirotor/src/lib.rs)

### Explicit non-claims

At this stage, Luminous Dynamics has not demonstrated:
- a validated full-scale aircraft aerodynamic model;
- a flight-qualified battery, motor, inverter, propeller, fuel-cell, or wireless-charging system;
- a passenger-carrying aircraft;
- an independently validated aircraft-level safety case;
- compliance with an FAA, EASA, or other national certification basis.

Simulation, analytical estimates, software tests, component bench tests, and flight evidence are separate maturity levels. Do not collapse them into a single "validated" label.

## 2. Paradigm shifts worth pursuing

### 2.1 Mission-first co-design instead of component-first design

Define payload, route, weather, altitude, speed, reserve, turnaround, noise, and dispatch assumptions before selecting an airframe or battery chemistry. Optimize across the complete mission rather than maximizing a motor, cell, or propeller metric in isolation.

The objective is a Pareto frontier over:
- energy per useful passenger-kilometre or tonne-kilometre;
- payload fraction and mission range;
- installed system mass and volume;
- acoustic footprint;
- thermal margin and degradation;
- dispatch availability and turnaround;
- lifecycle cost and lifecycle emissions.

Never hide trade-offs inside one weighted score. Publish the Pareto set and the assumptions that move it.

### 2.2 Aircraft topology as a first-class design variable

Maintain distinct archetypes and comparable mission definitions:

| Archetype | Primary use | Distinct engineering problem |
|---|---|---|
| Battery-electric fixed wing (CTOL/STOL) | Training, cargo, regional feeder | Lift/drag, stall, climb, runway, cruise efficiency |
| Lift-plus-cruise eVTOL | Urban/regional point-to-point | Hover energy, transition, independent lift and cruise systems |
| Tilt-prop / tilt-wing eVTOL | Urban/regional point-to-point | Conversion corridor, control coupling, actuator and failure cases |
| Hybrid-electric fixed wing | Regional passenger/cargo | Generator sizing, fuel burn, battery peak power, thermal integration |
| Persistent solar-assisted aircraft | Observation and communications | Endurance, low-power propulsion, weather and energy balance |

Share components where the engineering case supports it; do not force shared airframe physics or certification assumptions.

### 2.3 Energy as a system, not a battery number

Model cell, module, pack, installed battery system, bus, inverter, motor, propulsor, cooling, and reserve separately. A high cell-level Wh/kg figure is not the installed aircraft's usable Wh/kg.

The mission energy ledger should include:
- propulsion across every mission segment;
- conversion and distribution losses;
- avionics, flight controls, communications, environmental control, pumps, and thermal conditioning;
- takeoff/hover and transition transients;
- contingency, diversion, holding, and landing reserves required by the selected certification and operating basis;
- cold/hot/high-altitude effects, ageing, imbalance, and end-of-life margins.

### 2.4 Digital twin as an executable evidence graph

Every run should retain:
- immutable mission and configuration IDs;
- aircraft, battery, motor, inverter, propeller, thermal, and software revisions;
- units, assumptions, data provenance, model version, and code commit;
- uncertainty distributions and sensitivity results;
- test fixture, calibration, raw-data digest, and independent reviewer where applicable;
- predicted-versus-observed residuals and unresolved discrepancies.

A model output is a design prediction. It becomes validation evidence only after comparison with appropriate physical observations and independent review.

### 2.5 Safety authority separated from adaptive intelligence

Use deterministic, bounded control and independent safety supervision for safety-critical functions. Learning or active-inference components may advise, estimate, or optimize within a verified envelope; they must not silently acquire actuator authority or override independent protection.

For Symthaea integration:
- preserve the existing trained-controller transfer path and cognition-causality acceptance tests as a microquad-level foundation; do not treat source presence as a passing test result;
- retain a conventional, testable baseline controller;
- enforce command limits and a runtime safety envelope independently of the learned policy;
- define loss-of-model, sensor disagreement, OOD, power, thermal, and communication responses;
- make every policy change replayable and attributable;
- require HIL and fault-injection evidence before any higher-authority role;
- require aircraft-regime-specific models and tests before using this work for fixed-wing or eVTOL decisions.

### 2.6 The vertiport is part of the aircraft energy system

Co-design aircraft charging with site generation, stationary storage, thermal management, power electronics, grid constraints, queueing, and dispatch. A technically excellent aircraft can still be operationally poor if several vehicles charging simultaneously exceed site capacity.

### 2.7 Wireless charging and supercapacitors have bounded roles

- **Wireless charging:** investigate resonant/inductive charging for stationary ground pads, with alignment tolerance, foreign-object detection, interlocks, electromagnetic compatibility, thermal limits, and measured wall-to-pack efficiency. Do not assume in-flight wireless power transfer or free energy.
- **Supercapacitors:** evaluate as high-power buffers for short transients, peak shaving, and recoverable energy where the duty cycle supports it. They are not a substitute for the main flight-energy store.
- **Renewables:** solar and other site generation can reduce lifecycle emissions and grid peaks, but their intermittency and storage losses must be represented in dispatch simulations.

## 3. Reference architecture

Use separable models with explicit interfaces, shared configuration IDs, and independent verification.

1. **Mission contract:** payload, occupants/crew, range, time, route, weather, altitude, reserve rule, diversion, noise, availability, turnaround, and operating jurisdiction.
2. **Vehicle configuration:** geometry, mass properties, structural margins, landing gear, control surfaces, rotor/propulsor placement, and centre-of-gravity envelope.
3. **Aerodynamics and flight mechanics:** atmosphere, lift/drag polars, induced/profile drag, stall, control authority, propeller slipstream, ground effect where relevant, and transition corridor for VTOL.
4. **Propulsion:** motor maps, inverter maps, propeller/fan maps, generator or fuel-cell maps, bus voltage, distribution losses, fault isolation, and cooling parasitics.
5. **Energy storage:** cell chemistry and test data, module and pack construction, usable SOC window, discharge/charge limits, impedance, ageing, imbalance, thermal propagation, and end-of-life performance.
6. **Thermal system:** cell, inverter, motor, generator, cabin, and ambient heat paths; transient peaks; degraded cooling; hot-day and cold-soak cases.
7. **Infrastructure:** charging mode, DC and AC power, wireless pad where applicable, grid connection, stationary storage, renewable generation, queueing, and site outage cases.
8. **Operations:** dispatch, maintenance, charging allocation, weather minima, route planning, alternate sites, crew, ground handling, and fleet availability.
9. **Assurance/evidence:** requirements traceability, hazard analysis, model qualification, test plans, configuration control, evidence digests, and independent review.

### Mission energy accounting

For a mission divided into segments (i):

`E_{mission} = \sum_i P_{bus,i} \Delta t_i + E_{contingency} + E_{reserve} + E_{degradation\ margin}`

For each segment, calculate bus demand from the required shaft/propulsive power and measured or justified conversion maps, then add actual electrical auxiliaries and thermal loads. Avoid double-counting conversion losses or treating a nameplate battery capacity as entirely usable.

For a hybrid configuration, publish separate energy and power flows for fuel, generator, battery, and buffer. The generator's contribution during cruise must not be used to conceal an undersized battery for a vertical-lift segment or a required failure case.

All energy outputs must report: gross and usable energy, battery SOC trajectory, peak and continuous power, reserve at landing, cell/pack temperatures, thermal margin, ageing state, and uncertainty interval.

## 4. Benchmark programme

The following are **proposed Luminous research gates**, not regulatory thresholds and not achieved results. Every comparison must use the same mission, payload, reserve, environment, measurement boundary, and battery state of health.

| Domain | Required measurement | Initial research gate | Stretch ambition |
|---|---|---|---|
| Mission energy | Wh per passenger-km and tonne-km, including all auxiliaries and reserve | Complete segment ledger; no energy-balance violations | Demonstrate a repeatable improvement over a matched aircraft/mission baseline |
| Accounting implementation | Segment-level DC-bus energy, explicit reserve profile, continuous/peak power, SOC margin | Known-answer tests; empty reserve, non-finite inputs, and power-limit breaches fail closed | Independent reimplementation and agreement with calibrated component data within declared uncertainty |
| Fixed-wing cruise model | ISA atmosphere, lift coefficient, drag polar, useful thrust power, shaft power, lift margin | Equation-level known answers and explicit model-domain rejection | Qualify against independently measured drag polars and propeller efficiency maps over a held-out speed/altitude envelope |
| Mass closure | MTOW, empty mass, payload, installed energy system, centre of gravity | Close mass and CG budgets with explicit growth allowance | Reduce installed energy-system mass without reducing reserve or containment |
| Battery | Cell, module, pack, installed Wh/kg and Wh/L; usable SOC; C-rate; ageing | Evaluate designs at present commercially supported cell data and measured pack overhead | Pack-level 250–300 Wh/kg as an aspirational design range; never infer it from cell data |
| Propulsion | Continuous/peak kW/kg, efficiency maps, cooling mass, vibration, fault response | Full operating maps and thermally sustainable continuous rating | Compare against NASA HEMM's stated 16 kW/kg and 99% targets at equivalent measurement boundaries |
| Thermal safety | Cell spread, peak temperature, heat rejection, propagation and degraded-cooling cases | No unbounded thermal state in the validated envelope; defined abuse and isolation tests | No propagation beyond the qualified containment boundary under the approved test programme |
| Charging | Wall-to-pack efficiency, power, temperature, queue, dwell time, grid impact | Measured wired baseline before adding wireless complexity | Wireless pad efficiency target ≥90% at declared power/alignment conditions, with independently measured losses |
| Turnaround | Landing-to-ready time, charge dwell, inspection and thermal soak | Publish complete turnaround breakdown | Sub-15-minute turnaround where battery life, thermal margin, and operating rules permit |
| Availability | Dispatch reliability, abort causes, maintenance hours, component removals | Instrument every dispatch and abort; separate reliability from safety | 99.5% demonstration dispatch readiness, progressing toward 99.9% mature-service objective |
| Model fidelity | Predicted versus measured energy, temperature, thrust, and transition states | Held-out validation set; publish residual distributions | ≤5% median mission-energy error and ≤10% 95th-percentile peak-power/temperature error on qualified cases |
| Acoustic impact | Defined dBA/EPNdB measurement, geometry, operating state, and observer location | Establish reproducible baseline | Reduce exposed area and peak community noise against a matched reference |
| Lifecycle | kg CO₂e per passenger-km, material sourcing, pack replacement, recycling | Publish full lifecycle boundary and sensitivities | Beat matched conventional and electric competitors on lifecycle impact, not only tailpipe emissions |
| Assurance | Requirement coverage, hazard controls, fault injections, traceability | Every safety requirement has a verification method and owner | No unresolved catastrophic single-point failure in the accepted safety analysis; independent evidence for every critical control |

The battery and motor targets above must not be treated as a single system-level score. The 250–300 Wh/kg battery range is a design aspiration at the pack boundary, while the NASA HEMM comparison is a machine-specific target. The wireless charging and turnaround targets are hypotheses to test, not claims about present industry performance.

### Required benchmark disciplines

- Compare battery numbers at cell, module, pack, and installed-system boundaries.
- Separate peak, short-duration, and continuous motor/inverter power.
- Include cooling, containment, busbars, contactors, cabling, mounts, and structural reinforcement in installed mass.
- Normalize all flight energy by useful delivered payload, not empty-aircraft range.
- Include reserve, diversion, degraded battery, ageing, and environmental conditions.
- Publish mean, median, 95th-percentile, and worst observed values where the sample supports them.
- Keep simulation, software test, component test, hardware-in-loop, ground test, and flight test as separate evidence classes.
- Benchmark against both a conventional aircraft and the best relevant electric/hybrid aircraft for the same mission. Do not compare an eVTOL hover mission with a fixed-wing cruise mission as if they were equivalent.

## 5. External state of the art and competitive implications

**Battery energy.** A slide in Ion Storage Systems' November 2022 vendor presentation included in the NASA Aerospace Battery Workshop deck lists 300 Wh/kg for a four-passenger, 50-mile urban eVTOL and 400 Wh/kg for a 10+ passenger regional eVTOL under “Enabling Battery Specific Energy.” Treat these as vendor-stated planning references, not NASA certification requirements, validated production-pack values, or a disclosed cell/module/pack measurement boundary. A 2025 review reports up to 330 Wh/kg for commercial lithium-ion cells and around 400 Wh/kg for experimental solid-state systems; cell-level figures cannot be substituted for installed pack performance.

**Electric machines.** NASA describes its 1.4 MW High-Efficiency Megawatt Motor (HEMM) as a development programme targeting 16 kW/kg and 99% efficiency. NASA's technical material identifies the 16 kW/kg figure against electromagnetic/active-component mass, not a complete installed motor-plus-cryocooler, inverter, cabling, containment, and aircraft cooling package. Compare at matched mass and thermal boundaries; do not promote a machine-level target into an installed-propulsion-system claim.

**Hybrid aircraft.** NASA's STARC-ABL concept describes a 2–3 MW electrical system and projected 7–12% fuel-burn reduction. NASA/GE's 2026 flight demonstration of a megawatt-class hybrid-electric system shows why hybrid-electric regional architectures deserve serious comparison against battery-only concepts.

**Certification.** The GAO's May 2026 report records FAA case-by-case evaluation and limited charging infrastructure as of its March 2026 fact base. For a fresher programme snapshot, Joby's 4 November 2026 Q3 disclosure says it completed a wide range of FAA “for-credit” tests in the fifth and final type-certification stage; that is not an announcement of a type certificate. Archer's 9 November 2026 disclosure reports a Market Survey Special Airworthiness Certificate for Midnight, and BETA's 3 November disclosure reports an A250 market-survey special airworthiness certificate ahead of type certification. Those permissions support defined demonstrations; they are not full type certificates or production approvals. EASA published MOC-5 SC-VTOL Issue 2 on 5 November 2026. The FAA's AC 21.17-4 provides powered-lift type-certification guidance and designated airworthiness criteria.

**Operations and infrastructure.** BETA's 3 November 2026 Q3 release reports more than 250,000 nautical miles across its CTOL and VTOL fleet, 168 charging sites, and market-survey authorization for the A250 ahead of type certification. This is unusually useful operational evidence, but fleet flight hours/miles and charger counts are not substitutes for mission-matched energy, reliability, thermal, or safety metrics. It demonstrates why infrastructure interoperability, uptime, and real-world operations are competitive dimensions—not secondary afterthoughts.

**Programme maturity.** Joby's 4 November 2026 report says its fifth and final certification stage included a wide range of “for-credit” FAA testing, while 17 aircraft were in production. Archer's 9 November release reports a market-survey special airworthiness certificate and the start of turboshaft testing for its separate hybrid VTOL programme. BETA's 3 November release reports A250 market-survey authorization and continued progress toward commercial certification. These are company-reported milestones from different aircraft categories and certification paths; they are not interchangeable readiness percentages.

### How Luminous should compete

Do not claim to beat Joby, Archer, BETA, NASA, or EASA on aircraft performance before there is a matched, independently measured test. Instead, pursue defensible advantages in:
1. transparent end-to-end mission energy accounting;
2. battery ageing and thermal-aware dispatch;
3. modular, fault-isolated electrical architecture;
4. interoperable charging and resilient vertiport microgrids;
5. uncertainty-aware multi-fidelity simulation;
6. evidence provenance and certification traceability;
7. lifecycle economics and repairability.

The strongest result would be a vehicle/system design that is better on a declared mission and whose advantage survives independent reproduction.

## 6. Immediate execution sequence

### P0 — freeze the mission design basis

Create versioned mission cards for:
- **Urban eVTOL reference:** NASA's four-passenger, 50-mile planning case; explicitly define crew, payload, reserve, route, weather, and turnaround.
- **Regional eVTOL reference:** 10+ passenger planning class; treat battery-only feasibility as an open question.
- **Regional hybrid-electric fixed-wing:** mission range and payload are configuration inputs; compare fuel, battery, generator, and reserve contributions.
- **Cargo/medical mission:** payload and landing-site constraints replace passenger comfort assumptions; include diversion and weather access.

No vehicle layout is selected until these mission cards are complete.

### P1 — build the mission-energy reference model

The first executable increment now exists on the Generation 4 branch as `tools/airborne-energy-observability-validator/src/mission_energy.rs`, exported as a separate `mission_energy` module. It is part of the root Cargo workspace and is covered by the repository's existing package-specific CI step.

A second analytical module now exists at `tools/airborne-energy-observability-validator/src/fixedwing.rs`. It provides:
- a tropospheric ISA atmosphere approximation from sea level to 11 km;
- steady, level, unaccelerated fixed-wing lift closure (lift = weight);
- induced drag from `CL² / (π · Oswald · aspect ratio)`, added to a supplied zero-lift drag coefficient;
- useful thrust power and propulsor shaft power using a supplied propulsive-efficiency value;
- an explicit error when the calculated cruise lift coefficient reaches the declared maximum-lift coefficient;
- a tested output seam from fixed-wing cruise shaft power into the battery mission-energy ledger.

Seven fixed-wing tests cover sea-level/tropopause atmosphere references, the drag/power equations, insufficient lift margin, invalid/non-finite input, unsupported atmosphere altitude, and end-to-end energy-ledger use.

This is a low-fidelity analytical model. It requires a sourced drag polar and efficiency inputs; it is not validated by wind-tunnel, component, or flight data. It does not model climb, manoeuvres, takeoff, stall dynamics, VTOL hover, conversion/transition, trim drag, compressibility, Reynolds effects, propwash, aeroelasticity, or structural/certification constraints.

The v0.1 ledger:
- requires planned-mission segments and a separate, explicit reserve scenario;
- converts declared propulsor-shaft power to pack DC-bus power using separate inverter and motor efficiencies;
- adds auxiliaries at the DC-bus measurement boundary;
- accounts for one bounded peak interval per segment without double-counting it on top of average power;
- checks continuous power, peak power, peak duration, and post-reserve usable-energy margin;
- reports end-of-mission SOC, energy margin, Wh/passenger-km, and Wh/tonne-km where inputs permit;
- rejects empty reserve assumptions, invalid efficiencies, non-finite values, and zero-energy mission profiles.

Eight focused mission-energy tests and seven fixed-wing cruise tests cover the analytical ledger and its first physics integration seam (plus 12 pre-existing receiver-observability tests in the same package). They are source-level tests, not a reported compile or CI pass. This first calculator does **not** predict lift/drag, propeller demand, battery voltage/current, non-linear SOC, thermal propagation, ageing, hybrid-generator dispatch, or uncertainty. Its required shaft-power profile must come from a separately justified model or measured data. The NASA four-passenger/50-mile reference mission card remains explicitly not-defined for flight segments and reserve; it must not be treated as a runnable aircraft design.

Next steps: add a strict JSON mission-card adapter, versioned efficiency and battery-map inputs, and independent known-answer test vectors. Then extend the ledger with thermal/voltage limits and hybrid energy-source accounting. Do not put an optimizer or AI controller in front of an unqualified energy model.

### P2 — build the design-space explorer

Use bounded, explainable multi-objective sweeps first. Explore:
- lift-plus-cruise versus tilt-prop/tilt-wing;
- battery-only versus series-hybrid and parallel-hybrid;
- propulsor count, disk loading, wing loading, and cruise speed;
- pack segmentation and thermal architecture;
- wired versus wireless ground charging;
- battery-only versus battery plus supercapacitor buffering;
- on-site generation and stationary storage.

Output Pareto fronts, sensitivity ranks, and rejected configurations with reasons.

### P3 — component and infrastructure evidence

Define test contracts for:
- motor dynamometer maps;
- inverter efficiency and fault isolation;
- propeller/fan thrust and acoustic maps;
- battery cycling, abuse, thermal propagation, and cold/hot performance;
- wired charging as the reference case;
- wireless charging under alignment, foreign-object, and thermal cases;
- stationary storage and site peak-demand simulations.

Use actual vendor datasheets as inputs only when revision, test method, and measurement boundary are known.

### P4 — model qualification and hardware-in-loop

Validate submodels against independent component data. Freeze datasets before tuning, retain held-out test cases, and publish discrepancies. Add deterministic fault injection for sensor failures, inverter trips, bus isolation, cooling loss, cell imbalance, and communication loss.

### P5 — flight-control integration

Develop dedicated fixed-wing/eVTOL flight dynamics and control interfaces only after the mission model and physics are grounded. Preserve the existing multirotor line; do not rename or stretch it into an airliner. The current trained-controller / cognition-causality work can continue on the microquad line, but its exact-head test evidence must be checked separately and it does not transfer aircraft validity. Symthaea's learned components should remain behind independently verified safety boundaries.

### P6 — certification and operational demonstration

Build a requirements-to-evidence matrix against the relevant authority and aircraft category. Engage experienced airworthiness and propulsion specialists before treating an engineering target as a compliance objective. Ground, HIL, and flight-test increments must have explicit stop criteria.

## 7. Non-negotiable engineering rules

- No sizing around speculative 500–700 Wh/kg batteries.
- No claim of aircraft range without payload, wind, reserve, temperature, ageing, and mission segment data.
- No battery-only architecture is selected until mass, power, reserve, and thermal closure pass.
- No supercapacitor is counted as meaningful cruise energy without measured, mass-inclusive evidence.
- No wireless charging concept is credited with zero-loss or in-flight energy.
- No learned controller receives unbounded safety-critical actuator authority.
- No simulation result is called flight validated.
- Unknown or contradictory evidence blocks the affected design decision; it does not silently become a favourable assumption.
- Every external benchmark records source, date, metric definition, and comparability limitations.

## 8. Decisions requested from the programme

The first reference benchmark should be the four-passenger, 50-mile urban eVTOL scenario recorded in [the mission card](electric-aviation/urban-evtol-4p-50mi-reference.mission.json). Its source is an Ion Storage Systems vendor slide in the 2022 NASA workshop deck, not a NASA-prescribed design basis. It allows controlled comparison across lift-plus-cruise and tilt configurations once payload, route, reserve, weather, and energy boundaries are resolved.

The second parallel track is captured in [the regional hybrid-electric fixed-wing mission template](electric-aviation/regional-hybrid-fixedwing-template.mission.json). NASA's 2026 megawatt-class hybrid-electric flight demonstration and the STARC-ABL work justify researching this path; the route and aircraft mission remain deliberately unspecified until we select a matched reference aircraft and reserve policy.

Wireless charging, supercapacitors, solar generation, and vertiport microgrids should be evaluated as subsystems within these missions—not marketed as independent breakthroughs.

## References

1. NASA, [Aerospace Battery Workshop — Next-Generation Solid-State Batteries for Aerospace Applications](https://www.nasa.gov/wp-content/uploads/2024/01/next-generation-solid-state-batteries-for-aerospace-applications.pdf), planning energy requirements.
2. Zhang et al., [Sustainable propulsion and advanced energy-storage systems for net-zero aviation](https://pubs.rsc.org/en/content/articlehtml/2025/ee/d5ee02865e), *Energy & Environmental Science* (2025), battery chemistry and readiness comparison.
3. NASA, [High-Efficiency Megawatt Motor (HEMM)](https://www.nasa.gov/eap-technology/electric-machines/hemm/), 1.4 MW development programme and target metrics.
4. NASA, [STARC-ABL](https://www.nasa.gov/eap-aircraft-concepts/starc-abl/), hybrid-electric concept and projected fuel-burn reduction.
5. NASA, [NASA, GE Aerospace Work Enables Hybrid-Electric Flight Demonstration](https://www.nasa.gov/directorates/rtmd/nasa-ge-hybrid-electric-flight/), 20 July 2026.
6. U.S. GAO, [Electric Aircraft: FAA Is Evaluating Designs for Certification](https://www.gao.gov/products/gao-26-107816), GAO-26-107816, 27 May 2026.
7. EASA, [Special Condition for VTOL and Means of Compliance](https://www.easa.europa.eu/en/document-library/product-certification-consultations/special-condition-vtol), including MOC-5 SC-VTOL Issue 2, 5 November 2026.
8. Joby Aviation, [Q3 2026 SEC filing](https://ir.jobyaviation.com/sec-filings/all-sec-filings/content/0001819848-26-000531/0001819848-26-000531.pdf), certification progress data as of 30 October 2026.
9. Archer Aviation, [Q3 2026 results](https://investors.archer.com/news/news-details/2026/Archer-Announces-Q3-2026-Results-Receives-FAA-Market-Survey-Certificate-Unlocking-LA-Operations-and-Boeing-Deal-Close-Imminent/default.aspx), 9 November 2026.
10. BETA Technologies, [Q3 2026 SEC filing](https://www.sec.gov/Archives/edgar/data/1784570/000162828026069120/q32026earningsrelease.htm), fleet and charging infrastructure metrics.
11. FAA, [Special Airworthiness Market Survey Certificate for BETA A250](https://www.faa.gov/newsroom/trumps-transportation-secretary-sean-p-duffy-announces-first-ever-special-airworthiness), 21 October 2026.
12. Symtropy and Symthaea source links listed in Section 1.
13. BETA Technologies, [Q3 2026 results](https://investors.beta.team/news-events/press-releases/detail/123/beta-technologies-inc-announces-third-quarter-2026-results), 3 November 2026: 250,000+ nautical miles, 168 charging sites, and A250 market-survey authorization ahead of type certification.
14. NASA, [HEMM](https://www.nasa.gov/eap-technology/electric-machines/hemm/) and [HEMM 2024 technical presentation](https://ntrs.nasa.gov/api/citations/20240008673/downloads/EATS%202024%20presention%20STI%20Final.pdf): 16 kW/kg active-component target and machine efficiency targets.
15. FAA, [AC 21.17-4 — Type Certification: Powered-lift](https://www.faa.gov/regulations_policies/advisory_circulars/index.cfm/go/document.information/documentID/1044836), issued 18 July 2025.

---

**Maturity label:** Research specification drafted. No aircraft performance, component capability, flight safety, or certification compliance has been demonstrated by this RFC.
