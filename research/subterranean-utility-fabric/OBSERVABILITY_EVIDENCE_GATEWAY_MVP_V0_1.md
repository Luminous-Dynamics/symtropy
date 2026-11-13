# Observability Evidence Gateway — MVP Specification v0.1

**Workstream:** OBSERVABILITY-EVIDENCE-GATEWAY-001 (#5006)  
**Programme:** SUF #4910  
**Status:** product-engineering specification; no deployed service claimed  
**Date:** 2026-11-13

## 1. Product definition

The first SUF product is a **read-only Observability Evidence Gateway**. It integrates one authorized DAS source/export with one utility-owned GIS/fibre inventory and produces a versioned, uncertainty-aware and independently reviewable account of what is observable and what requires investigation.

It is not:
- a new optical interrogator;
- a competing utility GIS or canonical asset registry;
- a SCADA or protection system;
- a generic alarm dashboard;
- an autonomous maintenance or dispatch agent;
- a robot fleet requirement.

The product should deliver value before a customer buys new sensing hardware or underground construction.

## 2. First vertical slice

The minimum vertical path is:

`authorized source export
→ immutable source artifact
→ source-observation envelope
→ exact fibre/route identity
→ calibration receipt
→ geographic position + uncertainty
→ event-conditioned observability state
→ proposed follow-up measurement
→ independent human/owner acceptance`

The first integration can begin offline with an exported bundle. Real-time streaming is not a prerequisite to prove the evidence chain.

## 3. Service boundaries

### 3.1 Source adapter

Inputs:
- one interrogator/vendor export;
- native configuration metadata;
- authorized waveform/source objects;
- exact export revision, owner and rights.

Responsibilities:
- preserve original bytes;
- calculate/verify digest after acquisition;
- record metadata exactly as supplied;
- label missing fields as unknown/unavailable/withheld;
- create a normalized envelope without changing source units or waveform content.

No adapter invents gauge length, sample rate, resolution, clock accuracy, fibre identity or calibration.

### 3.2 Spatial adapter

Inputs:
- one owner-approved fibre inventory/GIS export;
- exact record and route revisions;
- declared CRS, axis order, datum/epoch and spatial rights;
- independent endpoint/control references where available.

Responsibilities:
- preserve source records and conflicts;
- reference owner-issued identity rather than mint canonical asset IDs;
- distinguish fibre path from utility route geometry;
- produce explicit route hypotheses when ambiguous;
- carry access/disclosure classification.

### 3.3 Calibration service

Compose the Rust reference kernel. A result binds:
- source event observation digest;
- fibre identity and revision;
- endpoint identities and direction;
- interrogator configuration;
- calibration receipt/method revision and artifact digest;
- route geometry revision and artifact digest;
- phenomenon-time validity window;
- coordinate frame and horizontal-equivalent error bound.

The kernel is limited to one already-disambiguated span and projected metres. A caller must not mistake it for a geodetic transformation engine or survey certification.

### 3.4 Observability evaluator

The qualification cell is:

**segment × event class × minimum magnitude × coupling/environment profile × interrogator configuration × time interval.**

The evaluator records POD only with declared trials, independent truth, denominators and confidence intervals. Otherwise it emits unknown/not assessed rather than invented coverage.

### 3.5 Investigation planner

Compare:
- fixed calendar inspection;
- threshold alarm dispatch;
- risk/severity rule;
- spatial coverage greedy;
- random policy under the same budget;
- value-of-information policy using separate EIG and EVSI.

A high-information action does not automatically have positive decision value. Incomparable cost/loss units suppress a net recommendation. The output is advisory and never dispatch authorization.

### 3.6 Evidence and acceptance service

The gateway produces a review bundle containing:
- source artifacts and rights states;
- transformation/calibration receipts;
- event hypothesis and bounded claim level;
- uncertainty and blind/degraded coverage;
- expected value/action rationale;
- reviewer/adjudicator and disposition;
- post-work evidence reference if a separately authorized intervention occurs.

Mycelix/owner systems preserve authorization and receipts; a receipt does not prove physical correctness.

## 4. Deployment profile

### Pilot A — offline evidence integration

- One approved vendor export.
- One approved GIS/fibre inventory export.
- Synthetic or authorized route/control data.
- No live SCADA or control integration.
- No field dispatch.
- Reproducible source-to-position pipeline.

### Pilot B — read-only field observation

Only after Pilot A passes:
- exact fibre-owner/utility permissions;
- SCADA/telecom change approval and cybersecurity review;
- non-interference test and rollback;
- qualified independent survey control;
- human supervision and agreed event/maintenance procedures.

### Pilot C — targeted physical verification

Only after a specific decision-value case and provider envelope:
- technician, survey crew or qualified robot;
- permit/access/asset-owner authorization;
- retrieval and emergency plan;
- independent post-inspection acceptance.

## 5. Product acceptance metrics

### Evidence integrity
- exact source bytes/digest/byte length;
- parent-child derivative lineage;
- source rights and permitted purpose;
- timebase validity and gap accounting;
- deterministic replay.

### Spatial validity
- fibre identity/revision match;
- endpoint/direction match;
- calibration and route revision freshness;
- cross-track/along-track error separately;
- interval coverage and interval width;
- unknown and ambiguous route length.

### Event and operational value
- POD/critical misses by class and magnitude;
- false alarms per observable fibre-km-day;
- detection latency;
- actionable-event confirmation ratio;
- false physical dispatches;
- human review minutes;
- investigation cost and time saved.

### Lifecycle economics
- cost per qualified observable km-year;
- cost per independently accepted actionable event;
- bytes/compute/energy per km-day;
- survey/calibration cost;
- support and false-dispatch burden.

No weighted single score can conceal critical misses, blind intervals or invalid geographic transforms.

## 6. Initial commercial proposition

The first paid pilot should promise only a measurable, agreed investigation outcome—for example, lower time/cost to locate and verify a defined class of event on an identified route under a frozen evidence and access scope.

The price unit should be an agreed monitored asset portfolio or qualified observable kilometre-year, not a vendor's nominal maximum interrogator range.

The customer retains its canonical assets and operational authority. Luminous provides the evidence continuity, qualification, investigation logic and verification bundle.

## 7. Build sequence

1. Validate raw and processed source envelopes and exact artifact digests.
2. Import one authorized spatial inventory without altering owner identities.
3. Supply calibration control points and route geometry.
4. Run the Rust transform and preserve receipts.
5. Compute one observability map with explicit unobserved/degraded states.
6. Run fixed/threshold/coverage/VOI policies on a frozen scenario set.
7. Publish a read-only SensorThings-compatible representation and exportable evidence bundle.
8. Demonstrate independent replay and review.
9. Only then seek an owner-approved field pilot.

## 8. Claim ceiling

This is an MVP specification. It does not claim production software, a live vendor adapter, calibrated utility route, physical DAS performance, paid customer, cost reduction, safety approval or authority to control/dispatch/maintain infrastructure.
