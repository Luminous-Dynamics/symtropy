# SAFRB Global Interoperability Profile v0.1

**Status:** research specification / proposed integration profile  
**Programme:** SAFRB, Power Fabric and Luminous cross-project infrastructure assurance  
**As-of date:** 2026-11-26  
**Evidence maturity:** architecture and public-standard review; no global deployment or conformance claim  
**Primary integration:** Symtropy #7483; Mycelix #6692; Symthaea #13696; Sol-Atlas #106 / #895

## 1. Purpose

SAFRB began with a South African fuel-resilience scenario, but its core question is global:

> Can a declared organization, infrastructure network or community support a specific service obligation at a specific place and time, using resources that are admissible, accessible, deliverable, convertible and independently evidenced?

The global system must work across countries, infrastructure operators, products, standards, legal systems, languages, units, calendars, data maturity and connectivity levels. It must not assume that one jurisdiction's law, identifier system, measurement convention, evidence source or operating practice is universal.

This document proposes a portable semantic core with versioned industry profiles and independently governed jurisdiction overlays. It is a profile and architecture contract, not a new global asset registry, legal rules engine, optimizer, physical solver, identity system or evidence authority.

## 2. Global design principles

1. **Common semantics, local authority.** The protocol can standardize how a claim is represented without deciding whether a particular issuer, contract, regulator or operator is recognized in every jurisdiction.
2. **Federated ownership.** Each participant remains authoritative for its own inventory, contracts, prices, assets, capacity, releases and operational commitments.
3. **Evidence does not launder claims.** A representation change must not strengthen certainty, authority, currentness, specificity or completeness.
4. **Unknown is a first-class state.** Missing, stale, conflicting, preliminary, revoked, unsupported and not-evaluated are distinct; none silently becomes zero, available, safe or guaranteed.
5. **Physical, communication and evidence graphs are different.** Successful message delivery does not prove cargo movement; distinct network edges do not prove independent physical routes; a provenance link does not prove truth.
6. **Service is beneficiary- and time-specific.** A tonne, litre, MWh, nameplate rating or aggregate stock total is not a beneficiary service claim.
7. **Evidence maturity is explicit.** Synthetic, reported, observed, measured, independently modelled, independently verified, HIL-tested, commissioned and periodically requalified evidence must not be collapsed.
8. **Privacy and sovereignty by design.** Exact inventory, sensitive facility topology, customers, prices, contracts and worker locations are not globally replicated by default.
9. **Offline operation must remain honest.** Cached claims retain original observation, record and validity times; offline proposals are not commitments; reconciliation preserves conflict.
10. **Standards are adapters, not truth engines.** Standards conformance establishes conformance to a declared profile, not physical correctness, legal recognition or operational fitness.

## 3. Layered global architecture

### 3.1 Portable common core

The common core defines only the minimum semantics required to exchange claims:

- globally scoped or namespaced participant, issuer, provider, verifier and resource references;
- claim type, profile version, subject and scope;
- quantity, dimension, unit and uncertainty interval;
- temporal semantics and validity window;
- source snapshot, provenance, derivation and dependency references;
- evidence maturity, applicability and qualification disposition;
- conflict, supersession, withdrawal and revocation;
- purpose-limited disclosure and recipient constraints;
- exact parent-artifact and verifier references;
- explicit claim-language ceiling.

The core does not define a single legal interpretation, trust score, national inventory source, global currency, physical model or mandatory event transport.

### 3.2 Versioned industry profiles

An industry profile narrows the common semantics for a domain and pins exact standard editions, data models, code lists, units, constraints and tests.

Candidate profiles include:

- fuel and liquid products: commodity/product/grade/batch, assay, temperature/density basis where applicable, volume/mass/energy conversion policy, custody, quality release, storage and transfer;
- electricity and microgrids: electrical service profile, measurement boundary, power/energy distinction, grid topology references, model/measurement maturity, network feasibility and HIL lifecycle;
- maritime and ports: port/terminal/resource identities, port-call events, berth/transfer operations, S-100 product references and navigation-data claim ceilings;
- rail and inland freight: route section, train path, equipment and terminal capacity scope, timetable/operation windows and infrastructure-owner authority;
- water: source, treatment state, quality profile, storage, conveyance, environmental constraints and beneficiary service;
- communications: site and service availability, route/contact semantics, latency and redundancy;
- industrial process: feedstock compatibility, process yields, ramp, maintenance, output quality and production-service boundary.

