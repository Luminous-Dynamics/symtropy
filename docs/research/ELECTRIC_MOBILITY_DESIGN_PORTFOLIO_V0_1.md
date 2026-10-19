# Electric Mobility Design Portfolio v0.1

**Programme:** EV-PLATFORM-001  
**Owner:** Symtropy cross-sector design/evaluation substrate  
**Status:** Reference architecture and synthetic envelopes; not production specifications

## 1. Design thesis

Build a family of vehicles around shared interfaces and validated subassemblies, not around a single universal chassis. The highest-value commonality is usually in diagnostics, low-voltage electronics, software update/recovery, pack telemetry, service tooling, identity/provenance and selected power-electronics modules. The costly mistake is forcing the same battery form, axle, voltage class, cooling package, frame, suspension, tire, work tool or charge connector onto duty cycles that are physically different.

The optimization target is **useful service per mission, under declared payload, terrain, uptime and site-energy constraints**. Nominal battery capacity, peak motor power, range, charger rating or headline efficiency is not an adequate design objective by itself.

All numbers in `fixtures/vehicle_designs_v0_1.json` are synthetic engineering envelopes intended to exercise validation and arithmetic. They are not actual vehicle specifications, supplier data, tested range, payload certification, cost models, or deployment recommendations.

## 2. Portfolio map

| Design family | Core architecture | Why it has its own envelope | Primary benchmark |
|---|---|---|---|
| Cargo micro-mobility | Low-voltage assist drive, light replaceable pack, cargo structure | Human power, weather exposure, stability, load placement and repair simplicity dominate | Wh per delivery, loaded handling, repair time, safe braking |
| Compact passenger EV | Small pack, efficient road drivetrain, AC home/depot charging | Low daily energy and curb mass reward simplicity over oversized peak hardware | kWh/km, cost of ownership, winter/heat derating, usable cargo |
| Family passenger EV | Higher crash structure, passenger/cargo load envelope, thermal comfort | Safety structure, comfort loads and variable occupancy affect energy and mass | passenger-km, energy in hot/cold, accessibility, serviceability |
| Accessible shuttle | Low-floor/step-free body, ramp/lift, securement, HVAC reserve | Access hardware and dwell/boarding duty are first-class payload and uptime requirements | passengers served, dwell time, accessible boarding, reserve margin |
| Light commercial van | Modular cargo volume, serviceable shelving/body, depot charge | Payload and internal volume must remain after battery and conversion hardware | delivered kg-m³, route completion, payload retained |
| Rural all-terrain/service 4×4 | Protected underbody, recoverable modules, traction/thermal margin | Long distances, weak-grid access, poor roads, dust/water and field repair dominate | mission completion, recovery, ground clearance, local repair time |
| Emergency/resilience vehicle | Road platform plus isolated export-power interface where qualified | Vehicle mobility and safe stationary power are separate duty modes | emergency hours, critical loads served, reserve after export |
| Municipal work truck | Body upfit, hydraulic/electric accessory bus, stop-start duty | Work tools may consume as much as propulsion; idling is replaced by auxiliary power demand | route/work output, accessory kWh, uptime, noise/exposure |
| Smallholder tractor/implement carrier | Low-speed continuous torque, PTO/hydraulic interface, protected connectors | Implement work and high-torque thermal duty matter more than road range | hectares/task, drawbar/PTO output, shift completion |
| Row-crop/medium tractor | Larger traction pack or direct supply where routes allow, implement bus | Seasonal windows and high continuous power make sizing a system problem | hectares per weather window, soil/traction condition, pack aging |
| Compact construction equipment | Electric traction plus hydraulic/electric work circuit | Digging/lifting can create sustained load while vehicle speed is near zero | m³ moved, work cycles/hour, peak/continuous thermal headroom |
| Urban distribution truck | Dense stop-start route, regenerative braking, depot/opportunity charge | Payload, route windows, auxiliary loads and charger concurrency interact | tonne-km, route completion, charging queue, payload retained |
| City bus | Passenger load, HVAC, depot or terminus opportunity charging | Thermal comfort and passenger mass vary; service schedule is hard constraint | passenger-km, missed trips, hot-day HVAC range, charge-window fit |
| Long-haul truck | Heavy-duty chassis, axle/motor configuration, high-power conductive charging | Mass, grades, payload, continuous power and mandated rest/dwell windows dominate | tonne-km, route completion, payload fraction, charging dwell |
| Port/yard terminal tractor | Low-speed high-torque drivetrain, frequent duty-cycle charge windows | Dense cycles and fixed routes may support opportunity charging or pack logistics | moves/shift, queue and downtime, energy per move |
| Underground mine mobile machine | Mine-qualified chassis, thermal/ventilation-aware electric drivetrain, opportunity charge/swap comparison | Heat rejection, ventilation, grade, dust, fault recovery and production cycle are coupled | tonnes/hour, ventilation/cooling load, production lost to charge/fault |
| Open-pit haul truck: battery-only control | Large battery-only reference, explicitly bounded by energy and payload | Control case exposes energy and payload limits instead of assuming feasibility | net tonnes moved, battery reserve, charge queue, productive availability |
| Open-pit haul: trolley + battery | Direct supply on repeatable high-power corridors plus off-wire reserve | Potentially reduces onboard storage only if trolley coverage and transitions are sufficient | tonne-km, trolley route utilization, battery mass, detour/fault recovery |
| Warehouse/yard autonomous carrier | Low-speed, high-duty drive, safe-stop and recoverability, automatic charging | Predictable routes enable charging coordination but increase fleet concurrency | tasks/hour, energy/task, blocked-route recovery, safe stopping |

