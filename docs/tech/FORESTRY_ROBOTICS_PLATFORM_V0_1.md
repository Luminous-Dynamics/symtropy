---
title: Forestry Robotics Platform Architecture
version: 0.1
status: research-and-simulation-contract
date: 2026-10-17
---

# Forestry Robotics Platforms v0.1

## Executive direction

Design a family of small, modular, evidence-producing robots with one shared mission contract and distinct hardware classes. Start with low-energy surveying and inspection. Do not start by automating felling, saws, chemical application, or heavy log handling: these have different hazards and require machine-specific risk assessment, functional-safety engineering, and validation.

This is an architecture and simulation gate, not a build-ready chassis, certified safety case, real-time controller, or permission to deploy in a forest.

## Research baseline

A 2025 IEEE Transactions on Field Robotics report describes autonomous legged-robot forest inventory across forests in three European countries over 18 months. It reports plots up to one hectare surveyed in under 30 minutes and typical diameter-at-breast-height (DBH) accuracy around 2 cm. This is a useful published reference point, not a guarantee for our system.
- Mattamala et al. (2025), DOI 10.1109/TFR.2025.3583972: https://doi.org/10.1109/TFR.2025.3583972
- Earlier system report: https://arxiv.org/abs/2404.14157

A forest trial of the SPOT platform found that the default navigation LiDAR generated noisy point clouds and tree-position errors of at least 1.9 m; data-collection path strongly affected tree identification and DBH results. That is a useful lesson: a mobile robot's navigation sensor should not automatically be treated as a metrology sensor.
- Robotics in Forest Inventories: SPOT's First Steps (2023): https://www.mdpi.com/1999-4907/14/11/2170

A 2025 field-evaluation paper describes a mobile robot using LiDAR, imagery, SLAM, and tree-trait estimation. A 2026 outdoor-coverage preprint reports 93.1–96.1% of planned area covered in its own varied outdoor tests. Both are comparison points tied to their methods and test conditions, not transferable pass marks.
- Fasiolo et al. (2025): https://doi.org/10.3390/robotics14070089
- Outdoor coverage and navigation (2026 preprint): https://arxiv.org/abs/2609.01384

The frontier is therefore more than improved detection: a forestry robot should know when its pose and measurements are trustworthy, degrade safely when sensing fails, and produce traceable inventory records with explicit uncertainty.

## Platform family

### F1 — Forest Scout (first platform)

Purpose: survey and inspect without cutting, chemical application, tree contact, or dangerous payloads.

- Chassis: compare low-ground-pressure tracked and 4/6-wheel bases for trail and understory work. Consider quadrupeds only where stepping mobility justifies added power, cost, and maintenance.
- Sensors: RGB, depth/stereo or LiDAR, IMU, wheel odometry, optional GNSS/RTK, and a calibration target for repeatable scans. Thermal/multispectral payloads are opt-in.
- Outputs: tree detections, DBH and method, stem/log geometry, terrain map, trail traversability, log-deck inventory, and sensor coverage/quality layers.
- Modes: simulation, teleoperated collection, then supervised low-speed mapping inside a site-approved geofence. Autonomous movement is an experimental milestone, not the default.
- No chainsaw, felling head, autonomous harvesting, or chemical-sprayer capability in v0.1.

### F2 — Restoration utility rover (later)

Purpose: seedling inventory, planting-site assessment, non-invasive sample logging, marker placement and small-material support.

Reuses navigation and evidence contracts but each physical tool requires separate hazard analysis, tool identity, energy isolation, interlocks, payload checks, maintenance records, and a dedicated negative-control suite. Soil penetration, planting, chemical use, and load carrying are not enabled merely because a module is present.

### F3 — Mill-yard / log-deck inspection rover (parallel bounded use)

Purpose: scan log decks, read lot tags, compare declared lots with observed geometry, document stack changes and inspect residual bins.

Start in controlled yards with surveyed travel corridors. Do not enter swing/fall/drop zones of moving equipment. An inventory record must be reconciled with a physical lot; a market listing or map feature is not proof that a log exists.