A single resource may participate in several industry profiles. Adapters must preserve the exact parent claims and must not use an industry label to authorize cross-domain conversions.

### 3.3 Jurisdiction and locality overlays

A jurisdiction overlay supplies locally governed interpretation and applicability. It may identify:

- competent authority and authoritative source;
- applicable laws, regulations, licences, permits, contracts and regulator-issued profiles;
- issuer recognition and delegated authority rules;
- local asset identifiers and crosswalks;
- data residency, privacy, retention and cross-border disclosure constraints;
- local metrology rules and applicable measurement standards;
- local currency, tax and settlement requirements;
- local calendar, time-zone, language and script requirements;
- emergency-service and critical-infrastructure designation where lawfully available;
- effective, expiry, amendment and supersession dates.

A country code is not a legal decision. A syntactically portable credential is not automatically recognized for a local purpose. If authority, recognition or legal effect cannot be established, the result remains Unknown or NotEvaluable and requires qualified local review.

## 4. Canonical claim lifecycle

Keep the following claims separate and append-only:

1. **Declared capability:** what an asset or provider is rated or designed to do.
2. **Observed state:** what a source or sensor reported at a particular observation time.
3. **Evidence admitted:** what passed the applicable source, quality, calibration, scope and verifier gates.
4. **Available resource:** what is both physically present and within the applicable admissibility, authority and validity envelope.
5. **Proposed allocation:** what a planner proposes, not a reservation.
6. **Owner-authoritative commitment:** what the controlling owner has accepted through its own conflict-safe mechanism.
7. **Execution event:** what the responsible operator recorded as an action or movement.
8. **Observed outcome:** what a separate source observed after the event.
9. **Qualified delivery:** what an independent domain validator can support from the admitted evidence.
10. **Beneficiary service:** what was delivered at the named beneficiary boundary under the specified service profile.
11. **Service assurance claim:** what the complete evidence and qualification chain permits the system to say.

An ETA is not an arrival. An arrival is not quality release. A signature is not physical truth. A route proposal is not a booking. A generator's nameplate is not delivered electricity. An energized asset is not beneficiary service.

## 5. Time, quantity and uncertainty

Every event-bearing claim must preserve distinct time fields where available:

- event/business time asserted by the source;
- observation/acquisition time;
- repository record/ingestion time;
- valid-from / valid-until interval;
- decision/evaluation time;
- planned operation interval.

Use explicit time profile and half-open interval semantics where the domain owner requires them. Preserve source-local time, UTC offset, named time zone and pinned time-zone database version where relevant. Do not replace missing source time with ingestion time.

Every quantity must have:
- declared physical dimension and machine-readable unit;
- source value and any normalized value;
- exact conversion factor, rounding rule and conversion provenance;
- uncertainty representation and evidence basis;
- applicability envelope and measurement conditions.

An upper-bound quantity cannot support a lower-bound guarantee. Interval widening cannot improve a guarantee. Unknown is not zero. Incompatible products and grades cannot be summed. Conversions between crude, refined products, fuel energy, electrical energy and useful service require their own qualified process or physics contract.

## 6. Standards and interoperability candidates

The following are candidate adapters, not universal mandatory requirements. Implementations must pin the precise edition, profile, scope, status, source artifact and retrieval date actually used.

