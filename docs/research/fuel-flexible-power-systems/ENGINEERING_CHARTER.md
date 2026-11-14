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
| Capstone Energy+ C200 / C250 | FY2026 SEC filing confirms the C200 200 kW platform; reports natural gas, renewable biogas, landfill gas, wastewater methane, sour/low-BTU gas and up to 30% hydrogen blends across product literature. It describes a new C250 250 kW platform as having completed test runs but still advancing through commercialization. The filing mentions ~33% simple-cycle efficiency but does not specify net/gross terminal or LHV/HHV basis. It also describes an 800 VDC-native platform and claims up to 5% power-efficiency improvement versus legacy distribution. | A strong microturbine comparator with real fuel-quality tolerance, an emerging 250 kW block and native-DC product direction; do not rank either the ~33% or up-to-5% figures until the exact test boundary is confirmed. |
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
- **native service-interface optimization:** co-design AC and DC delivery, inverter stages, bus voltage, distribution losses, storage and customer load so the same useful service is delivered with fewer conversion losses; demonstrate it against matched AC-terminal and DC-bus baselines rather than citing a component-level percentage.

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

### H7 — Design the conversion chain around the useful load interface
Do not assume AC generation is always the best final interface. For data centres and DC-native industrial loads, investigate regulated high-voltage DC generation and distribution; for conventional industrial AC, retain the net-AC benchmark; for trucks and ships, optimize delivered propulsion work. Compare the complete chain from fuel or stored energy through conditioning, conversion, distribution and load. A claimed reduction in inverter stages is only valuable if the matched delivered-service measurement confirms a net system improvement.

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
- Capstone Energy+ FY2026 Form 10-K (C200, C250 development, fuels, 800VDC and simple-cycle efficiency): https://www.sec.gov/Archives/edgar/data/1009759/000110465926077817/cgeh-20260331x10k.htm
- Capstone FY2025 Form 10-K (multi-fuel injectors and hydrogen development programme): https://www.sec.gov/Archives/edgar/data/1009759/000155837025008930/cgeh-20250331x10k.htm
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


## 16. Maritime tank-system energy density and payload penalty

A separate literature-reference dataset is maintained at:

[MARINE_FUEL_STORAGE_REFERENCES_2026-11-02.json](data/MARINE_FUEL_STORAGE_REFERENCES_2026-11-02.json) and [MARINE_FUEL_STORAGE_SCHEMA_2026-11-02.json](data/MARINE_FUEL_STORAGE_SCHEMA_2026-11-02.json)

The 2026 container-shipping study reports tank-included volumetric energy-density values of approximately:
- MGO: 10.1 MWh/m³ (reference);
- methanol: 4.4 MWh/m³ (~2.30× MGO tank-system volume for equal LHV energy);
- LNG / bio-LNG: 3.1 MWh/m³ (~3.26×);
- liquid ammonia: 2.9 MWh/m³ (~3.48×);
- liquid hydrogen: 1.3 MWh/m³ (~7.77×).

The same study reports tank-included gravimetric energy densities: MGO 10.1 MWh/t, methanol 4.6, LNG 8.3, liquid ammonia 4.2, liquid hydrogen 5.6. Lithium-ion storage is reported separately at 0.3 MWh/m³ and 0.17 MWh/t; it is not a fuel and must not be assigned a fuel-tank volume ratio.

