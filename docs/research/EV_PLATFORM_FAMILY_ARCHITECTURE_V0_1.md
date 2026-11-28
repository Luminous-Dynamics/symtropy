# Modular EV family architecture v0.1 — passenger, cargo, utility and freight

**Programme:** Luminous Dynamics / Symthaea / Symtropy / Mycelix / Sol Atlas  
**Status:** concept architecture and benchmark targets; no validated vehicle specification, prototype result, homologation or production claim  
**Date:** 28 November 2026  
**Tracking issue:** #7888 — EV-PLATFORM-FAMILY-001

## 1. Executive design decision

Develop a **family of right-sized electric vehicles**, not one universal skateboard and not a portfolio of unrelated bespoke products.

The family should share the expensive, repeatable functions—battery subpack design, inverter/motor families, DC/DC, charging, thermal controls, low-voltage architecture, vehicle software, diagnostics, telematics, body-converter interfaces, manufacturing processes and evidence records—while keeping structural platforms distinct where gross vehicle mass, crash loads, axle loads, cargo geometry and duty cycle demand it.

First generation:

1. **MicroCargo** — a low-speed, four-wheel last-mile cargo pod/quadricycle, initially for campuses, depots, industrial sites and controlled urban routes pending local class/homologation determination.
2. **Light Commercial Platform (LCP)** — a 3.5 t GVW-class chassis family for cargo van, chassis cab, bakkie/utility and people-mover derivatives.
3. **Medium Commercial Platform (MCP)** — a separate 7.5–12 t GVW-class rigid-truck chassis for middle-mile logistics and municipal/industrial work.
4. **Passenger derivative** — a compact five-seat vehicle sharing selected LCP battery, drive, charging, thermal and electronics modules, but not forcing the cargo frame and passenger body to be identical.

Do not launch all models simultaneously. Start with a cargo van/chassis-cab demonstrator and a bakkie/utility mule; keep the medium truck at requirements and simulation stage until real routes, loads and depot constraints are admitted.

## 2. Why this architecture is better than a universal skateboard

The industry is moving toward modular commercial EV platforms. Kia's PV5 Chassis Cab uses its E-GMP.S PBV platform and converter ecosystem; its public 2025 release describes up to 1,005 kg unfinished payload and body options spanning dropside, tipper, box and refrigerated variants. MAXUS South Africa's eDeliver 3 is a local cargo-van comparator with 50.23 kWh, a stated 250 km WLTP combined range, 4.8 m³ cargo volume and 945 kg maximum payload. These are useful requirements anchors, not direct field-performance comparisons.

A universal platform becomes counterproductive if it forces a 3.5 t delivery van, a bakkie and a 12 t truck to share the same crash structure, axle geometry, suspension, pack placement and load paths. A common *functional architecture* is more valuable than identical castings.

The newest battery-integration literature argues that structural integration is a reassignment of mechanical, electrical, thermal and safety functions—not simply removal of modules and fasteners. It warns that deeper cell-to-body/chassis integration can reduce service granularity and complicate fault isolation, repair and material recovery. For early Luminous prototypes, the default should therefore be a bolted, replaceable battery enclosure with explicit load paths and repair access. Structural cell integration remains a later, evidence-gated option.

## 3. Proposed model portfolio

All ranges, energy use and payloads in this section are **design targets and sizing hypotheses**, not achieved or homologated specifications. Mass and energy models must be recalculated for every body conversion, tyre, axle ratio, payload, ambient condition and route.