### F4 — High-hazard forestry machine (separate program)

Felling, processing heads, heavy log handling and high-power tools are a separate machine program, not an add-on plugin for F1. They need dedicated hardware safety architecture, control-system lifecycle, guarding/interlocks, risk analysis, qualified test procedures, operator instructions, and machine-specific conformity review.

## Modular architecture

Mission/operator console
→ typed mission request, site permission, versioned safety profile
→ Rust mission preflight (this PR: synthetic, software-only)
→ route/task proposal and simulation
→ independent safety-control system with independent sensors/interlocks
→ hardware-specific drive/tool controller (not included in this PR)

Sensor path:
sensors → time synchronization → state estimation → local map → tree/log detection → uncertainty/coverage record → signed/hashed evidence receipt → Mycelix provenance → Sol-Atlas spatial context → Symtropy inventory/process model → Symthaea suggests next inspection/measurement.

### Contract boundaries

- Mission identity includes request ID, task, timestamp, operator/site references, source revision, and autonomy mode.
- Robot profiles declare supported tasks, payload, speed, slope and localization limits.
- Safety profiles name the approving authority, risk-assessment record, version, expiry, and numeric limits; they cannot make a robot profile more capable.
- Telemetry includes timestamp, localization quality/error, perception and obstacle-sensor health, obstacles/clearance, mobility-sensor state, independent safety-controller state, E-stop state, communications, slope, speed and battery reserve.
- The only v0.1 results are eligible for simulation only, blocked fail-closed, or rejected out of scope. Autonomous mode requires separate review in v0.1. No state means safe to actuate.
- Receipts bind the exact input corpus, software/configuration version and source revision. Unknown values stay unknown; they are never filled with plausible guesses.

Use ROS 2/Nav2 or an equivalent middleware through replaceable adapters, not as the source of safety authority. Enforce timestamp and coordinate-frame consistency in integration tests; Nav2 documents the map/odom/base/sensor transform tree and GNSS integration:
https://docs.nav2.org/rolling/getting_started/navigation_concepts/state_estimation/
https://docs.nav2.org/rolling/tutorials/general_tutorials/navigation2_with_gps/navigation2_with_gps/

An emergency stop cannot depend on cloud connectivity. A communications failure must trigger the locally designed degraded-state policy. Actual brakes, speed limits, contactors and tool-energy isolation require qualified hardware design and validation; an E-stop field in JSON proves none of those functions.

## Safety standards: review applicability, do not infer conformity

- ISO 11850:2011 is listed as current (last reviewed/confirmed in 2023) and specifies general safety requirements for self-propelled forestry machinery. The ISO page also says a replacement is under development and machine-specific hazards need additional standards/risk work: https://www.iso.org/standard/51182.html
- ISO 18497 concerns highly automated agricultural machinery and includes agricultural/forestry machines in scope, but the standard set and relevant part must be checked against the selected design and market. Its general principles do not replace type-C machinery standards or risk assessment: https://www.iso.org/standard/62659.html
- ISO 12100 and ISO 13849-1 are candidate references for risk assessment and safety-related control systems. The actual applicable editions and performance requirements must be confirmed by a qualified machinery-safety engineer before hardware design.

Minimum lifecycle: hazard identification → risk reduction → safety function requirements → independent hardware/control implementation → verification and validation → instructions and residual-risk disclosure.

## Benchmark ladder

All numeric goals below are proposed project targets, not standards or demonstrated robot results. Freeze test protocols, ground truth, terrain strata and measurement methods before comparing platforms.

### Gate 0 — contract and replay (software only)

- 100% of frozen adverse cases produce the expected decision and reason codes.
- Autonomous mission mode remains blocked for v0.1 pending a separate autonomy validation programme; teleoperated/supervised scenarios are the initial baseline.
- 100% block unknown/denied site permission, expired profiles, stale/future telemetry, unhealthy critical sensors, engaged E-stop, missing capability, excessive localization error, slope/speed over limits and low battery.
- 100% reject high-hazard F1 tasks even if all other fields look favourable.
- Identical corpus bytes and source revision produce identical normalized receipts.
- Unknown schema keys fail closed; no actuator command schema exists.

