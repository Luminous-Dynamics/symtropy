# Terrain Mobility Portfolio and Owner-Experience Benchmark v0.1

**Status:** research protocol and synthetic contract only. No candidate has been physically tested or qualified by this document.

## 1. Purpose

This benchmark connects two usually separated design questions:

1. Can a person or household complete useful journeys dependably, affordably, accessibly, and with an ownership experience they choose?
2. Can a ground robot complete useful work across its declared terrain envelope, manage energy and faults, and recover or stop safely when it cannot?

The shared question is **what mobility service was delivered, under which declared conditions, at what cost and with what evidence?** A vehicle class, terrain label, route planner score, simulated success, or attractive rendering is not an answer on its own.

This is a child of Symtropy's shared robotics scenario-pack work (#1595). It composes rather than replaces the family-vehicle substrate (PR #1762), the existing terrain simulator and sensor-coverage work (PRs #1776 and #1781), Symthaea's road/off-road dynamics owner (#6247) and external benchmark registry (#6266), Mycelix's vehicle lifecycle dossier (PR #5131), and Sol Atlas's evidence-aware spatial/service-capacity work (PR #116).

## 2. Paradigm shifts

### 2.1 Capability envelopes, not universal terrain claims

Every candidate declares an operating design domain (ODD): surface/material, slope and cross-slope, step/rut/clearance, payload and centre-of-gravity range, speed range, weather/visibility, sensor/localization mode, energy reserve, and relevant faults. Each claim binds to an exact configuration and evidence class.

Capability states are **supported**, **unsupported**, and **unknown/not evaluated**. Unknown is not a weak form of supported. A platform may reject or retreat from a scenario and still behave correctly. No platform receives permission because a feature, sensor, or evidence item is missing from the manifest.

### 2.2 Mission and service first, locomotion second

Compare conventional road wheels, independently steered/articulated wheels, tracks, wheel-leg hybrids, and legged ground robots only on task cells where the same useful work and constraints are applicable. Add a specialized amphibious surface platform only for explicitly modelled water-edge/surface tasks. Do not imply that one architecture should span paved commuting, deep sand, rubble, forestry, and planetary exploration.

The primary unit is mission-specific: passenger trips served, payload delivered, targets inspected, service tasks completed, or safe return achieved. Do not compare these units by flattening them into one number.

### 2.3 The terrain is part of the dynamics

A class such as “gravel” is insufficient. The scenario must expose measurable or declared geometry, slope, surface uncertainty/dynamics, contact/slip or sinkage observations where modeled, payload, environmental conditions, and sensor state. A navigation path is not terrain-interaction validation. Unknown voids, drop-offs, water, unstable slopes, and occlusion must remain explicit hazards.

### 2.4 Completion is not the only successful outcome

The evaluator should distinguish mission completion, valid safe retreat/stop, rejected out-of-domain, and indeterminate evidence. It must never reward an unsafe completion over a safe abort. A route that is mathematically reachable is not necessarily traversable, energy-feasible, recoverable, or safe to dispatch.

### 2.5 Personalization is agency, not an opaque “joy score”

For personal/family vehicles, test whether owner-selected settings and configurations are useful, understandable, repeatable, accessible, reversible, and private. Treat ride comfort, cabin noise/vibration, ingress/egress, cargo fit, control feel, profile restoration, maintenance transparency, and repairability as distinct measures. Satisfaction is user-reported evidence with declared collection methods; it must not be inferred from biometrics or silently learned.

Profile controls:
- Explicitly selected profiles only; no identity inference from who is holding a key or phone.
- Owner can inspect, change, disable, export, and reset personalization.
- A profile never relaxes braking, stability, traction, collision, legal, or other hard safety gates.
- No cloud account or paid subscription should be assumed as a prerequisite for basic manual control or safe use.
- Local/private storage is the default target; data collection and model-training use require separate consent.
- Household users can maintain separate profiles and resolve conflicts explicitly; a majority preference must not silently overwrite another person's settings.

## 3. Benchmark portfolio

The repository fixture freezes synthetic control cases for these families.

| Family | Representative conditions | Useful measures |
|---|---|---|
| Personal/family road vehicle | dry/wet pavement, gravel, washboard, potholes, payload/CG shift | trip completion, braking/stability observations, energy, availability, ride/noise comfort, access, cargo fit, lifetime cost |
| Articulated wheeled ground robot | rocks, roots, steps, cross-slope, changing contact | traversal success, slip/contact state, progress, stability margin, energy and recovery |
| Tracked ground robot | sand, mud, soft soil, loose slope | sinkage/immobilization, useful work per energy, turning cost, damage/wear, serviceability |
| Wheel-leg hybrid | obstacle/step transitions and irregular rubble | transition success, time/energy overhead, stability, failure modes, maintenance burden |
| Legged ground robot | roots, rocks, rubble, constrained footholds | locomotion test measures, mission utility, energy, falls/near-falls, recovery and wear |
| Specialized surface-water platform | only explicitly modelled surface-water tasks | water-entry/exit and stability under a separately declared envelope; no implied submersible capability |
| Cross-cutting system | low light, dust, GNSS denial, sensor faults, communications loss, stale calibration, energy/parts shocks | out-of-domain handling, detection latency, safe-state/retreat, human override, evidence completeness |

The included v0.1 corpus is a **hand-authored synthetic test vector set**, not a terrain-frequency sample, model training set, realistic fleet distribution, or platform ranking. Values in it define test inputs only; they are not safe limits for real hardware.

## 4. Metrics and initial targets

### 4.1 Hard gates (must not be traded away)

These are proposed acceptance targets for the benchmark and its future evaluator, not achieved results or certification standards:

- **0** unsafe dispatches and **0** unauthorized actuator commands in negative-control tests.
- **100%** of unsupported, stale, conflicting, or configuration-mismatched cases resolve to reject, out-of-domain, indeterminate, or an explicitly safe stop/retreat—not an optimistic pass.
- **100%** of energy-return controls with insufficient declared reserve reject dispatch or select a qualified safe recovery; no missing energy input is silently interpreted as zero consumption.
- **100%** of profile/identity conflicts require explicit owner choice; **0** silent personalization changes.
- **100%** of designated sensor/communications fault cases follow the scenario's declared fallback/stop behavior.
- **100/100** identical-input replays produce identical canonical output; **100/100** scenario-order permutations produce byte-identical canonical output.
- Every negative control remains in the report. No success-only filtering.

### 4.2 Mission and terrain performance

Initial engineering aspiration for an in-domain simulation campaign: at least **95% nominal mission completion** and **90% completion-or-safe-degradation** under the preregistered stress set. These are screening goals, not evidence of real-world reliability. They must be reported per terrain, payload, environmental, and platform stratum with denominators and confidence intervals; a strong average may not hide a weak stratum. Safe abort and out-of-domain rejection are scored as correct dispositions in the scenarios that require them, not as mission completion.

Begin with at least 100 paired, fixed-seed simulation repetitions per candidate/scenario for screening. For confirmatory claims, freeze the hypothesis, power/sample-size method, failure definition, exclusions, and held-out scenarios before seeing results. The screening sample is not a reliability certification. Use a simple transparent baseline (such as a conservative fixed controller/planner) before adding Symthaea-driven policies.

Measure separately:
- useful work and mission completion;
- progress per energy and energy per useful-work unit;
- path/arrival time and deviation, not just shortest distance;
- traction/slip/sinkage/contact observations where a qualified model exists;
- stability/load-transfer margin from a model appropriate to the platform;
- immobilization, detection latency, recovery/retreat time, and stranded energy;
- intervention/override rate and task continuation after faults;
- parts/tool wait, diagnosis, hands-on repair, verification, rework, and total out-of-service duration;
- whole-life ownership costs with financing, energy, maintenance, parts, insurance/licensing where applicable, depreciation, fallback transport, and downtime. Unknown cost remains unknown, never zero.

A 20% return-energy margin is a reasonable **initial design-study scenario value** to test sensitivity, not a universal specification. Real reserve requirements must be derived from platform, mission, uncertainty, return route, environmental conditions, and the consequence of being stranded.

### 4.3 Owner experience and personalization

Measure objective and owner-reported outcomes separately:
- repeatable restoration of every supported, explicitly selected profile setting;
- time and steps for common changes (seat/mirror/climate, driving response, navigation preferences, storage/cargo configuration);
- occupant-level vibration/noise under preregistered comparable conditions, without claiming a single comfort value fits everybody;
- ingress/egress and accessibility-task completion using consented, non-identifying task profiles;
- manual override discovery and completion; preference reset and data export/delete;
- cargo-fit utility, configuration-change time, repair-information usability and local serviceability;
- owner-rated comfort/control and whether settings match declared preferences, with scale, cohort, missing responses and uncertainty shown.

Do not collapse comfort, accessibility, cost, autonomy, ecological footprint, repairability and safety into a single weighted leaderboard. Show a Pareto frontier and the infeasible alternatives, alongside each non-negotiable gate.

## 5. Evidence and claims

Every result should include:
- scenario/corpus version and SHA-256;
- exact platform hardware/software/configuration and ODD profile digest;
- simulator, physics/solver, controller, calibration, sensor model and seed identities;
- environment, payload, mission, energy and fault inputs with units;
- raw observations and derived metrics, missingness, uncertainty, exclusions and negative controls;
- evidence class and validation scope;
- result disposition: completed, safe stop/retreat, out-of-domain, blocked/indeterminate, or failed;
- independent-verification identity where present.

Keep these classes distinct: synthetic fixture; simulator output; bench/HIL measurement; controlled field observation; independently inspected result; regulatory/roadworthiness receipt. Signatures, hashes, and reproducibility establish lineage/integrity, not truth of the underlying physics or adequacy of a model.

The first gate is deterministic semantic correctness. The next is model verification and comparison to a suitable independent reference. Then bench/HIL validation, supervised low-consequence field trials, and only later any domain-specific safety and regulatory assessment. Public-road vehicle qualification cannot be inferred from a robotics benchmark. Human/owner studies require consent, privacy review, and appropriate ethics/operational approvals.

## 6. Standards and research anchors

- [ISO 18646-1:2016](https://www.iso.org/standard/63127.html) describes locomotion performance tests for wheeled service robots in indoor environments. It is not an off-road vehicle safety standard.
- [ISO 18646-2:2024](https://www.iso.org/standard/82643.html) covers navigation performance such as pose accuracy/repeatability, obstacle handling, path deviation, narrow passages, and mapping. Its scope and limitations must remain explicit.
- [ISO 18646-5:2026](https://www.iso.org/standard/86850.html) covers locomotion performance criteria and test methods for legged service robots; its publication explicitly does not verify/validate safety requirements.
- NASA/JPL's [ERNEST rover field test](https://www.jpl.nasa.gov/news/nasa-testing-advanced-capabilities-for-moon-mars-rovers/) reports 16 miles of driving over 37 hours and an actively articulated/steerable four-wheel design. It is a valuable design case, not proof that the architecture is best for every terrain.
- [TartanGround](https://tartanair.org/tartanground/) describes 63 simulation environments, 878 trajectories, and about 1.44 million multimodal samples; its evaluation reports generalization weaknesses of models trained on existing datasets.
- The [terrain traversability evaluation survey](https://onlinelibrary.wiley.com/doi/full/10.1002/rob.22461) distinguishes direct terrain assessment from downstream task assessment and reviews the variety of sensors, robots and methods involved.
- Reuse the existing public Luminous Dynamics owners linked in issue [TERRAIN-MOBILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/1821) rather than duplicating vehicle dynamics, evidence, geospatial, or Mycelix record authorities.

## 7. Implementation sequence

1. Freeze and audit this schema/corpus; preserve every negative control.
2. Build a standalone Rust contract auditor and canonical report format, then compare it against an independently stated disposition oracle.
3. Attach the corpus to the existing Symtropy simulation/test-world scenario pack. Do not add a second terrain authority or motor-control API.
4. Add candidate profiles one at a time, each with exact configuration and evidence-bounded capability envelopes.
5. Compare transparent baselines before Symthaea/HDC/CfC/adaptive policy variants; use held-out environments to test transfer.
6. Link Mycelix vehicle/repair-event evidence and Sol Atlas terrain/site evidence through typed references, without moving their ownership into Symtropy.
7. Run physical/owner evaluations only through separately authorized, instrumented protocols.

**Claim ceiling:** this v0.1 artifact defines the questions and synthetic controls. It does not assert a platform winner, measured success rate, validated vehicle design, cost saving, physical capability, real-user preference, or safety qualification.
