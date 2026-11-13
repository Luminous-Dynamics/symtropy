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


## 9. Additional paradigm-shift gates

### 9.1 Design for calibratability, not merely sensor installation

The as-designed corridor must expose the gauge freedoms that remain in route registration. A design is not geographically calibratable merely because it has a fibre, a known endpoint or a CAD centreline. Specify which independent controls fix translation, orientation, scale, endpoint order and branch identity for the actual topology. Where controls are insufficient, preserve relative geometry and abstain from absolute registration.

### 9.2 Redundancy must be independent where it matters

A second DAS span that shares the same trench, chamber, power, clock, backhaul and ownership authority may fail with the primary. The design benchmark therefore includes common-cause cut sets and failure-domain diversity. Count independent failure domains, not sensor devices. Do not claim high availability by multiplying independent failure probabilities unless independence is supported.

### 9.3 Design for lifecycle requalification

Every splice, reroute, endpoint reversal, firmware/configuration change, coupling repair or datum/as-built change needs an explicit change-impact map. The design must identify which transformations and event-class qualifications become stale, and how to restore service without silently retaining obsolete geographic outputs.

### 9.4 Procure outcomes, not interrogator specifications

The acceptance object should be a qualified service cell (segment × event class × minimum magnitude × profile × time), with POD/false-alarm evidence, localization uncertainty, blind exposure, calibration freshness and owner access. Vendor range, gauge length and channel pitch are explanatory inputs, not service-level guarantees.

## 10. Lifecycle comparison rules

Report the same lifecycle boundary and price year for all alternatives. Include:
- acquisition/lease and owner-access cost for existing fibre;
- trench/duct, chamber, fibre, splice and reinstatement CAPEX;
- interrogator lease/purchase, calibration and commissioning;
- power, backhaul, storage, compute, retention and support;
- survey, periodic requalification, maintenance, repair and outage;
- human review, false dispatch, verification and post-event work;
- replacement, decommissioning and residual value.

Report undiscounted CAPEX/OPEX separately from discounted lifecycle cost, with discount rate and horizon explicit. Avoid assigning a monetary avoided-outage benefit without a defensible counterfactual and independently verified event outcomes. If risk reduction and financial cost do not share a defensible unit, show a Pareto frontier and do not invent a net-value ranking.

## 11. Standards and sources

- OGC MUDDI Part 1 v1.0 explicitly treats traceability to survey measurements, sensor observations and supporting evidence as a design requirement, and requires a mandatory core of underground network concepts with optional extensions. It is a conceptual model, not a complete implementation or security policy: https://docs.ogc.org/is/23-024/23-024.html
- OGC SensorThings API 2.0 supports HTTP and MQTT sensing interfaces. Its 2026 standard states authorization and stored-data integrity/confidentiality are implementation responsibilities: https://docs.ogc.org/is/23-019/23-019.html
- Li et al. (2026), DAS engineering-infrastructure review, DOI 10.1016/j.autcon.2026.106784: https://doi.org/10.1016/j.autcon.2026.106784
- Cohen et al. (2026), buried-fibre geolocalization, DOI 10.1109/TGRS.2026.3729504: https://doi.org/10.1109/TGRS.2026.3729504