## 3. Modular architecture: share interfaces, not every physical part

### A. Platform / body / mission module

- Keep mass accounting explicit: base curb mass, installed mission module, payload, gross limit and axle/tyre limits are separate quantities.
- Use replaceable body and task modules for vans, municipal bodies, rural utilities and industrial carriers where interfaces and crash/load paths can be validated.
- Do not make a cargo bicycle, family car, tractor, mine loader and highway truck share one physical frame or pack simply because the software model can describe them.

### B. Traction and work output

- Declare **continuous** power separately from short-duration peak power; include high-torque/low-speed heat rejection.
- Separate traction from PTO, hydraulics, refrigeration, HVAC, hoists, pumps and other auxiliary loads.
- Regeneration is opportunity, not free energy: capture only where the mission supplies deceleration/descent energy and the battery, inverter, grip and temperature permit acceptance.
- For agricultural and construction machines, benchmark useful implement output and work cycle—not kilometres alone.
- Keep mechanical braking and safe degraded operation valid when regeneration is unavailable.

### C. Battery and electrical architecture

- Size pack energy from duty, reserve, degradation, temperature and fallback—not nominal kWh alone.
- Treat nominal voltage, service isolation, insulation monitoring, contactors, pre-charge, thermal management, crash/impact protection and manual recovery as separate design obligations.
- Use chemistry as a trade-space, not a brand. LFP, NMC, sodium-ion and other technologies cannot be ranked without reference conditions for energy density, power, temperature, life, supply chain and pack-level safety.
- Pack exchange is only attractive when pack/vehicle interfaces, handling equipment, spare inventory, standardization, fire protection and utilization costs all work together.
- Do not use swappable packs as a universal design constraint for vehicles whose mass, energy, structural integration or safety requirements make modularization harmful.

### D. Charging and on-route power

| Pattern | Best candidate envelope | Must account for | Do not assume |
|---|---|---|---|
| AC home/depot | Passenger/light fleets with long dwell | service connection, charge window, cable access, efficiency | every rural connection has the required power |
| Fleet DC | Vans, buses, work vehicles | charger power, site import cap, simultaneous charging, heat | charger nameplate equals sustained pack intake |
| Megawatt conductive | Heavy road freight and compatible high-power platforms | vehicle inlet, cable cooling, charger, communications, site transformer and load diversity | all trucks accept full nameplate power through the whole charge |
| Opportunity charging | Bus termini, yard equipment, defined industrial pauses | dwell reliability, queue, alignment/connection, missed window | all route patterns have reliable dwell |
| Battery exchange | High-utilization fleets with standardized packs | inventory, charge cycles, lifting, human factors, spare packs, downtime | swapping is automatically faster or cheaper overall |
| Trolley/cable-electric | Repeatable high-power corridors, mine or port routes | route coverage, electrical clearance, transitions, off-wire reserve, failure recovery | corridor infrastructure suits diffuse roads or farms |
| Static/dynamic wireless | Select automated/contactless cases | coupling efficiency at matched boundaries, alignment/gap, contamination, shielding, field exposure, foreign-object detection and civil works | contactless means lossless or universally superior |
| Grid + stationary buffer | Weak/limited site connections with predictable peaks | charge/discharge power, usable energy, SOC reserve, round-trip loss, cycle life, power quality and controls | storage creates energy or removes the interconnection constraint |

