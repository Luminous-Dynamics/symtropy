# Textile Machine Observability and Retrofit Programme v0.1

**Status:** research/engineering proposal; no machine has been instrumented or qualified  
**Owner:** Symtropy #5311 — TEXTILE-MACHINE-OBSERVABILITY-001  
**Composition:** Symthaea apparel engineering #11731; MFG-PROC-000 #5686; Symtropy textile robotics #4865, knitting preflight #4878, multi-stage factory #4875 and cloth physics #456  
**Date:** 2026-11-15

## 1. Thesis

The opportunity is not to build one more dashboard for textile machinery. It is to make a mixed fleet of textile machines **observable, calibratable, comparable and upgradeable without throwing away good existing equipment or confusing telemetry with machine authority**.

The desired loop is:

machine and configuration identity
→ material, process and job identity
→ calibrated observation
→ state/deviation interpretation
→ operator-reviewed response
→ independently inspected output
→ maintenance, repair and requalification

Machine data are evidence about selected machine states. They are not a direct measure of accepted product quality. A motor-current pattern cannot prove seam strength; a stitch count cannot prove a correct stitch; machine availability cannot prove a garment was made right.

P0–P2 are read-only. No machine write path, direct actuation, safety-interlock modification or autonomous process correction is authorized by this programme.

## 2. Market and research baseline

### OEM capabilities already exist

Do not position Atelier/Luminous as if textile factories have no connected machine products.

- JUKI Smart Solutions offers automation, digitization and networked sewing-line management. JaNets is an IoT machine-operation management service that visualizes production status and volume. JUKI also documents numerical management and transfer of stitch-adjustment values such as feed pitch, feed-dog height, presser-foot pressure and needle-thread tension. These OEM functions are the incumbent baseline, not a straw man.
- SHIMA SEIKI sells computerized flat knitting and WHOLEGARMENT machine families; its own product material states that whole-garment knitting can eliminate sewing for the knitted article. This is a different manufacturing route, not merely a faster sewing machine.
- STOLL knit-and-wear similarly combines knitting, shaping and connecting operations within one machine process, producing near-ready-to-wear knitted garments.

Atelier should compare the actual offered configuration, price, installation, connectivity, supported signals, access/export, service, maintenance and output qualification. Do not compare an aftermarket sensor pack with an imaginary disconnected factory.

### Robotic sewing baseline

The open-access paper by Anatoliotakis, Zacharia and Koustoumpardis, published 21 September 2026, presents a single-arm SCARA system using computer vision, learned grasp-point estimation and force-regulated cloth tension with an unmodified industrial sewing machine.

The paper reports:
- mean seam deviation of approximately 1.2 mm;
- standard deviation of approximately 0.9 mm;
- maximum observed deviation of 2.5 mm;
- 79% of seam points within the selected offset tolerance;
- 10 sewing trials per fabric configuration and an operating speed around 84 mm/s.

Its limits matter: fabric placement and path selection are user-initialized; the study used uniform illumination; training and test categories came from the same material/geometry categories; it acknowledges generalization limitations for substantially different fabrics and large/highly elastic materials.

This is a useful task-specific comparator for #4865, not proof that general-purpose autonomous sewing is solved and not a directly transferable acceptance threshold for every garment.

Reference: https://doi.org/10.1007/s41315-026-00590-3

### Interoperability baselines

- MTConnect 2.5 (February 2025) is an open manufacturing equipment data standard.
- OPC UA for Machinery Part 1 v1.04.2 was published 1 November 2026 and provides common machinery building blocks, including machine/component identity and status-related concepts.
- These protocols are adapters and interoperability references. They do not establish the truth of a sensor reading, a textile capability, or the quality of a sewn/knitted output.

References:
- https://www.mtconnect.org/
- https://reference.opcfoundation.org/specs/OPC-40001-1
- https://reference.opcfoundation.org/specs/OPC-40001-1/full

## 3. The central design shift: machine–material–process co-observation

An equipment trace is not portable across material and process changes unless those changes are recorded.

For a sewing operation, the minimally meaningful context includes:
- exact machine asset/model/controller and configuration;
- machine firmware and relevant adjustable-feature state where available;
- operation type and declared seam/path geometry;
- fabric construction, material lot, mass/area, thickness and relevant directional properties;
- needle type/size, thread, presser foot, feed mechanism and attachment;
- exact process recipe and setup revision;
- work-order, garment configuration, pattern and BOM revisions;
- sensor identity, location, calibration, timebase, sampling and uncertainty;
- output specimen/coupon identity and independent inspection result.

