# Cross-Appliance Architecture Morphology and Hybrid Design Portfolio v0.1

**Status:** design-space exploration and testable hypotheses, not a selected product architecture. No CAD, physical prototype, comparative measurement or product conformity is claimed.  
**Portfolio owner:** Symtropy. Related: [large-appliance portfolio #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825), [decision engine #1862](https://github.com/Luminous-Dynamics/symtropy/issues/1862), [stakeholder profiles #1872](https://github.com/Luminous-Dynamics/symtropy/issues/1872), [fault curriculum #1875](https://github.com/Luminous-Dynamics/symtropy/issues/1875), [draft PR #1827](https://github.com/Luminous-Dynamics/symtropy/pull/1827).

## 1. The next paradigm: stop designing one appliance at a time

The expensive failure is often not a single bad component; it is an architecture that makes every repair wait on an inaccessible interface, proprietary diagnosis, expensive assembly replacement, unavailable part, or specialist visit. The opportunity is to develop **a family of product architectures with common service contracts but product-specific physical modules**.

The right shared platform is not one universal pump, compressor, power board or connector. It is the reusable knowledge and interfaces around them: component identity, diagnostics, safe-state semantics, part-fit evidence, service task boundaries, access conventions, verification receipts and lifecycle accounting. Mechanical/electrical interchangeability remains a product-specific claim that must be demonstrated.

The 2025 open-access appliance-module study shows how physical disassembly tests can reveal actionable design-for-repair changes across several cooking-appliance categories. A 2025 repair-barrier review also distinguishes physical product barriers from service-ecosystem barriers such as information access, time and economics. We use these as methods and hypothesis generators, not as proof a proposed cross-family design will work. [Design for repair in household appliances](https://doi.org/10.1016/j.procir.2024.12.004) · [Consumer repair-barrier review](https://doi.org/10.54337/plate2025-10288).

## 2. Cross-appliance design patterns worth combining

### M1 — Service spine, not blanket modularity

Put the expected inspection/replaceable service points on a deliberately designed access path. Use captive or standardized fasteners where justified, visible component/connector identity, reversible panels and enough hand/tool clearance. Keep a permanent structural/sealing joint where it measurably outperforms a service joint. Benchmark access path and reassembly error, not the count of removable covers.

### M2 — Replace the smallest defensible repair unit

Offer component-level replacement where diagnostics, precision and part supply support it; a functional submodule where component repair is uneconomic or too error-prone; and a major assembly only when the evidence justifies that cost. Do not make a module so large that a cheap failure forces a large expensive replacement.

### M3 — Hazard islands with safe outer service boundaries

Separate user-accessible tasks, qualified internal service, high-current/heat/rotating assemblies, wet interfaces, and sealed refrigerant circuits. External service interfaces can be standardized without opening hazardous boundaries. Refrigerant handling, high-voltage work, stored energy, pressure and heavy assemblies remain governed by product-specific qualifications and procedures.

### M4 — Separate fluid, air, thermal and control paths

Keep water/leak routes, air/dust/condensate paths, thermal-transfer interfaces and control wiring legible and independently inspectable. Cross-contamination or one inaccessible obstruction should not require unnecessary teardown of unrelated subsystems. Validate seal life, condensation, hygiene, corrosion, noise and tolerance stack-up.

### M5 — Diagnosis before parts swapping

Use a versioned diagnostic graph with competing hypotheses, safe discriminating checks, calibrated measurement identities, explicit unknown/contradictory states, and a repair receipt. A code is a symptom report, not a confirmed cause. Support offline service documents and stable, exact-model part identities.

### M6 — Performance-aware control instead of blind eco modes

Measure successful useful output. For washer/dishwasher: cleaning/rinsing and drying; refrigeration: temperature stability and energy under declared ambient/load; dryer: final moisture and energy; water heater: delivered hot-water quantity/temperature and recovery; HVAC: comfort delivery, seasonal energy and humidity/airflow where applicable; cooker: temperature uniformity and cooking outcome. Resource gains are invalid if useful function or safety falls below frozen limits.

### M7 — Retrofit and graceful aging

Design the durable pressure vessel/tub/cabinet/thermal shell to outlast the most replaceable controls, wear parts, fans, pumps and sensors where lifecycle evidence supports that split. Publish compatibility/supersession rules and safe lifecycle support. Avoid a cloud account or service endpoint becoming a single point of failure for basic appliance operation.

### M8 — Supply-chain resilience as an engineering dimension

A part is not serviceable merely because a web listing exists. Track exact-model/revision fit, dated quote/order, supplier independence, delivery, return handling, obsolete stock, lawful documentation access and total restoration cost. Keep source alternatives bounded by required ratings and safety qualification.

### M9 — Design for recovery without sacrificing service life

Mark materials and fasteners, limit unnecessary mixed-material bonding, identify refurbishable modules, separate batteries/electronics where present, and record the real mass of replaced and recovered material. Compare repair, refurbishment, remanufacture and replacement on a declared service profile.

### M10 — Common contracts, not universal hardware

Standardize machine-readable interface documentation, evidence records, fault vocabularies, verification receipts and service tiers across the portfolio. Use families of compatible mechanical interfaces and electrical ratings only after fit, thermal, leak, EMC, vibration, life and misuse testing. Commonality itself can create correlated failures and supply single points of failure, so measure that risk.

## 3. Expanded major-appliance scope

The design set now includes nine product families: six original priority families plus washer-dryer combos, cooking appliances and residential ventilation/heat-recovery equipment. Product-family boundaries matter because the fluid, thermal, mechanical, hazard and functional constraints differ.

### F1 — Washing machine

**Primary candidate:** accessible conventional architecture plus a targeted service spine. Keep water-affected modules and controls legible; test repairable bearing/tub as a separate, higher-risk branch rather than forcing it into the first prototype. Compare belt/direct-drive only on matched useful function, durability, noise, energy and repair results.

**High-value hybrid:** service spine + diagnostic graph + replaceable pump/valve/lock/wear parts, with bearing/tub modularity tested only if it survives seal, fatigue, vibration and lifecycle gates.

### F2 — Dishwasher

**Primary candidate:** front-access wet-service zone for filter/sump/drain pump and intended wear parts, removable toe-kick, clear leak path/containment, separate accessible door/spray-arm/filtration tasks, and qualified electrical/heater boundary. Keep the hydraulic path and control wiring mapped independently.

**High-value hybrid:** replaceable wet-process service submodule plus a separate electronics/actuation plane. Do not bundle every wet and electrical component into a single costly cassette if the common failures are narrower. Benchmark washing/drying performance and leakage after service.

EU dishwasher rules explicitly consider cleaning/drying function, energy/water, spare parts, repair information and material recovery. Use applicable current standards and jurisdiction-specific requirements; EU obligations do not automatically govern a South African design. [European Commission dishwasher guidance](https://energy-efficient-products.ec.europa.eu/product-list/dishwashers_en).

### F3 — Refrigerator/freezer

**Primary candidate:** accessible air-side service cassette: fan(s), accessible dust/air path, defrost drain access, door gasket/hinge and replaceable control/sensors. Design a replaceable sealed refrigeration subsystem only for qualified service, and do not treat quick disconnects as automatically safe or reliable.

**High-value hybrid:** durable insulated cabinet + serviceable air/defrost/control modules + a qualified sealed thermal service boundary. Compare door opening, temperature stability, ambient extremes, condensation, insulation life and kWh per declared cooling service. Redesign the gasket/hinge as wear interfaces before considering a second refrigeration loop.

EU rules for household refrigeration address energy, lifespan, maintenance, reuse and repairability. [European Commission refrigerator/freezer guidance](https://energy-efficient-products.ec.europa.eu/product-list/fridges-and-freezers_en).

### F4 — Heat-pump tumble dryer

**Primary candidate:** owner-accessible lint path + maintainable air-side heat exchanger/condensate path + separate qualified refrigerant loop and power-electronics boundary. Avoid requiring a deep cabinet teardown for routine lint/condensate service; make hidden secondary lint fouling testable.

**High-value hybrid:** air-loop service cassette + detachable fan/condensate module + sealed heat-pump service unit. Optimize final moisture content and energy per kilogram of successfully dried laundry—not just compressor efficiency. Measure lint accumulation, cleaning time, airflow degradation, leaks, noise and recovery after service.

EU dryer rules have recently evolved: current European Commission guidance records 2025 amendments addressing repairability information and parts/repair-information availability. Re-check the exact currently applicable act before a market design freeze. [European Commission dryer guidance](https://energy-efficient-products.ec.europa.eu/product-list/tumble-dryers_en).

### F5 — Washer-dryer combo

**Primary candidate:** shared drum/platform only if it does not create a single complex assembly that makes both functions expensive to repair. Keep wash hydraulic components, air/condensate loop and heat-pump subsystem independently diagnosable and serviceable as feasible.

**High-value hybrid:** reuse the washer's service contracts and moisture/energy verification with the dryer subsystem, but benchmark combined-cycle tradeoffs against separate washer + dryer. Track throughput, cycle time, peak power, drying quality, maintenance access and the cost of one fault disabling both functions. Do not assume an all-in-one product wins on space alone.

### F6 — Heat-pump water heater

**Primary candidate:** durable tank/pressure/insulation shell + accessible air filter/fan/condensate service path + replaceable sensors/controls and accessible sacrificial anode where applicable; keep water-pressure and refrigerant boundaries engineered for their specific hazards.

**High-value hybrid:** heat-pump head with a qualified sealed thermal subsystem and serviceable hydronic/tank components; evaluate solar/PV surplus or grid-signal integration only as an optional control layer. Benchmark delivered hot-water volume/temperature, recovery, standby loss, noise, condensate handling, corrosion/anode maintenance and lifecycle cost. Additional valves, controllers or storage layers must justify their complexity.

EU product regulations cover water-heater efficiency and product information, but the rules and required evidence depend on the product definition and market. [European Commission water-heater guidance](https://energy-efficient-products.ec.europa.eu/product-list/water-heaters_en).

### F7 — Residential HVAC / air-source heat pump

**Primary candidate:** independently serviceable air-handling path (filter, fan, condensate/drain, sensors, controls) and outdoor airflow/coil access, with refrigerant, high-voltage and pressure tasks confined to qualified service. Include a documented interface between the indoor air handler and outdoor thermal/power modules without promising arbitrary cross-vendor interchangeability.

**High-value hybrid:** separate air-path maintenance from sealed thermal-service work; expose safe diagnostics, accessible coils/drains and replaceable fans/controls. Benchmark seasonal delivered heating/cooling, humidity/comfort, airflow, noise, frost/defrost behaviour, part delay and qualified restore time across declared climates/load profiles. Never select compressor topology by peak COP alone.

European Commission air-conditioner guidance provides seasonal efficiency and sound metrics (SEER/SCOP and related label values) for the products within its scope. [European Commission air-conditioner guidance](https://energy-efficient-products.ec.europa.eu/product-list/air-conditioners-and-comfort-fans_en).

### F8 — Oven, cooker, hob and cooking appliances

**Primary candidate:** separate accessible controls, sensors, fans, door/hinge/seal and serviceable heating-zone components from the structural, insulation and hot-surface safety envelope. Design the assembly so a control or hinge fault does not require replacing an expensive cavity or whole hob.

**High-value hybrid:** zone-level components and repairable door/seal interfaces + replaceable control module + accessible cooling airflow paths, while preserving thermal insulation, fire/electrical safety and calibrated temperature distribution. Induction coils/power stages, resistive elements, gas trains and sensor arrays are distinct technology branches; do not force one shared physical module. Benchmark temperature uniformity, preheat/recovery, cooking output, hot-surface/protection behaviour, energy and post-repair thermal/electrical checks.

The 2025 functional-module repairability study covering cooker hoods, hobs, ovens and related cooking products is a useful physical-disassembly methodology reference for this family, not a basis for assuming one interface fits every cooking appliance. [Study](https://doi.org/10.1016/j.procir.2024.12.004).

### F9 — Residential ventilation / heat-recovery ventilation

**Primary candidate:** accessible filter, fan/blower, condensate/drain and controls with a replaceable heat-recovery core that can be cleaned/inspected or changed without removing unrelated duct runs; provide clear sealing, airflow direction and sensor references.

**High-value hybrid:** common airflow-module service contracts with HVAC while retaining ventilation-specific core/bypass, frost protection, condensate and indoor-air-quality performance requirements. Benchmark delivered airflow/pressure, heat-recovery effectiveness under declared conditions, leakage/cross-contamination, sound, fan energy, filter pressure drop, cleaning time and post-service balance. A common fan interface is only a candidate until acoustic, electromagnetic, pressure and life constraints match.

## 4. Cross-family hybrid designs worth prototyping on paper first

| Hybrid | Combine | Why it may outperform naive modularity | Main failure/trade to test |
|---|---|---|---|
| H1 — Service contract platform | Stable identity, fault, parts-fit, safe-state and post-check records across all families | Reuses service knowledge without pretending physical parts are universal | Common schema must represent distinct hazards and unknown states correctly |
| H2 — Wet-process cassette family | Selected washer/dishwasher access, leak-containment and service procedures | Reuses service ergonomics and evidence practices around water pathways | Different temperatures, detergents, pressure, flow and sanitary constraints |
| H3 — Air-side service cassette family | Dryer, refrigerator air-side, HVAC and ventilation maintenance interfaces | Separates routine cleaning/fan/condensate work from sealed thermal service | Airflow, condensation, dust, noise, coil geometry and pressure drop are product-specific |
| H4 — Qualified thermal island | Heat-pump dryer, water heater and HVAC diagnostic/service boundary concepts | Keeps hazardous refrigerant-side work separate from owner/technician air- and water-side tasks | Refrigerant, pressure, charge, flammability, joints and lifecycle leakage demand individual validation |
| H5 — Durable shell, replaceable wear layer | Washer tub/support, dishwasher sump, fridge gasket, cooker door and appliance cabinet | Places service joints where failures justify them; keeps durable structures intact | Joint wear, seal compression, corrosion, thermal cycling, vibration and tolerance drift |
| H6 — Common evidence, specialized controllers | Versioned diagnostics and safe-state contracts; device-specific control/safety electronics | Improves serviceability and updates without forcing one overloaded board into every appliance | Correlated failure, cybersecurity, EMC, configuration drift and loss of local operation |
| H7 — Laundry ecosystem optimization | Washer extraction/spin and dryer moisture/heat-pump cycle coordination | A dryer receives a better-defined moisture load; whole-laundry energy and throughput may improve | Extra sensing/control complexity, compatibility and combined-cycle tradeoffs |
| H8 — Thermal resource orchestration | Heat-pump water heater, HVAC and optional PV/thermal storage coordination | Can shift useful thermal loads and reduce resistive backup use in suitable contexts | Tariffs/climate vary; added tanks/valves/control failures may outweigh savings |
| H9 — Zone-level cooking repair | Replaceable cooktop/heater/induction power and oven door/control modules | Avoids discarding the whole appliance for one localized failure | Heat, fire, high-power electronics, temperature calibration and inter-zone coupling |
| H10 — Common fastener and access logic | Shared tools, fastening philosophy, diagrams and access conventions across families | Lowers technician training friction and helps independent repair networks | Do not sacrifice appliance-specific seals, ingress protection, torque, thermal or structural needs |

These hybrids are **architecture candidates**, not physical compatibility claims. Reuse service concepts and interface contracts first; share actual components only where ratings, environment, lifetime, safety and qualification evidence support it.

## 5. Morphed design matrix: where to spend the next experiment

Do not prototype every permutation. Use a staged morphology strategy:

1. **Low-cost desk/teardown stage:** map existing units and parts; score access path, modules, seals, safety boundary, likely replacement scope and evidence gaps. No performance claims.
2. **Control-only pilot:** validate instruments, service task scripts, repeatability and uncertainty with a frozen unit/model/revision.
3. **Single-factor experiments:** measure one consequential change (for example accessible air-side module or service spine) before stacking many changes together.
4. **Hybrid experiment:** combine only factors that pass individually and have a credible interaction hypothesis.
5. **Endurance and independent replication:** test new interfaces for leaks, looseness, corrosion, vibration, fouling, calibration drift, rework and recurrence.
6. **Portfolio transfer test:** try the method—not unvalidated hardware—on another family to see which service contracts generalize.

Keep a control baseline. Freeze test profiles, criteria, instruments, exclusions and methods before comparative measurements. Unknown remains unknown; negative controls remain in the corpus.

## 6. Benchmarks across the entire portfolio

Every family needs both common metrics and product-specific output metrics.

**Shared metrics:** median/P90 elapsed restoration time; hands-on time; diagnostic accuracy and safe abstention; first-visit verified restoration; repeat visits/recurrence; exact-fit part fulfilment and delivery; all-in lifetime cost per useful service; discarded/recovered mass; user-task success; documentation/tool availability; and failures introduced by new connectors, seals or modules.

**Product-specific denominators:**
- Washer: kWh, water and cleaning/rinsing/spin results per successful load.
- Dishwasher: kWh/water per load meeting cleaning and drying floors, leaks and programme time.
- Fridge/freezer: kWh per declared cooled-volume service, temperature stability, ambient/climate robustness and door-seal performance.
- Dryer: kWh/kg water removed, final moisture distribution, cycle time, lint/airflow degradation and condensate performance.
- Washer-dryer combo: successful wash and dry output per combined cycle and per elapsed machine-hour; disclose resource coupling.
- Heat-pump water heater: delivered litres at declared temperature, recovery time, standing loss and energy under declared inlet/ambient conditions.
- HVAC: seasonal heating/cooling delivered, energy, comfort/humidity, sound and defrost/part-load behaviour in declared climate bins.
- Cooker/oven/hob: cooking output/temperature distribution, warm-up/recovery, energy, seal/door function and thermal/electrical safety after service.
- Ventilation/HRV: delivered airflow, pressure drop, heat-recovery effectiveness, leakage, fan energy, sound and maintenance-induced performance drift.

### Proposed targets to validate, not assume achieved

- At least 25% lower median and P90 full restoration time versus a frozen baseline on a preregistered task mix.
- At least 90% completion of approved household-maintenance tasks in a representative usability study, with safe stop/referral included as a successful outcome when appropriate.
- Two independent, verified part-supply paths for high-consequence/common repairs where possible, with local exceptions shown.
- Zero unsafe instructions in the finite adversarial diagnostic corpus; zero critical safety failures in accepted candidate tests.
- 100% of claimed successful repair trials with required task-specific function and safety receipts.
- No energy/water/performance advantage claim if core useful-output gates fail.
- Robust total-lifecycle advantage (including module overhead, parts logistics, labour, failed attempts, downtime, rework and end-of-life) rather than an isolated repair-time win.
- Independent replication for any generalized architecture superiority claim; initial small control pilots establish repeatability, not population-wide reliability.

## 7. What not to standardize prematurely

- Do not invent a universal pump, heater, fan, compressor, control board or refrigerant connector.
- Do not make every module tool-less: vibration, fire, pressure, water and ingress constraints may require specific fasteners or torque.
- Do not add electronics/telemetry to products that can solve the problem with a durable mechanical interface.
- Do not let a single modular cassette create a large expensive single point of failure.
- Do not assume a common part source is resilient if all sellers depend on the same upstream supplier.
- Do not average all stakeholders or fault classes into one number; reveal the separate frontier and where it remains unresolved.

## 8. Research evidence and regulatory context

- [Design for repair: insights to develop functional modules in household appliances (2025)](https://doi.org/10.1016/j.procir.2024.12.004): physical disassembly and module analysis across cooker hoods, hobs, ovens and related cooking products.
- [Consumer repair-barrier scoping review (PLATE 2025)](https://doi.org/10.54337/plate2025-10288): product and service-ecosystem barriers including complexity, access, safety, obsolescence, time, cost and documentation.
- [EU dishwasher rules](https://energy-efficient-products.ec.europa.eu/product-list/dishwashers_en): function, efficiency, spare parts and repair information within scope.
- [EU refrigerators/freezers rules](https://energy-efficient-products.ec.europa.eu/product-list/fridges-and-freezers_en): efficiency and resource/reparability requirements within scope.
- [EU dryer rules](https://energy-efficient-products.ec.europa.eu/product-list/tumble-dryers_en): current ecodesign/label framework and 2025 amendments noted in official guidance.
- [EU air-conditioner metrics](https://energy-efficient-products.ec.europa.eu/product-list/air-conditioners-and-comfort-fans_en): seasonal efficiency and noise metrics within scope.
- [EU water-heater framework](https://energy-efficient-products.ec.europa.eu/product-list/water-heaters_en): efficiency and product information within scope.

These EU sources help define benchmark dimensions and show that product families have distinct functional/regulatory envelopes. They are not a claim that EU requirements govern South African products. Confirm current market-specific standards and safety requirements before any product decision.

## 9. Claim ceiling and next decision

This portfolio proposes a broad design morphology and hybrid shortlist. It does not prove a design works, that physical parts are interchangeable, or that any option is cheaper/safer/more efficient.

The next best step is not to CAD all nine families. Start with two contrasting, learnable pilots:
1. **Washer service spine + small justified repair units**, because the existing task, sourcing and decision protocols are already underway.
2. **Heat-pump dryer air-side service cassette**, because routine air/condensate maintenance and the sealed thermal boundary create a useful test of whether our service-spine principles transfer without conflating refrigerant work with user maintenance.

Run desk disassembly and parts-path comparisons first, then pick the first physical control based on access/cost/permission. The goal is to validate reusable design rules and evidence contracts on a second family before scaling into dishwasher, refrigeration, water heating, HVAC, cooking and ventilation.
