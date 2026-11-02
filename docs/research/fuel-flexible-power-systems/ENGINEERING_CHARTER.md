# Fuel-Flexible Power Systems — Engineering Charter

**Status:** Research programme / architecture proposal (v0.1)  
**Date:** 2026-11-02  
**Owners:** Luminous Dynamics / Symtropy / Symthaea  
**Evidence policy:** No performance, safety, certification, or commercial-readiness claim is accepted without traceable measurements or independently reproducible analysis.

## 1. Executive decision

Develop a **modular multi-fuel power-conversion family**, beginning with stationary generator applications and then deriving heavy-truck and marine configurations.

Do not pursue the misleading requirement "one engine burns every fuel." Fuels have incompatible ignition, injection, storage, materials, combustion, emissions and safety requirements. The stronger architecture is:

- a shared, serviceable mechanical and electrical platform where technically justified;
- fuel-specific conditioning, injection, ignition, combustion-chamber and aftertreatment modules;
- a typed fuel-property and provenance model;
- a constrained controller with deterministic safety interlocks;
- a co-simulation and evidence pipeline that separates model predictions from measured facts.

The first product hypothesis is a **fuel-flexible generator system**, not a universal combustion chamber. Truck and ship powertrains are derivatives with different duty cycles, packaging, certification and scale requirements.

## 2. What is differentiated — and what is not

Fuel-specific engine families on shared architectures are already an industry strategy. Cummins describes its X10 and X15 as part of its fuel-agnostic HELM platform; its announced MY2027 production ramp begins in 2027. Wärtsilä markets 32 and 46F methanol-capable marine engines. Everllence reported a factory acceptance test for a dual-fuel ammonia two-stroke in April 2026.

Therefore, "supports multiple fuels" alone is not a defensible novelty claim.

Potential differentiation worth testing:

1. **Fuel-quality-aware operation:** robust operation over measured composition/quality ranges, with uncertainty-aware derating and safe fallback rather than assuming a laboratory-grade fuel.
2. **System-level fuel flexibility:** jointly optimize fuel module, engine, generator, battery, aftertreatment, thermal recovery, storage mass/volume and duty cycle.
3. **Evidence-carrying digital thread:** every design candidate links to model version, calibration, uncertainty, test data, calibration certificates, emissions results and provenance.
4. **Lifecycle-and-duty-cycle optimization:** select the conversion pathway by delivered work (kWh, tonne-km, tonne-nautical-mile), not by engine efficiency alone.
5. **Repairable modularity:** common interfaces, replaceable fuel modules and regional serviceability, measured against parts count, repair time, uptime and whole-life cost.
6. **Active experimental design:** use Symthaea to select the next experiment with the greatest expected reduction in uncertainty or design risk.

These are research hypotheses, not proven advantages.

## 3. Scope and staged fuel portfolio

### Near-term candidates

| Family | Initial role | Engineering constraints |
|---|---|---|
| Petroleum diesel / low-sulfur diesel | Reference baseline | Reference calibration, regulated emissions, durability |
| HVO / renewable diesel | Near-drop-in liquid-fuel candidate | Verify specification, seals, lubricity, material compatibility and fuel provenance |
| Biodiesel blends | Controlled compatibility study | Blend-dependent oxidation, cold-flow, deposits, dilution and material constraints |
| Natural gas / biomethane | Dual-fuel generator candidate | Storage, methane slip, gas quality, pilot-fuel fraction and transient response |

### Later fuel-specific branches

| Family | Candidate conversion mode | Principal risks / constraints |
|---|---|---|
| Methanol / ethanol | Dedicated spark or pilot-ignited architecture | Low volumetric energy density, corrosion/materials, cold start, flame stability and toxic exposure |
| Hydrogen | Dedicated spark-ignition or other validated combustion concept; fuel-cell comparison | Storage, leakage, pre-ignition/flashback, NOx, embrittlement and infrastructure |
| Ammonia | Dedicated pilot-assisted marine combustion concept | Acute toxicity, leakage, low flame speed, ignition stability, NH3 slip, N2O/NOx and bunkering safety |