A change in machine, controller firmware, drive, motor, presser foot, needle, thread, material lot, pattern stack-up, recipe, sensor installation or calibration must invalidate or narrow only the dependent evidence. No stale capability inheritance.

## 4. Textile machine family map

The same machine-level observation architecture can support several textile process families, but each family needs its own channel definitions, metrology and safety boundary.

| Machine family | Process/output | Useful observations | Independent output evidence |
|---|---|---|---|
| Single-needle lockstitch | Straight/curved seams, topstitch, plackets | Shaft/needle phase, speed, stitch count, feed, power, vibration, optional tension, seam image | Seam path, stitch length, pucker, damage, seam strength |
| Overlock / coverstitch | Edge finishing, knit seams, hems | Needle/looper events, multi-thread path, differential feed, edge trim, speed, power | Edge finish, stitch formation, seam extension, damage |
| Programmable pattern sewing | Repeated pockets, bartacks, shapes | Program digest, clamp/fixture, cycle phases, stitch count, speed, fault state | Pattern position, dimensions, stitch quality, repeatability |
| Computerized flat knitting | Panels, fully-fashioned and whole-garment knit | Program/backend digest, carriage, bed/carrier state where exposed, yarn feed/take-down, alarms | Gauge, dimensions, stitch structure, dropped/stuck stitches, yarn mass |
| Spreading / cutting | Lay preparation and pattern cutting | Lay height/tension, vacuum, marker/job digest, tool/path, blade state, energy | Cut-edge deviation, part dimensions, ply alignment, material yield |
| Weaving | Fabric formation | Loom phase, warp tension, insertion/stop events, let-off/take-up | Width, pick density, defect map, tensile/tear specimens |
| Finishing / dyeing / thermal processing | Wet/thermal/mechanical finishing | Qualified temperature, time, flow, pressure, speed and process state | Process-specific material test, dimensional/colour/hand results, effluent/output evidence |

Do not reuse lockstitch sensor semantics for an overlocker or knitting machine merely because both have a motor and a timestamp. Machine-family adapters should be narrow, explicit and versioned.

## 5. First pilot: one lockstitch machine

The first pilot should be an ordinary industrial single-needle lockstitch machine suitable for AO-01 sample coupons, not a purpose-built robotic cell.

The exact model is not yet known. Do not select sensor mounting, wiring, input range, encoder resolution, sample rate or measurement tolerances until the machine is surveyed and its manual/controls/safety arrangement are understood.

### Minimum passive instrumentation candidate

1. **Native read-only state / export:** use supported OEM data first where it exists.
2. **Speed / phase:** use a non-contact optical observation or supported encoder signal where mounting can remain outside hazardous areas. Record angular/time resolution and alignment uncertainty.
3. **Electrical energy:** qualified metering at the appropriate supply boundary. Electrical installation must be performed by a competent person; no opening of a live control cabinet for this research.
4. **Vibration:** accelerometer on a stationary, approved machine-housing location. Record mounting, axis, bandwidth, sample rate and calibration.
5. **Work-zone imaging:** fixed camera looking only at the sewing work region, with calibrated illumination and geometric reference. No face, body or worker surveillance; no cloud video requirement.
6. **Fabric feed / seam geometry:** calibrated image-based observation, optical tracking or another verified method. State the measurement uncertainty and avoid assuming a fabric marker is non-invasive.
7. **Optional thread tension:** only after proving the sensor does not materially change the thread path/friction, tension, break rate or seam result versus a no-sensor control. It is not a required v0.1 channel.

Use the minimum sensor set that distinguishes a meaningful fault or process condition. More sensors do not automatically create better observability.

### Why not add automatic control immediately?

A closed-loop feed/tension system could eventually improve seam tracking and reduce rework, but it creates a different engineering problem: new actuators, control authority, interaction with OEM motion control, unexpected motion and safety-function implications.

P0–P2 therefore remain read-only. Recommendations may be shown to an operator for review, but they do not adjust feed, tension, speed, needle position, motor drive or interlocks. A future actuator study must be a separate machine-specific risk-assessed and validated work package.

## 6. Commissioning and experimental protocol

### M0 — machine survey

Record:
- make/model, nameplate and asset identity;
- manual, approved accessories and relevant OEM restrictions;
- controller/firmware and supported data interface;
- machine electrical/safety documentation and guard/E-stop layout;
- motor/drive type, speed envelope and duty assumptions;
- presser foot/feed system, needle/thread configuration;
- maintenance and calibration history available to the owner;
- known defects, vibration/noise and baseline condition.

