# LC-02 Cargo Van / Chassis Cab — engineering design brief v0.1

**Programme:** Luminous Dynamics EV family  
**Status:** concept engineering brief; not released CAD, production drawings or validated performance  
**Date:** 28 November 2026  
**Tracking:** #7892; platform-family architecture #7888 / draft PR #7889

## 1. Design intent

LC-02 is the first substantial demonstrator for the Luminous Light Commercial Platform (LCP). It must be a useful commercial vehicle before it is a technology showcase.

The primary design objective is **payload-bearing daily work per rand of vehicle and battery capital**, under a known duty cycle and independently qualified battery, route, charging and body configuration.

The lead product is a rear-drive 3.5 t-class electric cargo van with a converter-ready chassis-cab derivative. It must support:
- urban parcel and FMCG delivery;
- service trades and mobile maintenance;
- refrigerated delivery;
- dropside/flatbed and municipal work;
- later specialist body conversions such as tipper, ambulance and workshop.

The vehicle should remain repairable and converter-friendly. Do not start with a bonded structural battery or X-by-wire corner modules.

## 2. Concept package envelope

All dimensions below are initial packaging targets for a parametric model. They are not final hard points and must be checked against occupant packaging, crash zones, suspension travel, cargo geometry, turning circle and applicable homologation requirements.

| Parameter | Initial design envelope | Design rationale |
|---|---:|---|
| Vehicle class | N1 / target GVM class up to 3,500 kg | Keeps initial commercial vehicle in the light-duty fleet segment; local approval mapping required |
| Overall length | 4,600–5,200 mm | Compact/standard van; long-wheelbase variants require separate overhang checks |
| Overall width, excluding mirrors | 1,850–2,000 mm | Urban access and cargo width |
| Overall height | 1,900–2,300 mm | Standard/high-roof alternatives |
| Wheelbase | 2,900–3,400 mm | Standard and long-wheelbase package |
| Front/rear track | Parametric, approx. 1,600–1,700 mm study window | Suspension, wheelhouse and cargo-floor relationship |
| Ground clearance | 160–190 mm nominal study window | Mixed urban, service-road and uneven-road use; underbody loaded clearance must be measured |
| Cargo volume | 4.8–7.0 m³ depending body | Comparable to current small/medium urban vans while offering conversion range |
| Cargo floor height | Target 600–700 mm | Loading ergonomics balanced against underfloor pack, rails and e-axle |
| Turning circle | Target ≤ 11.5 m kerb-to-kerb | Urban access; verify against wheelbase and steering limits |
| Drive | Rear e-axle, 4×2 | Commercial traction and tow/load capability |
| Electrical architecture | 400 V class | Common light-platform voltage family |
| Nominal pack | 55 kWh and 70 kWh variants | Mission-based sizing, not maximum-range packaging |
| Motor | 100–130 kW peak concept class | Final peak/continuous split from grade, start, thermal and tow cases |
| AC charging | 11 kW target | Depot/night charging |
| DC charging | 150 kW peak-system design study; not a promise | Cell acceptance, temperature and charger availability determine achievable curve |
| Charge target | 10–80% in ≤30 min at qualified warm-pack conditions | Competitive target; publish full curve, not only headline minutes |

### 2.1 External anchors

- MAXUS South Africa's eDeliver 3 official page lists 50.23 kWh, 250 km WLTP combined range, 4.8 m³ cargo volume, 945 kg maximum payload and 22.4 kWh/100 km WLTP combined consumption. These are manufacturer-published specifications and a local product anchor, not loaded local-route observations.
- Kia's PV5 Chassis Cab release lists 51.5/71.2 kWh battery options, 89.4/120 kW, DC 10–80% in 30 minutes, and up to 1,005 kg unfinished payload. Its unfinished chassis-cab payload must not be compared directly to a finished van payload.
- The Luminous target is not to copy either vehicle. It is to make the completed body, payload definition, energy boundary, repair access and operating-cost evidence more transparent.

## 3. Layout and packaging logic

### 3.1 Longitudinal zones