| Model | Mission | First-pass architecture | Battery concept | Target duty performance |
|---|---|---|---|---|
| **MC-01 MicroCargo** | Parcel, food, campus and depot delivery | 4-wheel low-speed cargo pod; enclosed weatherproof cabin; modular rear cargo tub | 6–12 kWh LFP candidate, protected low-voltage or traction architecture selected after class analysis | 150–300 kg payload; 0.6–1.2 m³; 50–90 km controlled urban duty |
| **LC-01 UrbanCargo Van** | Dense city delivery and service calls | Compact LCP panel van, rear drive, low step-in cargo floor | 40–50 kWh LFP candidate; 400 V class | 500–750 kg finished-vehicle payload; 3.5–4.5 m³; 100–140 km loaded urban duty |
| **LC-02 Cargo Van / Chassis Cab** | Parcel, FMCG, trades, refrigeration, mobile workshop, ambulance and municipal conversion | LCP ladder/skateboard rails with bolt-on body interfaces; standard and long wheelbase | 55–75 kWh LFP candidate; 400 V class | 900–1,100 kg chassis/finished payload only where completed body and axle calculations permit; 4.8–7.0 m³ panel/box body; 160–200 km loaded route duty |
| **LC-03 Bakkie / Utility** | South African single/double-cab work, rural service, construction, agriculture, fleet operations | LCP high-clearance variant; rear drive baseline; optional front e-axle/4×4 only after cost and duty case | 70–90 kWh LFP candidate; 400 V class | 800–1,000 kg target payload only if GVM and axle margins pass; 170–210 km mixed loaded duty |
| **LC-04 PeopleMover** | 8–12 seat shuttle, employee transport and accessible transport | LCP long-wheelbase body with passenger restraint/floor/door structure | 75–100 kWh LFP candidate; 400 V class | 8–12 passengers; target 170–220 km urban duty with HVAC and full passenger load |
| **PC-01 Fleet Passenger** | Five-seat city/suburban fleet car and family vehicle | Passenger body using common selected battery, drive, charging, thermal and software modules; separate crash/body structure | 45–65 kWh LFP/LMFP candidate; chemistry decision by cost, mass and duty | 230–300 km mixed duty; final range requires matched cycle and independent meter |
| **MC-07.5 / MC-12** | 7.5–12 t rigid distribution, municipal and industrial work | Separate MCP 4×2 frame; modular axle, body rails, higher-voltage drive | 220–300 kWh LFP candidate; 700–800 V class subject to charging and component study | 120–180 km loaded local distribution target; payload is a separate mass-budget gate |

### 3.1 MicroCargo is a separate product, not a scaled-down van

A four-wheel low-speed carrier is a candidate for controlled routes, campuses, warehouses, municipal precincts, industrial estates and last-mile hubs. Its main economic lever is reduced vehicle mass, energy demand, parking footprint and loading cycle—not highway speed.

Do not assume a three-wheel layout: rollover stability, loaded centre of gravity, braking and lateral load transfer deserve priority over minimal wheel count. Public-road classification, occupant protection, lighting, braking, speed limits and operator rules must be settled before any road-use claim.

### 3.2 LC-02 is the first high-value platform

The cargo van/chassis cab should be the first serious engineering demonstrator because it tests the core modularity thesis:
- one rolling chassis and common cab interface;
- panel van, box, dropside, refrigerated and service-body candidates;
- repeated body datum rails, wiring and protected auxiliary power;
- exact finished-mass and axle-load accounting;
- route energy and depot-charging qualification;
- serviceable battery trays.

A chassis-cab payload is not the same as finished-body payload. The builder must subtract the actual body, refrigeration unit, liftgate, tool fit-out, passengers, accessories and all conversion mass from the verified allowable mass, and separately check each axle.

### 3.3 LC-03 bakkie is a strategically important South African derivative

The bakkie should not be treated as a van with a cut-off roof. It needs:
- a separate rear suspension and load-path study;
- payload under concentrated bed loads;
- towing and trailer-stability envelope;
- ground clearance and underbody protection;
- gravel-road vibration, dust and water-ingress validation;
- thermal performance under low-speed high-torque operation;
- optional front drive only if the real 4×4 use case justifies added mass, cost and control complexity.

Start with 4×2 rear drive; design hard points so a dual-motor derivative is possible, but do not burden the launch product with an unvalidated 4×4 system.

### 3.4 MCP truck is not an enlarged LCP

A 7.5–12 t truck has different axle, suspension, braking, thermal, charging and structural demands. Retain a separate chassis and e-axle system. Commonality should concentrate in electronics, controls, cell/subpack technology, diagnostics, telematics, connector philosophy, software and evidence tooling.

