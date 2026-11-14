# Extreme-Environment Platform Readiness Benchmark v0.1

**Status:** synthetic-only input contract and readiness auditor. No result is a platform recommendation, physical qualification, environmental permit, route clearance or deployment authorization.

## Paradigm shift: design mission systems, not one universal vehicle

The unit of design is a **mission system**: platform + mission module + energy/replenishment + operator and communications concept + recovery plan + site safeguards. The correct platform might be a rivercraft, light rover, tracked carrier, sled train, small aerial robot, or a combination with a stationary sensing network. These are not automatically interchangeable. Ground mobility is not always appropriate, and a bigger or more capable vehicle is not necessarily better.

The first benchmark spans the Amazon Basin, Congo Basin, Antarctica, the Sahara and Afghanistan. Each is a scenario archetype, not a claim that a whole region has one terrain, one culture, one legal regime or one operating condition. Real deployment needs site/season/mission scope and informed local participation.

## Environment-specific design hypotheses

### Amazon Basin: waterway-first health and cold-chain access

The World Bank's 24 April 2026 analysis describes remote Amazonian fishing communities facing multi-day river logistics and losses associated with delays and limited cold storage. A design hypothesis is a repairable shallow-draft electric craft with a removable cold-chain/medical module, plus a small low-disturbance sensor rover where it is appropriate. Avoid treating road building as the default answer: routes, docks, community transport, habitat disturbance and contamination are first-class constraints. Source: [World Bank, transport and the Amazon bioeconomy](https://blogs.worldbank.org/en/latinamerica/how-better-transport-can-unlock-jobs-incomes-amazon).

### Congo Basin: protect soils and peatlands

