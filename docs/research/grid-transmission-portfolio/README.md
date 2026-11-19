# Grid Transmission Portfolio — v0.1 research baseline

**Owner:** [GRID-TRANSMISSION-PORTFOLIO-001, Symtropy #6067](https://github.com/Luminous-Dynamics/symtropy/issues/6067)  
**Status:** Research charter / evidence map. No transmission design solver, route optimiser, physical test, utility approval or build-ready design is claimed.  
**Snapshot date:** 2026-11-18

## 1. Decision problem

The Power Fabric should not assume that energy always travels as electricity, that electricity always travels on overhead conductors, or that one transmission medium should cover an entire route.

The design object is:

`source -> carrier conversion -> transmission path -> intermediate conversion/storage -> delivered service`

The system must choose the **carrier, converter, medium, route segments, terminal equipment, storage and maintenance concept together**. It should preserve separate physical models and energy ledgers for each carrier. A common interface is useful at declared service boundaries; a universal internal physics model is not.

Keep these categories distinct:

- **Generation/conversion:** reciprocating gensets, microturbines, free-piston linear generators, fuel cells, PV/wind conversion and other sources.
- **Storage:** electrochemical, flow, mechanical, thermal, hydrogen and other stores.
- **Transmission:** conductors, cables, pipelines, thermal networks, mechanical links or wireless transfer.
- **Distribution:** downstream networks, buses, feeders and service interfaces.
- **Delivered service:** net AC, regulated DC, useful heat/cooling, hydrogen at a custody-transfer boundary, shaft/wheel/propulsor work or another typed output.

Batteries are not transmission media. A linear generator is a conversion topology, not a transmission line. Hydrogen in a pipeline is an energy carrier; the pipeline does not itself make hydrogen or convert it back to electricity.

## 2. Technology portfolio and applicability

This is an architecture map, not a universal ranking. The cost and performance of each option depend on voltage/power, distance, duty profile, geography, system strength, civil works, environmental constraints, terminal equipment and reliability requirements.

| Family | Candidate methods | Where to evaluate it | Principal constraints that must remain visible |
|---|---|---|---|
| Overhead AC | Lattice, monopole, compact, multi-circuit; conventional and advanced conductors | Bulk grid corridors, regional networks, terrain where wayleave is feasible | Right-of-way, visual/ecological impact, weather, sag/clearance, corona, stability, structures and foundations |
| Overhead DC | LCC-HVDC, VSC/MMC-HVDC, bipolar or other declared configurations | Long-distance bulk transfer, asynchronous interconnection, controllable corridors | Converter cost/losses, terminal footprint, fault isolation/protection, control interactions, electrode/return design where applicable |
| Underground AC | Direct-buried XLPE, duct banks, trenches, cable tunnels | Urban/constrained sections, crossings, high land-use or visual constraints | Cable charging/reactive power, thermal backfill/soil limits, joints, repair time, route heat accumulation and civil cost |
| Underground DC | HVDC cable systems, including XLPE where qualified | Long cable sections, submarine/underground constrained routes, converter-connected links | Converter stations, DC protection, insulation qualification, joints, repair logistics and terminal redundancy |
| Submarine | HVAC or HVDC subsea cables and offshore converters | Offshore wind, island/port interconnection, sea crossings | Water depth, seabed geology, burial/protection, landing points, cable repair vessels, converter topology and long repair lead times |
| Gas-insulated line | AC or DC GIL in sealed enclosures; SF6-free insulation pathway | Dense urban/industrial corridors, tunnel or constrained crossings, high-capacity compact routes | High capital cost versus overhead lines, assembly duration, gas leakage/GWP, insulation validation and thermal/civil integration |
| Superconducting | HTS AC; emerging HTS DC; cryostats and refrigeration plant | Very high power density, constrained urban links, special high-capacity corridors | Cryogenic parasitic load, refrigeration redundancy, quench protection, joints, cryostat integrity, materials/supply chain and still-limited operating references |
| Grid-enhancing | Dynamic line/transformer rating, topology optimisation, phase-shifting/FACTS, advanced power-flow control, reconductoring, voltage uprating, storage-as-transmission | Existing congested assets before or alongside greenfield expansion | Weather/state dependence, contingency and stability constraints, substation bottlenecks, protection upgrades, sensor/calibration quality and firmness of unlocked capacity |
| Molecular carriers | Hydrogen pipelines; ammonia/methanol/synthetic methane/liquid fuels by pipeline, ship, rail or road | Power-to-X hubs, industrial feedstock/heat, long-duration storage, export and remote loads | Conversion losses, compression/liquefaction/synthesis/cracking, purity, leakage, embrittlement, toxicity, materials compatibility and demand certainty |
| Thermal networks | District heating/cooling, industrial hot-water/steam/thermal-fluid networks | Dense heat demand, industrial clusters, waste-heat/geothermal/solar sources | Temperature grade, distance, insulation, pumping, supply/return temperature, seasonal mismatch, thermal storage and heat-pump lift |
| Mechanical/fluid power | Shaft, belt, hydraulic or pneumatic transmission | Bounded industrial machinery or local specialist applications | Distance, conversion stages, pressure/flow losses, leakage, safety and maintenance; compare at delivered mechanical work |
| Wireless near-field | Inductive/resonant and capacitive transfer | Device charging, sealed equipment, moving machinery, selected EV/robot applications | Coil alignment/gap, power density, shielding, EMC, thermal limits, foreign-object detection and end-to-end efficiency |
| Wireless far-field | Microwave or laser power beaming | Research, selected remote/space or inaccessible niche applications | Conversion chain, beam capture, line-of-sight, weather, spectrum, eye/biological safety, aviation and land footprint; not a default bulk-grid substitute |

### Important maturity boundaries

- **HVDC is a mature transmission family**, but LCC, VSC/MMC and multi-terminal/meshed DC are not interchangeable. Select them against grid strength, controllability, fault-clearing and terminal requirements.
- **525 kV XLPE HVDC cable is moving into industrial-scale subsea production.** Nexans and TenneT announced the start of production for LanWin2 on 16 October 2026. This is a manufacturing/delivery milestone, not proof that every 525 kV route has the same cost or performance.
- **GIL is a real alternative for special constrained corridors, not a cheap universal substitute for OHL.** ENTSO-E describes its compact form and high-capacity use, while also noting substantially higher cost than overhead lines and longer assembly times. New insulation gas and lifecycle impacts need explicit review.
- **HTS AC and HTS DC should be separate maturity classes.** The 2026 superconducting-cable roadmap reports AC cable maturity at a higher level than DC, with DC still in demonstration/development. An individual commercial distribution demonstration does not establish long-distance multi-GW economic viability.
- **Wireless power is not one technology.** Near-field inductive/capacitive transfer is relevant for short-range charging and sealed/mobile equipment. Far-field microwave/laser systems add emitter/receiver conversion, beam-control, exposure and siting constraints. Treat space-solar and long-range beaming as research/option studies unless independently demonstrated at the specified scale.
- **Hydrogen transport competes for a different delivered service than electric transmission.** Compare hydrogen pipeline delivery at a defined purity/pressure/mass boundary against electricity delivered to an electrolyser or end-use plant, with the complete conversion chain shown. A cheap pipeline does not erase electrolysis, compression, storage, reconversion or end-use efficiency.
- **Heat networks are valuable when the customer needs heat.** They should not be ranked against electric transmission by comparing raw thermal MWh to electrical MWh without accounting for temperature grade, exergy, heat pumps and the actual service.
  
## 3. Evidence snapshot and external baselines

### Grid-enhancing technologies (GETs)

The IEA's *Electricity 2026* grid chapter gives indicative potential capacity increases and lead times for selected options:

| Option | IEA indicative capacity-increase range | Interpretation |
|---|---:|---|
| Dynamic line rating | 20–30% | Applicable to assets where real weather/thermal headroom is available; weather- and corridor-dependent |
| Dynamic transformer rating | 5–15% | Transformer thermal headroom, not a line-only upgrade |
| Topology optimisation | 5–15% | Depends on network switching flexibility and security constraints |
| Advanced power-flow control | 10–20% | Can redirect flows; does not create new generation or bypass all thermal/stability limits |
| Storage as a transmission asset | 30–40% | System/use-case dependent; duration, location and dispatch matter |
| Reconductoring | 50–100% | Selected candidate corridors; may require tower, substation or protection upgrades |
| Voltage uprating | 100–200% | High-capital, engineering/permitting-intensive option |
| New HV lines | No generic uplift | New capacity and route must be planned as a system project |

These are IEA portfolio-level indicative ranges, not guaranteed uplift on a South African line and not additive. Multiple upgrades can be bottlenecked by the same transformer, terminal, stability limit or contingency.

The IEA separately identifies dynamic ratings, advanced power-flow control, topology optimisation and remedial actions as technologies that can use existing assets more effectively. It classifies dynamic ratings as commercially available and well established, while operational adoption and confidence vary by technology and operator.

### Advanced conductors

The 2024 PNAS study *Accelerating transmission capacity expansion by using advanced conductors in existing right-of-way* models composite-core reconductoring in the US system and finds it could potentially provide major capacity expansion within existing rights-of-way. It estimates large system-level benefits under its US scenario and explicitly calls for project-specific load-flow, contingency and dynamic-stability studies.

The US DOE Advanced Conductor Scan also records mixed utility experiences and stresses that reconductoring is not always the best option: asset condition, substation constraints and the opportunity cost of not rebuilding at a higher voltage must be assessed.

**Design implication:** compare conventional reconductoring, advanced conductors, voltage uprating, rebuild, double-circuit and GET combinations on the same corridor. Do not optimise conductor mass without checking structures, foundations, clearances, terminals and stability.

### Undergrounding, GIL and tunnels

ENTSO-E's GIL technology sheet describes aluminium conductors enclosed in gas-insulated pipes, used in tunnels and constrained locations. It states that GIL is materially more expensive than overhead line and typically slower to assemble; longer AC GIL sections may need reactive-power compensation.

A 2026 *Nature Communications* route-level study compares overhead lines, underground AC/DC cables and GIL in selected mountainous renewable corridors. It finds that underground options can become competitive when route-specific ecological/landscape and civil costs of overhead lines are sufficiently high. Its numerical thresholds and cost results are study-specific. The useful general result is **segmentwise hybridisation**: use overhead line where unconstrained and underground only where the local external or construction cost warrants it.

The UK 2025 undergrounding study on 275/400 kV routes reports that cable ploughing can reduce civil-works cost materially in suitable ground and can lower total project costs in its scenarios. It also shows why HDD is a crossing tool, not a blanket replacement for conventional cable installation.

### HVDC and submarine cable

An October 2026 open-access review of VSC-HVDC describes VSC/MMC applications in renewable integration, weak-grid connection and multi-terminal DC development, while noting unresolved protection, topology and control challenges.

The Nexans/TenneT LanWin2 announcement on 16 October 2026 provides a current industrial milestone for 525 kV XLPE HVDC subsea cable manufacturing.

**Design implication:** the portfolio should include both point-to-point HVDC and candidate multi-terminal configurations, but only when converter topology, DC fault isolation/protection, grid-forming/control behaviour, redundancy and terminal costs are represented.

### Superconducting systems

The 2026 IEEE/IOP roadmap reports commercial/operational HTS AC cable deployments in grid applications and places DC HTS at a lower technology maturity. A 2026 multi-GW HTS HVDC techno-economic paper reports modeled break-even thresholds against conventional underground and overhead HVDC for its assumed 10 GW / 500 km case.

Treat that as a **model hypothesis to reproduce**, not an established procurement rule. The optimizer must count cryogenic plant capital, parasitic electricity, redundancy, outage risk, cooling losses, joints, replacement, and the value of corridor capacity.

### Hydrogen and thermal carriers

The IEA Global Hydrogen Review 2024 identifies pipelines as a cost-effective option for moving large volumes of hydrogen, while noting that utilisation and network assumptions matter. A 2026 review of hydrogen transmission in steel pipelines identifies repurposing as potentially cheaper than new construction in its examined cases, with compression, purification and embrittlement mitigation among important cost/technical issues.

Nature Reviews Clean Technology's 2025 district-heating review describes how networks can use waste heat, geothermal and solar heat, including heat from data centres and industrial processes. The economic boundary is useful heat at a customer, not electricity-equivalent energy alone.

### Wireless transfer

A 2026 review of near- and far-field wireless transfer distinguishes high-power short-range inductive/capacitive systems from lower-efficiency, more challenging radiative microwave/laser approaches. A 2026 Nature review of wireless energy conversion also identifies standardised interfaces, security and application-specific protocols as key gaps.

**Design implication:** keep near-field transfer in the candidate catalogue for device, EV, robotic and sealed-equipment use cases. Keep far-field transfer in a research/niche lane until the complete emitter-to-load chain, safety case, receiver footprint, weather availability and cost are independently evidenced at the proposed scale.

### South African context

NTCSA publishes the 2025–2034 Transmission Development Plan. The DBSA's Q1 2025 investment note summarises the plan as requiring approximately 14,500 km of new transmission lines and 133,000 MVA of transformer capacity over the decade, with an estimated R440 billion investment requirement. This is an execution challenge as much as a conductor/tower optimisation problem.

The South African portfolio should therefore quantify manufacturing throughput, tower/cable/converter availability, contractor capacity, wayleave/permitting, local environmental conditions, repair logistics and staged energisation—not just theoretical route cost.

## 4. Luminous related-work audit

The work below was checked against GitHub metadata and issue/PR descriptions on 2026-11-18. State is included so that designs and CI results are not confused with physical validation.

| Existing work | What it already owns | Current evidence / boundary |
|---|---|---|
| Symtropy #3400 | Energy digital-twin architecture, system ledgers and co-simulation boundaries | Open issue; defines initial 0D model scope and explicit solver boundaries |
| Symtropy PR #3401 | `symtropy-energy-systems` 0D fuel -> engine -> alternator accounting, batch and uncertainty basics | Open draft, base `main`; exact head `060a26de846fab07f8a56c3740c3b763abd7e6c8` had passing GitHub Actions across Ubuntu/macOS/Windows. Arithmetic/software evidence only, not physical engine validation |
| Symtropy #3409 | Separate internal ledgers for reciprocating, turbine, free-piston linear generator, fuel cell, battery and hybrid topologies | Open issue; architecture/competitor review. It explicitly prevents forcing every conversion type through an engine-shaft model |
| Symtropy #3404 / PR #3411, #3417, #3420 | Batch-bound evidence, matched-service FFER and independent energy-ledger reconstruction | PRs remain open drafts stacked on feature branches; the fetched descriptions do not claim exact-head CI for these child increments |
| Symtropy PR #3419 | Common delivered-service contracts for net AC, regulated DC and shaft/wheel/propulsor work | Open draft stacked on #3417; schema/fixtures are synthetic and CI is not claimed in its PR description |
| Symtropy #3418 | Grid-forming, islanding, black-start, DC microgrid, protection and resilience | Open issue; composes existing electrical/protection owners rather than pretending the 0D generator model validates grid transients |
| Symtropy #5158 / #5159 | Human-centred Power Fabric and vendor-neutral typed module contracts | Open issues; define role-specific views, AC/DC/thermal/measurement/control/protection ports and evidence classes |
| Symtropy #5180 / PR #5182 | Fail-closed module-compatibility kernel | Open issue / open draft PR stacked on #3401; compatibility metadata is not wiring approval, protection coordination or permission to energise |
| Symtropy #5183 / PR #5188 | Typed Electrical Service Graph and declared source-to-load path assessment | Open issue / open draft PR stacked on #5182; graph fixture is synthetic; graph paths are not live flow, load-flow, redundancy proof or isolation proof |
| Symtropy #3752 | PCC metrology and uncertainty | Existing owner for point-of-common-coupling evidence; transmission portfolio must consume, not duplicate, its metrology contract |
| Symthaea #10119 | Fuel-flexible generator, heavy-truck and marine powertrain comparison | Open issue; delivered-service and fuel/logistics boundary |
| Symthaea #10143 | Evidence-calibrated multi-objective optimisation and active experiment design | Open issue; names random, Sobol/LHS, NSGA-II, MOEA/D, CMA-ES, multi-objective Bayesian optimisation and canonical test suites |
| Symthaea #10151 | Competitor evidence ingestion | Open issue; evidence catalogue currently covers multiple generator/powertrain entities and preserves claims, tests, pilots and agreements separately |

### Gap found

The above work is a strong foundation for **generation, conversion, electrical module composition, resilience and service evidence**. It does not yet provide a route-level comparison of HVAC/HVDC, overhead/underground/subsea, GIL, superconducting transmission, hydrogen/thermal networks and wireless power on matched infrastructure service.

This v0.1 portfolio owner (#6067) fills that gap and must integrate rather than duplicate the current owners.

## 5. Proposed system architecture

1. **Service contract:** exact customer boundary, time-profiled demand, power/quality/pressure/temperature, reserve, reliability and availability requirement.
2. **Route segmentation:** geospatial and physical constraints partition a corridor into independently selectable segments.
3. **Candidate generator:** emit technically admissible carrier/medium/converter combinations for each segment.
4. **Topology and compatibility checks:** consume #3409, #3419 and #5159/#5182 contracts; incompatible or unknown interfaces fail closed.
5. **Domain model adapters:** electrical load-flow/transients/protection; cable thermal; GIL/insulation; pipeline flow/materials; thermal hydraulics; geotechnics; wireless propagation and safety. Each adapter declares solver, model domain, calibration and uncertainty.
6. **Lifecycle economics:** CAPEX, financing, energy losses, auxiliaries, O&M, replacement, outages, land/wayleave, ecological impacts and end-of-life.
7. **Pareto exploration:** first deterministic enumeration and conventional optimisation; later constrained multi-objective search and active experiment selection.
8. **Independent replay:** reconstruct the selected result from frozen inputs and independently check conservation, dimensions, constraints and the objective ledger.
9. **Human-readable design package:** route map, segment choice, single-line / carrier-flow views, assumptions, unresolved items, construction package inputs and maintenance concept. It is not an installation certificate.

## 6. Benchmark plan

### Frozen South African archetypes

| ID | Reference case | Key discriminators |
|---|---|---|
| SA-01 | Northern/Eastern Cape renewable-to-load bulk corridor | Distance, generation profile, grid congestion, weak-grid conditions, terrain, new-build vs GETs/reconductoring |
| SA-02 | Gauteng urban/industrial constrained corridor | Wayleave, traffic/reinstatement, existing services, tunnel/duct/GIL/cable choices, outage cost |
| SA-03 | Saldanha / West Coast power-to-X and port cluster | Grid electricity vs hydrogen/ammonia/methanol logistics, industrial heat, water, export and port infrastructure |
| SA-04 | Mountainous/ecologically sensitive corridor | Segmentwise overhead vs underground AC/DC vs GIL, geological difficulty and monetised/non-monetised externalities |
| SA-05 | Coastal/offshore/submarine interconnection | Subsea cable, landing points, converter terminals, repair vessels and offshore system topology |
| SA-06 | Remote mine or distributed microgrid | Local generation/storage, AC/DC, fuel logistics, islanding, black-start and service continuity |

Every archetype must have baseline and adverse-condition variants. All input values must be tagged as measured, independently verified, public dataset, literature, manufacturer-declared, modelled or analyst scenario.

### Required baselines

- Existing route/utility reference design and its conventional cost estimate.
- HVAC overhead with conventional conductor.
- HVAC overhead with advanced conductor/reconductoring where physically admissible.
- HVDC overhead and cable where technically admissible.
- Underground AC/DC cable using at least one direct-buried/duct-bank and one tunnel/trenchless installation case.
- GIL for constrained high-capacity sections.
- GET-only / operational upgrade packages.
- Hydrogen pipeline and thermal-network architectures only where the customer service is compatible.
- Wireless transfer only for near-field/mobile or explicitly experimental remote-power scenarios.
- Mixed-medium route chosen segment by segment.

### Core metrics

Report the full Pareto vector rather than a weighted scalar:
- firm and time-varying transfer capability (MW), delivered energy (MWh/year), curtailment and unserved service;
- end-to-end loss by stage, carrier and operating point;
- lifecycle net-present cost and annualised cost; cost per delivered MWh and cost per firm MW-km-year only where the denominator is well-defined;
- availability, forced outages, repair time, common-cause exposure and applicable N-1/N-2 performance;
- route lead time, installed km/year, crew-hours, productivity, permitting and wayleave burden;
- materials, embodied carbon, land/sea footprint, ecological disturbance, water and lifecycle emissions;
- thermal/voltage/pressure/quality margin and environmental derating;
- fault localisation, access, replacement time, spares and serviceability;
- expandability, stranded-asset risk and upgrade cost;
- prediction residuals, uncertainty coverage, evidence class and sensitivity.

### Optimiser and evidence metrics

- Pareto hypervolume with frozen reference point; IGD+ and epsilon indicator where a known reference front exists.
- Feasibility discovery rate, constraint violations and false-feasible rate.
- Evaluations/solver calls to target quality, p50/p95/p99 runtime, memory and solver cost.
- Candidate ranking stability across independent seeds and perturbations.
- Prediction-interval calibration and constraint-probability calibration.
- Expected value of information / decision-regret reduction per experiment cost.
- Independent replay agreement and exact-input/result digest stability.

### Proposed hard gates (research targets)

- Zero false acceptance in frozen critical interface, conservation, safety-boundary and service-identity negative controls.
- Unknown data never become zero, compatible, safe, or measured by default.
- No architecture violates a hard electrical, thermal, pressure, fire, environmental or reliability constraint.
- All comparisons use the same service, route, reliability and lifecycle boundaries.
- At least one realistic case shows an independently audited reduction in lifecycle cost or delivery time, without degrading required service or reliability. The improvement threshold is set after the baseline and uncertainty model are frozen, not retrofitted to a preferred design.
- Publish all non-dominated solutions and the sensitivity that changes the recommended route.

## 7. First implementation increments

1. Freeze a typed service/carrier/route-segment schema with evidence and units.
2. Hand-calculate synthetic conservation fixtures for electricity, hydrogen and useful heat, with separate ledgers and hostile unit/double-count controls.
3. Implement a deterministic small-route portfolio enumerator and transparent lifecycle-cost kernel.
4. Add SA-01 and SA-02 archetype schemas without inventing missing local costs or line ratings.
5. Connect existing generator/service/module/resilience contracts.
6. Add independent oracle/replay tests and mutation/staleness checks.
7. Benchmark enumeration, NSGA-II, MOEA/D and multi-objective Bayesian optimisation under the same problem definitions and evaluation budgets.
8. Only then connect validated electrical, pipeline, thermal and geotechnical solvers or site measurements.

## 8. References

- NTCSA, [Transmission Development Plan 2025–2034](https://www.ntcsa.co.za/transmission-development/).
- DBSA, [Transmission Infrastructure: Investment Models](https://www.dbsa.org/transmission-infrastructure-investment-models), Q1 2025.
- IEA, [Electricity 2026 — Grids](https://www.iea.org/reports/electricity-2026/grids).
- IEA, [Grid-enhancing technologies](https://www.iea.org/reports/modernising-grids-in-the-age-of-electricity/grid-enhancing-technologies).
- ENTSO-E, [Gas-Insulated Lines (GIL)](https://www.entsoe.eu/technopedia/techsheets/gas-insulated-lines/), 25 March 2025.
- B. et al., [Bulk electrical power extraction through tunnel transmission technology from renewable energy concentrated zones](https://www.nature.com/articles/s41467-026-71221-5), *Nature Communications*, 2026. Use only its study-specific route assumptions and thresholds.
- U.S. DOE, [Advanced Conductor Scan Report](https://www.energy.gov/sites/default/files/2024-08/Advanced%20Conductor%20Report%20December%202023.pdf).
- [Accelerating transmission capacity expansion by using advanced conductors in existing right-of-way](https://pmc.ncbi.nlm.nih.gov/articles/PMC11459140/), *PNAS*, 2024.
- Nexans, [Cable production launched for major German offshore wind grid connection](https://www.nexans.com/press-releases/cable-production-launched-for-major-german-offshore-wind-grid-connection/), 16 October 2026.
- [Developments and Prospects of Voltage Source Converter-HVDC Transmission Technology](https://doi.org/10.1049/hve2.70253), first published 18 October 2026.
- [Superconducting high-power cables and lines—development status and technology roadmap](https://snf.ieeecsc.org/files/ieeecsc/2026-02/noe_2026_supercond-sci-technol-39_023501.pdf), *Superconductor Science and Technology*, 2026.
- [Toward multi-gigawatt transmission systems: Optimization and techno-economic analysis of high-temperature superconducting cables](https://www.sciencedirect.com/science/article/pii/S0378779626006267), *Electric Power Systems Research*, 2026. Modelled results only.
- IEA, [Global Hydrogen Review 2024](https://iea.blob.core.windows.net/assets/89c1e382-dc59-46ca-aa47-9f7d41531ab5/GlobalHydrogenReview2024.pdf).
- [A review of the challenges, solutions and economics of hydrogen transmission in steel pipelines](https://doi.org/10.1016/j.jpse.2026.100451), 2026.
- [District heating in clean energy systems](https://www.nature.com/articles/s44359-025-00076-8), *Nature Reviews Clean Technology*, 2025.
- [A Review on Near-Field and Far-Field Wireless Power Transfer Technologies](https://www.mdpi.com/1996-1073/19/1/157), 2026.
- [Wireless energy conversion in wireless energy internet](https://www.nature.com/articles/s44287-026-00279-6), *Nature*, 2026.

---

**Claim boundary:** This document is a research map and proposed benchmark protocol. It does not provide route-specific prices, electrical ratings, construction drawings, cable/pipeline dimensions, tower calculations, protection settings, certified designs or evidence of SOTA performance. Numerical results from external publications remain attached to their original study assumptions.


## 9. Global portability: global core, local profiles

The Power Fabric is intended to work globally, but **global portability does not mean one universal engineering rulebook**. The programme uses common schemas, service boundaries, evidence semantics and benchmark methods, while physical designs, climate inputs, standards, grid codes, costs, permitting and accepted practices are attached to jurisdiction/region profiles.

New programme owners:
- [#6105 POWER-FABRIC-GLOBAL-PORTABILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/6105): standards, regional hazards, cost normalisation and global reference archetypes.
- [#6106 TRANSMISSION-INDUSTRIALISATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/6106): tower families, fabrication, supply networks, logistics and construction throughput.
- [#6080 GRID-ENHANCEMENT-FIRM-CAPACITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/6080): firm capacity and non-additive GET portfolios.

The global profile file at `data/global-portability-v0.1.json` provides 10 initial regional templates:
1. Southern Africa / SAPP.
2. North America.
3. Europe / United Kingdom.
4. India / South Asia.
5. East Asia.
6. Latin America.
7. Middle East / North Africa.
8. Sub-Saharan Africa beyond SAPP.
9. Oceania / Pacific Islands.
10. Arctic / High Latitude.

They are deliberately marked `template_unpopulated` with local standards adoption and cost data unknown. These are coverage templates, not national engineering profiles or legal-compliance determinations.

### Standards metadata snapshot (18 November 2026)

The machine-readable register records exact editions and status boundaries for 13 reference documents. Particularly important details:
- IEEE 738-2023 is active for conductor current-temperature modelling, but does not prescribe the weather or conductor inputs that a line-rating assessment must use.
- IEC 60840:2020+AMD1:2023 has a 2025 corrigendum in the current consolidated publication.
- IEC 60287-1-1:2023 provides steady-state cable current-rating and losses methods; site thermal resistivity and ambient parameters remain project inputs.
- IEC 60287-2-3:2024 gives a thermal-resistance method for cables in ventilated tunnels, but assumes identical cables, constant tunnel cross-section and negligible longitudinal heat transfer. Actual tunnel geometry and ventilation must be checked against those assumptions.
- IEC 62067:2022 covers extruded-insulation cable systems from above 150 kV through 500 kV, in its stated scope.
- IEC 60071-2:2023 remains the published edition in the current catalogue snapshot, while Edition 6 is under development.
- IEC TS 60071-4:2026 was published on 23 October 2026 and provides simulation-based insulation-coordination modelling guidance.
- ASME B31.12-2023 and ISO 14687:2025 are hydrogen references with different scopes: pipeline/piping design versus fuel quality. ISO/AWI 14687 Edition 3 is under development and is not yet a published replacement.
- NERC TPL-001-5.1 is listed as mandatory in the relevant NERC jurisdictional framework, while TPL-001-6 is filed/pending regulatory approval in the checked catalogue.
- India's CEA Manual on Transmission Planning Criteria 2023 is an India-specific planning reference, not a global HVDC break-even rule.

The registry intentionally does **not** infer national adoption. Standards, amendments, local deviations and utility grid codes must be checked with the relevant national standards body, regulator, system operator and project authority.

Primary sources:
- [IEC 60826:2017](https://webstore.iec.ch/en/publication/59954)
- [IEEE 738-2023](https://standards.ieee.org/ieee/738/10207/)
- [IEC 60840 consolidated version and 2025 corrigendum](https://webstore.iec.ch/en/publication/107576)
- [IEC 60287-1-1:2023](https://webstore.iec.ch/en/publication/68118)
- [IEC 60287-2-3:2024](https://webstore.iec.ch/en/publication/80333)
- [IEC 62067:2022](https://webstore.iec.ch/en/publication/66951)
- [IEC 60071-2:2023](https://webstore.iec.ch/en/publication/64145)
- [IEC TS 60071-4:2026](https://webstore.iec.ch/en/publication/107140)
- [IEC 60071-11:2022](https://webstore.iec.ch/en/publication/66648)
- [ASME B31.12-2023](https://www.asme.org/codes-standards/find-codes-standards/b31-12-hydrogen-piping-pipelines)
- [ISO 14687:2025](https://www.iso.org/standard/82660.html) and [ISO/AWI 14687 Edition 3 work item](https://www.iso.org/standard/94693.html)
- [NERC TPL standards catalogue](https://www.nerc.com/standards/reliability-standards/tpl)
- [India CEA Transmission Planning Criteria 2023](https://www.cea.nic.in/api/dashboardReports/view-pdf/en/8da5cbc3-88c4-461c-8694-e2a050cc4dbf)

### Global design and economics

Each route must carry:
- climate/hazard distributions and their time horizon, not a single "typical weather" value;
- site geology, groundwater, soil thermal resistivity, corrosivity and seismic/landslide/flood context;
- jurisdiction-specific standards, editions, national adoption, amendments and grid-code references;
- local labour, skills, factory capacity, import duties, logistics, FX and finance;
- land/wayleave, community rights, ecological, water and heritage constraints;
- local decision economics and a separately normalised technology-comparison view.

Local procurement costs must not be replaced by purchasing-power-parity estimates. Report local currency and real project cost as well as any normalised comparison currency and base year. A global average must not hide regional uncertainty or poor maintainability.

### Current global pressure and benchmark framing

The IEA's *Electricity 2026* identifies more than 2,500 GW of projects stalled in grid connection queues worldwide and says annual grid investment would need to rise roughly 50% by 2030 from about USD 400 billion today. The IEA also highlights that transmission planning and construction commonly take 5–15 years, much longer than many generation and large-load projects. This makes time-to-first-usable-capacity a first-class benchmark, alongside installed cost and firm capacity. [IEA Electricity 2026](https://www.iea.org/reports/electricity-2026/grids).

The IEA's *World Energy Investment 2026* estimates global grid spending rose to nearly USD 450 billion in 2025 and projects a further 17% increase in 2026, while noting inflationary pressure, constrained supply chains and shortages in specialised manufacturing capacity. [IEA World Energy Investment 2026](https://www.iea.org/reports/world-energy-investment-2026).

**Our global competitive target is not a one-number world record.** It is a reproducible, region-by-region proof that a chosen architecture delivers equal or better service, reliability and resilience with lower lifecycle cost, shorter time-to-service or improved maintainability under local conditions.


## 10. Lifecycle economics and cross-country comparability

The `data/lifecycle-economics-v0.1.schema.json` contract separates:
- the local decision case in the project's actual currency, price year and tax treatment;
- real versus nominal valuation and a matching discount-rate basis;
- study horizon and service denominator;
- itemised cost/credit categories across design, equipment, materials, fabrication, logistics, civil works, installation, commissioning, energy losses, maintenance, replacements, outages, land, permitting, environmental mitigation, decommissioning and residual value;
- imported versus local cost exposure and evidence references;
- an optional normalised comparison currency with explicit FX basis/date/source and a separately identified PPP sensitivity.

PPP is not a substitute for actual procurement cost. The schema requires the local case and normalised comparison to remain separate. A future evaluator must also enforce cross-field identities (currency consistency, period bounds, compatible real/nominal discounting, FX requirements, evidence references and no double counting).

The included `synthetic-lifecycle-economics-v0.1.json` is only a contract fixture with synthetic dollar values. It is not a cost estimate, discount-rate recommendation, NPV result or procurement benchmark.

The current GitHub Actions workflow now validates the route, global portability and lifecycle economics JSON Schemas against their fixtures, including date/URI formats; runs the Rust semantic-validator tests; checks Rust formatting and Clippy with warnings denied; and validates the checked-in route fixture. The run must still pass on the exact PR head before any CI claim is made.
