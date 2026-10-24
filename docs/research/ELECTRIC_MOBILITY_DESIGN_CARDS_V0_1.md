# Electric Mobility Reference Design Cards v0.1

These cards turn the portfolio into concrete architecture proposals. They are **reference designs to be modeled and tested**, not finished CAD or production instructions. They intentionally avoid unsupported claims about range, cost, lifetime, crashworthiness or field performance.

## Common platform contract

Every implementation shall declare the following before any comparison:

- **Identity:** design ID, variant revision, fitted modules, pack revision, software/calibration hashes and evidence sources.
- **Mass:** base vehicle, fitted work/accessibility/body modules, payload, gross mass and axle/wheel load envelope.
- **Power:** rated continuous and peak traction, work-tool output, auxiliary load, thermal derating curve and accepted regenerative power.
- **Energy:** pack nominal and usable energy, state-of-health and temperature envelopes, reserve, measured/estimated duty profile and fallback mode.
- **Charging:** accepted voltage/current/power envelope, connector or receiver interface, curve/taper, site share, charging window, charge communication and safe disconnect.
- **Operating envelope:** terrain, grade, speed, ambient temperature, dust/water/corrosion, hours/shift, payload, idle/work fractions and uptime target.
- **Safety:** hazard analysis, isolation/discharge, braking, safe-stop, emergency recovery, thermal event response and qualified interface boundaries.
- **Service:** diagnostics, replaceable subassemblies, repair tooling, expected skills, part provenance, end-of-life and update/recovery mechanism.

A design is not “modular” merely because its components can be unbolted. Modularity is accepted only when it improves total lifecycle outcomes without weakening structure, sealing, thermal interfaces, electrical isolation, safety or repair accountability.

## Interface families

| Interface | Shared where useful | Keep class-specific |
|---|---|---|
| Low-voltage power and vehicle network | diagnostics, update, logging, typed fault events and configuration identity | connector/pinout, redundancy, noise immunity and environmental rating |
| High-voltage energy path | naming, measurement boundaries, safe-state vocabulary and test methodology | voltage class, pack geometry, insulation, contactors, current path, cooling and service rules |
| Drive module | telemetry, thermal derate reporting, fault classes | axle rating, gear ratio, motor topology, reduction, torque duty and water/dust protection |
| Battery service boundary | isolation/lockout workflow, state-of-health semantics, evidence and serial provenance | pack dimensions, chemistry, mechanical retention, cooling plate and exchange method |
| Work-tool bus | power request/limit semantics, safety handshake, energy metering | PTO shaft, hydraulics, hoist, implement geometry, load paths and contamination protection |
| Charge interface | charge-state machine, power limits, authentication and logs | connector/receiver, cable cooling, inlet placement, automated coupling and charge standard |
| Safety/control | fail-safe principles, fault injection harness, watchdog and recovery record | vehicle-specific hazard analysis, driver/operator protection and certification path |
| Body/upfit | configuration identity, attachment interface version and weight ledger | crash/rollover structure, axle loads, accessible ingress, body seals and specialist mounting |
| Fleet/Mycelix evidence | service history, custody, maintenance events, certified part identity | access controls, privacy policy, regulated telemetry and operator-specific data |

## Reference design cards

### 1. Cargo micro-mobility assist — **cargo-micro-mobility**

**Package.** Low step-through cargo frame with a low-mounted, protected small pack; cargo deck or box attached to the load path rather than a cosmetic body panel; center stand and serviceable wheel/brake assemblies.

**Drive and energy.** Human assist plus limited electric drive; pack sized for the declared delivery shift and a reserve. Charging can use a removable low-energy pack only if connector, battery handling and anti-theft controls are safe and interoperable.

**Service and safety.** Braking and stability must be tested at maximum payload and with a failed motor/controller. No regenerative-braking credit in safety stopping distances. Validate loaded steering, tipping, curb/rough-surface behavior, rain ingress and rider visibility.

**Primary tests.** Wh/delivery, route completion under weather derating, loaded braking, handling, repair minutes per common failure and payload stability.

### 2. Compact passenger EV — **compact-passenger-ev**

**Package.** Compact road shell, short overhangs, efficient tire/aero package, accessible routine service points and minimal variant proliferation.

**Drive and energy.** Conservative continuous power for sustained driving, enough peak traction for the bounded acceleration/grade requirement and AC home/depot charge by default. DC fast charge is a secondary interface if the battery and thermal system support repeated sessions.

**Service and safety.** Separate passenger protection structure from replaceable exterior panels; isolate the HV pack before service; validate braking with cold/full/hot battery when regen is limited.