UNEP identifies Congo Basin peatlands as critical carbon sinks and warns that they face climate, biodiversity and pollution pressures. A design hypothesis is a waterway-enabled community logistics option combined with lightweight sensors and repairable low-pressure land platforms restricted to approved paths. Heavy carriers do not receive a default advantage from payload capacity; a peatland/soil gate fails closed if site-specific approval and bearing evidence are absent. Source: [UNEP, Critical ecosystems: Congo Basin peatlands](https://www.unep.org/en/news-and-stories/story/critical-ecosystems-congo-basin-peatlands).

### Antarctica: authorized science and resupply only

The Antarctic Treaty Secretariat lists Resolution 1 (2026), adopted 21 May 2026, as the current revised Guidelines for Environmental Impact Assessment in Antarctica. The platform architecture must make environmental review, spill/waste containment, wildlife/site limits, cold qualification and recovery planning explicit. A successful synthetic audit cannot stand in for a national authorization or required assessment. Sources: [ATS Resolution 1 (2026)](https://www.ats.aq/devAS/e/Meetings/Measure/874?s=1) and [environmental impact assessment overview](https://www.ats.aq/e/eia.html).

### Sahara: thermal and dust autonomy, not solar optimism

A desert-capable design needs a declared hot-soak envelope, sealed/filterable dust paths, serviceable wheels/tracks, energy and water accounting, wadi/flood and loose-sand limits, and a recovery plan. Solar input may extend endurance but must not be treated as guaranteed traction or power. NASA/JPL reported a desert field test of an advanced rover prototype in March 2026; use that as evidence that analogue tests are practical, not proof that any candidate is qualified for the Sahara. Source: [NASA/JPL, desert field test with advanced rover prototype](https://science.nasa.gov/photojournal/desert-field-test-with-nasa-advanced-rover-prototype/).

### Afghanistan: civilian humanitarian mission and strict route gate

WHO's March 2026 assessment describes physical-access constraints including winter weather, snow/ice, poor road infrastructure and insecurity that limit humanitarian operations. UNMAS documents ongoing civilian risk from explosive ordnance and the coordination needed for humanitarian mine action. Use cases here are non-armed medical supply, civil infrastructure assessment, disaster response and humanitarian logistics under the responsible local operator. Any suspected explosive-hazard area is a no-go until an authorized process has established safe access. Unknown route status stays `needs_gate_evidence`; no autonomous mine/ERW manipulation, armed payloads, targeting or tactical deployment is within scope. Sources: [WHO Afghanistan situation analysis](https://www.who.int/publications/m/item/public-health-situation-analysis---afghanistan---march-2026) and [UNMAS Afghanistan](https://www.unmas.org/en/afghanistan).

## Shared architecture: common interface, different carriers

Potential reusable elements:

- **Mission interface:** mechanically keyed cargo/sensor modules with declared mass, center-of-gravity envelope, restraint verification and operator checklist.
- **Energy interface:** separately documented voltage/current envelope, isolation, ingress protection, thermal monitoring and safe disconnect. No universal battery chemistry is assumed.
- **Data and controls:** local/manual stop and recovery path, logged mode changes, explicit loss-of-communications behavior, offline-capable maps where lawful, and data minimization for civilian missions.
- **Repair model:** replaceable common wear items, documented diagnostic paths, basic tools, safe battery handling, parts provenance, and locally serviceable design where feasible.
- **Recovery interface:** defined lift/tow/drag/manual recovery points, mass and load limits, and a compatible recovery plan before the vehicle leaves the staging site.
- **Environmental safeguards:** site-specific disturbance thresholds, invasive-species cleaning procedures, waste and spill containment, wildlife/privacy boundaries and clear authority to suspend a mission.

Interoperability should be measured, not assumed: connectors, payload attachments, telemetry and service procedures need versioned contracts and conformance tests. Sharing interfaces should not force the same chassis or powertrain into every environment.

## What the auditor does

The Rust tool validates a strict schema and synthetic corpus, checks that each assessment declares every required gate and metric exactly once, enforces units and valid interval bounds, and emits one of four dispositions:

- `blocked_hard_gate`: at least one mandatory gate fails.
- `needs_gate_evidence`: no failure is declared, but at least one required gate is unknown.
- `not_assessable_incomplete_metrics`: gates pass, but required metric intervals are unknown.
- `eligible_for_human_review`: all declared gates pass and metric intervals are complete.

Precedence is intentionally fail-closed. Even an assessment marked eligible for human review carries `operational_authorization_granted: false`. The auditor does not rank candidates, optimize a score, generate routes, control a platform or certify safety.

## Benchmark targets

- **Hard-gate integrity:** zero failed or unknown-gate candidates enter human-review-eligible status.
- **Mission completeness:** 100% of assessments provide the exact mission-specific gate and metric inventory; duplicates, missing records and undeclared IDs reject the input.
- **Adversarial gate coverage:** all 3^5 = 243 combinations of five gate states match an independent disposition oracle.
- **Reproducibility:** byte-identical report after reordering inputs, provided semantic content is unchanged.
- **No-go integrity:** suspected explosive-hazard uncertainty blocks mission readiness; no autonomous hazardous-object manipulation or operational authorization is emitted.
- **Environmental integrity:** zero declared violation of a scenario's mandatory ecological or contamination gate admitted; ecological thresholds must be agreed per site and assessment method, not borrowed from a single global index.
- **Serviceability:** field report MTTR, spare-part availability, repair tasks completed using documented local tools, and recovery completion for each mission class.
- **Reliability:** report mission completion and its confidence interval separately for each declared operating envelope and environmental stratum. Do not pool unlike missions to advertise one global success rate.
- **Physical qualification:** require instrumented cold/heat soak, ingress/dust, slope/traction, water/landing, payload stability, communication-loss, stop/recovery and spill-containment testing before supervised pilots.

An eventual reliability target should be tied to a defined claim and sample design. As a statistical rule of thumb, about 59 independent failure-free trials are needed to support a 95% one-sided lower confidence bound of roughly 95% success; about 299 are needed for roughly 99%. Real tests should stratify by environmental conditions and mission, since repeated runs are often not independent and pooled averages can hide failures.

## Synthetic baseline

The fixture contains five environment scenarios, thirteen platform archetypes and fifteen synthetic assessments. Current authored expected dispositions are six eligible for human review, four hard-gate blocked and five requiring gate evidence. The metrics are constructed intervals, not manufacturer specifications or environmental measurements. Gate statuses are test controls, not evidence of actual authorization.

## Research sources

- Antarctic Treaty Secretariat, [Resolution 1 (2026), revised Environmental Impact Assessment Guidelines](https://www.ats.aq/devAS/e/Meetings/Measure/874?s=1).
- World Bank, [How better transport can unlock jobs and incomes in the Amazon](https://blogs.worldbank.org/en/latinamerica/how-better-transport-can-unlock-jobs-incomes-amazon), 24 April 2026.
- UNEP, [Critical ecosystems: Congo Basin peatlands](https://www.unep.org/en/news-and-stories/story/critical-ecosystems-congo-basin-peatlands).
- NASA/JPL, [Desert field test with NASA advanced rover prototype](https://science.nasa.gov/photojournal/desert-field-test-with-nasa-advanced-rover-prototype/), published 18 June 2026.
- WHO, [Public Health Situation Analysis — Afghanistan, March 2026](https://www.who.int/publications/m/item/public-health-situation-analysis---afghanistan---march-2026).
- UNMAS, [Afghanistan programme](https://www.unmas.org/en/afghanistan), updated 5 March 2026.

## Next phase

Replace synthetic gates with versioned evidence records: source, date, site/season, operator, qualification method, confidence/uncertainty, authority, and expiry/revalidation criteria. Then define simulation and hardware-in-the-loop tests per architecture, run controlled site tests, and pilot only under local expert supervision and the relevant environmental, aviation, medical, and humanitarian approvals.