No fuel is admitted to an engine configuration merely because its name appears in this table. Each fuel requires a specification, approved handling concept, materials review, hazard analysis, validated calibration and applicable emissions/certification route.

**Initial recommendation:** diesel/HVO baseline first; then natural-gas/biomethane dual-fuel only after a separate gas train, ventilation, leak detection and safety review. Methanol, hydrogen and ammonia remain separate research branches rather than promised drop-in capabilities.

## 4. System architecture

### 4.1 Typed fuel and energy-carrier model

Each fuel record must identify, with units and source:

- composition and permitted composition range;
- lower and higher heating value;
- density and physical state at stated reference conditions;
- viscosity, vapor pressure, flash point and autoignition properties where relevant;
- cetane/octane or other ignition-quality descriptors where relevant;
- stoichiometric air/fuel ratio and relevant combustion-property data;
- contaminants, water, sulfur and trace species;
- storage temperature/pressure and boil-off or venting assumptions;
- compatible wetted materials and seal constraints;
- upstream pathway and well-to-tank emissions factors;
- source, date, test method, uncertainty and provenance.

Unknown values remain explicitly unknown. Do not silently fill gaps with a single-point estimate.

### 4.2 Modular conversion stack

1. **Storage and bunkering:** tank, containment, venting, pressure/temperature control and detection.
2. **Fuel conditioning:** filtration, separation, pumping, compression, vaporization and metering.
3. **Fuel module:** injectors, rails, valves, mixers, ignition/pilot hardware and safety shutoffs.
4. **Combustion core:** common base only where stress, heat transfer, knock/ignition and durability evidence supports it; fuel-specific heads, piston bowls, compression ratio or chamber geometry are allowed.
5. **Air path and turbocharging:** intake, intercooling, EGR where appropriate, boost control and filtration.
6. **Exhaust and aftertreatment:** fuel-specific oxidation, SCR, particulate and slip-control systems as required.
7. **Power conversion:** crankshaft, gearbox/PTO or generator/alternator, rectifier/inverter and optional battery.
8. **Thermal system:** coolant, oil, charge air, exhaust heat recovery and safe heat rejection.
9. **Control and diagnostics:** deterministic ECU state machine, calibrated maps, watchdogs, fault containment and event logging.

A shared part is not automatically a good part. Commonality is accepted only where it does not compromise efficiency, safety, service life or certification.

### 4.3 Two powertrain branches

**Generator-first branch**
- Engine-generator sets in modular sizes.
- Multiple sets can be dispatched in parallel so each engine operates in an efficient and emissions-compliant region.
- Battery buffers load steps, transient response and spinning reserve.
- Evaluate net AC electrical efficiency, not just brake thermal efficiency.

**Vehicle / vessel branch**
- Compare direct mechanical drive with series/parallel hybrid layouts.
- For trucks, optimize payload, gradeability, transient response, refuelling time and tonne-km.
- For ships, distinguish medium-speed diesel-electric installations from low-speed two-stroke direct propulsion; do not scale a truck engine geometrically into a large ship engine.

## 5. Symthaea and Symtropy responsibilities

### Symthaea — discovery, optimization and evidence reasoning

- Multi-objective candidate generation across efficiency, regulated emissions, lifecycle emissions, cost, reliability and maintainability.
- Bayesian/active experimental design: choose experiments that reduce uncertainty, not merely produce more runs.
- Robust optimization over fuel composition, ambient temperature, altitude, load, ageing and sensor uncertainty.
- Model discrepancy tracking: compare model predictions to measured data and widen uncertainty or reject a model when it fails.
- Calibration search with hard constraints; no unconstrained learned control of physical hardware.
- Route- and fuel-supply-aware dispatch studies with Sol-Atlas where relevant.

Symthaea is initially an **offline engineering assistant**. Any later online controller must be a separately validated, bounded, deterministic controller with independent protection layers.

### Symtropy — system digital twin and physical integration