No retrofit work proceeds if the machine identity, documentation or safety condition is unknown.

### M1 — no-retrofit baseline

Freeze a small coupon corpus and exact process conditions:
- straight seam;
- curved seam;
- corner/turn;
- two- and three-layer stack-up;
- pocket-mouth reinforcement;
- cuff/placket stack-up;
- side-vent reinforcement.

Record operator/setup, machine settings, fabric/lot, needle/thread/foot, speed, sample dimensions and output inspection. This is the stock-machine reference.

### M2 — passive observability pilot

Install only approved non-invasive sensor equipment, then repeat matched coupons.

Check whether:
- the sensor installation changes machine behaviour;
- timebase and phase are sufficiently synchronized;
- each signal has traceable calibration and uncertainty;
- the output quality is at least non-inferior to the baseline;
- detected states correspond to independently observed events;
- false alarms and missed detections are measured.

Include a sensor-absent or sensor-disabled negative control. Sensor failure must not create an artificial production pass.

### M3 — operator-reviewed decision support

Only after M2, test offline or advisory recommendations such as:
- inspect needle/thread/foot after a measured defect signature;
- re-run a calibration coupon after sensor or material-lot change;
- request a manual setup check when a signal is out of the qualified envelope.

Record whether a person accepted/rejected the recommendation, what action followed and whether the independently inspected output improved. Do not call an unaccepted recommendation a maintenance intervention.

### M4 — upgrade experiments

Potential mechanical/process upgrade candidates:
- interchangeable seam guides and material-specific fixtures;
- improved work-zone lighting and camera mounts;
- low-friction, replaceable material guides;
- operator-adjustable setup aids with explicit calibration;
- improved fabric support and feed presentation;
- material/operation-specific setup cards;
- maintenance indicators based on validated state/quality observations;
- eventually, external feed/tension assistance in a separate safely engineered cell.

Every upgrade receives its own configuration identity and matched before/after experiment. A physical retrofit is not approved by this research document.

## 7. Benchmark specification

### 7.1 Product-quality outcomes

For each coupon, report:
- seam centreline / edge deviation: mean, P95, maximum;
- stitch length mean, deviation and coefficient of variation;
- skipped/broken stitch count with a fixed denominator;
- pucker/appearance assessment under a declared method and blinded inspection where feasible;
- seam strength under the applicable test method;
- material holes, feed marks, distortion and local damage;
- first-pass acceptance, rework and reject rate.

The machine signal is a predictor or explanatory observation. The physical coupon inspection remains a separate output authority.

### 7.2 Production and resource outcomes

- accepted seam length/coupons per operating hour;
- setup and changeover time;
- availability and stoppage causes;
- restart/recovery time;
- kWh per accepted functional unit with a declared metering boundary;
- thread/fabric waste per accepted unit;
- first-pass yield and repair/rework;
- calibration and maintenance labour burden.

Report OEE components separately. An aggregate OEE must not conceal poor product quality, missed defects or worker burden.

### 7.3 Observability outcomes

- channel coverage and data completeness;
- timestamp synchronization error;
- calibration drift and re-calibration frequency;
- fault/event detection precision and recall;
- false-positive and false-negative rates;
- time from fault onset to detection;
- localization/diagnosis accuracy where independent ground truth exists;
- uncertainty calibration and abstention when the machine/material domain is unknown;
- telemetry and analysis reproducibility from exact raw-record digest.

### 7.4 Generalization

Do not randomly split samples from the same material roll and call that generalization.

Evaluate:
- leave-one-material-lot-out;
- leave-one-fabric-family-out;
- leave-one-machine-model-out;
- leave-one-operation-family-out;
- changed lighting and work-area background;
- changed speed within the named allowed envelope;
- single- and multi-layer stack-up;
- worn versus serviceable components only where condition is independently assessed.

A model may abstain outside its qualified domain. Unknown is not failure, and unknown is not permission.

## 8. Baseline comparisons and target-setting

Compare on the same machine/task/material/recipe where possible:

1. stock manual operation with its standard OEM configuration;
2. OEM-native monitoring/setup features, if supported;
3. passive external retrofit;
4. passive retrofit plus operator-reviewed advice;
5. robot-assisted seam execution only as a distinct task, using #4865 and its task-specific benchmark.

For a fair robot-sewing comparison, reproduce the published task definition and metrology from the 2026 paper before comparing seam deviation. Do not treat its ±2.5 mm observed maximum as a universal tolerance.

