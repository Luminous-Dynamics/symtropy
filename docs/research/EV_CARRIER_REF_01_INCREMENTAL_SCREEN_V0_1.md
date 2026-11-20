# EV-CARRIER-REF-01 — incremental logistics-carrier integration profile v0.1

**Status:** reference-integration and synthetic-screen contract  
**Primary product owner:** Symtropy [EV-CARRIER-REF-001 #6109](https://github.com/Luminous-Dynamics/symtropy/issues/6109)  
**Base vehicle owner:** [EV-PLATFORM-001 #2112](https://github.com/Luminous-Dynamics/symtropy/issues/2112) and [EV-VALIDATION-001 #2114](https://github.com/Luminous-Dynamics/symtropy/issues/2114)  
**Benchmark / operating owners:** [MFB-1K #5422](https://github.com/Luminous-Dynamics/symtropy/issues/5422), [carrier/UAV co-availability #6083](https://github.com/Luminous-Dynamics/symtropy/issues/6083), [3D cargo / release / resupply #6147](https://github.com/Luminous-Dynamics/symtropy/issues/6147), [mode options #6153](https://github.com/Luminous-Dynamics/symtropy/issues/6153)

## 1. Product decision and scope

EV-CARRIER-REF-01 does not create a new vehicle family. It specializes and composes two existing synthetic reference designs:

- `urban-distribution-truck` — medium commercial chassis with load body; platform family `commercial-and-municipal-trucks`.
- `light-commercial-van` — common front drive/electrical module with cargo or service body; platform family `light-commercial-utility`.

The initial engineering question is whether a dedicated autonomous-logistics module creates enough useful-service value to justify its mass, cargo displacement, power/energy demand, charging infrastructure, maintenance, production complexity and regulatory burden.

The product shall retain the existing base design's identity, mission, battery, traction, charging and evidence lineage. A carrier variant may only claim shared chassis, battery, body or electrical interfaces when mass, load paths, axle, thermal, voltage, environmental, service and safety compatibility have evidence. Shared software interfaces do not establish physical interchangeability.

The first flight-system architecture assumes the carrier is stationary for launch and recovery. Moving-truck launch/recovery, rendezvous with a moving carrier and en-route operations are separate research and physical-qualification tiers.

## 2. Existing synthetic base envelopes

The following arithmetic is extracted from the current EV-PLATFORM-001 fixture and existing `vehicle_design_screen`; it is intentionally labelled **synthetic**.

| Base design | Urban distribution rigid truck | Light commercial van |
|---|---:|---:|
| Base curb mass | 11,000 kg | 2,700 kg |
| Installed base module | 1,200 kg | 250 kg |
| Target payload | 8,000 kg | 850 kg |
| Declared maximum gross mass | 26,000 kg | 4,000 kg |
| Loaded mass in base screen | 20,200 kg | 3,800 kg |
| Arithmetic gross-mass margin | 5,800 kg | 200 kg |
| Nominal pack energy | 420 kWh | 95 kWh |
| SOH × temperature × reserve factors | 0.95 × 0.90 × 0.82 | 0.95 × 0.90 × 0.85 |
| Usable energy after stated reserve | 294.462 kWh | 69.04125 kWh |
| Synthetic shift battery demand | 230 kWh | 52 kWh |
| Arithmetic battery-energy margin | 64.462 kWh | 17.04125 kWh |
| Continuous power rating / demand | 240 / 200 kW | 90 / 65 kW |
| Peak power rating / demand | 420 / 330 kW | 180 / 150 kW |
| Depot site budget | 500 kW | 66 kW |
| Concurrent base EVs | 2 | 3 |
| Effective charger per vehicle | 250 kW | 22 kW |
| Base site load at concurrent allocation | 500 kW | 66 kW |

These figures are **not** available pod mass, payload certification, axle/CG margin, actual usable range, measured efficiency or charger acceptance. In particular, the truck's 5,800 kg and van's 200 kg are arithmetic slack in synthetic envelopes; they cannot be spent on a pod without verified as-built configuration, axle/wheel loads, CG, body/load path and applicable vehicle limits.

The 500 kW truck and 66 kW van site allocations have no arithmetic headroom in the base concurrent-charging scenario. Any drone charging from the site must therefore be scheduled in a noncoincident window, backed by a qualified larger grid connection / buffer, or charged through an independently metered vehicle-battery path with its conversion loss counted. No free energy is assumed.

## 3. Architecture tournament

The product comparison must retain distinct operating architectures:

| Variant | What it is | Primary trade-off to measure |
|---|---|---|
| Truck-only control | Existing EV doing road distribution | Baseline service, energy, cost and depot load |
| Integrated enclosed bay | Aircraft and handling fixtures installed within cargo body | Protection/serviceability versus lost cargo volume, mass, CG and fire/thermal integration |
| Detachable drone pod | Reusable module transferred between carriers and depots | Asset sharing and variant flexibility versus pod mass, connection, transfer and idle/repositioning cost |
| Mobile drone station trailer | Truck transports and deploys trailer-based drone station | Temporary hub coverage and truck independence versus trailer coupling, mass, stability, site setup and extra assets |
| AMR handoff | Truck transfers totes to ground robots / receivers | Weather-independent ground handoff versus curb/terrain accessibility, ground fleet and handling resources |
| Fixed drone hub | Separate depot/store/hub aircraft service | Dense-demand scale and stable infrastructure versus fixed-site capex and coverage limitations |

A fixed hub is a network baseline, not an incremental truck module, so it is evaluated in MFB/mode-options rather than the carrier mass-delta CLI.

### Competitive novelty boundary

A mobile drone station trailer is already present in published 2026 literature. Shi, Hong and Li's September 2026 *Transportation Research Part E* paper formulates TSP-MDST: a truck carries a trailer with multiple drones, deploys it as a temporary drone station, and may either wait or continue serving customers while drones operate. It uses a MILP and a routing-structure-guided ALNS. The existence of that architecture means “truck + mobile drone station” alone is not a novel Luminous claim. Benchmark against it directly: https://doi.org/10.1016/j.tre.2026.104975.

The more differentiated research contribution must be the **integrated product and operating system**: EV mission/pack/charger co-design, physically validated cargo state and accessibility, multiple drone and resupply roles, multi-truck rendezvous, demand-adaptive activation, depot/grid planning, repairable manufacture and evidence-qualified real-world feedback.

## 4. Cross-domain interfaces

### Mechanical and cargo
- Base vehicle configuration and as-built identity.
- Pod/trailer mass, dimensions, centre of mass and mounting/load-path geometry.
- Gross vehicle mass, axle/wheel loads, wheelbase, CG and suspension envelope after each configuration.
- Cargo volume displacement, doors/hatches, restraint points, package access and extraction sequence.
- Retention under braking, cornering, vibration, impact and emergency manoeuvres.
- Trailer coupling, braking, stability and articulation for the trailer variant.
- Human/robot separation, emergency egress, safe access and maintenance clearance.

The MFB cargo validator currently checks synthetic axis-aligned boxes, non-overlap, straight-line extraction and a supplied mass/CG envelope. It is not an axle-load, restraint, crash or robotic reachability qualification. Those owners must supply qualified envelopes and physical evidence.

### Electrical and energy
Keep these boundaries distinct:

1. **Traction battery terminal:** incremental energy withdrawn from the EV traction pack, including vehicle-to-UAV converter losses.
2. **UAV battery terminal:** phase-resolved mission energy, reserve and pack capability supplied by the aircraft owner.
3. **External site meter:** energy supplied by a depot/site charger, including charger losses.
4. **Depot/site peak:** time-aligned vehicle chargers, UAV chargers, AMR chargers, other loads, storage and grid import caps.

A traction-battery withdrawal is counted once as EV mission demand and is replenished via the EV charger model. Do not count the same energy again as external UAV site energy. External site-meter energy is a separate source boundary. The initial incremental screen implements this separation.

Power must be separated into continuous and peak vehicle-bus loads. The v0.1 screen adds declared concurrent module power to base mission demand; it does not yet model a detailed time-series overlap. Actual qualification requires synchronized duty-cycle profiles and thermal derating.

### Thermal, fire and environmental
- Thermal exposure and cooling paths for aircraft batteries, EV battery, charger, inverter and pod interior.
- Independent fault isolation and energy disconnection.
- Battery fire/thermal propagation, venting, heat/smoke paths and occupant/cargo exposure.
- Rain, dust, corrosion, vibration, wash-down and ambient extremes.
- Emergency service / responder access and clear module removal.

No thermal runaway containment or fire-safe status is claimed by the synthetic screen.

### Data, trust and maintenance
- Vehicle, pod, aircraft, charger, pack and software/calibration configuration identity.
- Signed service/inspection/maintenance records, currentness and revocation.
- Cargo manifest, custody and proof of handoff through Mycelix.
- Candidate site/route context through Sol-Atlas; geospatial suitability is not airspace permission.
- Safe-state, watchdog, communications-loss and manual recovery contracts.
- OTA/update, cybersecurity and rollback qualification with the existing vehicle owners.

## 5. Incremental screening semantics

The new `ev_carrier_configuration_screen` consumes the existing `vehicle_design_screen` report and adds only deltas:

- `incremental_installed_mass_kg`: aircraft, pod, dock, fixtures and all other masses not already in base design.
- `incremental_payload_mass_kg`: only cargo not already inside the base target payload; packages moved from truck to drone must not be double counted.
- `incremental_vehicle_battery_draw_kwh`: battery-terminal energy taken from EV pack, including conversion losses.
- `incremental_vehicle_bus_continuous_power_kw` and `incremental_vehicle_bus_peak_power_kw`: concurrent additional load.
- `incremental_external_site_meter_energy_kwh` and `incremental_external_site_peak_power_kw`: separate external charging boundary.
- `external_site_peak_coincident_with_vehicle_charging`: required when an external peak exists; unknown concurrency is indeterminate.
- `incremental_external_site_charging_window_hours`: window for the external charge. The reported charge-time check is energy / peak power, a lower bound only; fitting that bound is not proof of a real charge curve or schedule.

Any unknown mass, energy, power or site-concurrency field remains null and makes the variant indeterminate unless a known hard blocker already exists. No unknown is zero-filled. The screen does not output an architecture ranking.

## 6. SOTA benchmark programme

Benchmark the **same useful service**, not merely a drone flight or route distance.

### Routing and synchronization
- Reproduce the published TSP-MDST mobile trailer problem, MILP and RSG-ALNS where instances/code/rights permit.
- Reproduce multi-visit truck-drone pickup/delivery with release dates, lockers and distinct resupply-drone type (Liang et al., EJOR 2026): https://doi.org/10.1016/j.ejor.2026.10.020.
- Reproduce multi-rendezvous resupply where one resupply flight can serve multiple trucks (Tran et al., TR-E 2026): https://doi.org/10.1016/j.tre.2026.105082.
- Treat moving-truck en-route launch/recovery and heterogeneous multi-visit aircraft (Liu et al., TR-E 2026) as a later, separately qualified tier: https://doi.org/10.1016/j.tre.2026.105114.
- Compare exact small-instance MILP/branch-price methods, LKH-style routing, ALNS, robust/stochastic recourse and rolling-horizon replanning under matched instances, time limits and hard constraints.

### Physical cargo
- 3D packing validity and accessibility.
- Rehandling count/time, loading sequence, package release time and handoff proof.
- Gross mass, axle and CG margin after every pickup/delivery/resupply.
- Multi-drone scheduling with limited pads, crew, chargers, airspace and carrier dwell.
- False-feasible rate and zero silent unknown-to-zero assumptions.

### Energy and fleet economics
- Whole-system kWh per successful authorized on-time delivery, with truck, UAV, AMR, converter, charger and site energy boundaries disaggregated.
- Depot peak import, charging queue, session tails and outage resilience.
- Vehicle/pod/drone fleet utilization and idle/repositioning/activation cost.
- Lifecycle cost per successful delivery including capex, labor/remote supervision, maintenance, battery replacement, failed attempts, insurance, facilities and end-of-life.
- Compare truck-only, fixed-hub, mobile trailer, integrated bay, detachable pod, pooled drones, AMR and mixed operations with identical demand/eligibility/regions/operating windows.

### Factory and service
- Good-unit yield, takt, rework and station bottlenecks.
- Module install/remove time and whether the same module can be serviced across multiple carriers.
- Repair diagnosis time, parts availability, return-to-service and local skills/tooling.
- Variant-specific tooling, inspection and end-of-line functional tests.
- CAPEX per good carrier and total lifecycle cost, using source-qualified supplier/energy/labor data.

### Proposed programme targets (not achievements)
Retain MFB-1K #5422's proposed matched truck-only targets—15% makespan and 10% whole-system operational-energy improvement, with stretch 25% / 18%—only after the benchmark includes return/recovery, reserve, weather, payload, release time, resupply, conversion losses, battery degradation and site charging. Publish per-scenario regressions and confidence intervals; do not hide a worse region or failure class inside a fleet average.

Additional qualification targets:
- zero accepted hard mass/energy/power/site/permission violations in the frozen synthetic corpus;
- exact agreement with exhaustive oracles on tiny routing and loading instances;
- canonical reproducibility for input-order permutations;
- zero false-feasible decisions on authored negative controls;
- held-out regional and demand/weather generalization;
- no claim of SOTA superiority until exact matched paper baselines have been reproduced.

## 7. Manufacturing and South African deployment

The module and carrier should be co-designed with the existing modular/flexible and CKD factory archetypes, not create a new factory planner. The product decomposition should determine pod subassemblies, fixtures, HV test, inspection, pack isolation, final vehicle integration and variant-specific end-of-line tests.

For South Africa, measured route/grade, climate, grid/tariff, site and supplier/repair data must replace the current synthetic profiles before affordability, payload, range or TCO claims. The SACAA's current guidance contains specific applicability and approval questions for autonomous and delivery operations; obtain a written regulator determination on aircraft classification, RLOS/BRLOS, delivery/object release, operator/aircraft/site/airspace approvals, mobile launch/recovery and C2 before any flight or package-release experiment. Ground carrier/pod/charging and cargo fixtures can be qualified separately under controlled low-consequence procedures.

## 8. Current executable increment and acceptance boundary

The incremental screen is wired into the existing rural/industrial EV evaluator workflow. It reads the generated EV design report; two truck-only synthetic controls are expected to pass their declared arithmetic screen, while integrated bay, detachable pod, mobile trailer and AMR handoff alternatives are intentionally indeterminate because the physical module values remain unknown.

This establishes the product/energy/mass interface and identifies the measurements required next. It does **not** establish a CAD design, manufacturing BOM, supplier quote, FEA/CFD, qualified battery/charger, axle loads, restraint, fire protection, crashworthiness, roadworthiness, airworthiness, flight legality, commercial economics or production readiness.

## References

- Shi, Hong & Li (2026), Truck-drone collaborative delivery with a mobile drone station trailer, *Transportation Research Part E*, 213, 104975. https://doi.org/10.1016/j.tre.2026.104975
- Liang, Chen & Du (2026), Multi-visit truck-drone pickup and delivery problem with release dates and drone resupply, *European Journal of Operational Research*. https://doi.org/10.1016/j.ejor.2026.10.020
- Tran et al. (2026), Coordinated truck routing and multi-rendezvous drone resupply under shipment release dates, *Transportation Research Part E*, 215, 105082. https://doi.org/10.1016/j.tre.2026.105082
- Liu, Zhu & Zhang (2026), Heterogeneous drone-truck routing problem with en-route operations, *Transportation Research Part E*, 215, 105114. https://doi.org/10.1016/j.tre.2026.105114
- Paper et al. (2026), GPS-based HD truck electrification and charging load profiles, *eTransportation*, 29, 100616. https://doi.org/10.1016/j.etran.2026.100616
- https://github.com/Luminous-Dynamics/symtropy/issues/2112
- https://github.com/Luminous-Dynamics/symtropy/issues/2114
- https://github.com/Luminous-Dynamics/symtropy/issues/2354
- https://github.com/Luminous-Dynamics/symtropy/issues/2323
- https://github.com/Luminous-Dynamics/symtropy/issues/5422
- https://github.com/Luminous-Dynamics/symtropy/issues/6083
- https://github.com/Luminous-Dynamics/symtropy/issues/6147
- https://github.com/Luminous-Dynamics/symtropy/issues/6153