**Boundary rule:** energy generated at a plant reaches a site through a qualified electrical or thermal network. Wireless charging is a short-gap vehicle receiver interface, not long-range transmission from a generator.

For heavy-duty conductive charging, use the current MCS reference family and interoperability test requirements where applicable. CharIN describes MCS as a high-power conductive system and lists a standards set that includes IEC 61851-23-3:2026, IEC TS 63379, ISO 15118-10 and ISO 15118-20 amendments; treat these as references for specialist review, not proof that any concept is compliant:
- https://www.charin.global/technology/mcs/
- https://www.charin.global/news/iec-61851-23-3-2026-completes-mcs-standards-framework/
- https://saemobilus.sae.org/standards/j3271_202503-sae-megawatt-charging-system-electric-vehicles

## 4. Control, safety and recovery principles

These are design requirements to be verified by qualified engineering, not certifications claimed by this catalogue.

1. **Safe energy state:** independent physical isolation, robust fault indication and a documented service lockout/recovery procedure.
2. **Derate explicitly:** battery, inverter, motor, connector, coolant or site limits reduce delivered power transparently; no controller should silently promise peak power beyond the active thermal/electrical envelope.
3. **Braking:** friction/mechanical braking remains sufficient when regenerative braking is unavailable, the pack is full/cold/hot, or the inverter faults.
4. **Safe stop:** autonomous and remote-operated platforms have defined lost-communications, obstacle, sensor-fault, localization-fault and emergency stop behavior.
5. **Functional safety and cybersecurity:** hazards, safety goals, threat models, secure updates, key/certificate lifecycle, logging and rollback need class-specific engineering and test evidence.
6. **Human factors:** visibility, ingress/egress, accessible boarding, service ergonomics, acoustic alerts, dust/night operation and emergency egress are evaluated for the intended population and environment.
7. **Environmental protection:** specify water/dust, vibration, corrosion, shock, wash-down, temperature and cooling conditions. “Off-road” is not a complete environmental requirement.
8. **Recoverability:** identify how a disabled vehicle is isolated, lifted, towed, pulled or safely removed without creating a new hazard.
9. **Maintenance:** fault isolation and common tools should minimize mean time to diagnose/repair, not just reduce component count.
10. **Lifecycle:** repair, pack remanufacture, second-life use and recycling need traceable serial/configuration history, safe state information and accountable operators.

## 5. Evaluation kernel and what it does not model

The Rust `vehicle_design_screen` checks five bounded questions for each reference design:

1. Does base mass + installed mission module + target payload fit the gross-mass limit?
2. Does usable energy after state-of-health, temperature derating and reserve cover declared battery-side mission energy?
3. Are continuous and peak power demands within their distinct traction ratings?
4. Can the shift's battery energy be replenished inside the charge window, after charger efficiency and shared site power are applied?
5. Is the evidence class sufficient for a result to be rankable?

Unknown evidence is **indeterminate**, not a pass. Known constraint failures are **blocked**. No aggregate score is produced and no candidate is declared safe or production-ready.

Energy formula:
```text
usable_kWh = nominal_kWh × state_of_health × temperature_capacity_fraction × (1 − reserve_fraction)
grid_energy_to_replenish_kWh = battery_energy_per_shift_kWh / charging_efficiency
effective_charger_kW = min(vehicle_charger_rating_kW, site_power_budget_kW / simultaneous_vehicles)
charge_hours = grid_energy_to_replenish_kWh / effective_charger_kW
```

The shift energy for a trolley/opportunity-assisted vehicle is the **battery-side residual after the declared direct-supply contribution**. Its boundary must match its mission data. The arithmetic model assumes constant charge power and efficiency and does not implement battery electrochemistry, taper curves, traction, grade, thermal dynamics, load-flow, charging queues, tyre/axle limits or fault simulation.