1. **Front crash / service zone:** steering, front suspension, braking hardware, HVAC evaporator, low-voltage distribution, service access and crash energy management. Do not place non-serviceable HV joints in the primary crush zone.
2. **Cabin zone:** two- or three-seat cab; upright commercial seating; protected occupant cell; accessible egress; low-glare instrument cluster; physical controls for demist, hazard, emergency isolation information and cargo functions.
3. **Underfloor energy zone:** bolted battery enclosure between longitudinal chassis rails, protected by replaceable underbody shields. Preserve removal path, lifting points, vent path and coolant service.
4. **Rear drive zone:** rear e-axle, reduction gear, inverter as a serviceable assembly or separately replaceable unit, half-shafts and rear suspension. Verify service access with the body fitted.
5. **Cargo zone:** low, flat load floor; wide side/rear apertures; flush tie-downs; wheelhouse intrusion minimized; body converter mounting rails and declared centre-of-gravity envelope.

### 3.2 Battery service layout

First-generation layout:
- one sealed, bolted pack enclosure or two independently removable pack sections, selected after crash/load-path and service-access modelling;
- no routine cell-level service by unqualified workshop personnel;
- pack isolation and HVIL at service boundaries;
- separate fuse/contactor protection and monitored zones;
- venting designed away from cabin and high-voltage harnesses;
- coolant, electrical and sensor interfaces visible and replaceable;
- a safe pack-removal path with body mounted and a separate process for conversions that obstruct removal;
- lifting/tooling requirements specified before underbody structures are frozen.

A modular pack must not create unsafe parallel packs with uncontrolled state of charge or fault propagation. Series/parallel topology, pre-charge, contactor sequencing, current sharing, isolation monitoring and fault containment are detailed electrical design tasks.

### 3.3 Body / chassis interface

Create a Luminous Body Interface (LBI) with fixed mechanical datums and a signed body manifest.

Mechanical interface must define:
- datum coordinate frame and mounting coordinates;
- approved fasteners, grades, torque and witness evidence;
- allowable vertical, lateral, longitudinal and torsional mounting loads;
- body mass and centre-of-gravity envelope;
- permitted roof, side and rear load paths;
- axle load, tyre, frame and GVM restrictions;
- no-drill/no-weld prohibited zones around pack, HV, crash and restraint structures.

Electrical interface must define:
- 12 V and 48 V protected distribution;
- auxiliary loads by continuous and peak rating;
- isolated/interlocked HV access only for approved refrigeration/PTO applications;
- separate measurement boundary for traction, cabin HVAC, refrigeration, liftgate and body electronics;
- safe states on body disconnect, communication loss or crash event.

Data interface must define:
- converter identity and body model;
- software and firmware versions;
- CAN FD/Ethernet signal catalogue, message ownership and diagnostic behaviour;
- authenticated, versioned body configuration;
- exact vehicle/body serial pairing.

Converters must not be able to override traction, brake, steering or HV safety control through an undocumented interface.

## 4. Cargo is an energy system

The cargo body is not a passive mass. A refrigerated box, liftgate, hydraulic tipper, compressor, power tool system or mobile workshop can change route feasibility.

Measure each energy domain independently:
- traction battery DC energy;
- inverter/motor losses where available;
- cabin HVAC and thermal conditioning;
- refrigeration compressor, fans and defrost;
- liftgate / hydraulic / PTO duty;
- 12/48 V body loads;
- DC charging energy and wall/EVSE meter boundary.

A vehicle-range model must include body energy:
`E_mission = E_traction + E_cabin + E_body + E_auxiliary + E_losses`

The vehicle must not use traction range based only on chassis energy while hiding body power demand in an unmetered accessory channel.

### 4.1 Refrigeration design opportunity

Treat the refrigerated body as a thermal service with its own duty, not just a compressor. Research:
- high-efficiency variable-speed DC compressor and fans;
- insulated body and door-opening leakage reduction;
- thermal storage / phase-change material (PCM) preconditioning;
- depot pre-cooling while plugged in;
- thermal-state prediction from planned route and door events;
- independent body energy budget with a reserve policy;
- safe thermal fallback if the body controller or vehicle data link is unavailable.

Do not assume PCM or pre-cooling saves energy. Compare measured kWh per delivered tonne-hour at specified setpoint, ambient, door-opening frequency and cargo thermal load.

**Illustrative sensitivity only:** a 2 kW average refrigeration load over an eight-hour shift consumes 16 kWh. In the 70 kWh example below, the traction range would fall from roughly 179 km to about 120 km if that 16 kWh is drawn from the same trip-energy window and all other assumptions remain unchanged. This is not a field consumption estimate; it shows why auxiliary measurement matters.