Symtropy's current core is an N-dimensional rigid-body and coupled-system physics engine. It is not presently a validated reacting-flow CFD, chemical-kinetics, combustion or structural-fatigue solver.

Use Symtropy for:
- engine/generator/skid/vehicle/vessel rigid-body layout and interaction;
- mounts, shafts, driveline and vessel-system dynamics where supported by validated models;
- operational scenario orchestration, asset states, duty cycles and energy-flow bookkeeping;
- deterministic experiment replay, provenance and visualization.

Do **not** represent in-cylinder combustion or emissions chemistry with rigid-body collision physics or consciousness/Phi coupling. Integrate validated domain solvers through explicit data contracts:
- 0D/1D cycle and gas-exchange models for broad design sweeps;
- chemical kinetics / thermochemistry for reaction and ignition studies;
- CFD for in-cylinder flow, spray, mixing and combustion when justified;
- FEA and fatigue solvers for thermal-mechanical durability;
- hardware-in-the-loop and dynamometer measurements for control and calibration.

Candidate tools may include Cantera for thermochemistry/kinetics and OpenFOAM or specialist commercial engine-CFD tools for CFD, but suitability must be demonstrated on validation cases before adoption. The solver owns the physical result; Symtropy owns system orchestration, state and evidence integration.

## 6. Safety and assurance boundary

- No experimental fuel handling or combustion test without a facility-specific HAZID/HAZOP, risk assessment, operating procedure, emergency plan and qualified supervision.
- Ammonia and hydrogen require dedicated hazard engineering; no prototype fuel storage or plumbing is implied by this charter.
- Physical safety trips, overspeed protection, overpressure protection, leak detection and emergency shutdown must remain independent of Symthaea.
- Fuel switching is not assumed to be instantaneous. It is permitted only within a tested configuration and validated transition envelope.
- Out-of-distribution fuel quality, sensor disagreement, loss of cooling, abnormal combustion or emissions-control faults must trigger a defined safe response.
- Emissions compliance is configuration-, market- and duty-cycle-specific. No simulation result is a certification.

## 7. Benchmark programme

### 7.1 Reference comparisons

Every candidate is compared against:
1. a current production engine/generator in the same power and application class;
2. a best-in-class fuel-specific engine where reliable data exist;
3. battery-electric and/or fuel-cell alternatives where the duty cycle permits;
4. a baseline diesel vehicle/vessel using identical payload, route, speed, weather and reserve assumptions.

Report the measurement boundary, test procedure, fuel properties, ambient conditions, accessories, aftertreatment state and uncertainty. Do not compare peak engine BTE against net generator efficiency or a different duty cycle.

### 7.2 Core metrics and proposed gates

| Metric | Definition / boundary | Programme gate |
|---|---|---|
| Brake thermal efficiency | Brake work / fuel LHV input; gross/net convention declared | Match the same-class fuel-specific reference before claiming a viable shared core |
| Net electrical efficiency | Net AC output / fuel LHV input, including auxiliaries | Match or exceed the same-class reference at weighted operating points |
| Fuel Flexibility Efficiency Retention (FFER) | Integrated net AC efficiency on the matched frozen duty cycle / fuel-specific reference | Stage the internal target: ≥0.90 feasibility, ≥0.95 strong platform target, ≥0.97 stretch; require matched useful output and report each segment and uncertainty |
| Transient performance | Load acceptance, settling time, frequency/voltage excursions, smoke/knock/combustion stability | Meet generator/OEM power-quality and application requirements |
| Fuel-quality robustness | Performance and fault response across characterized fuel-property distributions | No unsafe state; bounded derating and detectable rejection outside validated envelope |
| Regulated emissions | NOx, PM/PN, CO, THC/CH4, NH3 slip and N2O where relevant | Meet the target market's applicable limits with measurement uncertainty included |
| Lifecycle GHG | Well-to-tank + tank-to-use, with CO2, CH4 and N2O as applicable | Publish pathway-specific gCO2e/kWh and gCO2e/tonne-km or tonne-nautical-mile |
| Reliability | Availability, forced outage rate, service hours, overhaul and component life | 1,000-hour demonstrator endurance gate; production target must match or exceed incumbent service intervals |
| Maintainability | Mean time to diagnose/repair, tool count, part commonality, technician skill | Demonstrate repairability with documented field procedure and timed trials |
| Economics | Capital, fuel, storage, aftertreatment, maintenance, downtime, residual value | Whole-life delivered-energy / transport cost within 5% of baseline before claiming flexibility premium is commercially justified |
| Model credibility | Prediction error, uncertainty coverage, calibration drift and out-of-domain detection | Independent held-out validation; failed validation blocks model use for design sign-off |

