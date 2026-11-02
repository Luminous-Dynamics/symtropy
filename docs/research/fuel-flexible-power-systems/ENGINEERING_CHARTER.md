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
| Fuel Flexibility Efficiency Retention (FFER) | Shared-architecture efficiency / optimized fuel-specific reference, at matched points | Target ≥0.97 on a weighted steady-state map; report every point and uncertainty |
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

- Use ISO 8178 methods/cycles for applicable non-road and stationary reciprocating-engine emissions work.
- Use the applicable heavy-duty on-road certification procedures and transient/steady-state cycles (e.g. WHTC/WHSC where required by the target jurisdiction).
- For marine engines, use the NOx Technical Code 2008 and applicable MARPOL Annex VI requirements, including the correct Tier / ECA scope.
- For ships, report EEXI/CII-relevant operational metrics and IMO well-to-wake lifecycle intensity; engine g/kWh alone is insufficient.
- Add real duty-cycle trials: South African high ambient temperature, altitude, dust, fuel-quality variance, remote maintenance and route/load profiles; port/shore-power availability for vessels.

The team must verify the currently applicable local certification rules before selecting hardware or claiming a market-ready configuration.

## 8. SOTA baseline and honest comparison

| Industry reference | Public evidence | Implication |
|---|---|---|
| Cummins HELM / X10 / X15 | Cummins identifies fuel-agnostic engine platforms; MY2027 production ramp is scheduled through 2027 | Shared architecture across fuel-specific variants is already commercial strategy |
| DOE SuperTruck 2 | DOE reports >52% engine brake thermal efficiency demonstrated by all teams, with three pathways to 55% | A 52–55% peak-BTE frontier is a serious heavy-duty benchmark; specify whether waste-heat recovery is included |
| Wärtsilä 32 Methanol | Commercial marine methanol engine, 3.4–5.2 MW; 46F Methanol can use conventional fuels and methanol | Methanol dual-fuel marine technology is already commercial |
| Everllence / MAN B&W ammonia | 2026 factory-acceptance milestone for dual-fuel 6G60ME-LGIA | Ammonia two-stroke is advancing beyond lab-only status |
| Battery-electric / fuel-cell HDVs | Recent lifecycle analysis reports significant GHG reductions for BEVs and FCEVs, depending on vehicle class, duty cycle and energy source | Combustion is not the default climate winner; compare full transport work and regional energy pathways |

**No "better than SOTA" claim** until a like-for-like dataset is available. The proposed competitive wedge is lower flexibility penalty, fuel-quality robustness, verified lifecycle outcomes, maintainability and whole-system cost—not a claim that Symtropy already simulates combustion better than specialist tools.

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

- U.S. DOE, SuperTruck results and SuperTruck 3 goals (10 June 2024): https://www.energy.gov/cmei/vehicles/articles/fotw-1346-june-10-2024-us-doe-supertruck-3-initiative-seeks-reduce
- Cummins, MY2027 X10/X15 HELM production transition (2026): https://investor.cummins.com/news/detail/700/cummins-announces-phased-model-year-2027-product-launch-plan
- Wärtsilä, marine methanol engines: https://www.wartsila.com/marine/products/engines-and-generating-sets/methanol-engines
- Everllence, ammonia engine factory acceptance test (20 April 2026): https://www.everllence.com/company/press-releases/details/2026/04/20/ammonia-engine-passes--huge-milestone--en-route-to-commercial-debut
- U.S. DOE Alternative Fuels Data Center, fuel properties comparison: https://afdc.energy.gov/fuels/properties
- ISO 8178-1, exhaust emission measurement for reciprocating internal combustion engines: https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3A8178%3A-1%3Aed-4%3Av1%3Aen
- IMO, NOx Regulation 13 and marine engine certification: https://www.imo.org/en/ourwork/environment/pages/nitrogen-oxides-%28nox%29-%E2%80%93-regulation-13.aspx
- IMO, lifecycle GHG intensity guidelines for marine fuels: https://www.imo.org/en/ourwork/environment/pages/lifecycle-ghg---carbon-intensity-guidelines.aspx
- IMO, EEXI/CII framework: https://www.imo.org/en/mediacentre/hottopics/pages/eexi-cii-faq.aspx
- IMO, Net-Zero Framework status: https://www.imo.org/en/mediacentre/hottopics/pages/faqs-the-imo-net-zero-framework.aspx
- PNNL / Environmental Science & Technology, cradle-to-grave HDV pathway comparison (2026): https://www.pnnl.gov/publications/cradle-grave-lifecycle-analysis-us-medium-and-heavy-duty-vehicle-fuel-pathways

---

**Decision:** Proceed with the evidence baseline, typed data contracts and transparent 0D energy-balance model. Do not claim a fuel-agnostic engine exists until a defined configuration has passed an instrumented, independently reviewed test programme.