**Primary tests.** Wh/km by temperature/speed, occupant/cargo envelope, power derate on long grade, repair time, full lifecycle cost per passenger-km.

### 3. Family passenger crossover — **family-passenger-ev**

**Package.** Higher cabin/cargo envelope, rear-seat and child-seat access, flexible cargo restraint, roof/load constraints and thermal comfort designed together.

**Drive and energy.** Balanced pack size against real daily missions; cabin HVAC and defrost load belongs in the profile, not an afterthought. Battery preconditioning must expose its energy cost.

**Service and safety.** Safe operation with maximum occupancy and luggage; battery and charging access must not compromise access/egress or crash structure. Validate cold/hot range with family use rather than empty-vehicle cycles alone.

**Primary tests.** Loaded passenger-km, hot/cold energy, luggage/passenger packaging, thermal comfort and roadside recovery.

### 4. Accessible shuttle — **accessible-shuttle**

**Package.** Low-floor or otherwise accessible boarding geometry, ramp/lift and wheelchair securement as engineered modules, high-frequency door duty, clear turning/waiting spaces and safe egress.

**Drive and energy.** Route and dwell-time matched depot/terminus charging. Track traction energy, ramp/lift cycles, HVAC and door operation separately.

**Service and safety.** Boarding time and securement are part of service performance. Failure of a ramp/lift must be detected and have a documented safe response; a vehicle that moves efficiently but cannot board a passenger fails its purpose.

**Primary tests.** Boarding completion with representative users, dwell-time distribution, passenger-km, missed-trip rate, accessible recovery and usable energy at temperature extremes.

### 5. Light commercial van — **light-commercial-van**

**Package.** Common front drive/electrical module with multiple rear modules: plain cargo, shelving/service, refrigeration where justified, or accessibility/service upfit. Preserve structural mounting points and declared load paths.

**Drive and energy.** Route energy tied to payload, route stops and accessory load. Charging schedules must be coordinated across the depot import limit.

**Service and safety.** A body/upfit cannot bypass HV isolation, crash/retention, axle-load or thermal requirements. Keep cargo-volume and payload ledgers separate.

**Primary tests.** Delivered kg·m³/route, payload retained after upfit, route completion, auxiliary kWh, charger queue and body-module swap time.

### 6. Rural all-terrain service 4×4 — **rural-all-terrain-utility**

**Package.** Protected battery enclosure, sealed serviceable connectors, tow/recovery points, replaceable skid protection, accessible mechanical recovery and configurable utility/cargo body.

**Drive and energy.** Traction reserve for rough roads, soft ground and grades. Solar on the body/roof can offset auxiliary loads when conditions permit but is not counted as guaranteed traction energy.

**Service and safety.** Field repair, lifting/recovery, water/dust ingress, corrosion, tire availability and parts lead time are first-class constraints. No claim of an extreme-environment vehicle until immersion, impact, thermal and recovery tests exist.

**Primary tests.** Mission completion on defined terrain, stuck/recovery events, pack/drive derating, payload and ground clearance, local repair time and charging access.

### 7. Emergency/resilience vehicle — **emergency-resilience-vehicle**

**Package.** Serviceable response body, protected emergency equipment and clearly separate driving, stationary-work and export-power configurations.

**Drive and energy.** Maintain an explicit drive reserve before any power export; separately meter vehicle auxiliaries and delivered AC/DC loads. Optional bidirectional transfer is an isolated subsystem, not a casual accessory outlet.

**Service and safety.** Export-power mode requires qualified isolation, grounding/bonding, overcurrent protection, anti-islanding where relevant, load compatibility, safe connection and operator training. Do not use a moving vehicle as a site source without an engineered interface.

**Primary tests.** Response hours, critical kWh delivered, reserve after export, restart/recovery, electrical fault injection and safe disconnection.

### 8. Municipal work truck — **municipal-work-truck**

**Package.** Commercial rolling chassis with body modules such as sweeper, refuse support, utility service or small hoist. Module weight and its center-of-gravity/axle effects remain explicit.

**Drive and energy.** Provide a separately metered auxiliary/work-power bus. Electrify hydraulic pumps, brush drives and refrigeration as appropriate, while budgeting their simultaneous peaks and thermal duty.

**Service and safety.** Work-tool interlocks, manual safe recovery, body isolation, service access and municipal wash-down requirements. Emergency/work lamps, controls and the body system are part of the total electrical load.

**Primary tests.** Work jobs/shift plus travel, kWh/work cycle, accessory peak, noise and exposure, downtime and repair time.

### 9. Smallholder tractor and implement carrier — **smallholder-tractor-carrier**