## 5. First-pass battery/range sizing

Use this model only for initial sizing:

`E_trip = E_nominal × 0.92 × (0.90 − 0.15)`

`R_est = 100 × E_trip / e_duty`

The 0.92 pack-usable factor and SOC 90% to 15% window are provisional design assumptions. Real usable energy depends on cell, pack architecture, temperature, ageing, BMS limits and power demand.

| Pack | Assumed loaded duty energy | Trip energy window | Calculated distance before reserve |
|---:|---:|---:|---:|
| 55 kWh | 27 kWh/100 km | 38.0 kWh | 141 km |
| 70 kWh | 27 kWh/100 km | 48.3 kWh | 179 km |
| 70 kWh | 30 kWh/100 km | 48.3 kWh | 161 km |
| 70 kWh | 34 kWh/100 km | 48.3 kWh | 142 km |

The correct output is not a range number but a route-feasibility interval under uncertainty. Simulate gross mass, stop-start patterns, grade, speed, ambient temperature, wind, rain/wet tyres, tyre pressure, HVAC, refrigeration, payload distribution and battery age. Report the P05/P50/P95 energy requirement and remaining reserve; return INDETERMINATE when the route or vehicle state is outside the qualified envelope.

## 6. Mass and payload budget

For each body:
`payload_available = GVM_limit − curb_mass_as_built − occupants − options − conversion_equipment`

The mass model must contain:
- bare chassis/body-in-white and closures;
- battery including coolant, enclosure and service hardware;
- motor, inverter, reduction drive and shafts;
- suspension, brakes, wheels, tyres and steering;
- glass, seats, trim, HVAC, wiring and 12/48 V system;
- crash structure, bumpers, underbody shields and towing equipment;
- production fluids and tools;
- driver, passengers and accessories using the legally applicable payload definition;
- each conversion body, refrigeration unit, liftgate, shelving, spare wheel, roof rack and tool fit-out.

Required outputs:
- total mass and margin to GVM;
- front/rear axle mass and margin;
- left/right wheel-load distribution;
- CG location and height for empty, rated payload and worst-case asymmetric loading;
- tyre, wheel, suspension, brake and frame limits;
- payload after final body conversion.

No marketing or engineering payload figure is valid until the definition and finished configuration are fixed. A 1,005 kg unfinished chassis-cab number from another OEM cannot serve as a finished-product benchmark without converting the boundary.

## 7. Performance and duty requirements

| Requirement | Initial target / gate | Verification |
|---|---|---|
| Rated GVW class | 3,500 kg concept ceiling | Approved mass schedule, axle/tyre/brake verification |
| Finished van payload | 900 kg design goal; no commitment until mass budget closes | Scales and axle pads on production-representative vehicle |
| Cargo volume | ≥4.8 m³ standard body; target 6–7 m³ high-roof/long-wheelbase | 3D envelope and physical volume method |
| Loaded grade launch | 20% grade start at rated mass target | Repeated hot/cold launch and parking-brake test |
| Sustained grade | 8–10% route case at rated mass, thermally sustainable | Instrumented hill route and repeatability |
| Loaded duty range | 160–180 km target for 70 kWh pack under defined route assumption | Independent DC energy meter and fixed route/load/weather record |
| Charge | 10–80% ≤30 min under defined warm-pack conditions | DC charger/vehicle meter, cell temp and charge curve |
| Payload safety | Zero GVM/axle/tyre over-limit | Empty, rated and asymmetric load tests |
| Braking | Stable ABS/ESC and regen blend at empty/full load | Instrumented proving-ground test |
| Thermal | No cell/motor/inverter derating that violates declared mission envelope | High-ambient repeated-duty test |
| Service | Pack isolation/removal and e-axle access without damaging body structure | Timed service demonstration with repair manual |
| Body integration | No unsafe state on comms loss/disconnect; measured body load | Hardware-in-loop and physical converter test |

The motor continuous-power rating, gear ratio, gradeability and towing capacity must be sized together. Peak power alone is an insufficient vehicle specification.

## 8. Variant matrix