## 4. Battery and electrical architecture

### 4.1 Cell and subpack strategy

- LFP is the baseline candidate for high-cycle commercial duty where pack mass and volume permit. It is not inherently safe; abuse tolerance, venting, thermal propagation, electrical isolation and manufacturing quality remain mandatory.
- LMFP or nickel-rich alternatives may be studied for passenger and longer-range variants where energy density provides a measurable whole-vehicle benefit and cost, durability, supply and repair trade-offs are justified.
- Sodium-ion may be screened for low-speed MicroCargo or stationary-buffer use only when supplier qualification, cold/hot performance, cycle life, pack mass, power and cost evidence support it. It is not a default production chemistry.
- Evaluate cells on delivered duty-cycle energy, power, ageing, temperature, charge acceptance, safety and recoverability—not headline cell Wh/kg alone.

Proposed commonality target: one family of cell formats and pack-control interfaces, with serviceable subpack variants sized around roughly 20–25 kWh where a supplier and voltage architecture make that practical. This is a packaging hypothesis; detailed electrical series/parallel design, isolation, fault containment and current limits must determine the final unit size.

### 4.2 400 V light and 700–800 V medium architecture

Use a 400 V-class LCP architecture for the initial 3.5 t family unless component and charging studies show a superior alternative. A 700–800 V MCP architecture is worth investigating for higher power, lower current at a given power and faster charging, but it adds component, insulation, service and charging interoperability requirements.

The MCP voltage decision must be co-optimized with actual depot supply, connector availability, charger power, cable cooling, pack acceptance, duty dwell and utility constraints. Do not select 800 V solely as a premium feature.

### 4.3 Battery pack should be repairable before it is structural

First generation:
- bolted pack enclosures with explicit crash load paths;
- replaceable pack trays/subassemblies with controlled isolation;
- independently protected electrical zones and contactor/fuse coordination;
- defined vent path away from occupants and critical wiring;
- replaceable cooling plates, seals, sensors and service disconnects;
- accessible isolation verification and fault localization;
- pack-level digital configuration and service history;
- designed drainage, coolant recovery and material identification.

Second generation may evaluate cell-to-pack, cell-to-body or cell-to-chassis integration. Promotion requires evidence that every removed module/enclosure function has been reassigned and that thermal safety, crash, vibration, service, disassembly and recovery performance remain better at vehicle-life-cycle level.

### 4.4 Thermal architecture

- Separate battery, motor/inverter and cabin thermal-control requirements even where heat exchange is shared.
- Battery preconditioning should be duty- and charge-event-aware, not always-on.
- Protect hot-climate charge acceptance, parked-soak behaviour, low-speed high-load operation and repeated fast-charge cycles.
- Evaluate cooling-plate thermal uniformity, pressure drop, leak integrity, corrosion, fouling, joining, repair and end-of-life drainage.
- Use cabin heat pumps where their real climate and duty-cycle benefit justifies system cost and service complexity.
- Treat refrigeration bodies as an additional energy load, not an accessory omitted from range estimates.

## 5. Modular body and converter interface

Create a **Luminous Body Interface (LBI)** with a versioned configuration manifest.

Mechanical:
- repeatable chassis datums and mounting rails;
- controlled fastener classes and torque evidence;
- defined body centre-of-gravity envelope;
- verified front/rear axle and local frame loads;
- standard hard points for dropside, box, high-roof, reefer, tipper and service bodies.

Electrical:
- protected 12 V and 48 V supplies;
- isolated, interlocked high-voltage auxiliary interface only where required;
- protected body power distribution and emergency isolation;
- separate metering for traction, cabin HVAC and body auxiliaries;
- optional electric PTO power envelope with declared continuous/peak limits.

Data and control:
- CAN FD and automotive Ethernet where appropriate;
- authenticated body identity and signed conversion configuration;
- declared signal ownership, rates, failure states and safety responsibilities;
- no converter is allowed to silently override traction/braking or high-voltage safety logic;
- conversion-specific software and calibration are bound to exact vehicle identity and revision.