These are **proposed programme gates**, not published industry standards or achieved results. The 1,000-hour test is a demonstrator milestone, not a claim of production durability.

### 7.3 Test procedures and duty cycles

- For complete AC generator-set performance and test methods, evaluate ISO 8528-5:2025 and ISO 8528-6:2023; ISO 8528-1:2018 remains published but ISO/DIS 8528-1 is under revision. Keep the exact edition and application classification in the test record.
- For applicable stationary/non-road engine emissions, evaluate ISO 8178-1:2020 test-bed methods and ISO 8178-2:2021 field methods. ISO 8178-1 is under revision; check the current edition before testing.
- Use the applicable heavy-duty on-road certification procedures and transient/steady-state cycles separately; a generator-set test does not qualify a road engine or vehicle.
- For marine engines, use the NOx Technical Code 2008 and applicable MARPOL Annex VI requirements, including the correct Tier / ECA scope. Regulation 13 applies to installed marine diesel engines over 130 kW subject to scope and exceptions.
- For ships, report EEXI/CII-relevant operational metrics and IMO well-to-wake lifecycle intensity; engine g/kWh alone is insufficient.

See [GENERATOR_BENCHMARK_PROTOCOL.md](GENERATOR_BENCHMARK_PROTOCOL.md) for the detailed standards register, synthetic load profiles, measurement boundary, test matrix, uncertainty and FFER calculation.
- Add real duty-cycle trials: South African high ambient temperature, altitude, dust, fuel-quality variance, remote maintenance and route/load profiles; port/shore-power availability for vessels.

The team must verify the currently applicable local certification rules before selecting hardware or claiming a market-ready configuration.

**IMO Net-Zero Framework status snapshot (2026-11-02):** the framework should be treated as proposed / under negotiation, not as adopted law. IMO's published schedule lists MEPC 85 for 30 November–3 December 2026; the resumed second extraordinary session is listed for 4 December subject to confirmation by MEPC 85. Freeze this status with a retrieval date and re-check before any vessel investment or compliance claim.

## 8. SOTA baseline and honest comparison

