# Power Fabric Hybrid AC/DC and Solid-State Roadmap v0.1

**Status:** architecture research; not an electrical design, procurement specification, installation drawing or interconnection approval  
**Date:** 2026-11-14  
**Programme:** [#3400 digital twin](https://github.com/Luminous-Dynamics/symtropy/issues/3400), [#3409 conversion topology](https://github.com/Luminous-Dynamics/symtropy/issues/3409), [#3418 resilience](https://github.com/Luminous-Dynamics/symtropy/issues/3418)  
**Module contract:** [#5159](https://github.com/Luminous-Dynamics/symtropy/issues/5159)  
**Human interaction:** [#5158](https://github.com/Luminous-Dynamics/symtropy/issues/5158)

## 1. Thesis

Build a modular energy platform, not a monolithic solar-plus-battery appliance. The architecture should combine appropriate AC and DC domains, multiple generation/storage technologies, explicit thermal paths, replaceable modules, local protection, and a human-understandable as-built model.

The optimization target is not "maximum percentage of solid-state components". It is:

- delivered service and power quality;
- low lifecycle cost and resource use;
- high availability and graceful degradation;
- safe fault isolation;
- low maintenance burden and short repair closure;
- interoperability and upgradeability;
- understandable operation and professional verifiability.

A component with no moving parts can still be a poor choice if it has high thermal stress, poor repairability, short service life, weak evidence or high replacement cost.

## 2. Reference architecture archetypes

These are conceptual design patterns. Voltage levels, grounding, protection, equipment ratings, conductor sizing and installation details must be selected and verified for a particular site by competent practitioners.

### A. Building-scale hybrid

- Local generation: fixed PV is a natural first candidate where site conditions permit.
- DC conversion: source-specific MPPT and DC/DC interfaces.
- Storage: independently protected battery modules with explicit power and energy limits.
- AC service: inverter / approved grid interface serving AC loads and grid interaction.
- DC service islands: direct DC only for appropriately designed and approved DC-native loads.
- Thermal: passive-first enclosure and heat paths, with active cooling where the measured heat flux requires it.
- Supervisory: display actual source, reserve, operating mode, backup boundary and unresolved state.
- Protection: local electrical protection and physical isolation independent of supervisory compute.

### B. Neighbourhood / campus microgrid

- Multiple independently identifiable source and storage nodes.
- Segmented AC distribution with optional DC sub-networks for suitable native loads.
- Bidirectional conversion at defined boundaries.
- Local protection and island capability for declared segments.
- Multiple grid-forming assets only where their interaction and protection are qualified.
- Building-level service floors and staged restoration.
- Federated monitoring and permission, with local operation during loss of wide-area communications.

### C. Industrial / port / data-centre microgrid

- Separate the service boundary for process equipment, IT loads, cooling, charging and critical auxiliary systems.
- Represent pulsed, nonlinear, motor-starting and high-ramp loads explicitly.
- Bind energy and thermal models to the same timebase and asset/configuration identity.
- Treat compute flexibility as conditional: workload deferral counts only if useful work, SLA, cooling, water, energy rebound and grid-facing behavior are measured.
- Use multi-vendor grid-forming, grid-following and protection interaction as an explicit qualification problem, not a marketing label.

### D. Remote / off-grid / planetary analogue

- Pair intermittent generation with storage selected for the full darkness / weather interval and restart requirement.
- Include start auxiliaries, thermal management, communications and water/service dependencies in the energy budget.
- Compare batteries, regenerative fuel cells and other long-duration stores at matched delivered-service boundaries.
- Preserve a local safe mode when communications, central coordination or high-level AI are unavailable.
- Do not claim a mission-ready solution from synthetic scenarios alone.

## 3. AC/DC hybridization

A hybrid architecture can reduce unnecessary conversions for DC-native sources and loads while retaining compatibility with AC networks and equipment. It also introduces important design obligations: grounding and earthing semantics, fault-current behavior, arc detection, selective protection, DC interruption, pre-charge, insulation monitoring, isolation and multi-vendor coordination.

Prefer a **small number of declared, repeatable interface patterns** over arbitrary converters everywhere. Every AC/DC boundary should be a first-class module in the service graph and should account for conversion loss, auxiliary load, thermal rejection, fault behavior and mode transitions.

An 800 VDC or ±400 VDC archetype may be relevant to selected data-centre or industrial applications, but higher voltage is not automatically more efficient or safer. Evaluate the complete chain at the customer load: conversion, cable, transformer, switchgear, protection, UPS/storage, cooling, auxiliaries and service continuity. Treat ecosystem proposals and product claims as candidates, not as proof of savings.

## 4. Solid-state and no-moving-parts opportunity map

"Motionless core" is not "maintenance-free system". Score each technology on wear mechanisms, thermal stress, degradation, repairability, failure containment, auxiliary dependencies and evidence maturity.

| Component / technology | Motion profile | Opportunity | Residual complexity / caveat |
|---|---|---|---|
| Fixed photovoltaic module | No moving parts | Mature modular generation candidate | Degradation, connectors, seals, soiling, hail, hot spots and mounting still require inspection |
| SiC/GaN converters | Solid-state switching | Compact, efficient conversion at suitable voltage/frequency/current envelopes | Semiconductor thermal cycling, gate drivers, capacitors, EMI, cooling and control stability |
| Grid-forming inverter | No rotating machine required | Local voltage/frequency reference and fast power response | Current/energy limits, controls, protection, weak-grid interaction and system-level qualification |
| Solid-state DC breaker | Semiconductor switching | Very fast fault interruption | On-state losses/heat, surge stress, cost, control dependence; semiconductor off-state is not a visible galvanic isolation gap |
| Hybrid DC breaker | Solid-state plus mechanical path | Combine rapid interruption with lower steady-state conduction loss / physical isolation functions | More components and coordination; exact topology-specific qualification is necessary |
| Solid-state transformer | Power-electronic conversion with magnetic components | Flexible AC/DC conversion and controllable interfaces | High-frequency conversion, cooling, cost, component count, availability and repair complexity |
| LFP / sodium-ion battery modules | No rotating electrochemical core | Modular electrical storage | Electrochemical ageing, BMS, contactors, thermal propagation, enclosure/fire risk and replacement lifecycle |
| Flow battery | Electrochemical stack plus fluid loops | Energy/power scaling separation and serviceable electrolyte path | Pumps, seals, valves, tanks and flow balance are mechanical service items |
| Fuel-cell stack | Motionless electrochemical core | Electricity generation without a rotating prime mover | Air/fuel supply, pumps/compressors, heat, water management, degradation and start/ramp limits |
| Thermoelectric generator | Solid-state conversion | Niche waste-heat conversion without moving parts | Low conversion efficiency and economics depend strongly on temperature gradient/materials |
| Heat pipe / vapor chamber | Passive two-phase heat transport | Passive heat spreading without a pump/fan | Finite heat transport envelope, orientation/gravity effects for some designs, dry-out and integration constraints |
| Phase-change thermal store | Passive thermal storage | Shifts thermal demand without converting it to electricity | Finite capacity, conductivity, encapsulation, cycling and reset/recharge path |
| Fixed-angle solar thermal collector | No tracking required | Simple low-maintenance thermal service | Site/temperature constraints; pumps may still be required depending on the loop |
| Wind turbine / reciprocating generator / turbine | Moving mechanical system | Useful where dispatchable or resource conditions justify it | Bearings, seals, vibration, lubrication, noise, fatigue and planned maintenance |

### Design rule

Prefer passive thermal design, fixed PV, solid-state conversion and modular electrochemical storage where evidence and economics support them. Keep mechanical machinery as optional, replaceable dispatchable modules where its service value justifies the maintenance.

Do not force every subsystem to be solid-state. A mechanical disconnect or hybrid breaker may be the better safety boundary. The architecture should optimize failure containment and lifecycle service, not minimize the count of moving parts in isolation.

## 4.1 Solid-state transformer maturity and hybrid strategy (research refresh — 14 Nov 2026)

Solid-state transformers (SSTs) deserve a serious modularity track, but not a blanket assumption that an SST should replace every conventional transformer.

The IEA's *State of Energy Innovation 2026* describes SSTs as modular power-electronic building blocks that can interface AC and DC, support bidirectional flow, perform voltage/power-quality functions and provide rapid local fault isolation. It also makes the deployment caveats explicit: pilot projects are underway, while cooling and protection robustness, supplier diversity, part-load losses, high cooling demand, power-quality tuning, non-traditional fault behaviour, integration with legacy assets and bankability remain material issues. It notes that hybrid approaches combining conventional transformers with power electronics have been more successful in addressing supply-chain and bankability risk in some pilots.

A June 2026 IEEE Open Journal of Power Electronics article compares DC/DC converter topologies for modular SSTs and reports a 98.55% peak-efficiency result for one experimentally validated case study. That is a specific topology/case result, not a universal whole-SST or delivered-service efficiency figure.

**Power Fabric decision:**
- Keep conventional transformers as the default reference where they provide simpler, mature service.
- Evaluate SSTs as modular AC/DC energy-router nodes where controllability, DC integration, footprint, power quality or fault isolation can justify complexity.
- Prioritize hybrid transformer-plus-converter architectures as an intermediate deployment path.
- Benchmark peak and part-load efficiency separately, plus full-chain conversion, cooling and auxiliary losses, power quality, protection/selectivity, fault response, EMI, cybersecurity, maintainability, lifecycle cost and supply-chain resilience.
- Require repeatable module-level testing and multi-vendor interoperability before treating SST ports as plug-and-play.

Sources:
- IEA, *The State of Energy Innovation 2026*, SST deployment discussion: https://www.iea.org/reports/the-state-of-energy-innovation-2026
- IEEE OJPEL, *A Review of DC-DC Converters for Modular Solid-State Transformers* (3 Jun 2026; DOI 10.1109/OJPEL.2026.3699737): https://doi.org/10.1109/OJPEL.2026.3699737

## 5. Multi-timescale storage and energy coupling

Do not ask a single battery chemistry to solve every time horizon.

- Fast response: power-electronic control and appropriately sized battery capability.
- Daily shifting: modular electrochemical storage with explicit cycling, efficiency and reserve.
- Multi-hour / multi-day: evaluate flow, iron-air, thermal, hydrogen/regenerative fuel cell or other candidates according to site conditions.
- Thermal demand: store heat/cold directly when that is the useful end service.
- Dispatchable backup: retain fuel-based systems only where the reliability value outweighs maintenance, fuel logistics, emissions and lifecycle cost.

All comparisons must include:
- delivered energy at the same service boundary;
- round-trip or conversion losses;
- auxiliaries and standby consumption;
- power rating versus energy capacity;
- reserve and restart energy;
- temperature/ambient derating;
- degradation and replacement;
- safety, maintenance and end-of-life handling.

Do not compare nominal kWh capacities without power capability, duration, reserve and useful delivered service.

## 6. Resilience and local autonomy

A Power Fabric node should retain safe local function if the wider network fails.

Required system properties:

- local protection operates independently of the internet and AI layer;
- stale telemetry or lost time synchronization cannot authorize unsafe switching;
- source loss triggers a tested, bounded response;
- islanded topology and grounding/earthing changes are explicit;
- black start includes the energy needed for controls, ventilation, fuel/cooling auxiliaries and staged pickup;
- communications partition does not create split-brain actuation;
- a node rejoins only after synchronization and protection requirements are met;
- critical service floors and human-approved load priorities remain explicit.

## 7. Industry and SOTA comparison

Compare against conventional generator-plus-battery systems, commercial microgrid controllers, grid-forming inverters, DC distribution and storage alternatives with matched service boundaries and the same scenario/observation budget.

Relevant existing baselines:
- NREL scenario-based grid-forming inverter experiments and UNIFI grid-forming specifications.
- IEC 60364-8-82 for prosumer low-voltage installations, including local sources/storage and islanded/stand-alone behavior.
- IEEE / industry data-centre AC/DC/hybrid power and IT-load protection work.
- Existing Symtropy #3418 test matrix for steady state, transients, islanding, black-start, interoperability, DC protection and cyber failure.

Do not claim SOTA superiority because a simulation has more features or a model has a higher idealized efficiency. Compare exact hardware/configuration and measurement boundary where physical data exist. Separate:
- manufacturer-declared claims;
- model predictions;
- measured prototype results;
- independent test evidence;
- field-qualified performance.

A promising candidate architecture must be allowed to lose to a simpler conventional design if it does not improve delivered service, lifecycle cost, availability, repair time or safety evidence.

## 8. Benchmark suite

### Electrical performance
- Net AC and regulated DC efficiency at matched boundaries.
- Voltage/frequency regulation, power factor, harmonics and DC ripple.
- Load-step response, ramp, overshoot, settling and nuisance-trip rate.
- Converter standby/idle and auxiliary consumption.
- Thermal derating and heat rejection.

### Resilience
- Largest-source loss.
- Storage module loss.
- Communications loss / stale observations.
- Planned and unplanned islanding.
- Black start from a genuinely dead bus.
- Critical-load sequence and cold-load pickup.
- Reconnection and synchronization.
- DC fault detection and selective isolation.
- Multi-vendor grid-forming interaction.

### Lifecycle and serviceability
- Availability and outage minutes at the service boundary.
- Mean time to detect, isolate, repair and restore (with event definitions).
- Component replacement time under an approved de-energized procedure.
- Failure containment: unaffected modules/services after a fault.
- Maintenance hours, spare-part diversity and supply-chain exposure.
- Degradation and replacement at matched duty.
- Lifecycle cost per delivered critical kWh.
- Materials recovery, battery reuse eligibility and hazardous waste.
- Embodied energy/carbon only with a declared lifecycle boundary and source quality.

### Human factors
- Correct source/circuit identification.
- Mislabel and stale-as-built detection.
- Backup/islanding comprehension.
- Error rates on de-energized compatibility mock-ups.
- Resident/operator confidence calibration.
- Time and accuracy for authorized professionals to trace topology and evidence.

### Qualification tiers

Maintain clear tiers:
1. Analytic / synthetic arithmetic fixture.
2. Validated component model.
3. System co-simulation.
4. Real-time simulation / controller-hardware-in-the-loop.
5. Power-hardware-in-the-loop.
6. Laboratory equipment test.
7. Site commissioning and field observation.
8. Independent or authority-recognized qualification, where applicable.

Promotion between tiers requires evidence. No synthetic result may be called a physical test, and no HIL result is automatically field qualification.

## 9. Initial priorities

1. Fixed PV + LFP + grid reference with a simple and explicit AC/DC interface boundary.
2. Evidence-bound typed module contract and human-readable as-built graph.
3. Protection, grounding, DC isolation and source-loss qualification.
4. Passive-first thermal model and measured auxiliary budget.
5. Grid-forming/islanding/black-start multi-vendor qualification.
6. Alternative long-duration storage comparisons at matched service boundaries.
7. Hardware-in-the-loop only after the models, test oracles and safety boundaries are independently reviewed.

**Decision rule:** every additional subsystem must justify its complexity through measured service, resilience, lifecycle or human-usability improvement. If the simpler architecture performs as well, keep the simpler architecture.