**Package.** Compact low-speed chassis with protected traction modules, implement mounting, optional front/rear tool interfaces and safe ballast attachment. Keep implement fitment and weight limits configuration-controlled.

**Drive and energy.** Declare traction, PTO and hydraulic demands separately. Use a measured or calibrated implement cycle; field energy cannot be inferred from travel distance alone. Depot charging or a shared farm buffer are baselines.

**Service and safety.** Dust, mud, wash-down, roll-over protection, operator egress, implement entanglement and emergency stop are specialized hazards. Avoid swappable-pack assumptions unless safe handling and the pack ecosystem are proved.

**Primary tests.** Hectares/task, drawbar/PTO work, energy per implement cycle, soil/grade conditions, operator shift completion and repair skill requirements.

### 10. Medium row-crop tractor — **medium-row-crop-tractor**

**Package.** Larger axle/frame, implement electrical/hydraulic bus, suitable cooling and wheel/ballast options. Validate front/rear axle loading across implements.

**Drive and energy.** Compare larger battery, exchange, depot/opportunity charge and direct-supply corridors on identical seasonal workload. Report battery-side residual energy if direct supply is used.

**Service and safety.** Seasonal peak usage and weather windows dominate; availability and repair logistics during planting/harvest count as hard service constraints. High continuous torque needs thermal mapping.

**Primary tests.** Hectares completed within weather window, continuous drawbar/PTO power, energy/ha, traction, battery aging, downtime and seasonal service cost.

### 11. Compact construction excavator — **compact-electric-excavator**

**Package.** Electric traction, electric hydraulic pump and work circuit, protected charge inlet, cooling sized for operation at low travel speed and a stable lifting envelope.

**Drive and energy.** Model traction and excavation/lifting separately; high hydraulic demand can occur while the chassis is nearly stationary. Opportunity charge is useful only if a reliable dwell window exists.

**Service and safety.** Hydraulic and electrical lockout must both be considered. Validate emergency lowering, isolation, lifting stability, thermal behavior under repeated heavy cycles, and safe recovery of a disabled machine.

**Primary tests.** Cubic metres moved, work cycles/hour, hydraulic kWh/cycle, peak and continuous thermal headroom, noise/exhaust exposure and charging downtime.

### 12. Urban distribution truck — **urban-distribution-truck**

**Package.** Medium commercial chassis with load body, safe delivery access and configurable refrigeration/auxiliary package.

**Drive and energy.** Regenerative braking can recover only the energy available from actual deceleration and within battery acceptance limits. Use route stop-start, payload and weather data; coordinate fleet charging under a site import cap.

**Service and safety.** Consider curbside charging, delivery access, low-speed pedestrian alerts, service body attachment and nighttime repair exposure.

**Primary tests.** Tonne-km and deliveries/shift, energy at payload, missed routes, payload retained, charger concurrency and maintenance availability.

### 13. City bus — **city-bus**

**Package.** Passenger-capacity platform, HVAC, doors, step-free access, passenger information and safe emergency egress all contribute to mass and loads.

**Drive and energy.** Compare depot charging with terminus opportunity charging only against the same timetable. HVAC and passengers produce strongly time-varying loads; charging-window misses count as service failures.

**Service and safety.** Accessibility, safe evacuation, fire/thermal event response, clear communication and recovery to depot are separate qualification streams.

**Primary tests.** Passenger-km, on-time trips, hot/cold HVAC energy, passenger loading, missed-charge sensitivity and service recovery.

### 14. Long-haul heavy truck — **long-haul-heavy-truck**

**Package.** Heavy-duty frame, axle/motor/retarder configuration, payload-safe battery placement, thermal package, trailer interfaces and serviceable high-power charge inlet.

**Drive and energy.** Model payload, grade, headwind, speed, auxiliary loads and legal/operational dwell. MCS-class charging is an interface candidate only where both vehicle and infrastructure are qualified and the real charge curve supports the required dwell.

**Service and safety.** Validate braking on long descents, thermal endurance, high-voltage isolation, connector cooling, coupling ergonomics and highway recovery.

**Primary tests.** Tonne-km, payload after pack mass, route completion, grade energy, charge dwell at battery temperature/SOC, availability and life-cycle cost.

### 15. Port/yard terminal tractor — **port-yard-terminal-tractor**

**Package.** Low-speed high-torque chassis, high-cycle hitch interface, operator visibility and protected electrical/mechanical modules.

**Drive and energy.** Fixed routes may support opportunity charging and/or trolley; charge strategy must include simultaneous fleet demand and queueing. High starting torque and frequent starts should be modeled separately from travel.