The converter manifest should record body make/model, serial identity, body mass, axle loads, centre-of-gravity estimate, wiring and software revisions, auxiliary loads, inspection evidence, test records and approval authority. Mycelix can preserve provenance and authorization; Luminous Platform can manage the configuration thread; Sol Atlas can bind vehicle geometry and configuration; Symtropy can execute the physical checks.

## 6. Drive, chassis and safety strategy

### Light platform
- Rear-drive e-axle baseline for utility/cargo traction and payload balance.
- Common motor/inverter family in power classes sized to van, bakkie and people-mover duties.
- Front suspension tuned for steering and load transfer; rear suspension selected by payload and conversion.
- Conventional, proven brake and steering systems for v1; regenerative blending must remain predictable on low-grip surfaces and under changing payload.
- ABS/ESC, hill-hold, trailer stability where fitted, axle-load and tyre-rating controls.
- Underbody battery protection for gravel, debris, water and repair access.

### Medium platform
- One 4×2 rigid configuration first.
- Central or e-axle drive selected on serviceability, ratio, cooling, mass, braking integration and supplier support.
- Frame rail and body mounting design for box, reefer, dropside, tipper and municipal body variants.
- Air or suitable mechanical suspension according to gross mass, route, maintenance and body.
- Regenerative brake blending qualified at full/empty load and with trailer where relevant.

Do not adopt fully integrated steer/brake/drive corner modules or full X-by-wire on the first prototype. They are a potential later modularity research lane but increase validation burden and service/supplier coupling.

## 7. Range and payload calculation protocol

For each concept, calculate usable trip energy from a declared battery window rather than nominal pack energy.

`E_trip = E_nominal × f_pack_usable × (SOC_departure − SOC_reserve)`

`R_est = 100 × E_trip / e_duty`

where `e_duty` is measured or modelled in kWh/100 km for the exact load/route/ambient case. The following examples are only early sizing checks using `f_pack_usable = 0.92` and a departure SOC of 90% to a 15% reserve:

| Model example | Nominal pack | Assumed duty use | Calculated energy window | Approximate distance before reserve |
|---|---:|---:|---:|---:|
| LC-01 UrbanCargo | 45 kWh | 24 kWh/100 km | 31.1 kWh | 130 km |
| LC-02 Cargo Van | 70 kWh | 27 kWh/100 km | 48.3 kWh | 179 km |
| LC-03 Bakkie | 85 kWh | 30 kWh/100 km | 58.7 kWh | 196 km |
| LC-04 PeopleMover | 90 kWh | 32 kWh/100 km | 62.1 kWh | 194 km |
| PC-01 Passenger | 65 kWh | 17.5 kWh/100 km | 44.9 kWh | 256 km |

These are not range predictions: the consumption inputs are assumptions. They exist to expose the relationship between pack size, operating reserve and actual mission efficiency. The medium-truck case should use kWh/km, full gross-mass and grade profiles, and measured auxiliary loads.

Payload budget:

`payload = certified_GVM − (vehicle_curb_mass + driver/passengers + options + conversion_body + accessories)`

Then verify per axle, tyre, wheel, frame, coupling, trailer and body-mount limits. A favourable total payload does not make an axle overload acceptable.

## 8. South African duty-cycle programme

Start with real route archetypes and customer interviews, not one generic WLTP proxy:

1. Urban parcel/FMCG multi-stop route with frequent doors and idle/auxiliary loads.
2. Retail replenishment and service route with mixed urban/highway driving.
3. Bakkie work duty: tools, towing, gravel, steep access and variable payload.
4. Municipal refuse/maintenance route with PTO, low-speed cycles and frequent stops.
5. 8–12 passenger shuttle/minibus duty with HVAC and passenger mass.
6. Regional middle-mile route with fixed depot, load, grade and departure window.
7. Cold-chain/refrigerated route with independently metered refrigeration load.