| Domain | Candidate standard / profile | Appropriate role | Important limitation |
|---|---|---|---|
| Issuer claims | W3C Verifiable Credentials Data Model 2.0 (Recommendation, 2025-05-15) | Portable issuer/holder/verifier claim envelope where suitable | Does not decide whom a verifier should trust or establish physical truth |
| Supply-chain events | GS1 EPCIS 2.0.1 (published 2025-07-01) paired with CBV 2.0.0 (published 2022-06-22) | Cross-enterprise visibility and traceability events; selected sensor summaries | Does not replace owner-native inventory accounting, metrology or source qualification |
| Trade/transport locations | UNECE UN/LOCODE Recommendation 16 and pinned code-list release | Port, airport, inland terminal and trade-location references | A location code is not a tank, berth, pump, route section or asset identity |
| Units | UCUM 2.2 | Machine-readable quantity/unit interchange and dimensional validation | Domain admissibility and physical conversion rules remain profile-owned |
| Geospatial features | OGC API - Features | Web feature access using interoperable API building blocks | A feature geometry does not establish asset identity, condition or authority |
| Geospatial discovery | OGC API - Records | Discovery of source/asset metadata records | A catalogue record does not establish rights, freshness or truth |
| Geospatial exchange | OGC Features and Geometries JSON (JSON-FG) 1.0.0 | GeoJSON-compatible feature exchange with richer geometry/CRS semantics | Must preserve the exact CRS, datum, transformation and source lineage |
| Provenance | W3C PROV-O / PROV-JSON | Exchange mapping for entities, activities and agents | Provenance presence is not proof of claim truth or causal independence |
| Electricity networks | IEC CIM (IEC 61970 / IEC 61968 profiles) | Utility asset/network semantics and system integration | Requires exact profile, version and mapping validation |
| Substation automation | IEC 61850 series; applicable IEC 62351 security profiles | Electrical automation data models, SCL and communications | Not a generic fuel/logistics protocol or a substitute for OT security qualification |
| Microgrid testing | IEC TS 62898-3-5:2026 | HIL, commissioning and periodic testing lifecycle reference | Does not qualify this synthetic work or confer certification |
| Language | BCP 47 | Language/script/region tags | Human translation quality and legal equivalence need separate review |
| Civil time | IANA Time Zone Database, version-pinned | Local civil-time and UTC-offset interpretation | Historical and future rules can change; pin version for reproducibility |
| Jurisdiction identifiers | ISO 3166 family and national subdivision profiles | Jurisdiction context and crosswalks | Country/subdivision identifiers do not determine legal authority |

Standards status must be explicit: published standard, technical specification, recommendation, industry profile, draft, proposal, withdrawn or superseded. A draft is not silently upgraded to a mandatory standard. A URL that resolves to a moving "latest" page is not an immutable standards pin.

### 6.1 Key semantic warnings

- W3C VC 2.0 is useful for portable claims, but issuer trust and purpose-specific recognition remain outside its data model. Mycelix must keep recognition and authorization explicit.
- GS1 EPCIS can exchange event visibility and aggregated sensor summaries. Raw measurements should remain available through permissioned references where appropriate; a sensor summary is not a calibrated measurement certificate.
- UN/LOCODE identifies trade/transport locations. Do not use it as a unique identifier for terminal assets or capacity-bearing equipment.
- UCUM supports dimensional clarity. It does not authorize every mathematically convertible unit in every domain.
- OGC feature/record interfaces support discovery and spatial projection. Sol-Atlas remains read-only with respect to canonical evidence and qualification.
- IEC CIM and IEC 61850 adapters must retain the underlying engineering semantics and cybersecurity boundaries; no generic "IEC compliant" boolean.
- IANA TZDB and BCP 47 versions must be pinned in reproducible cross-locale tests.

## 7. Cross-project ownership and data flow

### Symthaea — decision support and evidence acquisition

Consumes unresolved proof obligations and scenario/model envelopes. Recommends which evidence or investigation could change a decision and at what expected or robust cost. It does not authorize an inspection, create evidence, verify physical truth or recompute the canonical network certificate.

Reference: Symthaea #12403, #12515 and #13696.

### Mycelix — federation, identity, authority and evidence exchange

Owns participant/asset identity, scoped capabilities, issuer recognition, jurisdiction overlays, evidence lineage, permissioned disclosure, durable effect receipts, disputes, revocation and reconciliation. Each operator remains the source of truth for its own assets and commitments.

Reference: Mycelix #4900, #4901, #5695, #5762, #5945, #5954, #5963, #5920 and #6692.

