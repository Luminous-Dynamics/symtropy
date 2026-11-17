# Robotic Protective Shell Architecture v0.1

**Status:** engineering concept and qualification plan; no fabricated article or protection rating is claimed  
**Date:** 2026-11-17  
**Tracking:** [Symtropy #5868 — ROBOT-PROTECTIVE-SHELL-001](https://github.com/Luminous-Dynamics/symtropy/issues/5868)  
**Pilot interface:** [HUMANOID-JOINT-CARTRIDGE-001 #4337](https://github.com/Luminous-Dynamics/symtropy/issues/4337)  
**Scope:** non-weaponized humanoid, industrial, service, construction, inspection and rescue robotics

## 1. Design objective

Create a family of lightweight-to-heavy-duty protective modules that are specific to a robot's task, operating domain and hazard profile. The objective is protection per unit mass and lifecycle burden—not maximum armor thickness.

A protective shell is a coupled mechanical, thermal, sensing, electrical, ergonomic and manufacturing system. Every added part changes some combination of mass distribution, inertia, joint loads, cooling, sensor performance, cable routing, service access and human-contact behavior.

The design process therefore follows:

1. identify platform, mission, ODD and hazards;
2. divide the robot into protection zones and load-bearing interfaces;
3. define the protected component and its actual damage limit;
4. build a multi-objective mass / protection / thermal / sensing / mobility / service trade-space;
5. compare candidate materials, geometries and manufacturing processes;
6. model only the physical semantics supported by the selected toolchain;
7. fabricate traceable specimens and measure the as-built article;
8. test coupons, panels, joint modules and integrated robots at progressively higher gates;
9. repair or replace damaged modules and repeat applicable qualification.

A CAD concept, supplier datasheet, simulation, finite-element result, 3D-printed coupon or one impact event is not evidence that a complete robot is protected.

## 2. Scope boundary

This programme targets civil, industrial, service, research, construction, inspection and rescue systems. It does not design weapon systems or an offensive payload. Generic ballistic protection is outside this architecture. Any exceptional ballistic requirement must be separately specified, procured and tested by competent, independently accredited specialists; it must not be inferred from a generic polymer/composite shell or an enclosure impact rating.

The first pilot is a replaceable forearm/joint-guard module around the existing repairable humanoid rotary-joint demonstrator. Whole-body humanoid armor is not the first prototype: the joint/forearm article has bounded interfaces and exposes the most important trade-offs—articulation, cable flex, sealing, thermal behavior, mass, inertia, impact transfer and maintenance.

## 3. Architecture

### 3.1 Layered module

Compare the following functional layers. They need not all be present in every zone.

| Layer | Function | Candidate implementation | Main failure modes to test |
|---|---|---|---|
| Structural carrier | Mounting and load path | Existing rated frame interfaces, formed/machined metal brackets, reinforced polymer carrier | Mount tear-out, fatigue, load transfer into motor/gearbox, loosening |
| Replaceable outer skin | Abrasion, weather, debris and local impact surface | Thermoplastic, formed metal, composite, replaceable sacrificial panel | Cracking, denting, edge fracture, UV/chemical aging |
| Energy-management insert | Limit transmitted shock and protect clearances | Elastomer, foam, cellular/honeycomb or engineered lattice | Bottoming-out, rebound, creep, moisture, fatigue |
| Stand-off / spreader | Keep impact away from sensitive components | Ribs, rails, stand-offs, local hard points | Contact with protected unit, concentrated loads, resonance |
| Inner liner / seal | Prevent abrasion, dust/fluid ingress and cable chafe | Gasket, labyrinth, bellows, wiper, non-shedding liner | Seal wear, trapped water, friction, pinch, contamination |
| Thermal interface | Preserve heat rejection and thermal separation | Vents, thermal bridges, shields, isolated hot zones | Trapped heat, blocked exhaust, derating, hot spots |
| Sensor/service interface | Preserve sensing and maintenance | Qualified windows, apertures, service doors, captive fasteners | Occlusion, optical distortion, leakage, inaccessible isolation |

The protective load path should normally terminate at a rated structural interface, not at an actuator cover, encoder, gearbox seal, electronics lid or sensor bracket. Any exception requires evidence that the component is designed for that structural load.

### 3.2 Protection profiles

These are planning profiles, not certified levels and not one-dimensional rankings.

| Profile | Intended service | Typical design emphasis |
|---|---|---|
| P0 — wear skin | Indoor lab and light service | Scuff/abrasion, cleaning, replaceable covers, cable sleeves |
| P1 — environmental shell | Field service and light outdoor work | Dust/splash control, drainage, seals, UV/cleaner compatibility |
| P2 — industrial guard | Manufacturing, workshop and construction support | Specified tool/debris/edge impacts, replaceable absorbers, chip guards |
| P3 — heavy-duty field shell | Rescue, rough terrain and demanding industrial sites | Higher declared impact/crush cases, corrosion, weather, modular repairs |
| P4 — specialist hazard module | Heat, arc, corrosive, radiation, hygiene or high-pressure hazards | Specialist materials, standards, test labs and domain-specific safety review |

A robot may have P0 on a sensor window, P2 on a shin and P3 on a battery enclosure. The zone's hazard and protected-component limit—not marketing class—defines acceptance.

### 3.3 Articulated areas

Joint covers require their own design rules.

- Divide the cover into rigid islands joined by flexible bellows or overlapping non-contact skirts.
- Preserve full swept-volume clearance at every allowed joint combination and at all tolerance extremes.
- Prevent shearing, pinching, snagging and cable entrapment.
- Keep protective edges from becoming impact concentrators or sharp fracture points.
- Check flex fatigue, torsion, abrasion, pressure pumping, seal friction and temperature after repeated motion.
- Make wear parts replaceable without disturbing calibration-critical joint alignment.
- Preserve the joint module's existing local safety and service access.

Do not use a continuous hard shell across a moving joint merely because it is easy to manufacture.

## 4. Zone map

A common zone catalogue should be reused across humanoids and other robot morphologies.

| Zone | Primary protective concerns | Non-negotiable interfaces |
|---|---|---|
| Head / perception pod | Weather, dust, abrasion, accidental contact, fragile optics | Camera/lidar/radar/audio/thermal FOV and spectral windows; microphones and antennas |
| Torso / spine | Panel impact, debris, service access, thermal accumulation | Structural frame, compute, cooling, battery and emergency isolation |
| Shoulder / elbow / wrist | Joint impact, snagging, cable fatigue, flex cracking | Joint ROM, harness bend radius, torque/speed and maintenance access |
| Hip / knee / ankle | Ground debris, impact, abrasion, joint heat, gait interference | Foot clearance, balance, gait, joint thermal and actuator envelopes |
| Hand / gripper | Abrasion, tool contact, tactile damage | Fingertips, force/tactile sensors, contact compliance and tool interfaces |
| Foot / base | Toe/heel impact, abrasion, water, traction loss | Ground contact, drainage, mobility and access |
| Battery / power bay | Mechanical separation, ingress, thermal and electrical hazards | Isolation, fusing/protection, vent/containment strategy owned by battery safety specialists |
| Mobile platform | Bumper corners, wheel/track debris, spray and underbody strikes | Suspension travel, drive cooling, turning envelope, ground clearance |

The outer shell must never obscure emergency stops, disconnects, safety sensors, status indicators or required inspection points.

## 5. Candidate materials and routes

No material is selected in v0.1. These are families for coupon screening and a Pareto study.

| Family | Where it may fit | What must be measured |
|---|---|---|
| PC / PC blends / PC-ABS | Impact-tolerant outer panels and windows where grade permits | Grade-specific impact, UV, chemical stress cracking, heat, fire behavior, optical aging |
| PA12 / PA6 / reinforced polyamide | Complex shells, brackets and low-volume printed or moulded parts | Moisture conditioning, creep, anisotropy, layer/build effects, chemical and thermal aging |
| TPU / TPE | Bumpers, cable sleeves, bellows, compliant edge guards | Tear, abrasion, flex fatigue, creep, hydrolysis and cleaner/oil compatibility |
| EPP / EVA and cellular absorbers | Replaceable shock-management inserts | Force-displacement curve, bottoming-out, rebound, compression set, moisture and fatigue |
| Aluminum sheet/extrusion | Rigid guards, carriers and heat-spreading parts | Denting, fatigue, corrosion, galvanic compatibility, edge treatment and thermal effects |
| Stainless alloys | Wear/corrosion exposed small guards and fixtures | Mass penalty, corrosion chemistry, fatigue, edge finishing and temperature |
| Glass-fibre composites | Lightweight stiff panels with controlled conductivity | Impact damage, delamination, moisture, repair, fire and inspection visibility |
| Carbon-fibre composites | High stiffness-to-mass candidates only where electrically appropriate | Conductivity/EMI, galvanic coupling, impact damage, edge/splinter behavior, repair and end-of-life |
| Optical-grade polycarbonate / glass | Camera and sensor windows | Transmission by wavelength, distortion, scratch, fogging, coating, seal and thermal performance |
| Coated textile / abrasion sleeve | Cable and flexible-zone protection | Snagging, fray, flex cycles, abrasion, cleaning and fire behavior |

Candidate manufacturing routes:
- MJF/SLS/FDM for fit-checks, low-volume prototypes and selected qualified production parts;
- thermoforming or injection/compression moulding for suitable repeatable polymer skins;
- sheet forming, extrusion and machining for metal guards and carriers;
- composite layup or thermoplastic composite processes where stiffness-to-mass or geometry justifies them;
- CNC/laser/waterjet for panels and interfaces;
- moulded or cut-to-shape elastomers and cellular inserts for replaceable absorbers;
- purchased certified subassemblies when the hazard demands a qualification level unavailable in-house.

FDM material claims do not transfer automatically to MJF/SLS, moulded or continuous-fibre composites. Each process/grade/orientation/conditioning state is its own evidence subject. Datasheets establish manufacturer-declared values under their listed conditions, not as-built part properties.

## 6. Engineering calculations that must be explicit

### 6.1 Added mass and inertia

For modules with masses (m_i), positions (r_i), the added mass is:

[
\Delta m = \sum_i m_i
]

The new centre of mass must be calculated from the baseline robot mass properties plus the exact locations of every added module. The inertia change must use the parallel-axis theorem and the actual local module inertia tensors. A single total-mass figure is insufficient: equal mass at the wrist, torso and shin creates different control consequences.

Every candidate report should include:
- added mass by zone;
- new centre-of-mass vector;
- inertia-tensor delta about the selected frame;
- joint reflected-inertia implications where applicable;
- actuator torque/current/thermal and battery-runtime consequences.

### 6.2 Impact transfer

For a declared moving object, the initial kinetic energy is (E=\tfrac12 mv^2), but energy alone does not define the test. Contact geometry, stiffness, duration, boundary conditions, impact location, rebound and load path matter. The acceptance metric is not merely “panel did not break”; measure transmitted force/acceleration/strain at the protected interface and verify that the underlying component remains within its independently specified limits.

### 6.3 Thermal behavior

Model and measure the added thermal resistance and any blocked convection/radiation paths. Compare bare and protected robot under the same duty cycle and ambient conditions. Report hot-spot temperature, time to steady state, derating onset, cool-down time and the thermal effect of dust filters/blocked vents. Do not trade away an actuator's qualified continuous-duty envelope for a better impact result.

### 6.4 Sensor performance

For each sensor window or aperture, record exact sensor model/configuration, field of view, spectral range, transmission/reflectance assumptions, protective-window thickness/grade/coating, distance and alignment. Measure false detections, missed detections and calibration shift before/after installation and environmental conditioning. A clear-looking window is not proof of sensor transparency.

## 7. Manufacturing and digital thread

Reuse Symthaea ROB-REALIZE-001, MFG-PROC-000, the fabrication kernel, Mycelix manufacturing traceability, and Sol-Atlas material provenance. Do not invent parallel identities or ledgers.

Required chain:

1. protection requirement and hazard profile;
2. exact RobotDesignSubject and protection-zone geometry;
3. material grade and source document identity;
4. supplier and material lot/batch;
5. process definition, machine capability, recipe and tooling/fixture;
6. fabrication plan and exact toolchain/settings;
7. fabrication execution and operator/automation evidence;
8. physical article identity and as-built configuration;
9. dimensional inspection, mass-properties measurement and material/process inspection;
10. test campaign, instruments, calibration, raw data and uncertainty;
11. acceptance decision and applicable claim scope;
12. repair/rework generation, new as-built identity and requalification.

Preserve declared, modelled, simulated, measured and independently assessed evidence as distinct classes. A fabrication job marked complete is not evidence that the physical panel matches CAD. A material certificate does not prove that the right batch was used. Repair creates a new as-built generation; prior qualification does not silently transfer.

## 8. Qualification plan

### Stage 0 — requirements and hazards
Freeze the first platform, ODD, zone map, load-bearing interfaces, protected-component damage limits, mass/thermal/sensor budgets and acceptance criteria. Identify unknowns explicitly.

### Stage 1 — coupons
Test candidate materials and processes only against the relevant claims: mass/area, dimensional stability, flexural/tensile/impact/tear/abrasion, conditioning, UV, chemical compatibility, fire/smoke and seal aging as applicable. Preserve raw observations and uncertainty.

### Stage 2 — panel and joint modules
Instrument impact and fatigue tests; record input and transmitted loads, energy absorption, rebound, mount/fastener condition, crack/delamination, seal integrity, post-impact ingress, joint ROM, harness damage and replacement/reseal repeatability.

### Stage 3 — integrated robot
Compare bare and protected configurations under matched tasks and environmental conditions. Measure mass properties, energy per task, peak/continuous current, thermal rise/derating, sensor errors, task performance, gait/fall recovery, contact force and maintenance access. Test emergency stops and isolation access.

### Stage 4 — lifecycle
Perform panel replacement, repair/rework, inspection, reseal, requalification and lifecycle costing on the same identified article. Record yield, scrap, maintenance time, process energy and recovery/recycling route.

## 9. Standards and test mapping

Select only standards whose scopes match the robot and hazard:
- ISO 12100:2010 for machinery risk assessment/risk reduction; ISO/DIS 12100.3 is under development and is not yet the published replacement.
- ISO 10218-1:2025 and ISO 10218-2:2025 for applicable industrial robot/cell design and integration. Their scopes do not blanket-cover service, consumer, mobile, medical, defence, or every humanoid use.
- ISO/TS 15066:2016 where applicable to collaborative industrial robot systems.
- ISO 13849-1:2023 for relevant safety-related control-system design; it does not set the required performance level for every application.
- IEC 60529 for enclosure IP claims under the relevant test conditions.
- IEC 62262 for IK impact classification of electrical enclosures where the product standard and test specimen make it applicable. IK is not a universal armor rating.
- IEC 60068-2-6 and IEC 60068-2-27 for specified vibration and shock tests, with severities selected for the intended environment.

NIJ 0101.07 is a body-armor standard for defined law-enforcement torso protection tests; it is not an automatic certification route or generic rating for robot enclosures. No ballistic rating is claimed by this programme.

Standards mapping is not compliance. The responsible safety engineer must confirm applicability, current editions/addenda, test laboratory competence and acceptance criteria for the exact robot/product scope.

## 10. Benchmark vector and SOTA comparison

Preserve a vector rather than a single armor score:

- protection outcome by hazard, zone and damage threshold;
- transmitted force/acceleration/strain and absorbed energy;
- added mass, centre-of-mass shift and inertia tensor delta;
- joint torque/current/energy and task-time penalty;
- thermal rise, heat-path change, cooling burden and derating;
- sensor FOV/coverage, optical performance and perception error delta;
- ingress/leak and IK result only where independently tested;
- fatigue cycles to crack/tear/loosening and inspection sensitivity;
- panel replacement/repair/reseal time and post-repair conformity;
- cost per protected zone, yield, scrap, manufacturing energy and lifecycle burden.

Matched baselines:
1. unprotected mechanism/robot;
2. stock/OEM cover;
3. conventional sheet/panel guard;
4. polymer-only shell;
5. shell with replaceable energy-absorber;
6. composite candidate where justified.

Use the same platform, load, task, environmental conditioning, impact location, test fixture, instruments and acceptance rules. Report uncertainty and failure modes. Pareto-optimality is more useful than declaring one universally best material: mass, impact performance, thermal behavior, sensing, repair and cost are competing objectives.

## 11. Initial demonstrator

**One replaceable humanoid joint/forearm protection module** around the #4337 sealed rotary-joint demonstrator.

Proposed subsystem contents:
- one rigid carrier referenced to defined structural mounting interfaces;
- one removable outer guard/skin;
- replaceable, non-structural energy-management insert where justified by coupons;
- one flexible joint bellows/overlap guard;
- protected cable/harness route with measured bend-radius and flex-cycle requirements;
- gasket/labyrinth and drainage strategy matched to the actual exposure;
- sensor/thermal/service apertures preserved where the joint has them;
- captive/keyed fasteners and an inspection/replacement procedure.

This is an architecture proposal, not a final geometry or BOM. Exact thickness, material grade, impact severity, IP/IK target, fastener pattern and mass budget remain open until actual CAD, use case, load/thermal envelope and ODD are reviewed.

## 12. Prospective gates

1. Requirements: approved zone map, hazard profile, interfaces, mass/thermal/mobility constraints and safety owner.
2. Candidate: matched material/architecture/process trade study with source quality and explicit unknowns.
3. Coupons: calibrated methods, raw data, uncertainty and process/lot identity.
4. Module: exact design/fabrication/as-built lineage, measured fit, joint ROM, impact/thermal/environment/fatigue results.
5. Integrated: repeatable robot task, safety/sensing non-regression and measured mass/inertia/energy/thermal penalties.
6. Lifecycle: repair, replacement, inspection, requalification and material recovery on a known article.

A failure becomes a versioned regression case. A simulation or coupon pass cannot discharge an integrated or field requirement.

## 13. Claim ceiling

Claims apply only to the exact material, process, lot, specimen geometry, mounting, conditioning, impact location, test method, instrument, robot configuration and ODD that were qualified. No ballistic resistance, personnel protection, IP/IK certification, fire certification or whole-robot safety certification is implied.

## References

- [ISO 10218-1:2025](https://www.iso.org/standard/73933.html)
- [ISO 10218-2:2025](https://www.iso.org/standard/73934.html)
- [ISO 12100:2010 and ISO/DIS 12100.3 status](https://www.iso.org/standard/51528.html)
- [ISO/TS 15066:2016](https://www.iso.org/standard/62996.html)
- [ISO 13849-1:2023](https://www.iso.org/standard/73481.html)
- [IEC 60529](https://webstore.iec.ch/en/publication/2447)
- [IEC 62262:2002+AMD1:2021](https://webstore.iec.ch/en/publication/6673)
- [IEC 60068-2-6:2007](https://webstore.iec.ch/en/publication/544)
- [IEC 60068-2-27:2008](https://webstore.iec.ch/en/publication/514)
- [NIJ 0101.07](https://nij.ojp.gov/library/publications/ballistic-resistance-body-armor-nij-standard-010107)
