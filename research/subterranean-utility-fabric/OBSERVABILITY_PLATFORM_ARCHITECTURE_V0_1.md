# SUF Observability-First Platform Architecture v0.1

**Programme:** SUBTERRANEAN-UTILITY-FABRIC-001 (#4910)  
**DAS workstream:** FIBRE-OBSERVABILITY-QUAL-001 (#4972)  
**Status:** research architecture and interface proposal; not a production implementation or field qualification  
**Date:** 2026-11-13

## 1. Architectural decision

Make the primary SUF product an **infrastructure observability and verification platform**. Treat distributed fibre sensing as one potentially reusable sensing substrate, not as the platform itself. Deploy robots, survey crews and intrusive inspection only where their expected information or intervention value exceeds their cost and risk.

The platform shall not own a competing utility asset registry, interrogator, SCADA system, protection relay, GIS authority, work-order authority or engineering solver. It composes those systems through versioned evidence references and bounded adapters.

The product is an evidence-qualified view of what is known, what is observable, what remains uncertain, and what next measurement is worth acquiring.

## 2. Core paradigm shifts

| From | To |
|---|---|
| Robot patrol as the default source of underground knowledge | Reuse suitable existing fibre and existing records; deploy mobile systems selectively |
| A fibre route is either monitored or not | Event-class- and magnitude-specific probability of detection, uncertainty and blind-section accounting |
| DAS channel/chainage is a map coordinate | A versioned transformation chain from instrument range through fibre path and surveyed route to geographic coordinates |
| An alarm is a diagnosis | Separate signal anomaly, event candidate, event classification, asset attribution, geographic placement, causal diagnosis and maintenance decision |
| More data is always better | Acquire the next observation only when expected decision value justifies its cost and risk |
| A digital twin is a polished picture | An evidence twin that preserves revisions, conflicts, rights, missingness, uncertainty and claim ceilings |
| A robot mission is a success metric | Independently accepted information or maintenance outcome is the success metric |

## 3. Reference architecture

### Plane A — asset and rights context

The responsible utility or infrastructure owner retains canonical asset identity, operational state, route-access permissions and authoritative engineering limits. Sol-Atlas holds provenance-bearing spatial representations and transformations; it does not silently overwrite owner records or create an alternative asset register.

Inputs may include MUDDI-aligned asset/network records, existing GIS, as-built drawings, survey control, utility-owner records, work history and permitted public data. Every source keeps its own identity, version, rights and epistemic class.

### Plane B — sensor acquisition

A vendor-neutral adapter accepts observations from approved interrogators and complementary instruments. Candidate inputs include:

- DAS strain/strain-rate or vendor-native measurement products;
- distributed temperature or strain/temperature products where separately supported;
- fibre-optic current sensing such as CAMOS, kept distinct from DAS;
- SCADA-derived status/telemetry only through an owner-approved, read-only boundary;
- independent survey, tap-test, acoustic, visual, electrical-test and robot observations.

Each adapter declares exactly what it can ingest. It must not imply that every interrogator exports the same raw signal, units, calibration, clock, gauge length, channel pitch or resolution. Preserve the original vendor artifact and configuration alongside any normalized derivative.

### Plane C — observation and evidence

Represent each measurement as a claim-bearing observation with:

- unique observation and source-artifact identities;
- fibre, asset and route revisions referenced rather than assumed;
- instrument model/serial or an explicitly unavailable identity;
- interrogator mode, gauge length, channel pitch, pulse/repetition rate, sample rate and configuration revision where applicable;
- phenomenon time, result time, clock source, synchronization status and known offset;
- observed property, unit, encoding and processing method;
- calibration revision, route-transform revision and their validity interval;
- raw/derived artifact URI, byte length, media type, digest and custody/rights state;
- epistemic class, quality state, coverage state, uncertainty and reviewer/adjudicator.

A digest binds bytes; it does not authenticate a source or prove that a physical interpretation is true. An observation supports, contradicts or supersedes another observation; it does not mutate canonical asset state.

### Plane D — calibration and spatial registration

Use an explicit coordinate ladder:

1. **Instrument optical range** — the interrogator's distance/position axis as emitted by a specific configuration.
2. **Physical fibre chainage** — cumulative length along the actual optical path, including patch leads, slack, coils, splice offsets and path topology where known.
3. **Utility-route station** — station along a surveyed or otherwise evidenced route representation; it may differ from fibre length because the fibre can meander, loop, share a duct or take a different path.
4. **Geographic position** — coordinates in a declared horizontal CRS and, where supported, vertical datum/height reference and coordinate epoch.
5. **Asset attribution** — the claim that a position pertains to a named owner asset; this is a separate identity/evidence relationship, not a consequence of coordinate proximity.

Each transformation is an independently versioned, digest-bound activity with inputs, method, control points, residuals, uncertainty, validity interval and acceptance status. A transform can be one-to-many or ambiguous when paths branch, fibres are multiplexed, multiple routes fit the observations, or fibre identity is unresolved.

Do not assume a single global linear conversion from DAS bins to geographic metres. Do not collapse optical range, fibre length, route station and map distance into one field called chainage.

### Plane E — observability

The unit of qualification is not simply a fibre kilometre. It is a **route segment × event class × minimum relevant event magnitude × environmental/profile envelope × sensor configuration × time interval**.

Define a qualified probability-of-detection surface:

**POD(segment, class, magnitude, environment, configuration, time) = probability of detecting the declared event under that exact profile.**

Every estimate needs its trial denominator, reference truth, confidence interval, data split, profile envelope and calibration status. When these are missing, the state is unknown or unqualified, not 100% coverage.

At minimum, account for every in-scope route interval as one of: observable, degraded, unobserved, unusable, unavailable, not assessed or unknown. Preserve the reason and the relevant time interval. Event-class observability may differ on the same segment.

### Plane F — inference and event claims

Use a staged claim ladder. Higher levels require additional evidence and must not be inferred from a lower-level classifier score alone.

| Level | Claim | Minimum additional evidence |
|---|---|---|
| L0 | Signal/quality anomaly | Valid trace, timebase and instrument profile |
| L1 | Event candidate | Reproducible signal and independent trigger or robust repeatability |
| L2 | Event-class hypothesis | Class-specific validation on a matched profile; calibrated confidence and abstention |
| L3 | Asset attribution | Versioned route/fibre relationship and owner asset identity evidence |
| L4 | Geographic placement | Valid chainage-to-route transform, CRS/datum and uncertainty |
| L5 | Causal diagnosis | Independent physical/engineering evidence and domain-specific reasoning |
| L6 | Maintenance recommendation | Consequence assessment, qualified domain limits and decision-value analysis |
| L7 | Accepted work outcome | Authorized intervention and independent post-work acceptance evidence |

The system must be able to return Accepted, Rejected, Indeterminate or Unsupported. It must abstain when route, coupling, signal quality, source identity or test domain is inadequate.

### Plane G — investigation and intervention planning

Symthaea may rank information-gathering options such as another interrogator configuration, a calibration pass, a tap test, survey control, a technician visit, a robot inspection or a specialist physical test. It does not dispatch, authorize work or operate equipment.

Compare policies using the same scenario corpus and cost model:

- fixed calendar inspection;
- alarm-threshold dispatch;
- severity/risk rule;
- spatial-coverage greedy;
- random selection under the same resource budget;
- value-of-information / expected-value-of-sample-information policy;
- human expert selection where a fair, consented comparator is available.

A measurement is valuable only when it can improve a real decision. Report information gain separately from expected decision-loss reduction; more entropy reduction does not necessarily create more economic value. Do not calculate net value where losses and acquisition costs are in incomparable units.

### Plane H — authority and accepted maintenance

Utility owners and their authorized human operators retain asset-state, operational, protection and work-order authority. Mycelix may preserve authorization, role, custody, inspection and acceptance receipts using the existing ownership boundaries. It must not treat a ledger receipt as proof of physical correctness.

DAS data intake and platform inference are read-only relative to SCADA/protection. No DAS result may trip, isolate, switch, energize, dispatch, excavate or repair an asset. Operational use requires fibre-owner and SCADA/telecom-owner permission, cybersecurity review, change control, non-interference tests and a recovery plan.

## 4. Interoperability strategy

Use existing standards as composition points, not as a reason to invent a new platform API:

- **OGC MUDDI Part 1 v1.0** for underground feature concepts, identity relationships and network context. Sol-Atlas's A.1–A.4 conformance work remains separate and must be completed before any conformance claim.
- **OGC SensorThings API 2.0** for sensor/observation metadata and interoperable observation access. It was approved 2026-09-14 and published 2026-11-04. Implement a read-only serving profile for operational consumers; keep authorization in the utility owner's IAM/policy layer because the standard does not prescribe an authorization model.
- **OGC/ISO Observations, Measurements and Samples (OMS)** semantics for observations and feature-of-interest relationships.
- **OGC SWE Common Data Model** for declared structured measurement result types where appropriate.
- **W3C PROV-O** or equivalent provenance semantics for source, derivation and transformation activities.
- **HDF5/Zarr/vendor-native immutable waveform objects** as candidate payload formats, selected per dataset/interrogator and preserved with original bytes. The SensorThings observation record should reference a waveform object and its metadata/digest, not inline an unbounded high-rate trace.

A normalized event/observation envelope is a Luminous profile; it is not a new OGC standard and does not imply that every vendor exposes a common raw DAS interface.

## 5. Industry positioning

AP Sensing and OptaSense already offer interrogators, event detection/visualization products, and infrastructure monitoring capabilities. Public vendor specifications include long nominal monitoring ranges, but range, channel pitch, gauge length, spatial resolution, sensitivity and qualified detection performance are different quantities.

Luminous should not claim superior raw interrogator performance or generic event-detection accuracy without matched, independently witnessed tests. The initial differentiated product is:

1. vendor-neutral evidence intake and source preservation;
2. versioned route/fibre identity and calibration;
3. event-class-specific observability and blind-section maps;
4. cross-vendor replay and independent result qualification;
5. rights-bounded spatial context and interoperable observation metadata;
6. decision-value-ranked investigation;
7. independent human/robot verification and accepted-maintenance evidence.

The commercial unit should be a qualified service/asset-kilometre-year or a monitored critical-asset portfolio with declared coverage, false-alarm burden, response workflow and lifecycle cost—not raw interrogator range.

## 6. Performance and economic scorecard

Never hide critical failure modes in one weighted score. Publish separate vectors:

- event detection/critical miss rate by class and minimum magnitude;
- false alarms per monitored fibre-km-day and per 1,000 healthy channel-hours;
- alarm latency P50/P95;
- event chainage error and geographic placement error as different metrics;
- confidence/interval calibration and risk-coverage/abstention curves;
- observable/degraded/blind/unavailable length and duration;
- source-data and route-calibration freshness;
- availability and recovery time after sensor/interrogator/communications failures;
- inspection dispatches per confirmed actionable event;
- cost per qualified observable km-year and per independently accepted mapped km;
- expected decision-loss reduction and regret under matched budgets;
- mean human adjudication time and false-dispatch cost;
- post-intervention acceptance and recurrence.

Cost accounting includes interrogator ownership/lease, fibre access, installation/coupling, engineering survey, communications approval, compute/storage, data retention, support, false dispatches, physical verification, avoided survey/excavation, outage consequences and maintenance downtime. Use current local quotations and sensitivity ranges before making South African commercial claims.

## 7. Programme ownership

| System | Responsibility |
|---|---|
| Symtropy #4972 | DAS ingestion/replay, coupling qualification, event-screening and observability benchmarks |
| Symtropy chainage-calibration tranche | Coordinate-axis transforms, route topology, calibration residuals and failure cases |
| Sol-Atlas #755 | MUDDI-aligned spatial evidence, route geometry revisions, CRS/uncertainty, SensorThings interoperability profile |
| Symthaea | Bounded event triage, investigation ranking and value-of-information proposals |
| Mycelix | Owner permissions, custody, approval, work and independent acceptance provenance |
| Utility/fibre owner | Canonical assets, exact route rights, operational state, SCADA/security boundaries and work authority |
| Robot/human providers | Physical inspection, survey control, retrieval and independent verification |

## 8. Immediate research gates

1. Obtain authorized access to the 2026 paper datasets or author-confirmed reproduction materials.
2. Audit public DAS datasets for terms, metadata completeness, source populations and leakage risks.
3. Build read-only vendor adapters and immutable waveform object references.
4. Define the optical-range → physical-fibre → route-station → geographic transform contract.
5. Qualify per-class observability and blind-length reporting before claiming route coverage.
6. Compare detector and investigation policies on frozen site-held-out datasets.
7. Only then design the non-energized physical fibre/cable rig and owner-supervised field pilot.

## 9. Claim ceiling

This document specifies a research architecture. It is not a deployed monitoring service, a validated event detector, a route survey, a MUDDI/SensorThings conformance result, a safety case, a SCADA integration approval, an electrical protection function, a robot qualification or evidence of commercial savings. Synthetic plans and simulated observations remain explicitly distinct from measurements.