For every duty, retain trip/vehicle/pack/tyre/firmware identity, payload, elevation, weather, traffic, speed, SOC, battery temperatures, traction energy, regenerative energy, HVAC/PTO/refrigeration loads, charging energy, charger and meter identity, dwell, queue, and mission success. All time series require a common time basis and clock alignment evidence.

South Africa's October 2026 naamsa report lists 13,689 new light commercial vehicles and 770 medium commercial vehicles sold during the month. This provides category-market context only; it is not an EV demand estimate. Local competitor MAXUS eDeliver 3 provides a real product baseline. The business case still needs customer-specific daily distance, payload, financing, energy tariffs, maintenance and depot-capacity data.

## 9. Competitive benchmark scorecard

No single score should allow range, cost or performance to compensate for a safety, payload or reliability failure.

### Mission and efficiency
- kWh/100 km and kWh/100 payload-km;
- kWh/tonne-km for trucks;
- usable payload and cargo volume after conversion;
- mission completion and reserve distribution;
- worst-case underestimation, P95 energy error and uncertainty coverage;
- loaded and unloaded energy, temperature, grade, speed and HVAC strata.

### Chassis and vehicle dynamics
- GVM and axle-load margins;
- braking distance, thermal fade and regenerative blend stability;
- rollover margin / lateral stability under loaded and partially loaded states;
- grade start, gradeability and sustained climb thermal limits;
- turning circle, approach/departure angles and ground clearance;
- trailer stability and tow energy where fitted;
- vibration, fatigue, corrosion, ingress, dust and gravel durability.

### Battery and charging
- pack Wh/kg and Wh/L as secondary measures, with whole-vehicle payload consequence;
- usable energy window, power acceptance and thermal uniformity;
- charge time by SOC, temperature, pack age and grid power;
- 15-minute charging profile, peak demand, power factor and site capacity;
- pack fault isolation and safe de-energization time;
- repair time, subpack/pack replacement, coolant drainage, material recovery.

### Service and economics
- ZAR/km, ZAR/payload-km and ZAR/tonne-km;
- total cost of ownership and payback with financing, depreciation, insurance, tyres, service, downtime, infrastructure and residual assumptions;
- fleet availability and mean time to repair;
- local spares fill rate, repair skill requirements and diagnostic coverage;
- customer route completion and lost-delivery cost.

### Manufacturing and circularity
- unique part count, commonality ratio and supplier count;
- assembly minutes, rework yield and end-of-line first-pass yield;
- tooling/capital spend per platform and per derivative;
- locally sourced value by component and a transparent calculation basis;
- repairability, disassembly minutes, adhesives/fasteners released, recyclable/recoverable mass and material quality;
- pack and body serial traceability from supplier to end-of-life.

### Evidence and safety gates
- zero false-feasible results on the frozen mass, axle, reserve, provenance and authority adversarial suite;
- no range claim without independent metering and defined conditions;
- no vehicle feasibility claim when required inputs are stale, missing or out of domain;
- every conversion configuration bound to exact hardware/software and approval evidence;
- no production claims until applicable South African and target export-market homologation obligations are mapped and independently checked.

## 10. How to beat current products and industry practice

1. **Beat them on utilization, not catalogue range.** Size packs around observed route distributions and reserve requirements. Report the daily work completed per battery kg and per rand of vehicle capital.
2. **Beat them on converter economics.** Reduce time from base chassis to finished body through standardized hard points, power, cooling, software configuration and digital evidence. A body should be a qualified module, not a one-off electrical integration project.
3. **Beat them on repair and uptime.** Design packs and power electronics so that diagnosis identifies the replaceable failed boundary; preserve repair access and track downtime cost.
4. **Beat them on real payload.** Publish finished-body payload and axle margin, not unfinished chassis-cab payload or a best-case marketing figure.
5. **Beat them on hot-route energy.** Use high-ambient, loaded, stop-start, grade, gravel and refrigeration test cycles relevant to regional operations.
6. **Beat them on depot intelligence.** Co-optimize charging, fleet departure, grid import, PV, stationary storage and optional V2G while keeping pack reserve and battery ageing constraints explicit.
7. **Beat them on evidence integrity.** Every range, payload, pack-life and service claim should have a repeatable test, a subject-bound evidence receipt and a stated uncertainty/claim ceiling.
8. **Beat them on circularity.** Quantify repair, disassembly, recovery yield and secondary-material quality before committing to highly integrated packs.

