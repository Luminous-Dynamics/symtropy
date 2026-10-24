# EV Platform Families and Interface Contracts v0.1

The machine-readable source is tools/rural-industrial-electrification-evaluator/fixtures/vehicle_platforms_v0_1.json, validated against schemas/vehicle_platforms_v0_1.schema.json. It maps all 19 catalogue design IDs exactly once into 10 architecture families and defines 11 software/interface contracts.

## Architecture rule

Reuse stable semantics and evidence processes first. Do not assume hardware compatibility from a shared label.

Reuse approval levels:
- **Contract reuse:** same names, units, state-machine intent, event semantics and test vectors. This is the default.
- **Software-library reuse:** permitted after APIs, fault behavior, timing/resource requirements, configuration and independent tests are proven compatible.
- **Component reuse:** candidate only after voltage/current, thermal, mechanical, environmental, electromagnetic, safety and service requirements all match.
- **Physical platform reuse:** never assumed by this catalogue. It requires vehicle-specific load paths, mass/axle, crash/rollover, package, ingress, service and applicable qualification evidence.

## Platform families

| Family | Design IDs | Shared work | Must stay specialized |
|---|---|---|---|
| Cargo micro-mobility | cargo-micro-mobility | identity, low-voltage diagnostics, energy accounting, service events | frame/wheels/brakes, loaded stability, rider safety, battery retention |
| Passenger road | compact-passenger-ev, family-passenger-ev | configuration, HV-safe-state vocabulary, battery/charge/thermal telemetry | crash structure, restraints, axle loads, cabin/HVAC, pack shape/cooling |
| Accessible transit | accessible-shuttle, city-bus | charging/dispatch, passenger-service data, accessible-service test protocol | ramp/lift/securement, boarding geometry, egress, capacity, doors and HVAC |
| Light commercial/utility | light-commercial-van, rural-all-terrain-utility, emergency-resilience-vehicle | service/diagnostic contracts, energy metering and selected work-power semantics | cargo load paths, off-road recovery and sealing, emergency export-power isolation |
| Commercial/municipal trucks | municipal-work-truck, urban-distribution-truck | configuration, charge scheduling, traction/work energy accounting | axle/body mass, centers of gravity, work-tool interlocks, payload and route |
| Agricultural work | smallholder-tractor-carrier, medium-row-crop-tractor | implement-energy contract, task metering, repair and provenance records | PTO/hydraulics, implement attachment, soil/traction, ballast, rollover protection |
| Construction equipment | compact-electric-excavator | diagnostic, energy boundary and work-cycle vocabulary | hydraulic/arm geometry, lifting stability, undercarriage, emergency lowering, cooling |
| Heavy road/yard | long-haul-heavy-truck, port-yard-terminal-tractor | charging/dispatch, traction energy and service telemetry | highway grade/endurance/retarder vs low-speed hitch cycles and yard interaction |
| Mine mobile equipment | underground-mine-mobile-unit, open-pit-haul-battery-only-control, open-pit-haul-trolley-battery | energy accounting, fault event logging, charge/site coordination | underground environment/ventilation/egress vs open-pit haul mass, tires, trolley hardware, fault recovery |
| Autonomous material handling | warehouse-autonomous-carrier | configuration, charge docking state, dispatch, safe-stop event vocabulary | perception, protective fields, braking, localization failure and facility-specific recovery |

The family assignment organizes work and validation. It does **not** imply that all designs in a family use identical voltage, pack format, motor, inverter, axle, connector, charger protocol, cooling loop, frame or safety architecture.

## Eleven shared contracts

1. **Vehicle identity/configuration** — design and variant identity, installed modules, pack, software/calibration hashes and evidence links.
2. **Low-voltage diagnostics** — typed fault events, diagnostic snapshots, watchdog/update state and telemetry timestamps.
3. **High-voltage safe state** — isolation/discharge/service-lockout states and latched faults. Actual HV architecture and safety level remain class-specific.
4. **Battery energy boundary** — nominal vs usable energy, state of health, temperature capacity, reserve and SOC.
5. **Charging state** — requested/available/accepted power, session state, safe disconnect and metering boundary. Connector/protocol interoperability is not inferred.
6. **Thermal derating observability** — sensor identity, temperature, limit reference, derate reason, available power and sensor health. No common coolant loop is assumed.
7. **Traction/regeneration accounting** — positive wheel work, battery traction demand, potential/captured/curtailed regeneration. This does not replace friction brakes or certify stopping.
8. **Work-tool power** — requested/accepted power, energy meter, task counter and interlocks. PTO, hydraulics, hoists, ramps, pumps and refrigeration retain class-specific design.
9. **Site charging/dispatch** — site import cap, charger availability, simultaneous vehicles, queue and departure deadline.
10. **Maintenance provenance** — part/module identity, revision, service action, evidence, repair duration and end-of-life disposition.
11. **Safe-stop/recovery events** — trigger, requested/reached safe state, override state, recovery method and duration. The safety architecture itself remains specific.

