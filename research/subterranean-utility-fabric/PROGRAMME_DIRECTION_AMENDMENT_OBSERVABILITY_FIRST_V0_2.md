# SUF Programme Direction Amendment v0.2 — Observability First

**Date:** 2026-11-13  
**Supersedes:** the robot-first framing in `PROGRAMME_SPEC_V0_1.md` where the two conflict  
**Programme:** SUBTERRANEAN-UTILITY-FABRIC-001 (#4910)

## 1. Decision

The programme's primary product direction is now **observability-first underground infrastructure intelligence and verification**.

The original physical corridor, modularity and robotic maintainability research remains a valid application track. It is no longer the default starting point or the definition of the whole programme.

The new sequence is:

**reuse existing records and sensing assets → qualify observability → calibrate location and uncertainty → prioritize the next valuable measurement → dispatch a human, survey crew or robot only where justified → independently verify maintenance.**

The platform must create value before a customer commits to new underground construction or a Luminous-owned robot fleet.

## 2. Revised thesis

Underground infrastructure should become a continuously updated, uncertainty-aware and independently verifiable physical system. Existing communications fibre, where compatible and authorized, can provide one distributed sensing substrate. Existing GIS, utility records, surveys and specialist instruments provide complementary evidence. Targeted robots and human operators resolve questions that remain valuable and physically inspectable.

The platform sells a qualified understanding of infrastructure and its condition, not raw sensor range, a map picture, alarm count, robot mileage or a new proprietary asset registry.

## 3. Product layers and ownership

| Layer | Responsibility | Owner |
|---|---|---|
| Canonical asset identity and operational state | Utility-owned assets, topology, operating state and access rights | Utility / infrastructure owner |
| Sensing intake | Vendor-neutral, read-only acquisition adapters and source artifact preservation | Symtropy #4972 |
| Spatial evidence | Route revisions, CRS, uncertainty, provenance, rights and MUDDI-aligned representation | Sol-Atlas #755 |
| Event interpretation | Signal qualification, event hypothesis, abstention and investigation prioritization | Symtropy #4972, Symthaea |
| Decision value | Select next measurement / human / robot intervention using matched budgets and losses | Symthaea |
| Physical verification | Survey control, technician inspection, targeted robotic inspection and recovery | Qualified providers and applicable Luminous research tracks |
| Authorization and acceptance | Owner permissions, actor/custody records, work authorization and independent acceptance receipts | Utility owner + Mycelix |
| Physical intervention | Construction, switching, isolation, repair and commissioning | Authorized operators / competent contractors only |

No system acquires a new source-of-truth or physical authority by virtue of this programme.

## 4. Robotics becomes an evidence-directed capability

Robots remain strategically valuable for:
- measurements that fixed or distributed sensors cannot provide;
- high-risk or inaccessible inspection where the exact robot envelope is qualified;
- independent visual, acoustic, dimensional or physical verification;
- targeted mapping of route sections with high uncertainty and high decision consequence;
- post-maintenance acceptance and repeatable condition measurement.

Robots should not be deployed merely because a route exists or because a dashboard shows an anomaly. Mission selection must compare the expected value of information and intervention with access, safety, retrieval, operating and false-dispatch costs.

Existing sewer, cable-duct, walkable tunnel and general industrial robot results remain separate populations. A robot's presence does not establish inspection coverage or maintenance acceptance.

## 5. Revised success metrics

Primary platform metrics:
- fraction of in-scope route length with an explicit event-class-specific observability state;
- event detection/critical miss rate by class and minimum magnitude;
- false alarms per observable fibre-km-day;
- fibre-chainage and geographic-location errors reported separately;
- uncertainty interval coverage and abstention quality;
- blind/degraded/unavailable length and duration;
- time and cost per independently accepted mapped kilometre;
- investigation/robot dispatches per confirmed actionable event;
- expected decision-loss reduction under matched budgets;
- independent post-work acceptance and recurrence.

Physical corridor metrics remain relevant only when a specific construction or renewal option is being evaluated: accepted service, total lifecycle cost, safety/eligibility, restoration time, maintenance burden and independently verified outcome.

No universal composite score may conceal critical false negatives, blind sections, unqualified transforms or unsafe/unauthorized actions.

## 6. Immediate programme sequence

1. Reproduce the two separate 2026 DAS research results from authorized data or author-confirmed materials.
2. Build data-rights and source-population inventory for supplementary public research data; audit leakage before model evaluation.
3. Define vendor-neutral source ingestion and immutable waveform-artifact references.
4. Qualify the transform ladder from optical range to physical fibre chainage, route station, geographic coordinates and owner asset attribution.
5. Build class- and magnitude-conditioned observability coverage with explicit blind-section accounting.
6. Compare baseline, vendor, human and candidate investigation policies on frozen site-held-out data.
7. Connect Sol-Atlas MUDDI and SensorThings profiles without claiming conformance prematurely.
8. Integrate Symthaea value-of-information proposals with Mycelix/utility authorization boundaries.
9. Build a non-energized fibre/cable qualification rig and then an owner-supervised field pilot.
10. Expand into construction/renewal architecture evaluation where the evidence shows new physical corridors are economically justified.

## 7. Safety, rights and claim ceiling

- DAS remains read-only relative to SCADA and electrical protection.
- DAS does not provide protection-relay, switching, isolation, dispatch, excavation or repair authority.
- A missing signal is not evidence of no event when a segment is blind, decoupled, stale or unavailable.
- Fibre chainage is not a geographic coordinate; route/asset attribution requires separately versioned evidence and uncertainty.
- Rights to use fibre, raw traces, spatial data, commercial datasets and derived models must be explicitly established.
- Exact critical-asset geometry remains purpose-bound and restricted by default.
- Simulation, synthetic plans, desk research, authorized dataset reproduction, laboratory qualification and field qualification remain separate evidence classes.
- Construction and physical corridor designs still require competent domain-specific engineering, permissions and independent acceptance.

## 8. Claim ceiling

This amendment changes programme direction and sequencing. It does not claim a deployed observability platform, superior DAS performance, completed paper reproduction, calibrated route, MUDDI or SensorThings conformance, successful field detection, reduced theft, commercial savings or physical qualification.