| Industry reference | Public evidence | Implication |
|---|---|---|
| **Mainspring Linear Generator** | Product datasheet reports a 250 kW module, a 2.5 MW block made from ten 250 kW units, 46% net-AC LHV efficiency on natural gas/biogas, <1.5 ppm NOx and 0–100% output range. It states that the 46% result was measured under ASME PTC 50 at 15°C and 1 atm, including 1% assumed inverter/line loss. | **Closest direct competitor and mandatory primary benchmark.** Fuel-flexible stationary generation at this rating and a high net-AC efficiency claim already exist. A generic 250 kW multi-fuel generator is not a defensible novelty claim. |
| **Mainspring California Energy Commission demonstration** | A 230 kW beta unit ran at a grocery site for nine months; the 2024 final report shows 41.5% net-AC efficiency, above 40% over a 130–370 kW building-load range, and third-party source tests of 1.6 ppm NOx, 0.8 ppm CO and 8.4 ppm VOC. The report was prepared by Mainspring under a CEC-sponsored project and disclaims CEC approval of its accuracy; third-party status is stated for the emissions source test. | A real-site operational reference exists, not just a product concept. Preserve the distinction between the beta demonstration, the manufacturer’s later 46% product specification, and the separate emissions test. |
| **Mainspring / U.S. Air Force pilot** | The Department of the Air Force announced a 250 kW prototype pilot in March 2026 to collect fuel-efficiency, power, consumption and emissions data across natural gas, biomethane, syngas, ammonia, propane and hydrogen. | Ammonia and syngas belong in the competitor watchlist, but this is a pilot test programme—not published proof that the commercial product has already qualified every listed fuel. |
| **Mainspring commercial scale-up** | Mainspring announced an agreement with AEP for up to 1.6 GW through 2034; the first 190 MW tranche is announced for delivery beginning in 2027. | Commercial traction is substantial, but an announced agreement and future delivery schedule are not the same as independently observed installed capacity or fleet availability. |
| Cummins HELM / X10 / X15 | Cummins identifies fuel-agnostic engine platforms; MY2027 production ramp is scheduled through 2027 | Shared architecture across fuel-specific combustion variants is already an industry strategy. |
| Capstone C200S microturbine | Manufacturer/supplier materials report a 200 kW unit at 33% LHV electrical efficiency and gaseous-fuel compatibility including natural gas and biogas; exact net/gross boundary and site corrections must be checked in the controlled datasheet. | A commercial microturbine is a relevant same-scale comparator, particularly for footprint, service life and fuel-gas tolerance. |
| DOE SuperTruck 2 | DOE reports >52% engine brake thermal efficiency demonstrated by all teams, with three pathways to 55% | A 52–55% peak-BTE frontier is a heavy-duty engine benchmark, but BTE is not net AC efficiency and must not be compared directly with generator figures. |
| Wärtsilä 32 Methanol | Commercial marine methanol engine, 3.4–5.2 MW; 46F Methanol can use conventional fuels and methanol | Methanol-capable marine engines are already commercial. |
| Everllence / MAN B&W ammonia | Manufacturer reports Type Approval Test in June 2026 and first-unit Factory Acceptance Test on 20 April 2026 for dual-fuel 6G60ME-LGIA | Ammonia two-stroke development is advancing toward shipboard deployment; vendor-stated lifecycle GHG claims still require independent fuel-pathway verification. |
| Battery-electric / fuel-cell HDVs | Recent lifecycle analysis reports significant GHG reductions for BEVs and FCEVs, depending on vehicle class, duty cycle and energy source | Combustion is not the default climate winner; compare full transport work and regional energy pathways. |

### Mandatory Mainspring benchmark questions

Before any SOTA claim, obtain and reconcile:
- the full MSE3 / 2.5 MW test protocol, raw measurement boundary, fuel composition, LHV/HHV convention, ambient correction, auxiliaries and uncertainty;
- why the manufacturer cites ASME PTC 50 for the linear generator and whether that code's fuel-cell-system scope is applicable or an agreed adaptation;
- performance maps over low-to-full load, fuel switching transitions, starts/stops, step loads, parasitic loads, degradation and emissions;
- the meaning of the published >1.5 million **core run hours**: core-hours are not automatically single-unit elapsed hours, fleet availability or MTBF;
- clarification of the datasheet's 22-ton "unit weight" and whether this means one 250 kW module, the full package or another assembly;
- fuel-by-fuel qualification status: the standard product specification lists biogas, natural gas, hydrogen and propane; the U.S. Air Force 2026 programme separately proposes testing syngas and ammonia.

Mainspring describes its linear generator as a low-temperature, non-combustion reaction driving magnets through copper coils. It is therefore a distinct conversion topology—not simply another reciprocating engine. Our benchmark/model architecture must represent at least:
1. reciprocating engine → shaft → alternator;
2. free-piston / linear generator → direct electrical conversion;
3. microturbine / gas turbine → shaft → alternator;
4. fuel cell → DC power conditioning / inverter;
5. battery storage / inverter as a delivered-service comparator.

Do not compare brake thermal efficiency, net-AC efficiency, stack DC efficiency and battery round-trip efficiency as if they share one boundary.

### Revised competitive thesis

We cannot claim novelty for "a 250 kW fuel-flexible stationary generator", gaseous-fuel switching, 0–100% dispatchability, near-zero NOx or a 46% net-AC LHV figure alone. Those claims are already present in the Mainspring market / evidence baseline.