### Gate 1 — sensing and mapping (bench + supervised field)

- DBH median absolute error ≤ 2 cm as an initial target; report p90 error and stratify by species, bark, tree size, occlusion and terrain.
- Tree-detection precision and recall each ≥ 95% on held-out manually annotated cases. Report missed trees and duplicates separately.
- Report median/p95 position, diameter and volume error, plus calibration and coverage. Missing/occluded cases remain in the denominators or are explicitly counted as unmeasured.
- Report localization drift per 100 m, relocalization success, time without valid pose and performance under GNSS loss, repeated trunks and seasonal change.
- One hectare in ≤ 30 minutes is a stretch replication target under a declared terrain/understory protocol, informed by the published result—not a guarantee. Include setup, recovery, data-processing and operator intervention time.

### Gate 2 — reliability and safe degradation

- 100% of injected critical faults enter the declared blocked/degraded state; zero continued eligibility after stale pose, critical sensor loss, safety-controller fault or engaged E-stop.
- Measure E-stop response, stopping distance and residual movement on physical hardware only with qualified procedures across speed, load, slope and surface; do not invent a universal number before the platform exists.
- Report intervention rate, mission completion/recovery, false stops/clears, MTBF/MTTR, sensor fouling, ingress protection and wet/dry battery performance.
- Test communications loss, GNSS denial, SLAM divergence, wheel slip, blocked route and immobilization; recovery must not create new uncontrolled motion.

### Gate 3 — multi-robot inventory

- Every observation contains robot ID, timestamp, map frame, uncertainty, sensor/model/config revision and replay/duplicate identity.
- Zero double counting of trees, logs or lots under overlapping scans; conflicts produce explicit reconciliation records.
- Report coverage versus time and the marginal coverage gained per additional robot; subtract collision-avoidance delays, duplicate scans, radio use and recovery time.
- Local safe degradation remains effective when fleet coordination/cloud connectivity is lost.

### Gate 4 — economic and ecological usefulness

- Compare cost per qualified, traceable observation and cost per hectare—not distance traveled or image count alone.
- Measure soil rutting/compaction, vegetation damage, wildlife disturbance, noise, energy and battery replacement.
- Report field repair time, common-part ratio, maintenance burden, operator training and spare-parts availability.
- Productivity, safety, sustainability and emissions claims require paired trials and explicit measurement boundaries.

## First implementation

The companion Rust contract accepts synthetic missions and emits an exact-input/source-commit receipt. Its three results are eligible for simulation only, blocked fail-closed, and rejected out of scope. It contains no hardware interface, route executor, motor command, or tool actuation.

## Next sequence

1. Freeze the mission-decision corpus with exact reason-code checks and cross-platform Rust tests.
2. Add a deterministic terrain-grid corpus for slope, soft ground, branches, low visibility, localization dropout and network loss.
3. Define a typed Rust observation envelope with bounded payloads and provenance/calibration metadata.
4. Build a digital-twin/simulation harness for coverage, DBH/position/volume error, injected faults and recovery.
5. Select a low-risk off-the-shelf base only after weight, traction, ground pressure, ingress protection, battery and maintenance needs are quantified.
6. Collect supervised data under site permission and consent; publish only authorized data or aggregate metrics/synthetic fixtures.
7. Add intervention tools one at a time behind separate hazard reviews. High-power forestry machinery remains a separate program.

## Research references

- Autonomous legged forest inventory, IEEE TFR (2025): https://doi.org/10.1109/TFR.2025.3583972
- SPOT forest inventory trial (2023): https://www.mdpi.com/1999-4907/14/11/2170
- Forest mobile-robot field evaluation (2025): https://doi.org/10.3390/robotics14070089
- Outdoor coverage/navigation preprint (2026): https://arxiv.org/abs/2609.01384
- ISO 11850 forestry machinery safety: https://www.iso.org/standard/51182.html
- ISO 18497 automated agricultural/forestry machinery: https://www.iso.org/standard/62659.html