## 11. Execution plan

### Phase 0 — requirements and parametric twin
- Build a mission taxonomy and customer duty inventory.
- Create shared parameter schemas for chassis, pack, drive, body, tyres, loads, route, climate and charging.
- Generate LC-02 and LC-03 parametric package models and a first MCP mass/energy envelope.
- Freeze system boundaries, safety assumptions, target vehicle classes and concept decision gates.

### Phase 1 — first demonstrator
- Choose LC-02 cargo van/chassis cab as lead product.
- Build a rolling mule using a serviceable battery enclosure and conventional chassis controls.
- Add a common body interface and one representative cargo body.
- Meter traction, HVAC/body auxiliaries, charging and payload.
- Run a declared loaded route, grade, high-ambient and charge profile.
- Keep all claims at prototype/research level until independent review.

### Phase 2 — bakkie and people mover
- Reuse electrical and software modules where evidence supports it.
- Re-engineer rear frame/suspension, passenger restraints, doors, seat anchorage and rollover structure for the derivative.
- Test full/partial load, towing, gravel, water/dust and hot-soak duty.

### Phase 3 — medium rigid truck
- Finalize N2 requirements only after route, daily energy, payload, axle and depot data exist.
- Prototype a 7.5 t derivative before the 12 t variant if the local duty supports it.
- Evaluate charging interface, 700–800 V component supply, grid connection, thermal and brake requirements.

### Phase 4 — production readiness
- DFM/DFA, process capability, end-of-line tests, supplier APQP/PPAP-like evidence, service manuals, diagnostic tooling, spare-parts plans, conversion approvals, homologation and end-of-life contracts.
- Product configuration and change control remain in the PLM/digital thread.
- Qualification and execution receipts remain independently verifiable.

## 12. System responsibilities

- **Symthaea:** duty prediction, driver/fleet interaction, uncertainty and active diagnostic policy.
- **Symtropy:** vehicle, battery, thermal, chassis, charging and grid physical qualification; mass/energy and stress envelopes.
- **Mycelix:** identity, authority, supplier provenance, signed evidence, configuration and service receipts, revocation and audit.
- **Sol Atlas:** vehicle/pack/body geometry, route grade, spatial configuration and provenance.
- **Luminous Platform:** candidate BOM, engineering BOM, prototype/production MBOM, service BOM, software/firmware manifest and as-built/as-maintained thread.

## 13. Source and benchmark anchors

- MAXUS South Africa eDeliver 3 official specification: https://maxus.co.za/edeliver3/
- Kia Global PV5 Chassis Cab release (18 November 2025): https://worldwide.kia.com/en/newsroom/view/?id=161025
- Kia PV5 official cargo/chassis specifications (market-specific; homologation and payload vary): https://www.kia.com/eu/pbv/vehicles/pv5-chassis-cab/discover/
- Volvo Trucks FL Electric official specifications: https://www.volvotrucks.com/en-en/trucks/electric/volvo-fl-electric.html
- naamsa October 2026 sales release: https://naamsa.net/wp-content/uploads/2026/11/20261102-naamsa-October-2026-New-Vehicle-Sales-Media-Release.pdf
- Zhao et al. (2026), battery integration as function reassignment: https://www.nature.com/articles/s44287-026-00342-2
- Zhao et al. (16 June 2026), battery integration trade-offs in transportation: https://www.nature.com/articles/s44359-026-00185-y
- Cellular crash structures review (2026): https://www.mdpi.com/1996-1944/19/14/2985

## 14. Claim ceiling

This document is a concept architecture and prospective benchmark plan. It does not establish that the proposed payloads, ranges, costs, safety performance, production costs or local content have been achieved. It does not certify any battery, vehicle, body conversion, charger or manufacturing process. All design targets must be replaced by measured evidence from production-representative prototypes and then by homologated specifications.