A defensible Luminous research wedge must be demonstrated in one or more of these areas:
- independently reproducible fuel-quality robustness across measured composition and contaminant distributions, not just nominal fuel labels;
- validated liquid-fuel and marine duty-cycle integration (HVO, biodiesel blends, methanol and ammonia pathways) where the actual system boundary and safety case make sense;
- substantially better mass / volume / delivered-power packaging for mobile truck and marine installations;
- lower whole-life cost, maintenance downtime or supply-chain risk on a matched South African duty cycle;
- superior net-AC efficiency or lifecycle GHG on the exact same test boundary and fuel batch;
- evidence-carrying control and active experimental design that reduces qualification cost or decision regret versus established optimizers.

**No "better than SOTA" claim** until like-for-like independent data exist. Mainspring's commercial product page, the 2024 CEC-sponsored demonstration report, the 2026 Air Force pilot and its announced AEP agreement are different evidence classes and must not be merged into one undifferentiated performance claim.

## 9. Paradigm-shift research hypotheses

### H1 — Fuel-agnostic at the system level
The unit of design is not the engine block; it is the complete energy-conversion system: storage + fuel module + engine + generator/motor + battery + aftertreatment + thermal system + supply chain.

### H2 — Fuel as a probabilistic operating envelope
Treat fuel quality as a distribution, not a nominal point. Calibrate to measured ranges, quantify uncertainty and define safe operating envelopes and rejection boundaries.

### H3 — Flexibility as a measurable asset
Quantify the cost of flexibility: efficiency retention, extra mass/volume, capital cost, maintenance burden, emissions-system complexity and outage risk. Maximize useful flexibility per added cost and mass.

### H4 — Active learning for physical experiments
Select the next test by expected information gain / expected reduction in decision risk. Include replicated baselines, randomized run order, calibration checks, negative controls and held-out validation.

### H5 — Useful work is the objective
Optimize cost, lifecycle emissions and reliability per kWh delivered, tonne-km or tonne-nautical-mile. Include payload loss, fuel tank volume, charging/refuelling time, reserve power and route constraints.

### H6 — Repairability and regional resilience are design variables
Measure time to restore service, replaceable module count, local parts availability and skill requirements. Include South African logistics and service constraints from the first requirements baseline.

## 10. Work plan — start now, do not wait for runners

### Work package 0 — evidence baseline
- Freeze public competitor references and dates.
- Define the reference duty cycles and baseline machine.
- Create a source-controlled fuel-property dataset with units and provenance.
- Separate hard facts, vendor claims, model assumptions and proposed targets.

### Work package 1 — typed data contracts
Define versioned Rust types / schemas for FuelSpec, FuelBatch, ConversionModule, EngineMap, DutyCycle, EmissionsResult, TestRun, CalibrationRecord, ModelManifest and EvidenceReceipt. Every physical quantity has units; no unitless magic floats.

### Work package 2 — transparent 0D system model
Build a small, inspectable energy-balance and generator dispatch reference model. First tests:
- first-law energy balance closure;
- dimensional/unit checks;
- fuel mass-flow / LHV to input-power consistency;
- brake and electrical efficiency boundaries;
- auxiliary load accounting;
- net-vs-gross reporting;
- uncertainty propagation;
- invalid / missing / non-finite data fail closed.

Do not use guessed vendor performance values as calibration truth.

### Work package 3 — Symthaea optimizer
Search Pareto fronts over architecture, fuel module, load dispatch, battery size, thermal recovery and maintenance. Use deterministic baselines before learned search; preserve seeds, configuration and exact input digests. Candidate outputs are advisory until independently verified.

### Work package 4 — Symtropy co-simulation adapter
Add typed import/export of geometry, mass/inertia, thermal boundary conditions, shaft loads, mount loads, operating-state traces and evidence manifests. Keep CFD/chemistry solver APIs outside the rigid-body core.

