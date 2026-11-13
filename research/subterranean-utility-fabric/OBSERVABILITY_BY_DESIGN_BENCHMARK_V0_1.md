# Observability-by-Design Qualification Benchmark v0.1

**Workstream:** OBSERVABILITY-BY-DESIGN-QUAL-001 (#5008)  
**Programme:** SUF #4910  
**Status:** research protocol and synthetic campaign plan only  
**Date:** 2026-11-13

## 1. Thesis

Existing communications fibre can provide a brownfield sensing opportunity where its route, access and coupling support the required event class. New/renewed corridors can go further: they can be designed so sensing paths, calibration anchors, endpoint identities, access nodes and independent verification are part of the infrastructure design.

This is not a claim that every corridor should contain a dedicated DAS fibre. Each design must be compared against the best eligible no-additional-sensing or conventional-inspection baseline using the same service requirement and lifecycle boundary.

## 2. Paradigm shift

Move from:
- build the utility corridor;
- discover later that its records, route, access and coupling are insufficient;
- retrofit sensors and reconstruct undocumented topology;

to:
- declare the service, asset identity and risk requirements;
- compare sensing/inspection architectures;
- co-design fibre path, coupling, topology, endpoints, calibration anchors and repair access;
- preserve exact as-built and transformation evidence;
- commission event-conditioned observability;
- maintain calibration after every physical change.

The aim is not maximum sensor density. It is the minimum lifecycle cost to achieve an agreed, independently qualified observability and verification service.

## 3. Design alternatives

Compare on matched route, utility service, event classes, design life and price year:

- A0 existing corridor/records with conventional inspection;
- A1 reuse suitable existing spare/dark telecom fibre;
- A2 target utility and sensing fibre in shared trench/duct;
- A3 separate dedicated sensing duct/fibre;
- A4 fibre attached/coupled to the target duct/conduit, with installation qualification;
- A5 sparse point sensors plus targeted survey and robot inspection;
- A6 hybrid DAS + point sensors + surveyed anchors;
- A7 no added sensing, hardening and conventional inspection.

Eligibility is site/profile specific. A sensing fibre may be inappropriate because it cannot meet optical budget, rights, access, coupling or lifecycle constraints.

## 4. Design variables

- fibre type and ownership/rights;
- direct burial, conduit, shared trench or dedicated sensing duct;
- lateral/vertical separation from target utility;
- coupling, backfill, soil/moisture and surface condition;
- slack, microbend, loops, splice count, branch/multiplex topology;
- endpoint accessibility and endpoint identity;
- calibration/tap-test/survey anchor location and spacing;
- protected access/test chambers and physical reference points;
- timing, synchronization, local buffering, power and backhaul;
- repair/reroute procedure, as-built update and calibration invalidation;
- common-cause exposure to flooding, excavation, fire, power and communication outage;
- rights, disclosure, ownership and multi-tenant access.

## 5. Benchmark metrics

**Sensing and observability**
- POD by event class and minimum magnitude;
- false alarms per observable fibre-km-day;
- event-chainage error and geographic-route error separately;
- observable/degraded/blind/unavailable length and duration;
- coupling/SNR/attenuation profiles and uptime;
- performance after repair, splice and route change.

**Calibration and spatial truth**
- independent anchor count/spacing and survey effort;
- P50/P95 cross-track and along-track errors;
- 90%/95% interval coverage and interval width;
- endpoint/route identity errors and stale-transform rejection;
- cost/time per independently accepted mapped kilometre.

**Maintainability**
- access/test-point reachability;
- sensor/fibre repair and replacement time;
- calibration downtime after intervention;
- human/robot access and retrieval;
- service disruption and common-cause failure recovery.

**Economics**
- installed sensing CAPEX, access rights and commissioning;
- data/storage/compute and telecom fees;
- calibration and re-survey;
- false dispatch and verification;
- avoided survey/excavation/outage cost;
- lifecycle cost per qualified observable km-year and accepted actionable event.

No single composite metric may hide blind sections, critical misses, invalid mapping or access/repair failures.

## 6. Qualification controls

- Direct-burial and conduit installations are distinct strata.
- Channel pitch, gauge length, spatial resolution, sample rate and range remain separate properties.
- A route plan is not an as-built survey; physical acceptance requires independently surveyed/exposed truth where needed.
- DAS chainage is not a geographic coordinate.
- Every route/fibre/endpoint/configuration change invalidates dependent transforms until requalified.
- No sensing path is assumed to be owned, accessible or commercially usable without explicit rights.
- Sensor/telecom outages remain visible; no-event cannot be inferred from missing data.
- Robots are optional targeted verification tools; no universal robot access assumption is allowed.
- No safety-critical actuation, protection, switching, excavation or repair authority is created by observability software.

## 7. Proposed gates (not achieved results)

- 100% of in-scope route length accounted for by event-class-specific observability state after commissioning.
- 100% of calibration anchors and route transformations linked to exact as-built and survey revisions.
- Zero high-confidence geographic event locations after endpoint/splice/route revisions until re-calibration.
- 95% spatial interval coverage on held-out independently surveyed controls, with interval width published.
- At matched cost/service outputs, demonstrate an improvement in qualified observable km-year or lifecycle investigation cost over the best eligible baseline.
- Explicitly report cases where no added sensing or conventional inspection is economically superior.

## 8. Claim ceiling

This benchmark does not authorize any physical corridor design or establish optimal fibre placement, safe utility separation, coupling quality, construction cost or field benefit. Real designs require competent engineering, utility-owner requirements, permits and independent acceptance.