These are **study-specific tank-system references**, not universal tank design properties. [MARAD's December 2024 Energy Efficiency and Alternative Fuels Technical Guide](https://www.maritime.dot.gov/sites/marad.dot.gov/files/2025-03/Energy%20Eff.%20and%20Alt.%20Fuels%20Technical%20Guide%20-%202024.pdf) separately estimates tank volume at ~7.6× MGO for liquid hydrogen, ~4× for ammonia and ~2.3× for methanol; it gives ammonia volumetric LHV of 12.7 MJ/L. The older MARAD 2022 guide reports ~4.1× MGO for ammonia, while a Nordic technology report gives 7.2 MJ/L for ammonia in a 20-bar tank. These assumptions must remain separate rather than being averaged because tank pressure, temperature, insulation, containment and reference boundaries differ.

For maritime design, fuel choice affects cargo capacity, endurance, hull arrangement, tank mass, bunker frequency, safety spacing and port compatibility. Optimize delivered tonne-nautical-mile and retained payload, not engine efficiency alone. The reference data exclude some vessel-specific structure, piping, ventilation, bunkering and safety systems; detailed naval architecture and hazard engineering remain required.


## 17. Fuel flexibility is a validated operating envelope, not a fuel count

A fuel counts as supported only for an exact hardware configuration and evidenced operating envelope. A label such as "hydrogen capable", "multi-fuel" or "biodiesel compatible" is not sufficient.

For every fuel/configuration pair, freeze:
- batch-property ranges and composition, with source, units and uncertainty;
- exact conditioning/storage train and engine/combustion hardware;
- cold/hot start behavior, load map and minimum stable load;
- emissions across the applicable duty cycle;
- material, seal, lubricity, oxidation and corrosion compatibility;
- transient response and derating;
- transition procedure, purge/flush requirements, cross-contamination and transition fuel use;
- durability / inspection interval and maintenance burden;
- safety case, fuel supply availability and certification scope.

Use the machine-readable [FUEL_CONFIGURATION_QUALIFICATION_SCHEMA_2026-11-02.json](data/FUEL_CONFIGURATION_QUALIFICATION_SCHEMA_2026-11-02.json) to record exact configuration/fuel qualification status. A `qualified_within_scope` status requires a completed physical test and emissions test, independently reviewed physical evidence, an approved or confirmed-not-required safety/approval record, and no unresolved gaps.

Report a **flexibility vector**, never one opaque scalar:
1. **Validated Fuel Envelope Coverage:** fraction of a preregistered, rights-cleared fuel-batch distribution inside a physically tested and independently reviewed operating envelope. Report by fuel, batch property and supply region. Uncharacterized batches do not count as supported.
2. **Robust FFER:** median, 5th percentile and worst validated matched-duty-cycle efficiency retention across tested batches, with uncertainty. Do not report only a mean.
3. **Fuel Transition Reliability:** successful safe transitions / attempts, confidence interval, transition time, unserved power and transition fuel use. Do not assume instantaneous switching.
4. **Flexibility Overhead:** incremental capital cost, mass, volume, parasitic energy, maintenance hours and spare-parts burden versus the best fuel-specific reference.
5. **Fuel Supply Resilience:** service hours/days under frozen regional supply-disruption scenarios, using dated/provenanced availability evidence.
6. **Compliance Envelope:** configuration- and jurisdiction-specific emissions/safety/approval receipts. Planned fuel testing remains planned, not qualified.

A candidate advances only if it expands validated coverage or reduces flexibility overhead without degrading useful service, emissions, safety, durability or lifecycle performance.

## 18. Grid-forming, black-start and resilience are product capabilities

An energy-conversion platform should be assessed as a potential **microgrid resource**, not only as a source of net kWh. The capability to form voltage/frequency, island, black-start, restore staged loads and reconnect safely may be as commercially valuable as a small efficiency gain.

The 2026 IEA energy-innovation review describes grid-forming inverters as deployed in pilots and early utility systems, while standards, protection practices, low-inertia models and multi-vendor interoperability remain active development areas. NREL's hardware test protocol shows how a unit can pass black start yet fail a reported ROCOF test: capability claims must be test-specific, not bundled into a generic "grid-forming" label.

The benchmark matrix is machine-readable at [GRID_FORMING_MICROGRID_BENCHMARKS_2026-11-03.json](data/GRID_FORMING_MICROGRID_BENCHMARKS_2026-11-03.json) with [schema](data/GRID_FORMING_MICROGRID_BENCHMARK_SCHEMA_2026-11-03.json). It contains 7 test groups and 24 cases, with no universal pass thresholds invented; every case requires a standard / utility / mission acceptance source.

A more demanding multi-vendor reference is the **UNIFI 1-MW multi-vendor grid-forming hardware experiment** (National Laboratory of the Rockies, presentation 25PESGM3880, July 2025). It combines 7 GFM inverters (760 kVA), 4 GFL inverters (340 kVA), a synchronous generator and five controllable loads across five test configurations. GFM asset-mix ranges from 0% to 100%; GFM nameplate share ranges from 0% to 69%; inverter-based resource penetration reaches 100%. The report records that higher GFM share reduced some oscillations but exposed unexpected GFM-to-GFM interactions, ROCOF failures in the 68% and 100% GFM asset-mix scenarios, and unequal power sharing. Changing frequency droop from 0.6% to 1.2% allowed those two test scenarios to survive the described ROCOF condition; that tuning is test-specific and must not be transplanted as a universal setting.

The source was published under the laboratory's former NREL name. DOE renamed the institution the National Laboratory of the Rockies (NLR) effective 1 December 2025, and its official domain changed to nlr.gov in 2026. Cite legacy publication identifiers as printed but use the current NLR source records and URLs.

Use the separate [POWER-SYSTEM-RESILIENCE-001 #3418](https://github.com/Luminous-Dynamics/symtropy/issues/3418) workstream for:
- grid-following / grid-forming mode contracts by topology;
- black-start from a declared dead-bus state with auxiliaries included;
- planned / unplanned islanding and reconnection;
- source loss, load pickup, inrush and overload tests;
- active/reactive power control, voltage/frequency response, ROCOF and harmonics;
- fault protection in grid-connected and islanded modes;
- multi-vendor interaction, communications failure and cyber-safe degraded modes.

The current standards snapshot must distinguish published standards from revision projects:
- IEEE 2030.7-2017 and IEEE 2030.8-2018 are the published microgrid-controller specification/testing editions; P2030.7 (approved 27 March 2025) and P2030.8 (approved 26 March 2026) are active revision projects, not published replacement standards.
- IEEE 2030.12-2025 is an active microgrid-protection guide, published 23 October 2025.
- IEEE P3710.1 is an active project (approved 26 March 2026) for 300–1500 V DC distribution design in data centres.
- OCP Power Distribution published LVDC white-paper version 1.1.0 in September 2026; it is an open ecosystem reference, not a statutory code.

Use exact source editions and site utility/mission requirements for acceptance. The system model may support architecture comparison, but no bookkeeping PASS establishes black-start capability, grid-forming performance, protection coordination, cybersecurity, interconnection approval or operational permission.


## 19. 2026 dynamic marine powertrain literature baselines

A new machine-readable register is maintained at [MARINE_POWERTRAIN_LITERATURE_BASELINES_2026-11-03.json](data/MARINE_POWERTRAIN_LITERATURE_BASELINES_2026-11-03.json), validated against [MARINE_POWERTRAIN_LITERATURE_SCHEMA_2026-11-03.json](data/MARINE_POWERTRAIN_LITERATURE_SCHEMA_2026-11-03.json).

It records two new 2026 peer-reviewed, vessel-specific system studies:

- Elkafas & Lazakis, *Energy Conversion and Management* 366 (15 October 2026), DOI 10.1016/j.enconman.2026.121836: dynamic deep-sea container-ship model comparing e-ammonia/e-methanol/e-liquid-hydrogen dual-fuel engines and PEMFC systems. The study reports direct-H2 PEMFC as the strongest energy/lifecycle-GHG performer, while e-ammonia dual-fuel engines provide the lowest reported annual and CO2 abatement cost under its assumptions. It reports 3.4–5 percentage-point PEMFC efficiency penalty for ammonia cracking and 8.3–9.4 points for methanol reforming versus direct hydrogen.
- Zincir & Arslanoglu, *Next Energy* 13 (October 2026), DOI 10.1016/j.nxener.2026.101068: well-to-wake LCA/LCC using real operational data from a 6,177-GT Mediterranean/Black Sea general-cargo vessel. The paper reports e-methanol as the largest GHG-reduction pathway (76.2% vs MDO), hydrogen at 45.7%, e-ammonia at 32.4%; hydrogen's lifecycle cost ~2.7× LNG and tank volume ~3.7× MDO for equal propulsion energy, with estimated 6.8% cargo-deadweight displacement on that vessel.

These results must remain tied to their vessel, route, functional unit, energy conversion model and lifecycle assumptions. They reinforce the central thesis: **no fuel wins independently of conversion topology, storage, route, payload, electricity/fuel pathway and stakeholder objective**. Do not apply one study's ranking or tank-volume penalty to another vessel.


## 20. Reproducible matched-service and grid-resilience benchmark packs

Two additional machine-readable benchmark packs are maintained with schemas:

- [MATCHED_SERVICE_FFER_FIXTURES_2026-11-03.json](data/MATCHED_SERVICE_FFER_FIXTURES_2026-11-03.json) and [schema](data/MATCHED_SERVICE_FFER_SCHEMA_2026-11-03.json): four synthetic FFER scenarios (two valid comparisons; surplus excluded; under-delivery rejected; excessive tolerance rejected). The input-to-output arithmetic has been independently recomputed. These are software oracle fixtures, not physical performance evidence.
- [GRID_FORMING_MICROGRID_BENCHMARKS_2026-11-03.json](data/GRID_FORMING_MICROGRID_BENCHMARKS_2026-11-03.json) and [schema](data/GRID_FORMING_MICROGRID_BENCHMARK_SCHEMA_2026-11-03.json): seven test groups and 24 cases covering P/Q, load steps, black start, islanding, multi-vendor interactions, protection/cyber and DC-bus service. Acceptance thresholds remain unset until the specific standard edition, utility agreement and mission boundary are frozen.

These packs make the benchmark plans executable and machine-auditable without turning synthetic arithmetic or literature tests into product qualification.


The cross-topology service contract fixture pack is [TYPED_SERVICE_CONTRACT_FIXTURES_2026-11-03.json](data/TYPED_SERVICE_CONTRACT_FIXTURES_2026-11-03.json), validated against [TYPED_SERVICE_CONTRACT_SCHEMA_2026-11-03.json](data/TYPED_SERVICE_CONTRACT_SCHEMA_2026-11-03.json). It exercises:
- engine-generator versus linear-generator on a matched net-AC boundary and common fuel-LHV basis;
- microturbine-rectifier versus battery at a matched regulated-DC bus, with no invalid fuel-LHV versus stored-electricity efficiency ratio;
- diesel propulsion versus battery-electric propulsion at a propulsor-work boundary, also retaining distinct input-energy bases;
- explicit AC/DC boundary mismatch rejection;
- under-delivered service rejection.

These synthetic fixtures validate contracts and accounting only. They are not physical product results or lifecycle-energy comparisons.


## 21. South African electricity lifecycle baseline

A versioned, machine-readable register of the official 2024 national grid factors is maintained at [SOUTH_AFRICA_GRID_EMISSION_FACTORS_2024_PUBLISHED_2026-09-11.json](data/SOUTH_AFRICA_GRID_EMISSION_FACTORS_2024_PUBLISHED_2026-09-11.json), with [schema](data/SOUTH_AFRICA_GRID_EMISSION_FACTORS_SCHEMA_2026-11-03.json).

The DFFE published Government Notice 7891 on 11 September 2026 using 2024 calendar-year data. The four factors are:
- **DGGEF:** 0.994 tCO₂e/MWh (0.994 kgCO₂e/kWh), domestic generation boundary;
- **NGGEF:** 0.960 tCO₂e/MWh (0.960 kgCO₂e/kWh), national end-user consumption boundary, the intended location-based Scope 2 factor;
- **TLGEF:** 0.021 tCO₂e/MWh (0.021 kgCO₂e/kWh), transmission losses for relevant Scope 3 reporting;
- **DLGEF:** 0.071 tCO₂e/MWh (0.071 kgCO₂e/kWh), distribution losses for relevant Scope 3 reporting.

DFFE says the NGGEF excludes transmission and distribution losses; for typical facilities supplied through the distribution network, both TLGEF and DLGEF may apply to Scope 3 loss accounting. Facilities connected directly to transmission have a different boundary. DFFE also reports a residual imported-electricity factor of 0.244 tCO₂e/MWh for imports excluding South Africa, HCB and LEC hydro sources; it is a supporting methodology input, not a replacement for NGGEF.

The report says all four 2024 GEFs increased relative to 2023 (NGGEF 0.906→0.960; DGGEF 0.942→0.994). The factors are annual-average, location-based reporting values with an approximately two-year data lag—not hourly marginal dispatch factors, forward forecasts, PPAs or project-specific electricity factors.

**Platform implications:** battery charging, hydrogen electrolysis, methanol/ammonia production and DC-native data-centre supply must account for the actual electricity pathway and conversion losses. Do not label battery-electric operation "zero-carbon" from its tank-to-wheel operation; do not apply DGGEF as an onsite engine emission factor; and do not add transmission/distribution losses twice. The selected functional unit and Scope 2/Scope 3 lifecycle boundary must remain explicit.


## 22. Well-to-Service accounting

The next lifecycle/accounting workstream is [WELL-TO-SERVICE-001 #3421](https://github.com/Luminous-Dynamics/symtropy/issues/3421). It composes the typed delivered-service interfaces (net AC, regulated DC, propulsion work) with evidence-qualified upstream energy-carrier pathway graphs.

The first synthetic graph fixtures are [WELL_TO_SERVICE_FIXTURES_2026-11-03.json](data/WELL_TO_SERVICE_FIXTURES_2026-11-03.json) validated against [WELL_TO_SERVICE_SCHEMA_2026-11-03.json](data/WELL_TO_SERVICE_SCHEMA_2026-11-03.json). They exercise:
- a grid-charged battery delivering regulated DC service with DFFE NGGEF applied once to the metered electricity input;
- a hostile duplicate-emission-factor application to the same electricity lineage;
- two alternatives that deliver the same net-AC service but use unlike input-energy bases (fuel LHV and stored battery energy), which must not produce one efficiency ratio.

The synthetic battery fixture uses 100 kWh metered electricity, 90 kWh delivered DC and the 2024 DFFE NGGEF 0.960 kgCO2e/kWh. It computes 96 kgCO2e at the declared Scope 2 input boundary and 1.0667 kgCO2e per delivered kWh. Battery manufacturing/end-of-life are explicitly excluded. This is an accounting fixture, not a South African battery lifecycle assessment.

The design principle is **service comparability first; primary-energy and lifecycle comparability second**. A result may validly compare delivered service while reporting primary energy or lifecycle emissions as unresolved when the upstream carrier pathway is incomplete.


## 23. Grid-forming and AI data-centre benchmark upgrade — UNIFI V3 / IEEE 2800.2

The grid resilience benchmark has been upgraded from 7 groups / 24 cases to **9 groups / 32 cases**, using the current 2026 standards baseline and new frequency-domain / IT-load tests.

New source records:
- **UNIFI GFM Specifications V3**, NLR/TP-5D00-98381, DOI 10.2172/3016250. The 66-page 2026 report defines vendor-agnostic system and inverter requirements. V3 introduces frequency-domain criteria and quantitative pass/fail tests in addition to time-domain tests. Use the exact V3 test clauses and criteria; the benchmark deliberately does not invent threshold values. https://www.nlr.gov/docs/fy26osti/98381.pdf
- **IEEE P2800.1** is an active PAR, approved 10 December 2025, for functional capabilities and performance of power-electronic grid-forming equipment; it is not yet a published standard. https://standards.ieee.org/ieee/2800.1/12387/
- **IEEE 2800.2-2026** is a published active recommended practice, published 30 June 2026, for plant-level test and verification procedures demonstrating applicable IEEE 2800 conformity for bulk-power IBR plants. It is distinct from P2800.1 and does not replace microgrid-controller or islanding standards. https://standards.ieee.org/ieee/2800.2/10616/
- **IEEE Data Center Design for IT Load Protection white paper**, published 10 September 2026, covers AC/DC/hybrid distribution, redundancy, load priority, system/personnel protection, AI workload consumption, disturbance response, cooling and grid interaction. https://standards.ieee.org/ieee/White_Paper/12663/
- IEEE's Data Centers Standards Needs activity highlights fast-changing load time constants, grid interconnection, demand/forecast uncertainty, local generation and microgrids as cross-domain gaps. https://standards.ieee.org/industry-connections/activities/data-centers-standards-needs-analysis-and-recommendations/

Two new groups have been added:
1. **Frequency-domain GFM stability:** converter impedance/admittance scans, weak-grid impedance interaction, and multi-vendor fleet interaction scans. Preserve full complex frequency-response data, operating point, scan band, model validity and uncertainty. A time-domain pass alone cannot prove small-signal stability.
2. **AI data-centre IT-load protection:** workload ramp/burst replay, UPS/DC bus ride-through, priority shedding and cooling constraints, grid disturbance co-response, and flexible-load dispatch including deferred-work rebound and SLA constraints.

The resulting matrix contains 32 cases. Pass thresholds remain null until the exact UNIFI V3 / applicable IEEE clause, utility agreement and mission-specific requirement are selected. Hardware claims still require physical evidence; HIL and model evidence remain separately labelled.


## 24. Correlation-aware AI data-centre power and workload flexibility

The benchmark matrix is now **10 groups / 37 cases / 22 source records**. The newest addition is `DC-GRID-COORD-001`, which treats large AI data centres as workload-driven, non-stationary, potentially spatially correlated grid loads—not independent constant-power blocks.

New research inputs:
- Elsayed et al., *Scientific Data* (3 June 2026), DOI 10.1038/s41597-026-07496-6: open high-resolution dataset with 32 H100/B200 8-GPU node training sessions and 40 RTX 3060 sessions, more than 1.8 million samples, 50 Hz detailed traces, power/utilization/memory/temperature measurements. Public data: https://doi.org/10.6084/m9.figshare.31654879
- *Spatially Correlated Load Dynamics in AI Data-Center-Dominated Grids*, IEEE Transactions on Power Systems (31 August 2026), DOI 10.1109/TPWRS.2026.3729445: an IEEE 39-bus EMT study reports about 7× aggregate active-power variability and 11× frequency standard deviation for correlated versus independent loads in its studied conditions. Treat those ratios as study-specific, not universal.
- Caprara et al., *International Journal of Electrical Power & Energy Systems* 178 (May 2026), DOI 10.1016/j.ijepes.2026.111940: LAD-Flex estimates up to 22% load shedding during flexibility windows using more than one million Alibaba cluster tasks; this is trace-based modelling, not a field trial.
- Colangelo et al., *Nature Energy* 11 (2026), DOI 10.1038/s41560-025-01927-1: field demonstration on a 256-GPU cluster reduced power by 25% for 3 hours while maintaining declared AI QoS, with software orchestration and no hardware/storage additions.
- *Empirically-Calibrated H100 Node Power Models for Accurate AI Training Energy Estimation*, DOI 10.1088/2753-3751/ae2486: reports 11.4% MAPE on its measured H100 domain versus 27–37% for TDP-based estimates.
- *Wide-Area Power System Oscillations From Large-Scale AI Workloads*, IEEE Transactions on Power Systems (2026), DOI 10.1109/TPWRS.2026.3685506: studies periodic stochastic workload forcing on WECC 179-bus and NPCC 140-bus systems.

The new five-case group covers raw high-resolution trace calibration/holdout, correlation-versus-independence fleet aggregation, workload spectral forcing of grid modes, latency-aware demand response with rebound/SLA tracking, and matched replication of the 25%-for-3-hours field envelope.

Proposed internal research goals (not standards limits):
- beat the 11.4% H100 power-model MAPE on a comparable held-out workload/hardware domain, then stretch below 8% with calibrated prediction intervals;
- reproduce the 25%-for-3-hours field result under the same class of workload/QoS constraints, then test 30% for 3 hours without more energy per completed work or rebound peak;
- match or exceed the Alibaba study's 22% modelled flexible-load envelope while preserving deadlines, tail latency, cooling margin and total completed work;
- improve correlated multi-site aggregate P/frequency predictions against the independence assumption without using held-out future trace information.

These targets must be compared at identical task mix, hardware, baseline, response time, QoS/SLA, cooling state, geography, grid signal and rebound horizon. A percentage load reduction is not a win if the system completes less work, consumes more total energy, violates deadlines, overuses cooling water or creates a later coincident peak.


## 25. AI data-centre flexibility as a coupled compute-energy plant

The dedicated implementation programme is [AI-DATA-CENTER-FLEX-001 #3435](https://github.com/Luminous-Dynamics/symtropy/issues/3435). It composes the 37-case benchmark matrix and focuses on the coupled dispatch of:
- workload scheduling / deferral / migration and accelerator power caps;
- cooling, thermal inertia and water;
- UPS, battery, AC/DC distribution and protection;
- onsite generation and grid-interactive converters;
- grid signals, tariff, local electricity emissions and rebound.

The benchmark now has direct public research references for measured GPU load, correlation, power models and demand response. We should target the published 11.4% H100 power-model MAPE, 22% trace-model load deferral and 25%-for-3-hours field flexibility envelope as comparable baselines. Stretch targets (e.g. <8% model MAPE or ≥30% for 3 hours) are internal research goals, not standards or demonstrated capabilities.

A core anti-gaming rule is now explicit: **power reduction only counts when useful compute completed, QoS, thermal/cooling safety, total energy, grid response and rebound all remain within the frozen contract.**


## 26. Machine-auditable workload trace and meter hierarchy

The trace contract and synthetic hostile fixtures are:
- [AI_DATA_CENTER_WORKLOAD_TRACE_SCHEMA_2026-11-03.json](data/AI_DATA_CENTER_WORKLOAD_TRACE_SCHEMA_2026-11-03.json)
- [AI_DATA_CENTER_WORKLOAD_TRACE_FIXTURES_2026-11-03.json](data/AI_DATA_CENTER_WORKLOAD_TRACE_FIXTURES_2026-11-03.json)

They explicitly bind each power observation to a timebase, site, workload (where applicable), measurement point, calibration ID and evidence ID. Meter points form a hierarchy from accelerator/node/rack/UPS to facility PCC. A selected aggregate may not include both an aggregate meter and any of its descendants.

Cross-site correlation requires synchronized clocks with measured error below a declared tolerance and a minimum common timestamp count. A PTP/NTP label alone is insufficient; the actual alignment error and shared samples must be checked. The fixtures include a valid aligned multi-site case, a nested meter double-count rejection and an unsynchronized clock rejection.

A Rust implementation is being developed in [AI data-centre trace validation PR #3437](https://github.com/Luminous-Dynamics/symtropy/pull/3437). Its 11 unit tests cover meter lineage, cross-site alignment, common timestamps, workload/site joins, sample ordering and sign conventions. This is structural/ingestion validation only; it does not authenticate meter calibration or establish real grid correlations.


## 27. Matched-service flexibility evaluation

The evaluator work is in dependent draft [PR #3438](https://github.com/Luminous-Dynamics/symtropy/pull/3438), based on the trace validator #3437. It evaluates a grid-interactive compute event over a frozen baseline, response and rebound horizon.

Two electrical boundaries are deliberately distinct:
- **Grid response:** active import at the facility point of connection (PCC), used to assess the requested grid-support response.
- **Facility energy per work:** total electrical input at one declared non-overlapping facility service boundary, including grid, onsite generation and storage discharge, used to assess energy per completed unit of compute.

This prevents a battery from making PCC import fall while hiding battery discharge and claiming an efficiency improvement. The report also enforces identical workload/work-unit identity, completed work, response p99 latency, deadline misses, peak thermal limit, sample timing and rebound peak. Every sample retains an evidence ID and the result carries baseline/response configuration digests.

The six synthetic oracle fixtures are:
- [AI_FLEXIBILITY_EVENT_FIXTURES_2026-11-03.json](data/AI_FLEXIBILITY_EVENT_FIXTURES_2026-11-03.json)
- [AI_FLEXIBILITY_EVENT_SCHEMA_2026-11-03.json](data/AI_FLEXIBILITY_EVENT_SCHEMA_2026-11-03.json)

They include the published 25%-for-3-hours style of field reference, a battery-shift event with no site energy saving, under-delivered work, rebound spike, energy-per-work regression and SLA/thermal violation. These are synthetic accounting oracles, not field results.


## 28. Independent workload-schedule feasibility verification

Dependent implementation [PR #3443](https://github.com/Luminous-Dynamics/symtropy/pull/3443) adds a deterministic verifier for proposed compute schedules. It is deliberately **not** a scheduler or optimizer: EDF, MILP, MPC or a learned policy may produce a proposal, but the independent verifier checks whether the proposal actually satisfies its declared contract.

It checks:
- release/deadline windows and complete delivered work;
- work-unit identity and maximum per-slot job work;
- non-preemptible contiguous execution;
- checkpointable pause/resume overhead, converted to slot power and energy;
- deferrable workload allocation;
- compute capacity per slot;
- total facility input power;
- residual grid import after declared non-grid supply;
- cooling power capacity;
- integrated facility/grid/cooling energy.

The verifier's source contract distinguishes base non-compute load, base cooling, job IT power, job cooling, checkpoint energy and committed non-grid supply. It does not allow an optimizer to hide checkpoint energy or cooling behind a lower PCC import value.

The synthetic oracle pack is [AI_WORKLOAD_SCHEDULE_FIXTURES_2026-11-03.json](data/AI_WORKLOAD_SCHEDULE_FIXTURES_2026-11-03.json) with [schema](data/AI_WORKLOAD_SCHEDULE_SCHEMA_2026-11-03.json). Five cases cover a feasible two-job schedule, grid-cap failure, cooling-cap failure, non-preemptible fragmentation and checkpointable pause/resume. Manual arithmetic reconstruction gives zero discrepancies. These fixtures validate feasibility accounting, not schedule optimality or physical data-centre performance.


## 29. Stateful storage, UPS reserve and thermal assurance — 4 November 2026

New workstream: [AI-RESOURCE-STATE-ASSURANCE-001 #3575](https://github.com/Luminous-Dynamics/symtropy/issues/3575).

Dependent sibling draft implementation: [PR #3578](https://github.com/Luminous-Dynamics/symtropy/pull/3578).

The new Rust verifier advances the compute-energy work from static slot limits to explicit state trajectories:
- battery stored energy over consecutive intervals, with distinct charge/discharge efficiencies and power limits;
- critical-load ride-through reserve and an operating floor;
- thermal capacitance and ambient conductance in a one-zone RC model;
- independent IT heat, cooling heat removal and cooling electrical input;
- cooling COP and thermal/electrical capacity gates;
- facility input and residual grid import at the same declared boundary.

The machine-readable oracle pack is:
- [AI_STATEFUL_RESOURCE_FIXTURES_2026-11-04.json](data/AI_STATEFUL_RESOURCE_FIXTURES_2026-11-04.json)
- [AI_STATEFUL_RESOURCE_SCHEMA_2026-11-04.json](data/AI_STATEFUL_RESOURCE_SCHEMA_2026-11-04.json)

Five synthetic fixtures exercise a passing trajectory, reserve depletion, long thermal overrun, free-cooling accounting abuse and grid-cap failure. The Rust implementation has 14 authored tests. No exact-head compile/test or physical result is claimed; PR #3578 remains draft.

**Claim ceiling:** this v0.1 component has one thermal zone and constant forcing per interval. It is not CFD, a multi-loop liquid-cooling digital twin, battery degradation model, UPS runtime test or live controller. A passing calculation is conditional on its declared coefficients and evidence identifiers.

### Competitive thermal baseline

The June 2026 *Applied Energy* paper, “Exploring the Demand Response Potential of Liquid-Cooled Data Centers” (DOI 10.1016/j.apenergy.2026.127679), explicitly develops electro-thermal coupled liquid-cooled data-centre modelling and coordinated server/cooling demand response. This raises the bar beyond fixed cooling-parasitic estimates. Symtropy's first-order RC implementation is only an initial independently auditable feasibility kernel; the next step is calibrated multi-node coolant/thermal dynamics using measured held-out traces.

Proposed internal targets after obtaining comparable data are <=0.5°C thermal-state MAE and <=10% cooling electrical-power MAPE on held-out operating regimes, with prediction-interval coverage and zero false-negative hard thermal violations in the frozen hostile suite. These are research targets, not standards thresholds.

## 30. Risk-aware Bit-Watt scenario evaluation — 4 November 2026

New workstream: [AI-BIT-WATT-RISK-ASSURANCE-001 #3579](https://github.com/Luminous-Dynamics/symtropy/issues/3579).

Dependent sibling draft implementation: [PR #3581](https://github.com/Luminous-Dynamics/symtropy/pull/3581).

The new Rust evaluator accepts candidate results over a frozen joint scenario ensemble and independently reconstructs:
- expected operating cost, facility energy and completed work;
- worst-scenario completed work;
- power/energy shortfall probability;
- deadline, thermal and protected-reserve violation probabilities;
- weighted empirical VaR/CVaR for peak power deficit and energy shortfall;
- independent risk/service gates and evidence-bearing per-scenario receipts.

The six-fixture synthetic oracle pack is:
- [AI_BIT_WATT_SCENARIO_RISK_FIXTURES_2026-11-04.json](data/AI_BIT_WATT_SCENARIO_RISK_FIXTURES_2026-11-04.json)
- [AI_BIT_WATT_SCENARIO_RISK_SCHEMA_2026-11-04.json](data/AI_BIT_WATT_SCENARIO_RISK_SCHEMA_2026-11-04.json)

The weighted CVaR calculation consumes fractional probability mass at the quantile boundary. It does not round the tail to an integer scenario count. This is empirical CVaR for a finite scenario set, not a moment-based worst-case DRO guarantee.

### Competitive optimizer baseline

The September 2026 *International Journal of Electrical Power & Energy Systems* study, “Tail-risk-aware Bit-Watt co-optimization for data center integrated energy systems under joint forecast uncertainty” (DOI 10.1016/j.ijepes.2026.112227), uses joint PV/workload forecast uncertainty and worst-case CVaR. In its modeled case, hourly shortfall falls from 50.05% to 1.84% at a 2.76% cost premium over risk-neutral scheduling; a robust-box comparator reaches 0.26% but is 12.69% costlier than the proposed policy. These numbers are case-specific, not universal SOTA guarantees.

Benchmark at equal inputs, service contracts, scenario set and compute budgets:
- deterministic risk-neutral MILP;
- sample-average CVaR;
- moment-based distributionally robust CVaR;
- robust-box;
- MPC;
- scenario MPC.

Target a matched-system replication of <=1.84% hourly shortfall at no more than 2.76% cost premium, then a <=1% / <=2% stretch, without service, thermal or reserve violations. Do not claim superiority until out-of-sample reliability-cost frontiers dominate the relevant baselines under an independently replayable test protocol.

