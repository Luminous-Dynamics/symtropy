# Large-Appliance Repairable Architecture Program v0.1

**Status:** portfolio architecture proposal; no large-appliance prototype, CAD freeze, life-cycle cost proof, performance improvement, or product-safety qualification is claimed.  
**Tracker:** [APPLIANCE-PORTFOLIO-001 #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825)  
**Parent substrate:** [APPLIANCE-QUAL-001 #1805](https://github.com/Luminous-Dynamics/symtropy/issues/1805) and [draft PR #1806](https://github.com/Luminous-Dynamics/symtropy/pull/1806)  
**Evidence bridge:** [Mycelix APPLIANCE-EVID-001 #5156](https://github.com/Luminous-Dynamics/mycelix/issues/5156)  
**Diagnostics owner:** shared finite-hypothesis EIG/EVSI kernel [Symthaea PR #7695](https://github.com/Luminous-Dynamics/symthaea/pull/7695); appliance adapter research [PR #7843](https://github.com/Luminous-Dynamics/symthaea/pull/7843)

## 1. Mission: lower the cost of a decade of useful service

Do not optimize a large appliance for the retail checkout price alone. Optimize the **cost of delivering its useful function over its service life**, subject to safety, durability, performance and resource-use constraints.

A low initial BOM can be a false economy if a failed €20-equivalent pump requires discarding a welded tub, if the repair manual is hidden behind an account, if one unavailable control board strands the product, or if an otherwise efficient design is expensive to diagnose and restore. Conversely, modularity can also be wasteful: extra connectors, seals, brackets, housings, inventory and parasitic power may make a product worse. A module earns its boundary by showing a measurable service, resource or lifecycle advantage against a fair control.

Evaluate total cost of sustained capability:

- acquisition and installation;
- energy, water, consumables and cleaning products;
- routine maintenance;
- diagnosis, parts, shipping and labour for successful **and failed** repair attempts;
- waiting/transport, outage or spoilage, repeat visits and rework;
- material discarded for a repair, end-of-life disassembly and recovery;
- uncertainty, local utility prices, service coverage and likely use profile.

Keep measured costs separate from modeled costs. Never coerce unknown values to zero. Report both owner out-of-pocket cost and wider resource/lifecycle cost when evidence permits.

## 2. Paradigm shifts

### 2.1 Replaceable appliance parts -> function-oriented service architecture

Partition the product around the smallest units that can be independently diagnosed, safely isolated, removed, installed, verified and economically sourced. A part is not a service module merely because it has a connector. A replacement unit should have a published mechanical/electrical/fluid/thermal interface, configuration identity and post-repair test.

### 2.2 “One universal module” -> a common *interface contract*, not identical hardware everywhere

A washer pump, refrigerator fan and HVAC blower have different fluid, temperature, contamination, sound, duty-cycle, fire and safety requirements. Do not force them into one physical module. Reuse definitions for identity, connector documentation, limits, diagnostic evidence, compatibility, service instructions and qualification receipts. Share a physical module only when fit, ratings, performance, replacement availability and lifecycle advantage have been demonstrated for both hosts.

### 2.3 Black-box appliance -> observable appliance with independent safe controls

Expose non-invasive, well-documented measurements and a locally usable service mode: what was commanded, what was actually measured, what is stale or missing, and which procedure is allowed next. An AI may rank hypotheses or propose a test, but must not control a safety interlock, defeat protection, energize open machinery or certify the appliance safe. Essential functions and service documentation must not depend on a vendor cloud or login.

### 2.4 “DIY repair” -> clearly tiered service boundaries

Design normal user maintenance to be safe, obvious and tool-light where the task permits. Design deeper modules for qualified repair with clear isolation, lockout/discharge, water shutoff/drain, residual-pressure and post-installation verification. Easy access is not authorization to perform hazardous work.

### 2.5 Fault code -> verified restoration and follow-up

Keep distinct: symptom; diagnostic hypothesis; observed cause; work performed; part planned; part actually used; post-repair functional result; residual limitation; recurrence during a named observation window. A completed work order, clear fault code or powered-on display is not proof of restored capability.

### 2.6 Energy label -> delivered useful service per resource

Compare at matched useful output. Washer: cleaning/rinsing/spin-drying for a declared load per kWh and litre. Dishwasher: correct cleaning/drying per place setting per kWh and litre. Refrigerator: stable safe temperature in usable volume per kWh. Dryer: moisture removed to a declared endpoint per kWh. Heat-pump water heater: delivered hot-water heat under a declared draw profile per kWh. HVAC: delivered heating/cooling and comfort within a declared envelope per energy input. Keep performance, efficiency, duration and noise as separate outcomes so a single weighted score cannot hide harm.

## 3. The proposed architecture: service spine + hazard islands

This is a **design hypothesis to test**, not a certified architecture.

### Common service spine

- Stable design, BOM, unit, part, firmware/configuration, procedure and test-profile identities.
- Offline manual, wiring/hydraulic diagrams where appropriate, diagnostic tree, exact connector/pinout details, tool list, fastener list, fault limitations and post-repair checklist.
- Keyed and labelled connectors with published voltage/current/temperature/fluid/material/rating limits; no pinout guessing by wire colour.
- Reversible access panels and fasteners for common service tasks; avoid adhesives in routine repair paths unless function/safety demands them.
- Separate parts and consumables from labour, freight, travel, diagnosis and downtime in cost records.
- Repair logs preserve failed attempts, discarded/returned parts, unknowns and superseding corrections.
- No pairing lock or internet account as a condition of using a legally and technically compatible replacement part, subject to applicable safety and legal requirements.
- Common service-document structure and evidence contract, not a universal electrical voltage, fluid connector, refrigerant fitting or arbitrary module size.

### Hazard islands and hard boundaries

| Boundary | Design direction | Hard limit |
|---|---|---|
| Normal-user maintenance | Accessible filters, lint screens, accessible cleaning points, door seals, dispensers and documented external hoses where intended | Only operations in the manufacturer-reviewed user procedure; disconnect, cool, shut off or drain as instructed |
| Water/wet internals | Service access, leak capture/detection, replaceable pumps/valves/sensors, test points and leak checks | Mains plus water requires proper isolation and post-repair leak/electrical verification |
| Mains/high-current electronics | Segregated, labelled and replaceable board/drive/heater assemblies with defined service procedure | Not assumed DIY-safe; qualified isolation, stored-energy discharge and protective-earth/insulation checks as required |
| Rotating/heavy machinery | Lift/access clearances, documented support/lifting points, replaceable wear items, guards and controlled spin tests | No exposed rotor/drum powered testing; pinch, crush, imbalance and tip hazards remain |
| High temperature/pressure | Replaceable sensors, valves and accessible maintenance items; drain/cool/pressure-relief process | Scalding, steam, pressure vessel, combustion and over-temperature work requires product-specific qualified procedure |
| Refrigerant circuit | Make sensors, fans, controls, coils access and leak diagnostics serviceable; expose model and refrigerant identity | No amateur venting, charging, brazing, recovery or opening sealed circuits; refrigerant work follows applicable law, equipment procedure and qualified technician requirements |

## 4. Portfolio: architecture hypotheses by appliance family

### A. Front-load washing machine / washer-dryer — recommended first large-appliance study

**Why first:** high household consequence when unavailable; repeated, measurable cycles; mix of mechanical, water and electrical faults; no sealed refrigerant circuit in a plain washer. It is not low-risk: mains voltage, water, hot surfaces, heavy rotating parts, imbalance and flooding still require a controlled lab and qualified safety review.

**Architecture candidates to compare, not assume:**
1. Conventional appliance with better access/manuals as the control.
2. Service-first front/side service spine, with separately replaceable inlet valve, drain pump/filter, dispenser, door lock, sensor/harness, heater and controller/drive assemblies.
3. Drum/tub durability variants: welded tub with whole tub replacement vs bolted/serviceable tub and separately replaceable bearings/seals. Compare manufacturing cost, water integrity, bearing access, vibration/noise, service time, waste and repeat-failure risk before choosing.
4. Drive choices (belt/direct drive or other candidate) compared at matched wash/spin outcomes, not by aesthetic or claimed part-count superiority.

**DIY-intended tasks to validate:** lint/coin filter and drain clean-out, dispenser cleaning, door-seal inspection/cleaning, accessible inlet-filter cleaning, exterior hose inspection and safe user troubleshooting.

**Qualified service boundary:** mains internals, heater/drive work, heavy drum/tub disassembly, suspension replacement, high-voltage capacitor discharge, protective-earth/insulation checks and powered spin/imbalance validation.

**Core benchmarks:** cleaning/rinsing performance, water per successfully cleaned kg, energy per declared load, cycle duration, spin-drying performance, leakage, vibration/noise, task time and success, parts cost/delivery, failed-repair rate, bearing/tub repair economics and recurrence. Do not report energy improvements unless cleaning and rinse outcomes meet the same predefined floor.

### B. Dishwasher

**Architecture hypotheses:** front-access filter/sump; independently replaceable circulation and drain pump where economical; serviceable spray arms, seals, racks, valves, sensors, heater and controller; leak tray and accessible leak indicators; wiring/hose routes that can be inspected without destructive panel removal. Compare a pump/sump cartridge against independently replaced pumps: the cartridge may simplify service but can replace a larger and more expensive assembly than necessary.

**User tasks:** filter and spray-arm cleaning, salt/rinse aid where supported, door-seal maintenance and documented non-invasive checks.

**Qualified tasks:** mains heater, internal pump/valve/harness, leakage isolation and electrical tests.

**Core benchmarks:** correctly cleaned place settings per kWh/litre, drying outcome, eco-program duration, leaks, noise, access/task time, parts availability, total restoration cost and recurrence.

### C. Refrigerator/freezer

**Architecture hypotheses:** separately serviceable door gaskets, handles/hinges, shelves/bins, accessible condenser airflow path, evaporator/condenser fans where designed, defrost system, drain, sensors, lighting and control/inverter board. Compare replaceable control assemblies to component-level board repair, including cost, diagnostic quality and electronic waste.

**Key boundary:** keep the sealed refrigerant circuit closed for users. The design must not make compressors, refrigerant charge or brazed pipework appear to be casual DIY tasks. Explore sealed-circuit module/cassette replacement only as a professionally qualified, refrigerant-specific option after leak, containment, efficiency, transport and lifecycle analysis; do not assume quick-disconnect fittings are safe or appropriate.

**Core benchmarks:** kWh/year per usable litre at a declared ambient and door-use profile; compartment temperature stability and recovery; food-safe temperature boundary; gasket replacement time; defrost/drain reliability; noise; service cost; fault diagnosis/abstention; verified temperature recovery after repair. Spoilage risk and temperature excursions are critical outcomes, not averaged away.

### D. Heat-pump tumble dryer

**Architecture hypotheses:** easy lint/filter access; condensate container/drain/pump maintenance; serviceable blower, drum drive, door seal, sensors, controller and fan; cleaning pathways to heat exchangers consistent with the validated design. Test a replaceable heat-pump assembly against separately serviced components, including refrigerant handling and replacement cost.

**Qualified boundary:** mains/heater, thermal protection, interlocks and sealed refrigerant circuit.

**Core benchmarks:** kWh per kilogram of water removed to a specified endpoint, residual moisture, cycle duration, condensation performance, lint/airflow restriction detection, acoustic noise, filter-maintenance effort and recurrence. A fast cycle is not superior if it consumes more energy or damages fabric.

### E. Heat-pump water heater

**Architecture hypotheses:** independently identifiable heat-pump/control/fan module and water tank only if the hydraulic, thermal, corrosion, acoustic, electrical and refrigerant interfaces are defensible. Compare an integrated system against a factory-sealed heat-pump top module and a separately replaceable tank; a module split can improve serviceability but can also add standby losses, seals, cost, complexity and refrigerant-service overhead.

Make routine inspection access clear for filters, condensate path, anode where fitted, drain and manufacturer-specified user maintenance. Design qualified-service access for controls, sensors, valves, elements and heat-pump hardware.

**Qualified boundary:** mains, stored hot water, pressure relief, tank/drain plumbing and refrigerant circuit. A pressure-relief device is never bypassed; no pressure vessel is opened while pressurized/hot.

**Core benchmarks:** delivered hot-water energy versus electricity, COP under declared inlet/ambient/draw conditions, first-hour delivery/recovery, standby loss, water-temperature stability, scale/corrosion maintenance, condensate drainage, leakage and maintenance cost.

### F. Residential air-source heat-pump HVAC

**Architecture hypotheses:** clear separation and identity for indoor blower/filter, outdoor fan, controls/inverter, sensors, condensate handling and replaceable subassemblies; diagnostic service ports and fault trees; accessible coil cleaning. Design so routine filter/coil maintenance does not require live access. Evaluate board-level repair, module swap and whole-unit replacement against actual repair economics.

**Qualified boundary:** refrigerant circuit, high voltage/inverter, large capacitors, pressure, outdoor lifting, electrical installation and commissioning.

**Core benchmarks:** seasonal heating/cooling performance and energy under declared climate/load conditions, delivered capacity, comfort stability, noise, defrost performance, service time, refrigerant leak/containment checks, parts lead time, and cost/time-to-restore. Do not compare different climates, sizing or installation quality as though the equipment alone caused the result.

## 5. Portfolio selection and staging

Scores below are first-pass engineering prioritization, not measured market rankings. The next phase should collect local repair invoices, public parts prices, service manuals, representative retail controls and repair-shop interviews before freezing priorities.

| Family | Why pursue it | Principal complexity | Role |
|---|---|---|---|
| Washing machine | Repeated resource-intensive cycles; measurable output; serviceable wear items | Water + mains + heavy rotating assembly | **First large-appliance architecture and benchmark target** |
| Dishwasher | Similar serviceability/re-source problem; measurable cleaning function | Water + mains + leak containment | Second; reuses some wet-system/task evidence |
| Refrigerator/freezer | Continuous energy use; food-preservation consequence; long-lived sealed cooling system | Refrigerant, thermal integrity, food safety | Parallel requirements/parts/reliability research; sealed-circuit work qualified |
| Heat-pump dryer | High energy and moisture-management complexity | Refrigerant + hot/wet airflow + lint/fire risk | After dryer test protocol and service boundaries are clear |
| Heat-pump water heater | Large hot-water/energy benefit potential | Pressure, scalding, corrosion, mains, refrigerant | Separate qualified engineering workstream |
| Heat-pump HVAC | High lifetime energy/downtime impact | High voltage, refrigerant, installation/climate dependence | Highest integration/commissioning complexity; later physical work |

**Recommended cadence:** washer design/economic trade study first; dishwasher next using common water/leak evidence patterns; refrigeration runs as a parallel desk/parts/repairability study because its continuous-use and food-safety profile differs; dryer and heat-pump-water-heater modules proceed only with their own thermal/refrigerant/pressure safety cases; HVAC physical design comes after the measurement and qualified-service partnerships exist.

## 6. Benchmark protocol: no claims before a fair control

### 6.1 Controls and preregistration

For a selected model/use class, purchase or otherwise lawfully obtain at least three retail control units where practical; record exact make/model/revision, market, age/condition, price/date, usage assumptions, parts sourcing, repair history and missing data. Three units are a pilot floor to estimate variation, not a large enough sample for strong population claims. Add more units as risk and observed spread require.

Freeze before testing: workloads/cycles, soil/load/draw profile, ambient conditions, instrument identity/calibration, repeats, baseline inclusion/exclusion, repair-task script, stopping rules, analysis, missing-data treatment, target margin and whether an assessor is blinded to design. Keep raw observations and exact protocol/input digests.

### 6.2 Measure multiple dimensions separately

- **Functional output:** cleaning/rinsing/drying, temperature stability, moisture removal, hot water delivery, or HVAC delivered capacity/comfort, as applicable.
- **Resource use:** electricity, water, detergent/consumables, standby and losses using a fixed profile.
- **Serviceability:** time to access, diagnose, remove, replace, reassemble, leak/electrical checks, restore and clean up; tools, steps, destructive actions and damaged fasteners.
- **Repair economics:** parts, shipping, labour, diagnosis, second visit, waiting time, failed attempts, waste and downtime/outage.
- **Reliability:** failure/degradation mode, recurrence time, residual limitation, repeat repair and component-level survival under declared test conditions.
- **Sourcing:** published compatible part ID/revision, price/date, supplier count, stock status, delivery distribution, import/tax/shipping and approved substitute evidence.
- **Safety/conformity:** hazard analysis, applicable product standards, qualified inspection/test reports and separate release decision. This is a distinct gate—not a weighted score that can be offset by lower cost or energy.
- **Lifecycle:** sensitivity analysis for use intensity, energy/water tariffs, service cost, parts supply, lifespan and discount rate; report best/base/worst cases rather than one false-precision number.

### 6.3 Proposed targets to challenge after control data

These are program-level aspirations for preregistration, not results:

- Reduce median and P90 **total restore time** by at least 25% for a defined common-fault set versus the frozen retail controls, without increasing critical safety faults, repeat failures, or resource use beyond the declared noninferiority margin.
- Complete at least 90% of separately approved user-maintenance tasks successfully in a representative usability test, without tools or safety interventions beyond that task's permitted procedure. Do not include internal electrical/refrigerant/pressure repairs in the DIY denominator.
- Achieve at least 90% repair success at first visit for the in-scope qualified repair task set; include failed attempts and report per fault class, not a blended pass rate alone.
- Demonstrate a reduction in lifetime expected cost of delivered function after including the repair module's BOM/interface cost, inventories, consumables, energy/water, service and downtime; require positive advantage under reasonable sensitivity bounds, or report the trade-off as unresolved.
- Keep matched functional output non-inferior to controls using a preregistered uncertainty margin. Pursue resource-use improvement only after function and safety pass; a 5–10% energy/water improvement is a hypothesis to evaluate, not a promise.
- Provide publicly accessible manuals, BOM/compatible-parts matrix, tool list and replaceable-part identities for the chosen open design; target a 10-year parts-support commitment as a design goal, subject to feasibility and market law.
- Achieve zero unsafe test/repair recommendations in the finite adversarial test suite; critical faults require conservative abstention/referral and are reported individually.
- Reproduce key results on three candidate units and, for stronger claims, at least one independent builder/test team; one author-built sample cannot establish reproducibility.

Do not declare a 10-year service life from a short accelerated test. Accelerated tests require a validated physical failure mechanism and cannot silently substitute for real field follow-up.

## 7. Symtropy / Symthaea / Mycelix boundaries

- **Symtropy** owns the architecture variants, design/task graphs, cost/performance trade-offs and preregistered benchmark protocol.
- **Symthaea** provides optional local advisory fault hypotheses and candidate-measurement rankings via the existing finite-hypothesis kernel and evidence-chained posterior state. Do not create a second EIG/EVSI implementation.
- **Mycelix** binds source revision, BOM/substitute qualification, exact built unit, work-order/actual material events, post-repair proof and bounded recurrence through existing Manufacturing/RepairProductiveLoop and release/provenance owners.
- **Physical product** retains deterministic local controls and independent protection. A synthetic benchmark or signed record cannot certify a machine as safe.

No new generic repair ledger, sensor ontology, artifact store, firmware authority, safety system or universal optimizer is introduced by this profile.

## 8. Regulatory and reference research

Regulation varies by target market, model, intended use and current text. Treat the following as a requirements-discovery starting point, not legal advice or proof of conformity:

- European Commission, [washing machines and washer-dryers](https://energy-efficient-products.ec.europa.eu/product-list/washing-machines_en): Regulation (EU) 2019/2023 covers resource use and specified spare-parts access; the Commission summary describes a 10-year availability period for listed parts and repair information for professional repairers.
- European Commission, [dishwashers](https://energy-efficient-products.ec.europa.eu/product-list/dishwashers_en): Regulation (EU) 2019/2022 includes specified spare-parts availability and delivery expectations.
- European Commission, [refrigerators/freezers](https://energy-efficient-products.ec.europa.eu/product-list/fridges-and-freezers_en): Regulation (EU) 2019/2019 contains ecodesign provisions including reparability/recyclability; the framework also exposes energy, volume and noise information.
- European Commission, [tumble dryers](https://energy-efficient-products.ec.europa.eu/product-list/tumble-dryers_en): Regulation (EU) 2023/2533 applies from 1 July 2025 and includes product-specific efficiency/resource requirements. Check the current consolidated regulation and amendments before freezing a design.
- EU, [Directive (EU) 2024/1799, consolidated text dated 31 July 2026](https://eur-lex.europa.eu/eli/dir/2024/1799/2026-07-31): the repair obligation is scoped via Annex II and the applicable Union legal acts; national transposition, product scope and exceptions require confirmation. Do not assume all appliances or all repairs share one legal duty.
- Open Arsenal Specs, [Universal Appliance Standard concept](https://github.com/openarsenalspecs/Civic-Tech/blob/main/Universal%20Appliance%20Standard.md): examine the existing broad interoperability ambition and coordinate rather than proposing a competing universal product standard.
- Symtropy [#1805 / PR #1806](https://github.com/Luminous-Dynamics/symtropy/pull/1806), Symthaea [#7695 and #7843](https://github.com/Luminous-Dynamics/symthaea/pull/7843), Mycelix [#5156](https://github.com/Luminous-Dynamics/mycelix/issues/5156): internal architecture and evidence owners.

## 9. Decisions still open

- Initial geography/market, installation type and regulatory route for the first washer.
- Front-load vs top-load reference, capacity, target load profile and baseline controls.
- Drum/tub/bearing/drive architecture chosen by economic and physical evidence, not ideology.
- Tool-light/user task boundary and the final hazard review.
- Whether a shared low-voltage control/diagnostic interface is beneficial once isolation, EMC, electrical safety, maintainability and cost are measured.
- Local energy/water tariffs, repair labour, parts pricing, freight and availability data.
- Test-lab capability and independent repair/builder partners.
- Open-source licence and the terms of all third-party components, firmware and referenced standards.

Unknown stays unknown until evidence closes it.

## 10. Claim ceiling

This program currently defines a portfolio, architecture hypotheses and benchmark protocol. It does not claim that the designs are finished, that a large appliance has been built, that repairs are cheaper, that energy/water use is lower, that a product lasts longer, or that any design is safe or conforming. Those are later claims requiring the exact article, test protocol, physical evidence, and independent review.