| Variant | Body / mission | Additional design gates |
|---|---|---|
| LC-02A Panel Van | Parcel, FMCG, trades | Cargo tie-downs, access, floor durability, driver visibility |
| LC-02B High Roof | Bulky parcel / mobile service | Centre-of-gravity, crosswind, height limits, roof structure |
| LC-02C Refrigerated | Cold-chain | Insulation, door leakage, thermal recovery, body energy metering |
| LC-02D Dropside | Construction, agriculture, materials | Concentrated floor loads, side-gate latches, CG, tie-downs |
| LC-02E Tipper | Municipal/aggregate | Hydraulic/PTO energy, frame torsion, payload and interlocks |
| LC-02F Workshop | Tools, compressor, bench, auxiliary power | Body power, ventilation, secure equipment, mass distribution |
| LC-02G Ambulance | Specialist later derivative | Occupant restraint, medical equipment, crash, power continuity and regulatory scope |

LC-03 bakkie is not an LC-02 conversion. It uses common electronic/power modules only where justified and has separate rear frame, suspension, payload bed, towing, gravel, water/dust and occupant-body engineering.

## 9. Validation and evidence plan

### Digital
- Parametric CAD and mass-properties model with revision and configuration digest.
- Structural FEA for frame, body mounts, pack protection, torsion, roof/side and local loads; model correlation plan.
- Thermal-fluid model for pack and drive cooling with pressure-drop and coolant-flow budgets.
- Longitudinal vehicle model for route energy, grade, acceleration, regen and auxiliary loads.
- Charging model with cell acceptance, temperature, SOC and charger power.
- Body-interface HIL with comms, disconnect, fault, emergency and configuration mismatch cases.

### Prototype
1. Rolling chassis and serviceable pack mule.
2. Standard panel van body.
3. Fully instrumented mass/payload and axle-weight test.
4. Repeatable loaded urban-route, highway and grade campaigns.
5. High-ambient soak and repeated-stop/start duty.
6. Fast-charge curve and repeated-charge thermal campaign.
7. Dust, water, gravel, vibration, fatigue and corrosion exposure.
8. Braking, ABS/ESC, regenerative transition and full/empty payload dynamics.
9. Converter-body HIL and physical functional test.
10. Teardown, service-time and material-separation audit.

Every evidence record must bind vehicle and pack serials, body configuration, software/firmware, tyres, test load, route, weather, charger, meter, calibration, time alignment, procedure revision, raw data digest and independent witness/authority where needed.

## 10. Decision gates and known risks

| Risk | Mitigation / evidence gate |
|---|---|
| Battery pack consumes too much payload | Close a full as-built mass and axle budget before frame freeze |
| Cargo floor too high due pack | Optimize pack/rail depth and loading ergonomics; do not remove crash protection to chase floor height |
| Refrigeration destroys range economics | Independent metering, body thermal storage study and mission-specific derating |
| Chassis-cab body overload | Converter manifest and post-build axle scales; fail closed on missing mass |
| RWD traction insufficient on low grip | Loaded wet/gravel launch and grade tests; only add front drive if data justifies |
| Charging power exceeds pack acceptance or site power | Co-design pack curve, charger, grid and mission dwell |
| Highly integrated pack cannot be repaired | Preserve bolted enclosure/subpack boundaries in v1; measure repair/recovery time |
| Claims are based on simulated range only | No field range claim without independently metered route energy |
| Body software creates safety coupling | Signed configuration, restricted signal ownership and fail-safe state tests |

## 11. Release sequence

1. Freeze requirements, coordinate system, GVM definition, duty routes and payload semantics.
2. Build parametric LC-02 CAD with low/standard/high roof and wheelbase configurations.
3. Close a 55/70 kWh pack mass-volume-removal trade study.
4. Close rear e-axle, suspension, brake and cooling package.
5. Freeze LBI mechanical and electrical interfaces before first converter body.
6. Prototype standard panel van first; measure mass, energy and service access.
7. Add refrigerated body only after the body-energy metering and reserve policy exists.
8. Derive LC-03 bakkie from its own load-path and suspension package after LCP modules have evidence.

## 12. Claim ceiling

This document defines a concept package and validation plan. It is not CAD geometry, a released drawing, a costed production BOM, a vehicle specification, a range guarantee, a payload guarantee or evidence of homologation. No model, body conversion or operating envelope is promoted beyond concept status until the exact built configuration passes the required physical tests and independent review.
