# Passive Textile Machine Observability Kit — Reference Design v0.1

**Status:** non-committal hardware reference; no BOM, machine installation or safety review completed  
**Parent:** TEXTILE-MACHINE-OBSERVABILITY-001 #5311  
**Pilot:** one surveyed industrial lockstitch station for AO-01 seam coupons  
**Date:** 2026-11-15

## 1. Design goal

Instrument an existing sewing station so that machine state, material/process context and physical output can be compared without changing the machine's control or safety system.

This is an **observer kit**, not a machine controller.

The first kit should be removable, serviceable, low-intrusion and usable offline. It must not change feed, speed, thread tension, presser-foot pressure, needle motion, guards, interlocks or emergency-stop behaviour.

## 2. Functional architecture

\`machine / job / material\`
→ \`approved passive sensors\`
→ \`isolated acquisition\`
→ \`local timestamped store\`
→ \`read-only adapter / observation record\`
→ \`quality and process analysis\`
→ \`operator-reviewed evidence report\`

A future control path, if ever justified, is a separate safety-engineered project with its own machine-specific hazard analysis, OEM/owner approvals and validation. It is not an extension of this data path.

### Module A — machine-native state adapter

Use supported OEM telemetry or export first where available. Capture only declared read-only state: machine identity, state/mode, speed, counters, fault events, active program/recipe identity and supported adjustment state.

Do not reverse-engineer undocumented controller pins or write-protocols. If the OEM interface is unavailable, use external sensors instead of opening or modifying the controller.

### Module B — motion/phase observation

Candidate: a non-contact optical tachometer or phase sensor observing a safe, exposed reference target, or a supported native encoder output.

Purpose:
- estimate shaft speed;
- align stitch-count/needle-phase observations to a shared timebase;
- segment cycle/idle/start-stop states.

The exact target, mounting, resolution, sample rate and safe line-of-sight are machine-specific. Do not attach a sensor to a moving assembly or enter the needle/belt/pulley hazard envelope without a qualified design review.

### Module C — electrical energy observation

Candidate: a professionally installed, appropriately rated external power/energy meter at the declared machine supply boundary.

Purpose:
- measure idle, sewing and stop/standby energy;
- calculate energy per accepted seam coupon or other declared functional unit;
- distinguish machine electricity from compressed air, lighting, HVAC or other shared services.

Do not open an energized control cabinet or clamp conductors without a qualified electrical installer and documented isolation procedure. Meter category/rating, wiring, protection, installation and calibration must be selected for the actual supply.

### Module D — vibration observation

Candidate: a calibrated accelerometer on a stationary, approved part of the machine frame.

Purpose:
- capture repeatable vibration signatures;
- identify changes that may correlate with looseness, imbalance, bearing wear or process instability;
- support condition-based inspection hypotheses.

A vibration signature is not a diagnosis or remaining-useful-life result. Mounting location, direction, bandwidth, sampling, mounting stiffness and background conditions must be fixed. Validate any fault classifier against independently inspected machine conditions.

### Module E — work-zone vision

Candidate: a fixed, enclosed or guarded camera viewing only the presser-foot / needle / seam-work area, with diffuse lighting and a geometric calibration reference.

Purpose:
- track fabric edge or seam path;
- measure seam deviation;
- observe feed slip, bunching, puckering and material migration;
- create a traceable image-derived measurement with uncertainty.

Privacy defaults:
- no worker faces or body video;
- no audio capture;
- local processing and short retention by default;
- store derived measurements rather than images unless images are necessary and specifically authorized;
- no upload/cloud dependency for the pilot.

The camera must not obstruct lighting, operator visibility, machine access, guards or maintenance.

### Module F — fabric-feed observation

First choice: derive fabric motion/edge displacement from the calibrated work-zone camera or supported non-contact optical sensing.

Avoid adding rollers, guides or contact sensors until their effect on friction, feed, stretch and seam formation is quantified against a no-sensor control.

### Module G — thread-tension observation (optional research add-on)

An inline thread-tension sensor may provide valuable information, but it changes the thread path and can itself alter friction, tension and thread-break behaviour.

It must not be part of the minimum kit. First compare:
- no sensor;
- candidate sensor installed but not read;
- candidate sensor active.

Only retain it if independent seam results and thread-break behaviour remain within preregistered non-inferiority limits and the sensor is calibrated for the actual thread path.

### Module H — local acquisition and gateway

A local acquisition module should:
- use electrically isolated inputs appropriate to the sensors;
- provide a common monotonic timebase and synchronized wall-clock reference;
- retain data locally through network loss;
- write append-only raw observation records;
- expose read-only local data access;
- have no actuator outputs connected to the machine;
- have no network route to machine control except a separately approved read-only OEM interface;
- preserve sensor/calibration/configuration digests with observations.

For v0.1, offline operation is preferred. The kit must remain useful when disconnected from cloud services or the wider factory network.

## 3. Candidate channel set

| Channel | Unit | Source | Initial role |
|---|---|---|---|
| Machine state | enum | OEM read-only or operator event | Context |
| Head speed | rpm | Native or optical | Cycle segmentation |
| Shaft/needle phase | degree or tick | Native or optical | Stitch/cycle timing |
| Stitch count | count | Native or derived | Output denominator |
| Main electrical power | W | Meter | Energy accounting |
| Energy | Wh/kWh | Meter integration | Resource per accepted output |
| Frame vibration RMS | m/s² | Accelerometer | Condition observation |
| Seam edge offset | mm | Calibrated work-zone camera | Output geometry |
| Fabric feed displacement | mm | Optical/camera | Feed/slip observation |
| Thread tension | N or cN | Optional sensor | Research only; intrusive |
| Fault/stop event | enum | OEM or adjudicated event | Process context |

Channel names and units must be mapped to the shared machine observation schema. A channel is unavailable/unknown until the exact machine profile supports it.

## 4. Calibration and measurement plan

For each sensor/channel record:
- sensor make/model and serial/asset identity;
- calibration source, certificate/record and valid dates;
- physical location and orientation;
- measurement range and saturation behaviour;
- sample rate, resolution, filtering and signal chain;
- clock ID, synchronization error and data-loss detection;
- uncertainty estimate and method;
- installation revision and mounting method;
- any known interaction with the machine or fabric.

Minimum checks:
1. timestamp monotonicity and cross-channel synchronization;
2. known reference speed/phase comparison;
3. electrical meter plausibility against an approved reference;
4. accelerometer orientation/mount repeatability;
5. camera scale, lens distortion and coordinate-frame calibration;
6. explicit detection of clipping, disconnection, stale samples and missingness;
7. sensor-on versus sensor-off coupon comparison.

No derived feature should lose the raw channel identity, calibration, transform and uncertainty needed to replay it.

## 5. First physical experiment

The first sample campaign uses the same stock machine and named settings before and after observation-kit installation.

Use an AO-01 coupon set:
- straight seam;
- curved seam;
- corner/turn;
- two-layer and three-layer stack-up;
- pocket-mouth reinforcement;
- cuff/placket stack-up;
- side-vent reinforcement.

Record fabric construction/lot, measured mass, layers, needle, thread, presser foot, feed settings, machine speed, operator/setup context and exact seam path.

Inspect output independently for seam deviation, stitch length, skipped/broken stitches, pucker/appearance, damage and seam strength where applicable.

Compare:
A. stock machine / supported OEM monitoring;
B. stock machine plus passive sensor kit;
C. passive kit plus operator-reviewed recommendations (no automatic control).

Counterbalance order where feasible and keep machine, operator/skill, material lot, recipe and inspection method matched.

## 6. Upgrade path after the observer kit

Only after M2 evidence shows that the observer does not degrade output and creates useful information should the team investigate:
- improved fixed work-zone lighting;
- interchangeable machine-qualified guides/fixtures;
- low-friction replaceable wear guides;
- operator-adjustable setup aids and visual setup cards;
- better fabric support and feed presentation;
- service/maintenance indicators tied to independently verified machine condition;
- an external material-feed/tension assist as a separately engineered machine/cell project.

The first machine upgrades should improve fixture, inspection and operator feedback without taking motion authority away from the machine or operator.

## 7. Commissioning and safety gate

Before installation:
- record exact machine nameplate, model, firmware and manuals;
- obtain owner/OEM requirements and permission;
- inspect current guards, needle protection, interlocks, emergency stops and electrical condition;
- perform a machine-specific risk review;
- define sensor locations and ensure no hazard-zone entry, snag, entanglement, obstruction or new pinch point;
- obtain qualified electrical installation and competent machine technician review where required;
- document lockout/isolation, commissioning and rollback;
- confirm no machine safety or control function has been altered.

Any proposal to change drive, controller, feed, tension, presser foot actuation, needle motion, guarding or emergency-stop circuitry must stop this passive programme and enter a separate machine-modification engineering and safety-validation work package.

## 8. Acceptance criteria for the observer kit

- no machine control write path exists in hardware or software;
- no guard, interlock or safety function is modified or bypassed;
- sensor installation does not materially change the sewing process or accepted coupon quality;
- all accepted data bind to exact machine/configuration/job/material/recipe and sensor calibration;
- missing, stale, out-of-range and uncalibrated channels are explicit;
- output quality is independently inspected;
- timebase drift, sample loss and calibration uncertainty are reported;
- the kit can be removed and the machine returned to its documented stock configuration;
- installation, maintenance, downtime and total ownership burden are recorded.

## 9. Non-claims

This reference design does not specify exact vendor part numbers, electrical schematics, machine mounting dimensions, approved sensor placements, safety performance level, a calibrated machine model, or procurement prices. Those require the actual machine, its documentation and qualified engineering review.

It does not establish autonomous sewing, machine certification, predictive maintenance, reduced energy, improved productivity or safer operation.
