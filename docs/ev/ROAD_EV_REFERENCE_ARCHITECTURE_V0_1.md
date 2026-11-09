# Road-EV Reference Architecture v0.1

**Status:** Research architecture; not a vehicle design release, homologation dossier, safety case, or build authorization.  
**Date:** 2026-11-09  
**Parent benchmark:** [ROAD-EV-REFERENCE-001 #4346](https://github.com/Luminous-Dynamics/symtropy/issues/4346)  
**Related work:** [Symthaea EV-HEALTH-ADVISORY-001 #11239](https://github.com/Luminous-Dynamics/symthaea/issues/11239) · [Sol-Atlas EV-ROUTE-ENERGY-001 #664](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/664) · [Mycelix Mobility Commons #3708](https://github.com/Luminous-Dynamics/mycelix/issues/3708) · [Mycelix Mobility Configuration #3709](https://github.com/Luminous-Dynamics/mycelix/issues/3709)

## 1. Purpose and strategy

Luminous EV is a research and product-engineering programme for a **serviceable, efficient, interoperable and evidence-qualified mobility system**. The first proving unit is EV-REF-01: a compact passenger/utility reference configuration, not a declared production vehicle.

The programme optimizes the whole service system:

`vehicle + battery + route + charger + electricity supply + manufacturing + maintenance + end-of-life`

The primary product claim must be supported by matched operating conditions and independently checkable evidence. Range, peak charging power, acceleration, autonomy labels and simulated efficiency are not sufficient in isolation.

### Initial strategic priorities

1. Low real-world energy use under representative duty cycles.
2. Whole-life affordability and repairability.
3. Robust operation in hot climates, elevation changes, rough roads and constrained charging environments.
4. Open, interoperable charging and maintainable electronics.
5. Transparent configuration, service and software lineage.
6. Human-centred driver assistance with a separately qualified path toward bounded autonomy.
7. Manufacturing processes that can scale and localize without assuming every component should be made locally from day one.

Do not start by designing a family of seven vehicles. Establish one reference configuration and a reusable platform boundary; derive other body/mission variants only after comparative evidence justifies them.

## 2. Paradigm shifts to test

### 2.1 From range-first to mission-service design

Define a vehicle by mission classes and service envelopes: occupancy, payload, route type, ambient conditions, expected daily distance, charging access, reserve policy, duty cycle and downtime tolerance. Optimize energy and cost per passenger-kilometre or delivered service, not a single advertised range.

### 2.2 Co-design the vehicle and its energy environment

A battery cannot be optimized independently of motor/inverter maps, thermal management, charger capability, route grade, climate, tariff and grid constraints. Evaluate the complete vehicle-to-grid service path with named measurement boundaries and synchronized energy accounting.

Conductive charging is the reference baseline. Bidirectional charging is an optional configuration whose actual vehicle/EVSE interoperability, warranty terms, degradation cost, utility rules and site limits must be demonstrated. Dynamic wireless charging remains a separate research lane, not a prerequisite for a useful EV.

### 2.3 Treat battery integration as function reassignment

Cell-to-pack and structural integration may improve packaging, mass and manufacturing economics, but removing a module boundary also removes functions that must be accounted for: inspection, isolation, thermal management, crash load paths, electrical protection, repair, diagnostics and end-of-life separation.

Do not select an integrated or modular pack architecture by ideology. Compare multiple architectures against the same safety, mass, cost, thermal, service and recovery requirements. Every removed interface must have an explicit replacement function and validation obligation.

### 2.4 Make repairability an objective at concept stage

Model time-to-diagnose, time-to-repair, required tools, access, part availability, replaceable subassemblies, calibration, post-repair verification, disassembly effort and material recovery. A lower-cost pack that requires replacement of a much larger assembly may be worse over the vehicle lifetime.

### 2.5 Replace opaque “smartness” with evidence-qualified assistance

Symthaea can estimate, predict, explain uncertainty, identify anomalies and recommend maintenance or route alternatives. It cannot be treated as proof of physical truth, and it must not replace deterministic local protection or independently engineered control systems.

### 2.6 Optimize the fleet and charging network as a service system

Vehicle routing, charger queues, charger/site capacity, grid constraints, energy tariffs, maintenance, route closures and reserve requirements should be planned together. The network remains federated and interoperable; owners should not be locked into one operator or one vehicle brand.

### 2.7 Treat the digital thread as a product capability

The configuration of a physical vehicle includes design revision, parts, materials, process substitutions, tolerances, software/firmware, inspection state, repairs and operational restrictions. “Same model name” is not enough to establish engineering equivalence.

## 3. System ownership

| System | EV programme responsibility | Explicit boundary |
|---|---|---|
| **Symtropy** | Declared physical/energy model execution, road-load and vehicle-system dynamics, thermal/energy trajectories where qualified, scenario and fault replay, benchmark composition | A general rigid-body engine is not automatically a validated tyre, motor, electrochemical or thermal solver. Use typed adapters and specialist models. |
| **Symthaea** | Candidate architecture exploration, experiment selection, energy/route/maintenance reasoning, battery and traction health advisory, driver interaction and autonomy research | No sole authority over BMS protection, contactors, inverter torque, steering, braking or emergency actions. |
| **Sol-Atlas** | Spatial geometry, road and infrastructure graphs, terrain/elevation, route feasibility, charger geography/capacity evidence, read-only operational projection | The renderer is not a solver; map/advisory display is not physical truth or vehicle control. |
| **Mycelix** | Configuration/lifecycle lineage, identity and provenance references, supplier/repair records, permissions, attestations and transaction coordination | A cryptographic receipt binds evidence to an issuer/bytes; it does not prove physical truth or grant certification/operating authority. |
| **Luminous engineering platform** | Requirements, variant/configuration, BOM, change impact, engineering decision records, controlled drawings, supplier/process records, verification plan and claims register | It does not replace competent engineering review, regulators, notified bodies or homologation authorities. |

### Cross-system flow

`requirements → candidate designs → typed feasibility checks → Symtropy twin → independent audit → Pareto review → human engineering decision → Mycelix lineage receipt`

Operational evidence can later return through a governed update path:

`observation → validation → diagnosis hypothesis → maintenance/engineering review → approved change → impact analysis → revalidation`

A proposal cannot validate itself. A model output cannot be promoted to measured evidence. An approval record cannot substitute for a regulator's decision.

## 4. EV-REF-01 system breakdown

The following are work packages, not a frozen production architecture.

### 4.1 Vehicle structure and chassis

- Define occupant/package envelopes, seating, ingress/egress, cargo and accessibility requirements.
- Compare steel, aluminium and mixed-material structures against mass, crash load paths, joining, repair and local manufacturing capability.
- Define front/rear crash structures, side-impact paths, underbody protection, corrosion protection and battery isolation zones.
- Model suspension kinematics, steering geometry, braking, tyre limits, ride/handling and rough-road durability.
- Include service access, lifting points, repair procedures and post-repair dimensional inspection.
- Do not assume a skateboard or structural battery is automatically optimal.

### 4.2 Battery and high-voltage system

Compare pack architectures and chemistries under common requirements:
- LFP/LMFP and NMC as near-term candidate chemistry lanes.
- Sodium-ion for applications whose range, mass and power envelope fits the chemistry.
- Solid-state as a future scenario until pack-level manufacturability and field evidence justify selection.

Evaluate usable/gross energy, voltage, peak/continuous power, resistance, thermal gradients, cooling demand, charge curve, degradation, crash protection, isolation, serviceability, supply-chain concentration, second-life eligibility and material recovery.

The BMS and hardware protections remain independent safety functions. Research models are advisory unless separately engineered and qualified.

### 4.3 Propulsion and energy conversion

Compare motor types, winding/material choices, gear ratios, inverter topologies, switching devices, cooling and drive-unit configurations using source-backed efficiency maps. Record performance across speed, torque, temperature and voltage; avoid selecting by peak efficiency alone.

### 4.4 Thermal and cabin systems

Treat battery, motor/inverter and cabin thermal loads as a coupled but separately accountable subsystem. Evaluate heat pumps, refrigerant choice, thermal storage and waste-heat recovery with climate-specific duty cycles. Report winter and hot-weather energy penalties, preconditioning cost and occupant comfort.

### 4.5 Software, electrical architecture and interface

- Versioned power, data, diagnostics and safety interfaces.
- Clear separation of safety-critical and non-safety-critical networks.
- Secure boot, update authenticity, rollback/recovery and software inventory.
- Physical controls for critical driver tasks; accessible voice/touch/visual interfaces as supplements.
- Offline-safe operation and explicit degraded modes.
- Owner-controlled privacy and service-data disclosure.

### 4.6 Charging and energy ecosystem

Conductive AC/DC charging first; evaluate connector/protocol compatibility, complete power curves, station uptime, queueing, site/feeder constraints, thermal conditioning and payment/access interoperability.

V2L/V2H/V2B/V2G should be separate capability profiles with measured energy boundaries and explicit commercial/utility permissions. Solar and stationary storage can improve site resilience, but do not assume islanding or grid export is automatically legal or economically justified.

### 4.7 Manufacturing, inspection and supply chain

- Design for process capability, takt time, flexible variants and repeatable quality.
- Record material heats/lots, supplier part revisions, process recipe, machine/calibration, operator or automation identity, inspection and rework.
- Use open/neutral product-definition and quality formats where practical (e.g. STEP AP242 and QIF adapters).
- Measure first-pass yield, scrap, energy, water, rework and station bottlenecks.
- Include component substitution impact and revalidation.
- Localize based on quality, volume, logistics, skills, cost and resilience evidence—not a blanket local-content assumption.

### 4.8 Maintenance and circularity

Create diagnostic procedures, component replacement instructions, torque/calibration requirements, isolation steps, software configuration requirements, inspection criteria, repair verification and parts supersession rules.

Track refurbishment, remanufacture, quarantine, second-life qualification and end-of-life disassembly. Never infer a battery's second-life suitability from SOH alone.

### 4.9 Driving automation

Progress through driver assistance and restricted ODD pilots. Use camera/radar/lidar choices based on cost, operating conditions, safety performance and evidence—not sensor ideology. Local sensor observations, map priors, simulator oracle, V2X advisories and derived predictions remain separate evidence classes.

Begin autonomy research in simulation and closed/controlled environments. A driverless deployment claim requires a defined ODD, independently justified safety case, scenario coverage, physical testing, cybersecurity and regulatory approval. “L4” without an ODD and evidence dossier is not a meaningful programme acceptance criterion.

## 5. Shared measurement and evidence contract

Every result must bind:
- exact vehicle/configuration and component revisions;
- test cycle/route bytes and route/map version;
- payload, tyre, SOC, temperature, ambient/wind and auxiliary-load conditions;
- source, sensor, calibration, uncertainty and measurement boundary;
- model/code/solver version, build identity, parameters and random seed;
- raw observations and transformation lineage;
- independent audit results, deviations, exclusions and claim ceiling.

Keep these evidence classes separate:

`Assumption | Vendor claim | Source-reported measurement | Our measurement | Calibrated simulation | Uncalibrated simulation | Expert judgement | External approval`

Missing evidence is `UNKNOWN`, not zero. Conflicting measurements remain visible. An incomplete search cannot be called exact. A passing schema/CI check proves only the checked property.

## 6. Benchmark programme

### Vehicle energy and performance

Reference cycles: UN R154 / WLTP appropriate class and applicable EPA cycles (FTP-75/UDDS, HWFET, US06) when data and test definitions permit. Add measured local-route tests.

Report:
- wall energy, battery-terminal energy, DC bus energy and wheel energy separately;
- kWh/100 km and Wh/passenger-km / Wh-tonne-km with payload and occupancy;
- usable capacity and route-specific range with reserve and interval;
- acceleration, gradeability, sustained-speed performance, braking and regen;
- HVAC/thermal energy by ambient-temperature stratum;
- uncertainty, calibration and held-out model discrepancy.

### Model-fidelity targets (research gates, not achieved)

- After parameter calibration on training/reference conditions, median energy discrepancy ≤2.5% and no more than 5% on each predeclared held-out cycle/stratum, provided independent source measurements are sufficiently precise.
- No held-out tuning; publish per-vehicle/per-cycle results and uncertainty.
- At least one independent cross-check beyond the in-house evaluator (FASTSim is an initial external comparator, supplemented by analytical known-answer submodels).
- Zero energy-creation or unit/sign errors in known-truth fixtures.

### Performance-improvement targets (aspirational)

Seek ≥5% lower measured wall-to-wheel energy than the best comparable reference in the same class, payload, route/cycle, temperature and performance envelope; 10% is a stretch goal. Do not compare a smaller/slower vehicle to a larger/faster vehicle without explicit service normalization.

### Battery

- SOC error and calibration by chemistry, temperature, ageing and drive profile.
- SOH error, uncertainty coverage, degradation slope and held-out pack/cell generalization.
- Fault recall per critical fault family and false alerts per vehicle-hour.
- Charging curve and thermal rise over SOC/temperature.
- Disassembly/repair time, module/pack intervention scope and material recovery.
- No pooling across chemistry/temperature strata to hide failures.

### Charging / network

- Wall-to-battery efficiency and battery-to-grid efficiency at declared boundaries.
- Energy delivered in 10/15/20-minute windows, not advertised peak power alone.
- Session success, uptime, connector compatibility, queue and wait prediction error.
- Site peak demand, feeder limit compliance, renewable utilization and storage losses.
- Route reserve violation and false-feasible rate under charger outage/traffic/weather uncertainty.

### Manufacturing / serviceability / lifecycle

- First-pass yield, defect escape, rework hours, station cycle time, scrap and embodied process energy.
- Diagnosis and repair time, parts availability, repair cost and post-repair verification.
- Warranty cost and downtime per vehicle-km where measured.
- Full lifecycle cost per vehicle-km/passenger-km; currency, year, tax, tariff, financing, battery replacement and residual value explicit.
- Life-cycle GHG with grid mix, battery production, vehicle materials, service and end-of-life assumptions.
- Recovered material mass and quality; no “recycled” claim without chain-of-custody and process evidence.

### Autonomous driving

Score by ODD, not one combined autonomy score:
- scenario coverage and severity-weighted exposure;
- collisions, near misses and rule violations per exposure;
- collision avoidance / safe fallback under occlusion, weather, roadworks and vulnerable road users;
- false-positive/negative object and lane detections by class/condition;
- localisation integrity and uncertainty;
- interventions/disengagements with consistent definitions;
- minimum-risk manoeuvre success and time-to-safe-state;
- cyberattack/replay/stale-data resilience;
- simulation-to-real discrepancy and independent track/road validation.

Simulation success is not road safety evidence.

## 7. South African proving ground

First route family: Johannesburg–Durban / N3 in both directions, then urban Johannesburg routes and selected rural/low-density charging gaps.

Data requirements:
- versioned road geometry, elevation/grade and direction;
- measured speed trace when lawfully available;
- ambient temperature, wind, rain and traffic context;
- vehicle configuration, payload, tyres, SOC and thermal state;
- exact charger connector, access, curve, current status and feeder/site limit;
- synchronized wall/charger/vehicle energy and uncertainty.

The CHARGE N3 Geely E2 measurements are a source-labelled external example, not a fleet-average South African result. Include other vehicles and repeat observations before inferring regional duty-cycle distributions.

Design for heat, dust, potholes, rough surfaces, elevation, grid constraints, limited rural charger density and local service/parts availability.

## 8. Safety, security and legal engineering

Build a compliance applicability matrix, not a claim of compliance. At minimum investigate:
- ISO 26262 functional safety;
- ISO 21448 SOTIF;
- ISO 21434 cybersecurity engineering and UN R155;
- UN R156 / ISO 24089 software update management;
- UNECE vehicle categories, crash, braking, EMC, lighting and applicable homologation rules;
- ISO 6469 electrical safety;
- ISO 15118 / IEC 61851 charging;
- ISO 34502 scenario-based ADS evaluation and applicable local rules;
- battery transport, abuse, fire and recycling obligations;
- South African NRCS, SABS, vehicle homologation, electrical installation, grid-connection, POPIA and consumer/repair requirements as legally applicable.

An issue, simulation, test report or digital passport cannot certify a vehicle. Professional engineering and regulatory review are necessary before physical release or road use.

## 9. Immediate execution sequence

1. **EV-REF-01 benchmark contract** — [Symtropy #4346](https://github.com/Luminous-Dynamics/symtropy/issues/4346).
2. **Battery and traction health advisory** — [Symthaea #11239](https://github.com/Luminous-Dynamics/symthaea/issues/11239).
3. **Route-energy planner** — [Sol-Atlas #664](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/664).
4. **Engineering configuration lineage** — [Mycelix #3709](https://github.com/Luminous-Dynamics/mycelix/issues/3709) and [Mobility Commons #3708](https://github.com/Luminous-Dynamics/mycelix/issues/3708).
5. Audit existing battery model and digital twin before new estimation algorithms.
6. Create a Rust-first longitudinal evaluator with exact known-answer fixtures and independent audit.
7. Freeze the N3 route evidence manifest and public vehicle/cycle comparison inputs.
8. Only then expand into pack architecture optimization, factory simulation and physical demonstrators.

## 10. Qualification gates

No single weighted score can compensate for a hard failure. Each design candidate must pass:

- physical/energy feasibility;
- uncertainty and sensitivity disclosure;
- electrical/thermal/crash safety obligations;
- repairability and end-of-life obligations;
- cybersecurity and update obligations;
- configuration and supplier traceability;
- independent audit;
- professional and regulatory review before physical deployment.

A candidate can be promising but unqualified. Preserve that distinction in every report.

## 11. Initial claim ceiling

At this stage, Luminous has research software, models, benchmark programmes and an emerging architecture. This document does **not** establish a built prototype, a measured vehicle efficiency improvement, battery qualification, production capability, public charging uptime, autonomous-driving safety, homologation or superiority over an incumbent manufacturer.

The programme succeeds by turning each of those future claims into a falsifiable test with a predeclared comparator and an independently reviewable evidence package.