## 6. Benchmarks and gates

### Reference-kernel acceptance

- 100% of design IDs unique; deterministic sorted output.
- 100% agreement with independent arithmetic oracles for usable energy, gross mass, shared charger power and charge time.
- Zero passing designs with overload, power shortfall, insufficient usable energy or an unachievable charge window.
- Zero unknown or incomplete evidence silently ranked.
- 100/100 permutation replays produce identical canonical report content.
- Positive and negative fixtures for every hard gate, including a deliberately blocked battery-only open-pit control.

### Vehicle and fleet evaluation, once sourced data exist

| Metric | Correct functional unit | Pairing requirement |
|---|---|---|
| Energy use | Wh/passenger-km, kWh/tonne-km, kWh/hectare-task, kWh/tonne moved, or kWh/job | same mission, payload, weather, terrain and service output |
| Productivity | accepted passengers, deliveries, hectares, tonnes, or jobs per shift | include charge/fault downtime and schedule misses |
| Battery burden | pack kg and kWh per useful mission | same reserve, degradation and temperature envelope |
| Charging | dwell minutes/shift; site peak kW; missed charge windows | actual power-sharing and charger availability |
| Lifecycle cost | cost per useful service over declared life | quote-backed capital, finance, energy, maintenance, pack replacement and downtime |
| Reliability | availability, mean time to diagnose/repair, fault-recovery time | defined operating hours and failure taxonomy |
| Accessibility | boarding success, route coverage, securement and dwell | representative users and independently reviewed protocol |
| Safety | hazard coverage and verified mitigations | qualified class-specific safety case; never averaged into an economic score |
| Sustainability | lifecycle energy/material/emissions per service unit | consistent boundary, electricity mix, battery replacement and end of life |
| Repairability | time-to-diagnose, common-tool task rate, module replacement time | same skill/tool assumptions, independent technicians |

Targets should be numerical only after a reliable baseline is frozen. Do not invent a universal “percent better than diesel” claim from synthetic inputs.

## 7. Next build sequence

1. **v0.1 now:** versioned portfolio, synthetic archetypes, bounded Rust screen, negative-control test and CI.
2. **v0.2 physics:** longitudinal traction model for road/grade, rolling resistance, aerodynamic drag, rotating mass, drive efficiency, regenerative-braking saturation and auxiliary loads.
3. **v0.3 work machines:** PTO/hydraulic duty cycles, low-speed thermal model, soil/implement work, excavation/hoist cycle and mine route/ventilation coupling.
4. **v0.4 fleet/energy co-design:** minute/interval charging, queueing, site import caps, battery degradation and Pareto trade-offs across pack sizes, depot, swap, opportunity and trolley.
5. **v0.5 service and human factors:** accessibility, technician tooling, module provenance, regional parts lead times, repair and safe recovery.
6. **v0.6 evidence-based regionalization:** South African rural/urban/mine cases first, then other regions only when grid, climate, road, service and cost evidence exists.
7. **Physical validation:** matched bench, vehicle and route tests; calibrated uncertainty; independent safety / homologation review before any production or public-road claim.

## References

- CharIN, Megawatt Charging System: https://www.charin.global/technology/mcs/
- IEC 61851-23-3:2026 MCS framework summary: https://www.charin.global/news/iec-61851-23-3-2026-completes-mcs-standards-framework/
- SAE J3271, Megawatt Charging System for Electric Vehicles: https://saemobilus.sae.org/standards/j3271_202503-sae-megawatt-charging-system-electric-vehicles
- Hashemian et al. (2026), *Coordinated sizing of battery and charging systems for underground mining electric trucks*: https://www.sciencedirect.com/science/article/pii/S0142061526000943
- U.S. DOE, *A Market and Technology Assessment for Off-Road Vehicle & Equipment Energy and Emissions Innovation*: https://www.energy.gov/sites/default/files/2024-12/doe-eere-market-technology-assessment-offroad-vehicle-equipment-energy-emissions-innovation.pdf

## Claim ceiling

This catalogue is design-system groundwork. It is not CAD, FEA/CFD, a complete vehicle digital twin, a battery or thermal qualification, a field result, a supplier quotation, a certification, or permission to build or deploy any vehicle.