### Proposed targets (not achieved)

- 100% of accepted signals bind to machine asset/configuration, material lot, recipe, job and calibration identity.
- Zero write/actuation calls in the P0–P2 telemetry system.
- Zero uncalibrated, stale or missing observations silently promoted to valid.
- Zero sensor installation that materially degrades output quality against the no-sensor control.
- 100% of benchmark output claims link to independently inspected physical coupons.
- Detect a preregistered fault set with precision/recall and false-alarm targets frozen only after baseline.
- Demonstrate at least one useful improvement (output consistency, setup/changeover, defect detection, maintenance lead time or resource intensity) without worsening the remaining declared critical outcomes.
- Publish retrofit BOM, installation time, downtime, calibration workload, service/repairability and warranty/OEM constraints.
- No transfer claim without held-out machine/material family evaluation.

The target is not to maximize the number of dashboard signals. It is to improve the cost, quality and confidence of accepted textile output.

## 9. Safety and standards

Candidate references; determine applicability, full-text requirements and local adoption before any physical retrofit:

- ISO 10821:2005 — industrial sewing machine safety requirements; ISO catalogue currently reports it as current after its 2015 review.
- ISO 12100:2010 — general machinery risk assessment and risk reduction; currently published, with ISO/DIS 12100.3 under development in 2026.
- IEC 60204-1:2016 + AMD1:2021 — electrical equipment of machines; consolidated Edition 6.1 is valid, Edition 7 is under development.
- ISO 11111-1:2016 — common textile machinery safety requirements; a new approved work item / draft is under development in 2026.
- Applicable ISO 11111 parts for the exact textile-machine family, including fabric-manufacturing, finishing and other machinery scope.
- Any new safety-related control function must be engineered and validated against applicable machinery functional-safety requirements by competent parties.

References:
- https://www.iso.org/standard/2103.html
- https://www.iso.org/standard/51528.html
- https://webstore.iec.ch/en/publication/71256
- https://www.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/09/17/91742.html

No guard, needle shield, interlock, emergency stop, operator protection, protective bonding or electrical isolation may be bypassed. Do not add an actuator or modify the safety-related control system in this programme's passive pilot.

## 10. Architecture and Luminous ownership

| Layer | Owner / authority | Boundary |
|---|---|---|
| Machine process/capability/recipe semantics | Symthaea MFG-PROC #5686 | Generic capability and process identity; use existing owner |
| Native telemetry adapters | MFG-PROC-009 | OPC UA / MTConnect / vendor export projections, not a second protocol |
| Textile channel profiles and retrofit commissioning | This issue #5311 | Textile-specific passive sensing, calibration and benchmark |
| Cloth material/physics | Symtropy #456 | Named deformable-material profile |
| Cloth manipulation / robotic sewing | Symtropy #4865 | Robot task and actuation authority remains outside this issue |
| Knitout parsing / machine preflight | Symtropy #4878 | Static machine capability preflight; no knitting controller writes |
| Factory schedule | Symtropy #4875 | Multi-stage scheduling; not a machine controller |
| Product, lot, process, inspection and lifecycle evidence | Mycelix owner issues | Canonical evidence admission and lifecycle events |
| Garment intent, pattern, fit and accepted product | Symthaea #11731 / Atelier issues | Product authority; machine data cannot rewrite design intent |

## 11. Immediate implementation sequence

1. Freeze this machine observability profile and the machine survey checklist.
2. Define a small JSON Schema for read-only observation records and synthetic fixtures.
3. Build semantic checks for exact machine/configuration identity, sensor calibration, timestamp order, explicit missingness and prohibition of command fields.
4. Run the synthetic checks and publish the results; do not claim physical commissioning.
5. Survey the available local lockstitch machine and OEM documentation.
6. Freeze a stock-machine coupon baseline.
7. Select minimum-value sensor pack; obtain machine-owner and qualified electrical/safety approval.
8. Install and validate passive sensors with an independent no-sensor output control.
9. Run paired material/operation coupons and blind output inspection.
10. Only then evaluate advisory diagnostics or any mechanical process upgrade.

## 12. Claim ceiling

This document and its synthetic contracts establish only a research design and data boundary. They do not qualify any textile machine, retrofit, machine capability, safe operating envelope, process recipe, output quality or production rate. Read-only telemetry is not machine safety. A fitted sensor is not a calibrated sensor. A calibrated sensor is not independent product inspection. A machine prediction is not authority to alter an industrial process.