### Sol-Atlas — spatial/temporal observability

Projects source-qualified asset, route, hazard, service and dependency references. Each view must distinguish physical movement, communications/contact, evidence/reconciliation and infrastructure dependencies. Every rendered feature needs a reversible lineage to its exact source and owner-native certificate. Sensitive facility details remain purpose-limited.

Reference: Sol-Atlas #54, #59, #106, #115 and #895.

### Symtropy — domain physics and mathematical assurance

Owns fuel/resource accounting, transport-capacity and time-window validation, physical compatibility, conversion and generator models, network feasibility, deterministic replay, flow witnesses and appropriately typed infeasibility certificates.

Reference: Symtropy #7428, #7453, #7459, #7460 and #7483, plus transport #1611, #1616, #1639 and #3199.

### Power Fabric / SAFRB — beneficiary service contract

Owns service profile, beneficiary boundary, critical floor, service interval semantics, assurance scope and reporting. It consumes exact upstream receipts; it must not collapse bounds into unsupported point values or let a weighted score override a hard floor.

Reference: Symtropy #7338 and Power Fabric #6450.

## 8. Global conformance programme

Start with synthetic and public source snapshots. Do not claim worldwide readiness from one South African corridor.

### 8.1 Jurisdiction selection

Select at least 10 pilot jurisdiction profiles using a deliberate diversity matrix:
- common-law, civil-law and mixed legal traditions;
- mature and emerging energy/transport digital infrastructure;
- high and low connectivity;
- different regulator/utility structures;
- different privacy, data-localization and cross-border-transfer rules;
- different languages/scripts, calendars, UTC offsets and measurement conventions;
- large national operators and smaller municipal/cooperative actors.

Do not treat a test profile as a legal opinion. Every real jurisdiction overlay needs local expert review and source-pinned legal materials.

### 8.2 Language/time/unit matrix

At minimum:
- five languages/scripts including at least one right-to-left script and non-Latin scripts;
- multiple offsets, fractional-hour offsets, daylight-saving gaps/folds, and historical rule changes;
- SI and customary units with exact dimension checks;
- local decimal separators and localized human display while machine values remain canonical;
- currency identifiers and effective-date metadata for cost/settlement fields;
- non-ASCII identifiers and names without lossy normalization.

### 8.3 Operating and evidence matrix

Test:
- online, low-bandwidth, intermittently connected and fully offline periods;
- participant-owned APIs, manual/paper, CSV/barcode and signed event receipt ingestion;
- delayed synchronization, duplicate delivery, replay, conflict, supersession and revocation;
- open, restricted, purpose-limited and non-redistributable source data;
- partial asset/route coverage and unknown failure domains;
- measured, reported, estimated, modelled, synthetic and unverified source assertions;
- operator refusals and missing evidence;
- source/issuer/provider/independent verifier role separation.

### 8.4 Matched benchmark arms

A. Existing operator/planner workflow, no added SAFRB assurance.  
B. Same proposal bytes with SAFRB domain qualification.  
C. Same run with independent artifact/claim verification.  
D. Same unresolved case with Symthaea decision-aware evidence acquisition under a fixed cost/time budget.  
E. Federated participant-owned operation with delayed/offline reconciliation versus a central coordination baseline, under the same disclosure and service obligations.

Keep timeouts, abstentions, rejected proposals, unavailable evidence, operator refusals, unknowns and incomplete cases in the denominator. Do not compare a successful subset against a baseline that includes failures.

## 9. Prospective global targets

These are research targets, not achieved results, regulatory thresholds or certifications.

