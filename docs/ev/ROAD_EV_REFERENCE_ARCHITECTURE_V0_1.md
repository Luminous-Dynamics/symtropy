# Road-EV Reference Architecture v0.1

**Status:** Research architecture; not a vehicle design release, homologation dossier, safety case, or build authorization.  
**Date:** 2026-11-09  
**Reference configuration / integration:** [ROAD-EV-REFERENCE-001 #4346](https://github.com/Luminous-Dynamics/symtropy/issues/4346)  
**Benchmark owner:** [EV-BENCH-001 #2164](https://github.com/Luminous-Dynamics/symtropy/issues/2164)  
**Battery circularity owner:** [EV-BATTERY-CIRCULARITY-001 #2155](https://github.com/Luminous-Dynamics/symtropy/issues/2155)  
**Related work:** [Symtropy EV platform portfolio #2112](https://github.com/Luminous-Dynamics/symtropy/issues/2112) · [Symtropy user-fit/HMI #1801](https://github.com/Luminous-Dynamics/symtropy/issues/1801) · [Symtropy family-vehicle substrate PR #1762](https://github.com/Luminous-Dynamics/symtropy/pull/1762) · [Symtropy whole-life transport #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601) · [Symtropy repairability compiler #1773](https://github.com/Luminous-Dynamics/symtropy/issues/1773) · [Symtropy localization #1780](https://github.com/Luminous-Dynamics/symtropy/issues/1780) · [Symtropy continuity calibration #1791](https://github.com/Luminous-Dynamics/symtropy/issues/1791) · [GLOBAL-MOBILITY-001 #1832](https://github.com/Luminous-Dynamics/symtropy/issues/1832) · [Symthaea road dynamics #6247](https://github.com/Luminous-Dynamics/symthaea/issues/6247) · [Symthaea road autonomy benchmark #6256](https://github.com/Luminous-Dynamics/symthaea/issues/6256) · [Symthaea physical pilot ladder #6254](https://github.com/Luminous-Dynamics/symthaea/issues/6254) · [Symthaea family-vehicle engineering PR #7751](https://github.com/Luminous-Dynamics/symthaea/pull/7751) · [Symthaea EV-HEALTH-ADVISORY-001 #11239](https://github.com/Luminous-Dynamics/symthaea/issues/11239) · [observation-contract implementation draft PR #11249](https://github.com/Luminous-Dynamics/symthaea/pull/11249) · [SOH transfer benchmark #11252](https://github.com/Luminous-Dynamics/symthaea/issues/11252) · [Sol-Atlas EV-ROUTE-ENERGY-001 #664](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/664) · [Sol-Atlas EV-CHARGING-SITE-001 #665](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/665) · [Mycelix vehicle dossier PR #5131](https://github.com/Luminous-Dynamics/mycelix/pull/5131) · [Mycelix Mobility Commons #3708](https://github.com/Luminous-Dynamics/mycelix/issues/3708) · [Mycelix Mobility Configuration #3709](https://github.com/Luminous-Dynamics/mycelix/issues/3709)

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

Do not start by designing a family of seven passenger vehicles. Symtropy #2112 owns the wider duty-cycle-first EV and electric mobile machinery portfolio; use EV-REF-01 as the first integrated road passenger/utility reference configuration, then derive other variants only where #2112's mission-specific requirements and comparative evidence justify them.

## 2. Paradigm shifts to test

### 2.1 From range-first to mission-service design

Define a vehicle by mission classes and service envelopes: occupancy, payload, route type, ambient conditions, expected daily distance, charging access, reserve policy, duty cycle and downtime tolerance. Optimize energy and cost per passenger-kilometre or delivered service, not a single advertised range.

### 2.2 Co-design the vehicle and its energy environment

A battery cannot be optimized independently of motor/inverter maps, thermal management, charger capability, route grade, climate, tariff and grid constraints. Evaluate the complete vehicle-to-grid service path with named measurement boundaries and synchronized energy accounting.

Conductive charging is the reference baseline. Bidirectional charging is an optional configuration whose actual vehicle/EVSE interoperability, warranty terms, degradation cost, utility rules and site limits must be demonstrated. Dynamic wireless charging remains a separate research lane, not a prerequisite for a useful EV.

### 2.3 Treat battery integration as function reassignment

Cell-to-pack and structural integration may improve packaging, mass and manufacturing economics, but removing a module boundary also removes functions that must be accounted for: inspection, isolation, thermal management, crash load paths, electrical protection, repair, diagnostics and end-of-life separation.

Do not select an integrated or modular pack architecture by ideology. The quantitative pack-integration, repair-granularity and recovery comparison belongs to [EV-BATTERY-CIRCULARITY-001 #2155](https://github.com/Luminous-Dynamics/symtropy/issues/2155); the reference configuration records which candidate and evidence state it consumes. Every removed interface must have an explicit replacement function and validation obligation.

### 2.4 Make repairability an objective at concept stage

Model time-to-diagnose, time-to-repair, required tools, access, part availability, replaceable subassemblies, calibration, post-repair verification, disassembly effort and material recovery. A lower-cost pack that requires replacement of a much larger assembly may be worse over the vehicle lifetime. Use #2155 for the quantitative battery comparison and #2164 for cross-track benchmark composition.

### 2.5 Replace opaque “smartness” with evidence-qualified assistance

Symthaea can estimate, predict, explain uncertainty, identify anomalies and recommend maintenance or route alternatives. It cannot be treated as proof of physical truth, and it must not replace deterministic local protection or independently engineered control systems.

### 2.6 Optimize the fleet and charging network as a service system

Vehicle routing, charger queues, charger/site capacity, grid constraints, energy tariffs, maintenance, route closures and reserve requirements should be planned together. The network remains federated and interoperable; owners should not be locked into one operator or one vehicle brand. Use Symtropy #2164 and #3472/#3473 for fleet/charging benchmark semantics; Sol-Atlas #664 consumes vehicle configuration and route evidence for route-energy planning.

### 2.7 Design autonomy around counterfactual decision quality, not just perception accuracy

A capable scene detector is not a safe driving policy, and open-loop trajectory similarity is not closed-loop competence. Keep perception, motion prediction, planning, reactive multi-agent simulation, closed-loop behavior, compute latency, ODD coverage, physical pilot transfer and operational outcomes as separate benchmark families. Test whether a policy chooses a safer/better action than available alternatives under the same evidence and constraints; record why it abstains or enters a minimum-risk state. Symthaea #6256 owns the road autonomy benchmark portfolio, #6255 the external benchmark identity/rights waist, and #6254 the low-consequence physical pilot ladder.

### 2.8 Treat the human interface as an owner-controlled, measurable subsystem

Symtropy [VEHICLE-USER-FIT-001 #1801](https://github.com/Luminous-Dynamics/symtropy/issues/1801) owns the user-fit/HMI and personalization contract. EV-REF-01 consumes it rather than creating a separate profile manager. Evaluate task completion, errors, glance/workload, accessibility by user subgroup, profile switching, guest privacy, offline capability, export/reset/recovery and repairability of user-facing modules. Keep safety-critical controls and ADAS thresholds non-overridable; profile changes affecting operating contexts must be limited to approved parked-state procedures. Personalization is a usability hypothesis to measure, not an assumed improvement.

### 2.9 Treat the digital thread as a product capability

The configuration of a physical vehicle includes design revision, parts, materials, process substitutions, tolerances, software/firmware, inspection state, repairs and operational restrictions. “Same model name” is not enough to establish engineering equivalence. Reuse Mycelix #3708/#3709 rather than creating a separate vehicle passport or identity system.

## 3. System ownership

| System | EV programme responsibility | Explicit boundary |
|---|---|---|
| **Symtropy** | Energy-system design/twin composition, declared battery/charging/thermal trajectories where qualified, scenario/fault replay and EV-REF-01 integration | Does not claim a complete passenger-road-vehicle tyre/contact/braking model. Reuse Symthaea #6247 for road-dynamics ownership; specialist solvers require explicit validity envelopes. |
| **Symthaea** | Road/off-road dynamics profile semantics through #6247; candidate architecture/experiment reasoning; energy/route/maintenance reasoning; battery/traction health advisory; driver interaction and autonomy research | The #6247 profile is an ownership/qualification contract, not automatic proof of a validated tyre or vehicle model. No sole authority over BMS protection, contactors, inverter torque, steering, braking or emergency actions. |
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

The road/off-road dynamics profile is owned by Symthaea [TRANSPORT-ROAD-001 #6247](https://github.com/Luminous-Dynamics/symthaea/issues/6247), under multimodal systems architecture #6244. EV-REF-01 must consume its exact tire/contact, traction/slip, steering/handling, braking/regen/friction-brake, thermal/fade, grade/surface/friction/weather and mass/CG/inertia semantics. The family-vehicle corpus and continuity proxy in Symtropy PR #1762 are not a calibrated road-vehicle physics model; calibration/held-out evidence is separately tracked in #1791.

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

The BMS and hardware protections remain independent safety functions. Research models are advisory unless separately engineered and qualified. The quantitative battery pack architecture/repair/recovery evaluator is [#2155](https://github.com/Luminous-Dynamics/symtropy/issues/2155).

### 4.3 Propulsion and energy conversion

Compare motor types, winding/material choices, gear ratios, inverter topologies, switching devices, cooling and drive-unit configurations using source-backed efficiency maps. Record performance across speed, torque, temperature and voltage; avoid selecting by peak efficiency alone.

### 4.4 Thermal and cabin systems

Treat battery, motor/inverter and cabin thermal loads as a coupled but separately accountable subsystem. Evaluate heat pumps, refrigerant choice, thermal storage and waste-heat recovery with climate-specific duty cycles. Report winter and hot-weather energy penalties, preconditioning cost and occupant comfort.

- Versioned power, data, diagnostics and safety interfaces.

- Versioned power, data, diagnostics and safety interfaces.
- Clear separation of safety-critical and non-safety-critical networks.
- Secure boot, update authenticity, rollback/recovery and software inventory.
- Physical controls for critical driver tasks; accessible voice/touch/visual interfaces as supplements.
- Offline-safe operation and explicit degraded modes.
- Owner-controlled privacy and service-data disclosure.

### 4.6 Charging and energy ecosystem

Conductive AC/DC charging first; evaluate connector/protocol compatibility, complete power curves, station uptime, queueing, site/feeder constraints, thermal conditioning and payment/access interoperability. Sol-Atlas #665 owns site siting/buildability, load-side grid-import evidence, civil/electrical construction and commissioning-stage evidence; #664 consumes qualified site/charger information for route-energy planning. The site planner must not infer connection or commissioning from a map pin.

V2L/V2H/V2B/V2G should be separate capability profiles with measured energy boundaries and explicit commercial/utility permissions. Solar and stationary storage can improve site resilience, but do not assume islanding or grid export is automatically legal or economically justified. Existing Sol-Atlas #662/#519/#293 owners supply spatial/site and resource/connection evidence; #665 adds the EV charging consumer/site profile rather than duplicating grid-flow or renewable-yield models. Existing multi-receiver and source-to-load charging owners are #3472 and #3473; EV-BENCH #2164 owns the wider fleet and charging comparison.

### 4.7 Manufacturing, inspection and supply chain

- Design for process capability, takt time, flexible variants and repeatable quality.
- Record material heats/lots, supplier part revisions, process recipe, machine/calibration, operator or automation identity, inspection and rework.
- Use open/neutral product-definition and quality formats where practical (e.g. STEP AP242 and QIF adapters).
- Measure first-pass yield, scrap, energy, water, rework and station bottlenecks.
- Include component substitution impact and revalidation.
- Localize based on quality, volume, logistics, skills, cost and resilience evidence—not a blanket local-content assumption.
- Reuse Symtropy's manufacturing digital-thread and conformity tournament in #2164 where it genuinely matches the EV process; do not create a vehicle-only MES/QMS.

### 4.8 Maintenance and circularity

Create diagnostic procedures, component replacement instructions, torque/calibration requirements, isolation steps, software configuration requirements, inspection criteria, repair verification and parts supersession rules.

Track refurbishment, remanufacture, quarantine, second-life qualification and end-of-life disassembly. Never infer a battery's second-life suitability from SOH alone. Battery pack repair granularity and end-of-life recovery are owned by #2155; the generic physical-artifact and maintenance lineage is owned by Mycelix #3709.

### 4.9 Driving automation

Progress through driver assistance and restricted ODD pilots. Use camera/radar/lidar choices based on cost, operating conditions, safety performance and evidence—not sensor ideology. Local sensor observations, map priors, simulator oracle, V2X advisories and derived predictions remain separate evidence classes. Symthaea #6256 owns the benchmark suite and #6254 owns the physical pilot ladder; #6255 governs external benchmark identity, rights and claim ceilings.

Begin autonomy research in simulation and closed/controlled environments. A driverless deployment claim requires a defined ODD, independently justified safety case, scenario coverage, physical testing, cybersecurity and regulatory approval. “L4” without an ODD and evidence dossier is not a meaningful programme acceptance criterion. Reuse Symthaea's existing road-autonomy benchmark/scenario work, Symtropy replay and Mycelix V2X provenance; this architecture does not create a parallel autonomy evaluation stack.

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

## 6. Benchmark programme and ownership

The authoritative cross-track EV benchmark is [EV-BENCH-001 #2164](https://github.com/Luminous-Dynamics/symtropy/issues/2164). This document defines EV-REF-01 configuration and measurement needs; it does not fork that harness. The battery integration / repair / recovery comparison is [EV-BATTERY-CIRCULARITY-001 #2155](https://github.com/Luminous-Dynamics/symtropy/issues/2155).

### Vehicle energy and performance

Reference cycles: UN R154 / WLTP appropriate class and applicable EPA cycles (FTP-75/UDDS, HWFET, US06) when data and test definitions permit. Add measured local-route tests.

Report:
- wall energy, battery-terminal energy, DC bus energy and wheel energy separately;
- kWh/100 km and Wh/passenger-km / Wh-tonne-km with payload and occupancy;
- usable capacity and route-specific range with reserve and interval;
- acceleration, gradeability, sustained-speed performance, braking and regen;
- HVAC/thermal energy by ambient-temperature stratum;
- uncertainty, calibration and held-out model discrepancy.

### Model-fidelity and performance targets

EV-BENCH-001 #2164 owns the authoritative matched-comparison protocol. Do not freeze a universal percentage improvement or model-error threshold before the reference measurements, instrument uncertainty, cycle coverage and repeatability are known. First establish a calibrated baseline, measurement resolution and repeat-run variance. Then preregister the primary outcome, minimum meaningful effect, sample size/power, held-out strata and uncertainty procedure before confirmatory comparison.

- No held-out tuning; publish per-vehicle/per-cycle results and uncertainty.
- Use FASTSim as one independent comparator, not as ground truth; add analytical known-answer submodels and another specialist comparison where scope allows.
- Zero energy-creation or unit/sign errors in known-truth fixtures.
- Report every per-scenario regression and preserve failed/ineligible/indeterminate cases in the denominator.
- Any claim of an improvement must compare the same vehicle class, payload, route/cycle, temperature, reserve and performance envelope; a smaller/slower vehicle is not a like-for-like energy victory.

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

## 7. 2026 competitive frontier and interpretation

### Energy efficiency and real-world route comparison

The [CAA's September 2026 22-vehicle common 230 km route test](https://www.caa.ca/news/2026-caa-ev-circuit-evs-go-the-distance-but-charging-efficiency-and-costs-vary/) reported 11.43–28.91 kWh/100 km. Its best 15-minute displayed-range addition was 178 km for the Lucid Gravity, which was also its highest-energy-use vehicle at 28.91 kWh/100 km. This is a concrete warning: displayed kilometres added are not a comparable measurement of charging throughput. Record metered kWh, average/peak power, SOC band, vehicle consumption, temperature and energy boundary. The figures establish the importance of vehicle class and test boundary; they are not a single SOTA target for EV-REF-01. The programme should compare the same class, payload/occupancy, ambient conditions, route/cycle, reserve and usable performance.

### Ultra-fast charging

The [IEA's 2026 ultra-fast charging review](https://www.iea.org/reports/ultra-fast-charging-batteries) describes new cell, power-electronics and pack developments, including early-charge C-rates up to 10C/12C for some announced technologies and passenger-vehicle high-voltage systems above 800 V. [BYD's second-generation Blade/FLASH launch](https://www.byd.com/za/news-list/byd-unveils-2nd-generation-blade-battery-and-flash-charging-technologyw) claims 10–70% in 5 minutes and 10–97% in 9 minutes; those are manufacturer claims, not an independent matched vehicle/charger test. The engineering metric is the complete SOC/temperature-dependent curve and energy delivered at the vehicle boundary, including thermal preconditioning, taper, site/feeder limits, queue, repeated-session behavior, degradation and infrastructure cost.

Luminous should design interfaces that can accommodate higher-voltage/faster-charging variants without making a 1 MW-class charger a dependency of the affordable base configuration. The IEA reports that fewer than 5% of the 2025 EV stock could use chargers above 250 kW. A high-power architecture is only useful where cells, busbars, contactors, cable cooling, connector, EVSE, site import, feeder and thermal envelope all support it. The [IEA's 2026 V2G review](https://www.iea.org/reports/vehicle-to-grid-technology) counts 22 V2G-capable models, less than 1.5% of EV models, and notes low multiparty interoperability. V1G/grid-aware charging is the nearer-term default; V2H/V2G should be optional and not a base-case revenue assumption.

### Battery integration and circularity

The 2026 literature shows why this is not a simple modular-versus-structural binary. [Nature Reviews Clean Technology](https://www.nature.com/articles/s44359-026-00185-y) reports system-level benefits from integrated structures, but also risks in diagnostic access, intervention granularity and end-of-life material recovery. [Nature Reviews Electrical Engineering](https://www.nature.com/articles/s44287-026-00342-2) frames integration as function reassignment: every deleted boundary still had mechanical, thermal, electrical and safety functions. #2155 must evaluate the candidate architectures on that whole-life basis.

### Manufacturing strategy

Industry platforms already offer integrated CAD/CAM, process simulation, inspection and manufacturing digital-thread features; #2164 already contains the capability map and a matched manufacturing comparison protocol. Luminous should not claim novelty merely for “having a digital twin.” The differentiator worth testing is an interoperable, vendor-neutral evidence layer that binds exact CAD/PMI, process, lot, machine/calibration, inspection result, repair and as-built configuration without requiring an enterprise to replace its existing PLM/MES/QMS stack.

### Autonomous driving measurement

Symthaea #6256 already covers Waymo, nuScenes, Argoverse, CommonRoad, nuPlan, CARLA, Waymax, ASAM OpenSCENARIO and MLPerf Automotive as distinct evidence tracks. [NIST's four-tier Measurement Science for Automated Vehicles programme](https://www.nist.gov/programs-projects/measurement-science-automated-vehicles-0) is a particularly useful independent pattern: surrogate safety metrics, predicted outcomes, counterfactual comparison against baselines, and recurrent weakness diagnosis. Align our evaluation with these measurement questions while explicitly avoiding any claim of NIST endorsement or certification.

## 8. South African proving ground

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

## 9. Safety, security and legal engineering

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

## 10. Execution sequence and work ownership

1. **EV-REF-01 integration/configuration:** [Symtropy #4346](https://github.com/Luminous-Dynamics/symtropy/issues/4346).
2. **Industry comparison:** [Symtropy #2164](https://github.com/Luminous-Dynamics/symtropy/issues/2164).
3. **Battery integration and circularity:** [Symtropy #2155](https://github.com/Luminous-Dynamics/symtropy/issues/2155).
4. **Battery and traction health advisory:** [Symthaea #11239](https://github.com/Luminous-Dynamics/symthaea/issues/11239).
5. **Route-energy planner:** [Sol-Atlas #664](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/664).
6. **Mobility configuration lineage:** [Mycelix #3709](https://github.com/Luminous-Dynamics/mycelix/issues/3709) and [Mobility Commons #3708](https://github.com/Luminous-Dynamics/mycelix/issues/3708).
7. Audit existing family-vehicle work (#1762 / #7751 / #5131), battery model and digital twin before any new implementation.
8. Freeze mission, packaging and vehicle-level requirements; create the component/interface graph and candidate BOM; link exact configs into existing benchmark owners.
9. Run public-data and analytical comparisons before committing to a physical demonstrator.

## 11. Qualification gates

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

## 12. Initial claim ceiling

At this stage, Luminous has research software, models, benchmark programmes and an emerging architecture. This document does **not** establish a built prototype, a measured vehicle efficiency improvement, battery qualification, production capability, public charging uptime, autonomous-driving safety, homologation or superiority over an incumbent manufacturer.

The programme succeeds by turning each of those future claims into a falsifiable test with a predeclared comparator and an independently reviewable evidence package.
