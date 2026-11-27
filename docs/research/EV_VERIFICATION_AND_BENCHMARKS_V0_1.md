# EV Verification and Useful-Service Benchmarks v0.1

Programme: [EV-VALIDATION-001 #2114](https://github.com/Luminous-Dynamics/symtropy/issues/2114)  
Portfolio: [EV-PLATFORM-001 #2112](https://github.com/Luminous-Dynamics/symtropy/issues/2112)  
Implementation: [Symtropy PR #2084](https://github.com/Luminous-Dynamics/symtropy/pull/2084)

## 1. Paradigm shift: design around service, not vehicle metrics

Range, nominal battery capacity, peak kW and charge rate describe components. They do not say whether the system reliably carries passengers, delivers goods, cultivates a field, excavates material or moves mine tonnes within a working window.

Make the **useful-service mission** the unit of comparison:
- passenger vehicles / transit: passenger-km and completed scheduled trips;
- delivery / freight / terminal: delivered kg-m³, tonne-km or accepted moves;
- agriculture: hectare-task, drawbar/PTO output and seasonal hectares completed;
- construction: m³ moved and accepted work cycles;
- mining: tonnes moved, tonne-km and productive hours;
- autonomous handling: accepted material moves per hour;
- emergency response: response hours and qualified critical-energy service delivered.

Every report must include failures, missed trips, charging queue/dwell, repair time, payload loss and service availability in the denominator. Never report energy only for missions that finished successfully.

## 2. Design the energy ecosystem as one system

Treat vehicle pack, work-tool load, site distribution, charging, stationary buffer, queues, route/dwell windows, direct supply and fallback as one resource-constrained system. Co-design the architecture with the mission profile.

The IEA's 2026 outlook identifies site-capacity and high-power charging as growing constraints; heavy-duty charging also depends on depot access, route/dwell and actual compatible parking/charging layouts. V2G could provide flexibility, but commercial offers, capable models, standards and regulation remain uneven. That supports co-design and measured interoperability, not assuming bidirectionality is ubiquitous:
- https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-charging-chap-6-and-10
- https://www.iea.org/reports/vehicle-to-grid-technology
- https://www.iea.org/reports/ultra-fast-charging-batteries

For each duty, compare the actual feasible choices: depot AC, managed fleet charging, public DC, high-power conductive, opportunity, battery exchange, site stationary storage, trolley/direct supply and (where justified) wireless receiver coupling. Each candidate receives the same mission, functional unit, reserve requirement, climate/route and service window.

## 3. Explicit energy boundaries

Every comparison shall separate:
1. wheel/track-side mechanical work;
2. battery-side traction demand;
3. battery-side work-tool and auxiliary demand;
4. regeneration available versus actually captured;
5. vehicle charge-inlet energy;
6. charger AC/DC output and site input;
7. direct trolley/cable energy;
8. stationary buffer charge/discharge and round-trip losses.

Rules:
- Captured regeneration can reduce net energy but is not a safe-braking guarantee.
- Direct-supply energy is not also battery energy. The trolley/battery mission profile in v0.2 represents off-wire reserve, with trolley electricity counted on a separate path.
- Charge-inlet power is not the same as site input; use measured charging efficiency and shared site capacity.
- Nominal battery energy is not usable energy. Expose state of health, temperature capacity, reserve and charge acceptance.
- Vehicle-to-building/grid export only exists when its isolation, protection, mode control and regulatory requirements are independently qualified.

The current v0.2 longitudinal model is intentionally first-order: segment-average speed, constant road-load coefficients, explicit auxiliary/work energy and capped regenerative energy. It lacks calibrated motor/inverter maps, transient speed traces, battery electrochemistry, charge taper and detailed work-tool physics. Synthetic pass means only arithmetic screen success.

## 4. Verification matrix

The machine-readable matrix is `tools/rural-industrial-electrification-evaluator/fixtures/vehicle_verification_matrix_v0_1.json`; schema is `schemas/vehicle_verification_matrix_v0_1.schema.json`. It assigns **five release-blocking evidence gates to each of 19 designs (95 gates total)**:

1. **Mass and structural envelope** — weigh the actual configuration, installed module and payload; measure axle/wheel/track load; compare to signed-off limits with uncertainty.
2. **Mission energy and service output** — replay the frozen mission with instrumented speed, grade, payload, pack, auxiliaries, work output and environmental traces.
3. **Charge and site co-availability** — measure vehicle and site energy, charge curve, charger concurrency, queue, outage and departure-window fit.
4. **Safety fault response** — inject class-specific faults; verify observed safe state, isolation/braking or stop, warnings, mitigation and recovery with independent hazard review.
5. **Serviceability and lifecycle evidence** — repeat critical maintenance tasks with declared tools/skills, configuration and part provenance; measure diagnosis and repair time.

Each design has a dedicated test scope: accessibility and ramp/lift recovery for the shuttle; PTO/soil duty for tractors; hydraulic pressure release/emergency lowering for the excavator; thermal/ventilation/egress for mine equipment; collector loss for trolley haulage; export isolation and reserve for emergency vehicles; and safe stop/localization/network faults for autonomous carriers.

## 5. Benchmark targets

### Immediately enforceable software and data invariants

| Benchmark | Target |
|---|---:|
| Catalogue identities | Exactly 19 unique design IDs |
| Mission coverage | Exactly one declared baseline profile per design ID |
| Platform mapping | Every design maps to one known platform family and one known architecture pattern |
| Subsystem architecture | Exactly six unique subsystem blocks per design at v0.1 |
| Verification coverage | Exactly five release-blocking gates per design (95 total); no duplicate test IDs |
| Negative controls | 100% of mass, energy/SOC, power, charging-window, safety and unknown-evidence gates have a test that must be blocked/indeterminate |
| Deterministic reports | 100/100 order permutations yield byte-identical canonical output |
| Energy accounting | Explicit boundaries; no direct-supply or regenerative-energy double count |
| Evidence provenance | 100% of physical claims include configuration, method, timestamp, raw-data reference, calibration and uncertainty |
| Release rule | 0 safety or evidence gate failures silently downgraded into a numeric average |

### Physical benchmarks to freeze only after a baseline

Do **not** invent universal kWh/km, charging speed, range, repair-time, availability, thermal or lifecycle improvements. For each physical metric, first preregister the test protocol, configuration, mission, payload, ambient envelope, baseline, uncertainty model, exclusions and acceptance threshold. Then report the distribution (at minimum median and tail percentile), sample count, confidence/uncertainty interval, failure rate and all excluded/aborted attempts.

Suggested physical metrics:
- energy per useful service unit and gross/usable battery mass;
- mission completion and schedule adherence;
- continuous/peak power margin and derate duration;
- maximum battery/coolant/motor/inverter temperature and pack temperature spread;
- accepted charge power versus state of charge/temperature; delivered kWh, charging dwell and queue;
- productive availability, unserved task count, repair time and spare-part lead time;
- safety-test coverage and unresolved hazards (never collapsed into an economic score);
- cost and lifecycle impact per useful service unit using an explicit boundary and source provenance.

## 6. Failure-first experiment design

For every new gate, pair at least one nominal case with the nearest meaningful negative control. Examples include overload by declared payload, low initial SOC, battery cold/hot derating, charger reduced to a shared site cap, failed charger, lost opportunity window, unavailable regen, thermal derate, failed ramp/lift, emergency export protection trip, tool interlock failure, loss of trolley supply, blocked autonomous route and unavailable repair part.

The point is to test whether the evaluator can **refuse an unsafe or infeasible claim**, not merely calculate plausible results.

## 7. Research inputs and limits

### Charging and site constraints

The IEA's 2026 work separates heavy-duty charging from light-duty infrastructure and discusses the practical issue that not every nominal public charging space is manoeuvrable or suitable for heavy vehicles. Model cable handling, parking geometry, dwell, connection uptime, queue and the actual site power budget in addition to a charger nameplate:
- IEA Global EV Outlook 2026 charging chapter: https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-charging-chap-6-and-10
- SAE J3271 includes coupler, communications/control, cable handling/cooling, grid connection/bidirectional use cases and interoperability testing topics: https://saemobilus.sae.org/standards/j3271_202503-sae-megawatt-charging-system-electric-vehicles

### Mining and work machines

A 2026 published underground-mine study jointly sizes batteries and fast charge, swap or trolley assistance and finds distinct Pareto frontiers under a hypothetical mine's assumptions; that is evidence for coordinated optimization, not a universal winner. A 2026 electric mine-truck thermal paper considers high ambient and high-load conditions and evaluates pack maximum temperature and spread; that reinforces worst-case thermal testing over nominal drive cycles:
- https://www.sciencedirect.com/science/article/pii/S0142061526000943
- https://www.sciencedirect.com/science/article/pii/S1290072925008622
- https://www.sciencedirect.com/science/article/pii/S0959652625018025

### Safety and cybersecurity

SAE J3271's information report covers interoperability and several system interfaces but is not by itself proof any vehicle is compliant. ISO/SAE 21434:2021 provides a lifecycle cybersecurity engineering framework; ISO lists an edition-2 work item as under development in October 2026. Use the current applicable version with a specialist rather than pinning a certification claim to a draft:
- https://saemobilus.sae.org/standards/j3271_202503-sae-megawatt-charging-system-electric-vehicles
- https://www.iso.org/standard/70918.html
- https://committee.iso.org/standard/95437.html

### South African validation path

Start with measured local work missions rather than imported energy averages: Johannesburg urban/arterial fleet routes; rural weak-grid and field-support routes; municipal task cycles; and mine haulage, route grades, shift windows, charging and thermal conditions. Record tariff/site capability, actual outage history, ambient/dust/water, tyre/service supply, locally available skills/parts and vehicle-use patterns. The dtic's 2023 White Paper is a policy anchor for connecting EV engineering with local production and consumption; it is not a substitute for local technical baselines:
- https://www.thedtic.gov.za/wp-content/uploads/EV-White-Paper.pdf
- https://www.gov.za/news/cabinet-statements/statement-cabinet-meeting-29-november-2023%C2%A0-30-nov-2023

## 8. Release and claim ceiling

No design progresses to build definition until all applicable gates have configuration-bound evidence and a reviewed specialist safety/regulatory plan. A passing data/schema or synthetic simulation run shows only the evaluator works against the declared checks. It does not qualify battery cells/packs, thermal systems, brakes, crash/rollover structures, charge interoperability, public-road use, accessibility, farm/construction work, emergency export power, underground mine use or autonomous safety.

The next physical-model layers are: measured speed-time traces; road-load, tire and drivetrain efficiency calibration; battery voltage/current/temperature and SOC-dependent charge curves; motor/inverter and thermal transient models; PTO/hydraulic work physics; charging queues/outages; life-cycle economics; and independent on-vehicle validation. Keep each extension separately falsifiable.
