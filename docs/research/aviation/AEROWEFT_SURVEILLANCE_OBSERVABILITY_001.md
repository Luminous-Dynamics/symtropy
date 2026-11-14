# AeroWeft — Surveillance Observability and Unobserved-Traffic Evidence

**Tracking issue:** [AEROWEFT-SURVEILLANCE-OBSERVABILITY-001 #5275](https://github.com/Luminous-Dynamics/symtropy/issues/5275)  
**Consumes:** Symtropy #2279/#2297/#5163/#5186/#5270; Symtropy #3909/#4079; Symthaea #10775/#10826; Mycelix #435/#458/#5904; Sol-Atlas #189/#825  
**Status:** synthetic-first coverage/observability contract  
**Claim ceiling:** finite claim-admission and unresolved-occupancy semantics only; no sensor qualification, DAA system, collision-avoidance performance, surveillance service or operational airspace safety claim.

## 1. Research thesis

A missing track is an observation about a surveillance system, not a fact about the physical world.

AeroWeft must distinguish:
- no cooperative transmission received;
- cooperative transmitter outside qualified receiver coverage;
- a cooperative message received but stale, malformed or integrity-uncertain;
- a non-cooperative aircraft that the available sensor class is not qualified to detect;
- a sensor detection that did not become a stable/qualified track;
- a track with position/time uncertainty too large or too old for the downstream use;
- qualified coverage for a specifically declared traffic class and environment.

**No report is not an empty sky.** A missing, stale, out-of-coverage or unqualified track cannot be turned into an empty occupancy set.

## 2. Standards and operational references

1. **ASTM F3442-25 — Standard Specification for Detect and Avoid System Performance Requirements.** Active specification last updated 15 May 2025. Its stated scope concerns smaller uncrewed aircraft with maximum dimension at most 25 ft and airspeed below 100 kt in lower-risk airspace. It expects mixed cooperative and non-cooperative crewed traffic, says performance applicability is determined by the competent authority, assumes no tactical ATC separation service to the UA, and does not address UA-to-UA, terrain/obstacles or birds/natural hazards. It is a bounded comparator, not a universal DAA standard for every AeroWeft aircraft/operation. DOI: https://doi.org/10.1520/F3442-25
2. **FAA Part 107 waiver evaluation information — Detect and Avoid.** FAA guidance says DAA performance is evaluated at the system level after the avoidance manoeuvre, not from sensor detection alone; it also requires cooperative and non-cooperative traffic performance to be measured separately and warns that blending them can create misleading results. AeroWeft stops before the avoid manoeuvre and supplies evidence/unknown occupancy to existing planning owners. https://www.faa.gov/sites/faa.gov/files/uas/commercial_operators/part_107_waivers/Part-107-Waiver-Section-Specific-Evaluation-Information.pdf
3. **EUROCAE ED-267 — OSED for Detect and Avoid in Very Low Level Operations.** Defines an operational services and environment basis for UAS DAA, including the relationship to other airspace users and supporting services. This is an operational-design reference, not a sensor benchmark score. https://www.eurocae.net/product/ed-267-osed-for-detect-and-avoid-in-very-low-level-operations/
4. **NASA — Detect-and-Avoid Surveillance Range Requirements for EO/IR Sensors (2021).** Describes DAA surveillance, alerting and guidance and the need to detect cooperative and non-cooperative traffic with onboard and/or ground surveillance. It is a research reference for sensor-specific coverage assumptions, not a transferable range guarantee. https://ntrs.nasa.gov/citations/20210025061
5. **2025 ADS-B spoofing TDoA multilateration study.** Evaluates receiver-network spatial distribution and signal redundancy for a TDoA approach using OpenSky. It identifies receiver-network redundancy as a prerequisite; multiple receivers do not automatically imply independent clocks, power, networks, antenna sites or upstream feeds. DOI: https://doi.org/10.1016/j.trpro.2025.10.010

## 3. Paradigm shift

AeroWeft should not build an omniscient synthetic air picture. It should build a **qualified observability map with explicit blind/unknown cells**.

A coverage claim is scoped to:
- 3D volume and time interval;
- traffic class and minimum target characteristics;
- sensor modality, receiver/sensor identity and configuration;
- calibration and profile revision;
- evidence method: measured, certified, modelled, synthetic or unknown;
- separate detection, track initiation and track continuity claims;
- position/time uncertainty and update/latency bounds;
- weather, lighting, clutter, obstruction and line-of-sight conditions;
- PNT/time quality and reference frame/vertical datum;
- common-mode domains: power, time, antenna, site, network, firmware, provider and data pipeline;
- limitations and evidence validity interval.

No sensor range, probability of detection or coverage area is inferred from a modality name, vendor label, map point or unvalidated nominal geometry.

## 4. Ownership and non-duplication

| Responsibility | Existing owner | This work consumes |
|---|---|---|
| 4D planning, conflict screening and broadphase | Symtropy #2279/#2297 | Coverage and unresolved-occupancy claims |
| Aircraft occupancy/reachability | Symtropy #5163 | Bounded envelopes; no aircraft physics duplicate |
| Candidate schedules and resources | Symtropy #5186 | Observability constraints for candidate policy scoring |
| PNT/clock evidence adapter | Symtropy #5270 | Qualified time/integrity and typed unknown states |
| PNT/communications and timing degradation | Symtropy #3909/#4079 | Upstream service-state receipts |
| Navigation health and clock correlations | Mycelix #435/#458 | Qualified time and health facets |
| Common-mode independence | Mycelix #5904 | Exact source/failure-domain snapshot |
| Receive-side PNT assessment and oracle provenance | Symthaea #10775/#10826 | Hypotheses, confidence/abstention, ground-truth manifests |
| Spatial source and rights context | Sol-Atlas #189/#825 | Source-qualified volume/context snapshots |
| Reciprocal options, recovery, service quality, blocking | Symtropy #5207/#5224/#5248/#5261 | Consequences of observability gaps |

This track owns only the coverage/observability claim contract, traffic-class-separated coverage results, unresolved possible-occupancy representation, tiny synthetic fixture and measurement protocol. It does not own the receiver network, sensor, detector, fusion engine or avoidance policy.

## 5. Traffic classes must remain separate

At minimum V0 distinguishes:
1. **Cooperative, valid** — a cooperative report with current integrity and qualified time evidence.
2. **Cooperative, stale/invalid/unknown** — a report exists but fails time, position, source or integrity requirements.
3. **Non-cooperative crewed traffic** — no cooperative position report is assumed; only a qualified modality-specific coverage claim can support detection/observability.
4. **Uncrewed/unknown traffic** — included only where the declared coverage profile explicitly supports that target class. Do not extend ASTM F3442-25's crewed-aircraft scope to UA-to-UA encounters by assumption.
5. **Unclassified possible traffic** — source, modality or target class is unknown; it remains unresolved.

No pooled detection rate may conceal a failure on one traffic class.

## 6. Coverage and absence states

For each traffic class and volume/time cell, the coverage status is one of:
- QUALIFIED_COVERAGE — exact profile and evidence support the named class/environment for the cell;
- PARTIAL_COVERAGE — only a sub-volume/time or subset of conditions is supported;
- DEGRADED — upstream service is degraded under the declared profile;
- NOT_QUALIFIED — modality or environment is outside the admitted profile;
- INDETERMINATE — source, time, calibration, PNT, dependency or validity evidence is unknown/contradictory.

The observation result is a separate dimension:
- QUALIFIED_TRACK — a current track from a qualified source matches this traffic class/cell/time;
- NO_OBSERVATION — no report/track arrived;
- INVALID_OR_STALE_TRACK — a report exists but fails freshness/integrity/clock requirements;
- UNRESOLVED — a report/claim exists but cannot be qualified.

A QUALIFIED_COVERAGE result is not proof that no traffic exists. It is a bounded claim about a particular sensor/profile's ability to observe a particular class under declared conditions.

For NO_OBSERVATION, NOT_QUALIFIED or INDETERMINATE, downstream receives an unresolved possible-occupancy region/time or a typed no-assessment result. It must never receive an empty traffic set solely because no cooperative track was received.

The observability adapter does not invent unobserved aircraft kinematics. Unknown traffic envelopes must come from a declared scenario/operating profile or a separately qualified encounter-envelope source. Where none exists, return NOT_ASSESSABLE rather than inventing a radius, speed or direction.

## 7. Tiny synthetic benchmark

Golden fixture: fixtures/aeroweift-surveillance-observability/tiny-coverage-gap-v0.json.

All cells, aircraft, sources, time intervals, modalities and failure domains are fictional. The fixture covers:
1. cooperative aircraft inside a valid ADS-B coverage claim;
2. cooperative aircraft outside that claim;
3. non-cooperative crewed aircraft in an ADS-B-only volume;
4. EO/IR coverage that is qualified only in a declared lighting/environment regime;
5. three nominal receivers sharing a time/power/network failure domain;
6. TDoA receiver sites that are spatially distinct but have unknown common timing dependency;
7. PNT/time integrity degradation that invalidates a previously time-qualified track;
8. weather/obstruction that withdraws EO/IR coverage for a cell/time;
9. no report with no independent qualified sensor;
10. positive controls with separate, explicitly qualified cooperative and non-cooperative coverage claims.

Expected outcomes:
- ADS-B-only coverage does not qualify non-cooperative traffic observability.
- A cooperative report outside coverage/validity is not promoted to a current track.
- Three receiver IDs with one shared failure-domain family do not count as three independent families.
- Unknown TDoA timing dependency yields INDETERMINATE, not qualified multilateration coverage.
- Losing PNT/time integrity invalidates the affected time-dependent track claim, not the physical aircraft.
- Missing reports without independent qualified coverage become NO_OBSERVATION with unresolved possible occupancy, never an empty sky.
- EO/IR claims apply only within their declared environment envelope.
- Positive controls prove the evaluator can accept class-specific qualified coverage; it must not reject everything by default.

## 8. Required metrics

Report separately by traffic class, volume, environment, sensor profile and time:
- qualified/partial/degraded/unknown volume-time fraction, with denominator and evidence completeness;
- probability of detection only where independent event truth and a defensible sampling frame support it;
- false alarms per receiver-hour and per operational day;
- detection latency and track initiation latency;
- track continuity, drop/reacquisition rate and age-of-information;
- position/time error bounds and uncertainty calibration;
- independent sensor/receiver families and common-mode domains;
- blind/unobserved cell-time volume and exact cause;
- number and duration of unresolved possible-occupancy intervals sent downstream;
- false-empty rate (target zero on frozen negative controls);
- propagation latency from PNT, clock, network or sensor fault to loss of affected coverage claim.

Do not assign a detection probability to an unobserved cell merely because no labelled intruder happened to occur there. Keep coverage, detection, tracking, classification and physical occupancy as distinct claims.

## 9. Matched baselines

On identical synthetic target events, volume/time cells, environments, receiver topology, sensor-failure scenarios and truth manifests:
1. ADS-B-only cooperative surveillance;
2. multi-receiver ADS-B with a declared TDoA estimator class;
3. cooperative ADS-B plus separately qualified non-cooperative sensor class;
4. one ground-based DAA coverage claim with weather/obstruction limitations;
5. multi-sensor coverage with common-mode dependencies;
6. coverage-unaware planning that treats no report as empty airspace (optimistic negative control; must fail);
7. class-specific observability adapter feeding #2279/#5163 with unresolved occupancy.

ASTM F3442-25 is a bounded system-performance reference for its stated smaller-UA/crewed-traffic/lower-risk scope. FAA guidance's cooperative/non-cooperative separation is an evaluation principle. Neither turns this adapter into a complete DAA system or covers every AeroWeft use case.

## 10. Falsification and metamorphic controls

- Removing a cooperative report cannot create a stronger empty-airspace claim.
- Restricting a coverage volume cannot increase qualified coverage.
- Shortening a validity interval cannot increase qualified time.
- Widening time/position uncertainty cannot create a more precise track.
- A sensor outside its declared lighting/weather/obstruction envelope cannot support that cell.
- ADS-B-only observations cannot qualify non-cooperative traffic coverage.
- Three receivers with a shared critical clock/power/network dependency do not count as three independent families.
- An unknown dependency graph is not independence.
- Loss of PNT/time integrity invalidates the affected time-dependent surveillance claim.
- A fresh signature does not repair stale or physically unqualified sensor evidence.
- Input-order/ID permutation preserves semantic outputs.
- NO_OBSERVATION, NOT_QUALIFIED and INDETERMINATE never become an empty occupancy set.
- The positive control retains qualified class-specific coverage.

## 11. Acceptance targets

- 100% exact agreement with an independent finite coverage/claim oracle on the tiny corpus.
- Zero false-empty outcomes in every frozen negative-control case.
- Zero cooperative-only coverage claims promoted to non-cooperative traffic coverage.
- Zero shared/unknown dependencies promoted to independent corroboration.
- 100% explicit volume/time/traffic-class/environment scope on qualified claims.
- 100% explicit unknown/unqualified coverage reporting.
- Exact deterministic replay and permutation invariance.
- No DAA manoeuvre, tactical instruction, operational clearance, surveillance service or aircraft-control output.

These are synthetic claim/coverage semantics, not a sensor-performance qualification, real-world DAA safety claim or proof that an airspace volume is empty.

## 12. Execution order

1. Freeze the ten-case coverage-gap fixture with independent traffic-event truth.
2. Implement a small coverage-claim oracle and class-specific admission semantics.
3. Consume PNT/time status and clock evidence from #5270/#435/#458/#5904.
4. Consume spatial/source validity from Sol-Atlas #189/#825.
5. Export qualified coverage and unresolved possible-occupancy inputs to #2279/#2297 and #5163.
6. Score #5186 candidate schedules against unresolved-occupancy constraints; do not build a new scheduler.
7. Compare against ASTM F3442-25's stated applicability and task-level DAA references before any future sensor-specific study.

**Core invariant:** no cooperative report is not no aircraft; an unobserved volume is an evidence gap, not free airspace.