### Work package 5 — experiment and HIL plan
Specify sensor calibration, crank-angle-resolved pressure measurement, fuel mass flow, air flow, torque, speed, exhaust gas analysis, temperatures, vibration and data acquisition. Design the single-cylinder campaign before any procurement.

### Work package 6 — demonstrator
Only after model validation and safety review, select an instrumented research engine and test facility. Progress through baseline liquid fuels, approved renewable liquid fuels, then a separately engineered gaseous dual-fuel branch.

## 11. Minimum reproducibility receipt

Each run must produce:
- immutable run ID and timestamp;
- source commit SHAs and tool/solver versions;
- model and calibration digests;
- fuel batch and laboratory-property record;
- hardware configuration and sensor calibration IDs;
- duty-cycle and ambient profile;
- raw-data digest plus derived-data digest;
- uncertainty method and confidence interval;
- pass/fail gates and every violated constraint;
- independent verifier result and reviewer identity.

A queued run is not a pass. A simulation is not an experiment. A vendor claim is not an independent measurement.

## 12. Open decisions

1. Initial generator rating and target market: provisional 50–250 kWe industrial range; finalize from customer duty cycle and service requirements, not intuition.
2. First fuel transition after baseline: HVO compatibility first, then gas dual-fuel only with a suitable test facility and safety case.
3. Shared-core boundaries: select from measured commonality and thermal/mechanical stress analysis.
4. Whether the first commercial target is remote industrial power, port equipment, mining support or coastal cargo auxiliary generation.
5. Required regulatory jurisdiction(s): South Africa first, plus export markets only after mapping their current certification requirements.

## 13. Sources and dated baseline

- Mainspring Linear Generator product specifications (manufacturer claims; accessed 2 November 2026): https://www.mainspringenergy.com/product/
- Mainspring 2.5 MW / 10 × 250 kW product datasheet R40322: https://linear-power.files.svdcdn.com/production/Mainspring-Linear-Generator-2.5MW-Datasheet-R40322.pdf
- California Energy Commission, CEC-500-2024-037 project page: https://www.energy.ca.gov/publications/2024/high-efficiency-and-ultra-low-emissions-linear-generator-demonstration-project
- California Energy Commission / Mainspring, final project report PDF: https://www.energy.ca.gov/sites/default/files/2024-05/CEC-500-2024-037.pdf
- U.S. Department of the Air Force / AFMC, 250 kW Mainspring multi-fuel pilot (4 March 2026): https://www.afmc.af.mil/News/Article-Display/Article/4421866/air-force-pilots-fuel-flexible-generator-for-travis-afb/
- Mainspring, U.S. Air Force pilot project announcement (4 March 2026): https://www.mainspringenergy.com/news/mainspring-energy-awarded-u-s-air-force-pilot-project-for-multi-fuel-resilient-power-generation-at-travis-air-force-base
- Mainspring / AEP up-to-1.6 GW sales agreement (13 October 2026): https://www.mainspringenergy.com/news/mainspring-energy-signs-landmark-1-6-gw-agreement-with-aep-for-reliable-affordable-power-capacity
- ASME, Performance Test Codes catalogue (PTC 50 is listed for Fuel Cell Power Systems Performance): https://www.asme.org/codes-standards/publications-information/performance-test-codes
- Capstone C200S microturbine: https://www.capstoneenergyplus.com/products/capstone-microturbines/c200s
- U.S. DOE, SuperTruck results and SuperTruck 3 goals (10 June 2024): https://www.energy.gov/cmei/vehicles/articles/fotw-1346-june-10-2024-us-doe-supertruck-3-initiative-seeks-reduce
- Cummins, MY2027 X10/X15 HELM production transition (2026): https://investor.cummins.com/news/detail/700/cummins-announces-phased-model-year-2027-product-launch-plan
- Wärtsilä, marine methanol engines: https://www.wartsila.com/marine/products/engines-and-generating-sets/methanol-engines
- Everllence, ammonia engine Factory Acceptance Test (20 April 2026) and Type Approval Test (June 2026): https://www.everllence.com/marine/products/two-stroke-engines/ammonia-engine
- U.S. DOE Alternative Fuels Data Center, fuel properties comparison: https://afdc.energy.gov/fuels/properties
- ISO 8528-1:2018 and revision status: https://www.iso.org/standard/68539.html
- ISO 8528-5:2025 generator sets: https://www.iso.org/standard/85960.html
- ISO 8528-6:2023 set-level test methods: https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/07/10/71010.html?browse=ics
- ISO 8178-1:2020 test-bed emissions measurement: https://www.iso.org/standard/79330.html
- ISO 8178-2:2021 field emissions measurement: https://www.iso.org/standard/75546.html
- IMO, NOx Regulation 13 and marine engine certification: https://www.imo.org/en/ourwork/environment/pages/nitrogen-oxides-%28nox%29-%E2%80%93-regulation-13.aspx
- IMO, lifecycle GHG intensity guidelines for marine fuels: https://www.imo.org/en/ourwork/environment/pages/lifecycle-ghg---carbon-intensity-guidelines.aspx
- IMO, EEXI/CII framework: https://www.imo.org/en/mediacentre/hottopics/pages/eexi-cii-faq.aspx
- IMO, Net-Zero Framework status: https://www.imo.org/en/mediacentre/hottopics/pages/faqs-the-imo-net-zero-framework.aspx
- PNNL / Environmental Science & Technology, cradle-to-grave HDV pathway comparison (2026): https://www.pnnl.gov/publications/cradle-grave-lifecycle-analysis-us-medium-and-heavy-duty-vehicle-fuel-pathways