| Dimension | Prospective target |
|---|---|
| Semantic preservation | Zero adapter-induced authority, certainty, freshness, specificity or completeness promotion |
| Time/unit integrity | 100% exact preservation or explicitly declared, reversible/lossy transformation |
| Evidence lineage | 100% parent artifact and derivation reconciliation for completed claims |
| Rights and privacy | Zero disclosure outside declared purpose/recipient permissions |
| Offline consistency | Zero stale-cache or replay promotion into current availability/commitment |
| Service assurance | Zero false guaranteed-service results on the frozen independent hard-negative suite |
| Jurisdiction | 100% unsupported legal/recognition decisions remain Unknown/NotEvaluable |
| Positive controls | ≤1% false rejection on at least 300 independently constructed admissible controls |
| Global holdout | At least 90% quality retention on pre-registered held-out jurisdiction/profile families, with the same claim ceiling |
| Runtime | Report P50/P95/P99 verifier, adapter and reconciliation latency; target ≤10% of declared decision-loop budget where applicable |
| Accessibility | Manual/low-bandwidth path must preserve the same core claim semantics and disclose any reduced evidence strength |

For zero observed errors in N independent trials, report the approximate one-sided 95% upper bound 3/N as well as exact intervals. Correlated copies from the same topology, issuer, instrument or scenario family do not count as independent trials.

Global benchmark reporting must disaggregate by jurisdiction/profile, provider size, language, evidence maturity, connectivity, asset type, service type and adapter. A single aggregate score may hide poor performance in one locale or underserved cohort.

## 10. Phased deployment path

1. **Portable synthetic core:** frozen examples, schema/profile IDs, time/unit/claim semantics and independent reference outputs.
2. **South African bounded demonstrator:** Durban–Gauteng corridor, with source-qualified assets and claims only; no extrapolation to national availability.
3. **Second jurisdiction:** choose only after local operator participation, lawful source access, data rights and a local authority/review partner are established.
4. **Multi-industry extension:** electricity, water and communications profiles with domain-specific physics and service boundaries.
5. **Federated shadow operation:** read-only participant connectors; no OT writes or inventory/capacity mutations.
6. **HIL/commissioning and operational qualification:** only within applicable standards, authority and independently reviewed test plans.
7. **Periodic requalification:** standards, law, source schemas, equipment, model versions and evidence validity can change.

## 11. Non-goals and claim ceiling

This profile does not:
- establish universal legal title, regulatory compliance or cross-border legal recognition;
- create a central global inventory, capacity or infrastructure database;
- declare that a credential, signature, digest or provenance graph proves physical truth;
- authenticate a sensor without source, calibration and instrument qualification;
- prove physical pipeline, port, rail, generator, grid or water-system capability;
- replace SCADA, EMS, TMS, WMS, ERP, utility control or safety systems;
- grant dispatch, reservation, maintenance, withdrawal or access authority;
- claim standards conformity, field readiness or global SOTA superiority.

A successful conformance result means only that the declared adapter and claim semantics passed the specified test profile. Real service claims still require source-appropriate evidence, domain qualification, authority and independent verification.

## 12. Public references

- W3C Verifiable Credentials Data Model 2.0, Recommendation 15 May 2025: https://www.w3.org/TR/vc-data-model/
- W3C PROV-O: https://www.w3.org/TR/prov-o/
- GS1 EPCIS 2.0.1 archive/version page: https://ref.gs1.org/standards/epcis/archive
- GS1 EPCIS 2.0.1: https://ref.gs1.org/standards/epcis/2.0.1/
- GS1 CBV 2.0.0: https://ref.gs1.org/standards/cbv/2.0.0/
- UNECE UN/LOCODE Recommendation 16: https://unlocode.unece.org/recommendation16/
- UCUM 2.2: https://ucum.org/ucum
- OGC API - Features: https://www.ogc.org/standards/ogcapi-features/
- OGC API - Records: https://www.ogc.org/standards/ogcapi-records/
- OGC JSON-FG: https://www.ogc.org/standards/json-fg/
- IEC 61850 overview: https://iec61850.dvl.iec.ch/
- IEC 61970-301 CIM base: https://webstore.iec.ch/en/publication/62698
- IEC TS 62898-3-5:2026: https://webstore.iec.ch/en/publication/84175
- IANA Time Zone Database: https://www.iana.org/time-zones
- IETF BCP 47 / RFC 5646: https://datatracker.ietf.org/doc/rfc5646/

---
**Review status:** proposed. Standards editions, adapter scope, jurisdiction profiles and the benchmark selection matrix require independent review before implementation or deployment claims.