## Functional architecture of an electric mobility product

- **Structure and mission body:** load paths, payload, accessibility, crash/rollover and attachment interfaces.
- **Energy storage and protection:** cells/pack, monitoring, contactors, pre-charge, isolation, enclosure, restraint and cooling.
- **Traction:** inverter/controller, motor, reduction, axle/final drive, tires/tracks, friction brake and regenerative limits.
- **Work systems:** PTO/implement, hydraulic pump, hoist, lift/ramp, refrigeration, onboard power export or other declared task loads.
- **Thermal system:** battery, inverter, motor, charger inlet and work-system heat sources are separately observable before deciding what can share a loop.
- **Charging and site:** vehicle inlet/receiver, communication and control, charger, distribution/protection, site import limit, buffer/storage and queuing.
- **Control and assurance:** driver/operator HMI, diagnostics, safe stop, service update/recovery, cybersecurity, configuration and safety evidence.
- **Service lifecycle:** diagnostic access, safe isolation, spare parts, repair process, versioned modules, provenance and end of life.

## Energy boundary and direct supply

The 19 mission profiles map one-to-one to the 19 design catalogue IDs. For trolley/battery designs, the mission model covers off-wire battery-reserve duty only. Trolley-supplied traction energy must be assessed in the companion energy-path/site model and cannot be added again as battery energy. Wireless charging is treated as a local receiver interface, not long-range power transmission.

## Design review gates

No family moves from synthetic architecture to build definition until it has:
- a signed-off mission and functional unit;
- gross/axle/payload and work-body load accounting;
- duty-cycle power/energy/thermal evidence, including temperature and reserve;
- a physical electrical/mechanical/thermal interface definition;
- hazard analysis, failure responses, safe service and recovery;
- diagnostic, update, cybersecurity and maintenance-provenance plan;
- cost and lifecycle boundaries with sourced data;
- a calibrated bench/vehicle/site test plan and qualified independent review.

## Standards and evidence boundary

MCS and other charge interfaces must be evaluated against the applicable current standards, vehicle/infrastructure combinations and interoperability tests. The catalogue links to the CharIN MCS hub, IEC 61851-23-3:2026 framework summary and SAE J3271, but it does not claim conformance:

- https://www.charin.global/technology/mcs/
- https://www.charin.global/news/iec-61851-23-3-2026-completes-mcs-standards-framework/
- https://saemobilus.sae.org/standards/j3271_202503-sae-megawatt-charging-system-electric-vehicles

Every numerical vehicle and mission fixture remains synthetic. The current artifacts are design-system and analysis scaffolding, not finished CAD, verified components, certified safety cases, supplier specifications, production readiness or permission to deploy.

## Per-design subsystem architecture trees

The machine-readable reference tree lives in `tools/rural-industrial-electrification-evaluator/fixtures/vehicle_subsystem_architectures_v0_1.json`, validated against `schemas/vehicle_subsystem_architectures_v0_1.schema.json`. It maps all 19 designs to one of nine architecture patterns and to their platform family. Each design decomposes into six reviewable subsystem blocks:

1. **Structure and mission body** — frame/body/implements, load paths, payload and access envelope.
2. **Traction drive** — motors/drive units, reduction, axle/track interface, regen and friction-brake boundary.
3. **Energy storage and protection** — pack placement, protection/isolation, reserve and service boundary.
4. **Thermal management** — battery, drive and work-system heat sources; no common coolant circuit is assumed.
5. **Charging / energy interface** — AC/DC depot, high-power conductive, opportunity charging, trolley or dedicated export interface according to duty and site.
6. **Controls, safety and service** — driver/operator controls, fault handling, safe-stop, diagnostics, recovery, updates and maintenance provenance.

These are conceptual trees with an explicit qualification gate per subsystem. No specific cell supplier, motor/inverter part, system voltage, connector pinout, wiring harness, frame dimensions, axle rating or cooling-loop design is asserted. A platform-family or interface match is a work-allocation and software-contract statement—not proof of physical interchangeability.

For a complete build definition each architecture still needs CAD and packaging, mass properties and axle/wheel loads, duty-cycle drive and work maps, battery cell/pack calculations, thermal and electrical schematics, protection/grounding, DFM/repair bill of materials, hazard analysis, controlled interface drawings, production/test procedures, and applicable independent qualification. Those deliverables should be created class-by-class from sourced requirements rather than fabricating a nominally complete BOM from synthetic envelopes.