---

**Decision:** Proceed with the evidence baseline, typed data contracts and transparent 0D energy-balance model. Do not claim a fuel-agnostic engine exists until a defined configuration has passed an instrumented, independently reviewed test programme.


## 14. Screening-only reference fuel properties

A dated, machine-readable screening catalogue is available at:

[AFDC_REFERENCE_FUEL_PROPERTIES_2026-11-02.json](data/AFDC_REFERENCE_FUEL_PROPERTIES_2026-11-02.json)

It preserves the U.S. DOE AFDC raw values and units alongside normalized SI values for diesel, renewable diesel, B100, B20, CNG, LNG, ethanol, methanol, hydrogen and propane/LPG. It separately records the IPCC 2006 default mass-basis NCV and 95% bounds for gas/diesel oil.

**Use restrictions:**
- These are published reference values, not batch measurements, supplier certificates or engine calibration maps.
- The AFDC source explicitly notes that actual fuel energy content and properties vary with geography, fuel mix and feedstock.
- Volumetric values remain MJ/L. They must not be converted to MJ/kg without a compatible density at a declared reference temperature.
- The AFDC renewable-diesel category is not proof that an individual product is HVO.
- No uncertainty is invented where the source does not provide one; unknown remains unknown.
- A production calculation requires the actual batch's accepted LHV evidence and a flow measurement linked to that batch and time interval.

The dataset is intended to exercise typed data contracts and establish rough screening bounds. It is not admitted as physical performance evidence.


## 15. Machine-readable competitor evidence catalogue

The dated industry baseline is also maintained as structured evidence:

- [COMPETITOR_BENCHMARKS_2026-11-02.json](data/COMPETITOR_BENCHMARKS_2026-11-02.json)
- [COMPETITOR_BENCHMARK_SCHEMA_2026-11-02.json](data/COMPETITOR_BENCHMARK_SCHEMA_2026-11-02.json)

The catalogue separates Mainspring's product specification, CEC-sponsored beta demonstration, third-party emissions source test, Air Force pilot plan and AEP commercial agreement. It also records Capstone, Cummins, DOE SuperTruck 2, Wärtsilä, Everllence and a battery-electric HDV lifecycle comparator.

The catalogue deliberately distinguishes measured outputs, manufacturer claims, government pilot announcements, peer-reviewed lifecycle studies and future commercial delivery plans. Null values mean unavailable, not zero. The dataset is an auditable benchmark register, not a calibrated engine map or a ranking.