**Service and safety.** Hitch, trailer movements, dust/salt/corrosion, night operation, automated connection and manual recovery require port-specific validation.

**Primary tests.** Moves/shift, kWh/move, productive availability, charger queue, hitch duty, corrosion repair and safe interactions with people.

### 16. Underground mine mobile unit — **underground-mine-mobile-unit**

**Package.** Mine-qualified frame and protected energy system, high-torque drivetrain, safe charging/service zones and route-aware thermal/ventilation accounting.

**Drive and energy.** Compare battery-only, opportunity/DC charge, swap and trolley/cable concepts on the same mine cycle. The catalog uses battery energy after assumed direct supply only as a synthetic boundary; replace it with a measured route model before comparison.

**Service and safety.** Battery/fire strategy, emergency egress, ventilation, cooling, roadway grade, water/dust, communications loss and machine recovery must be co-designed with mine specialists. This catalogue grants no underground-use qualification.

**Primary tests.** Tonnes/hour, ventilation/cooling energy, production lost to charging/faults, battery degradation, emergency recovery time and mine-life cost.

### 17. Open-pit haul truck: battery-only control — **open-pit-haul-battery-only-control**

**Package.** Large heavy-duty haul chassis with battery mass and gross/axle limits explicitly counted.

**Drive and energy.** The fixture intentionally requests more battery-side shift energy than the derated pack can supply after the reserve. It should fail the energy gate, even though power and charge-window arithmetic may pass. This is a negative control, not a conclusion that battery-only haul trucks are always infeasible.

**Service and safety.** Independent haul-road, braking, tire, thermal, fire response and mine-safety engineering remains mandatory.

**Primary tests.** The screen must report blocked and name the energy shortfall; changing input order must not alter the result.

### 18. Open-pit haul truck: trolley plus battery — **open-pit-haul-trolley-battery**

**Package.** Same mission/output and mass assumptions as the battery-only control, with a declared trolley corridor, high-power receiver, contact transition and battery for off-wire operation.

**Drive and energy.** Battery energy is the residual after direct-supply segments, not total route energy. Model trolley length, route coverage, queueing, grade, speed, current collection, outages and off-wire recovery before drawing any cost or productivity conclusion.

**Service and safety.** Electrical clearances, contact-system failure, route-side recovery, maintenance access and loss-of-trolley behavior need specialist design.

**Primary tests.** Paired useful work, net battery energy, battery mass, trolley utilization, uptime, fault recovery and total infrastructure lifecycle cost.

### 19. Warehouse/yard autonomous carrier — **warehouse-autonomous-carrier**

**Package.** Low-speed drive module, payload interface, safe sensing/control envelope, protected pack, accessible service panels and robust automatic charge/dock target.

**Drive and energy.** Predictable routes make scheduling possible, but concurrent fleet arrival creates queues. Energy planning must reserve a safe return/recovery path; do not dispatch based on nominal SOC alone.

**Service and safety.** Define safe stop under sensor, localization, network and control faults; account for blocked lanes and manual recovery; protect people working in the same zone.

**Primary tests.** Tasks/hour, Wh/task, charger queue, safe-stop test coverage, recovery time and availability.

## Pairwise design selection rules

- **Battery-only vs direct supply:** compare the same payload and useful work, and include direct-supply route coverage and infrastructure. Do not count trolley electricity twice as both battery charge and delivered traction.
- **Fixed vs swappable pack:** account for all packs, charger utilization, handling equipment, pack aging, safety, downtime and standardization costs.
- **Depot vs opportunity charge:** replay the actual schedule with charger failure and queueing; nominal window duration is not guaranteed availability.
- **AC vs DC/hybrid distribution:** calculate the full site chain and protection/voltage requirements; don't presume DC wins on all scales.
- **Passenger vs goods platforms:** keep functional units distinct and include accessibility/passenger service or payload delivery, not kilometres alone.
- **Road vs off-highway:** never transfer road duty-cycle coefficients unmodified to tractors, excavators or mine equipment.
- **Autonomy:** safety behavior and operational capability are separate gates from energy performance.
- **Regionalization:** South African rural, municipal and mine configurations should use local route, climate, tariff, grid reliability, service/parts and regulatory evidence; absent evidence means indeterminate, not a fabricated local baseline.

## Release gate

No reference variant advances from concept envelope to build definition until its mission data, architecture boundary, mass/power/thermal model, interfaces, safety hazards, service plan, credible bill of materials and test plan are reviewed. A model passing v0.1 means only that it passed a limited arithmetic screen on the declared input values.