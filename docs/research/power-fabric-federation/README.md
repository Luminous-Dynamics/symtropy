# Power Fabric: Four-Scope Federated Resilience Architecture v0.1

**Status:** research design / draft contract  
**Issue:** [POWER-FABRIC-FEDERATION-001 #6214](https://github.com/Luminous-Dynamics/symtropy/issues/6214)  
**Programme:** [POWER-FABRIC-BENCH-001 #5241](https://github.com/Luminous-Dynamics/symtropy/issues/5241)  
**Snapshot:** 2026-11-19

## 1. Executive thesis

Power Fabric should coordinate resilience across four scopes without turning those scopes into a rigid command hierarchy:

1. Household / individual site
2. Organization / community
3. Town / municipality
4. City / metropolitan system

The architecture is **federated, locally autonomous, evidence-qualified and authority-bounded**. Each scope can operate locally when upstream connectivity fails. Participants exchange service needs, capacity offers, reservations, observations and restoration commitments; they do not transfer ownership or physical-control authority merely by joining the federation.

Four scopes are a useful common operating model, not a universal legal taxonomy. Some jurisdictions combine town and city functions inside one municipality; some communities span several municipal boundaries. The model therefore needs jurisdiction profiles and a graph of real relationships rather than hard-coded assumptions about administrative levels.

## 2. Separate scope, actor, ownership and authority

A common modelling error would be to treat "household", "company", "community", "town" and "city" as five or four equivalent classes. They are not the same kind of thing.

- **Scope** answers: at what scale is coordination occurring?
- **Actor type** answers: who is participating? Examples include household, company, cooperative, community association, hospital, municipality, utility, regulator, emergency service or system operator.
- **Ownership** answers: who owns or controls an asset or resource?
- **Operational responsibility** answers: who operates and maintains it?
- **Authority** answers: who is legally, contractually or technically entitled to request, commit, dispatch, switch, curtail, disconnect or restore it?
- **Evidence** answers: what is known, how is it known, and how fresh is that knowledge?

Companies and communities share the second coordination scope, but must remain different actor and governance types. A company may make contractual capacity offers; a community may require representative decision-making, notice, participation, grievance and benefit-sharing rules. A company must not acquire community consent authority simply by being a large local employer or asset owner.

Town and city should be distinct configurable scopes only where the jurisdiction actually distinguishes them. The protocol must permit a municipal entity to cover one or both.

## 3. Four coordination scopes

### Scope 1 — household / individual site

Examples: households, farms, small premises, prosumer sites and small isolated facilities.

Potential contributions and needs:
- rooftop PV, batteries, thermal storage and backup generation;
- electricity demand flexibility and essential-load categories;
- water storage, pumping and treatment;
- local communications and backup power;
- food storage, charging and accessibility requirements;
- voluntary capacity offers with an explicit time window.

The default data exchange is a minimal offer or requirement, not raw household meter data, occupancy, appliance traces or personal schedules. Participation in non-emergency resource sharing is opt-in and revocable. An offer does not authorize a remote controller to operate a breaker, inverter or appliance.

### Scope 2 — organizations / communities

Examples: companies, cooperatives, farms, campuses, hospitals, schools, housing associations, neighbourhood groups, shelters, community centres and local service providers.

This scope is a federation of distinct actor types, not a single governance category. Each actor has a declared operator, ownership and authority model. Community decisions require a representation model and accessible dispute/withdrawal path; commercial actors participate through explicit contracts and service obligations.

Typical functions:
- pool local generation, storage, water and communications resources;
- coordinate demand response and backup priorities;
- make skilled crews, depots, vehicles and maintenance capacity discoverable;
- maintain local critical-service plans;
- support mutual aid and service restoration without surrendering local autonomy.

### Scope 3 — town / municipality

Examples: local government, municipal service departments, licensed distribution interfaces, public works, water and sanitation, depots, shelters, local roads and municipal emergency coordination.

This scope coordinates local public services and interfaces with the actual licensed utility/distributor and competent authorities. It does not assume that municipal service authority, asset ownership, distribution licensing and day-to-day operation are held by the same entity.

### Scope 4 — city / metropolitan system

Examples: metropolitan municipalities, inter-municipal service systems, hospitals, major transport corridors, bulk water, large distribution interfaces, telecommunications, emergency coordination and critical supply chains.

This scope resolves cross-town and cross-service dependencies, coordinates scarce specialist resources and provides a wider view of service restoration. It must not override local safety controls, private ownership or legally reserved authority.

### Regional and national peers

Transmission/system operators, neighbouring municipalities, regional authorities and national regulators connect through explicit federation interfaces. They are not assumed to be a mandatory fifth layer of this four-scope model. Their authority and operational role are represented explicitly in each jurisdiction profile.

## 4. Topology: federation graph, not a command tree

The four scopes should not be implemented as a single parent-child tree. Actual dependencies are a graph:

- a household may participate in a cooperative and receive service from a municipal distributor;
- a hospital may span a town-level boundary but rely on a regional water supplier;
- a company may own a battery whose output is offered to a community microgrid under contract;
- two municipalities may share a water plant, depot, feeder or emergency route;
- a city may depend on a nationally operated transmission system.

Represent the relationships as typed edges such as:
- serves / receives-service-from;
- owns / operates / maintains;
- offers / reserves / commits;
- depends-on / shares-failure-domain-with;
- governed-by / licensed-by / authorised-by;
- has-delegated-authority-from;
- participates-in / represents.

Every edge needs provenance, validity, scope and source. Unknown relationships must remain unknown. Missing edges are not proof of independence.

## 5. Core federation record

The initial message schema is at `data/federation-envelope-v0.1.schema.json`.

A message envelope carries:
- schema version and stable message identity;
- issuance and expiry timestamps;
- message purpose;
- sender actor and coordination scope;
- subject/resource/service identity;
- bounded payload;
- authority assertion and scope;
- evidence references and evidence class;
- consent or contractual basis where applicable;
- integrity metadata.

The envelope is a transport contract, not a command. Each receiver must validate:
1. schema and canonical payload;
2. authenticated sender and key status;
3. message identity, expiry, replay and idempotency;
4. jurisdiction and authority scope;
5. ownership/operator/delegation constraints;
6. evidence class, freshness and provenance;
7. local policy, interlocks and current resource state;
8. conflicts with existing reservations or commitments.

Unknown, stale, conflicting or unverifiable state fails closed for the affected mutation. It must not be converted into positive capacity.

### Evidence and lifecycle states

Keep these states distinct:
- **declared** — a participant claims a capability or requirement;
- **observed** — a measurement or accountable field observation was recorded;
- **inferred** — derived from an explicit model and named assumptions;
- **simulated** — generated by a simulation or synthetic fixture;
- **unknown** — evidence is absent, stale, conflicting or insufficient.

Likewise, keep resource lifecycle states separate:
- declared;
- offered;
- reserved;
- committed;
- observed available;
- locally dispatchable;
- unavailable;
- unknown;
- reconciled.

A resource is not dispatchable merely because it was offered or reserved. Dispatchability requires current local state, the relevant authority, valid constraints and the appropriate controller/engineering protections.

### Authority is not inherited by tier

A city-level actor may request or coordinate a service contribution, but it does not automatically gain authority to:
- control a household inverter or breaker;
- curtail a private company;
- switch or energise a utility network;
- release a land-access workfront;
- operate a water-treatment plant;
- direct a community association.

Authority must be explicit, purpose-bound, time-bounded, revocable where applicable and independently checked at the point of action. Emergency powers are jurisdiction-specific and must not be invented by the software.

## 6. Partition-safe operation

The architecture assumes that links can fail during storms, load-shedding, cyber incidents, telecommunications outages or municipal disruption.

Each site or scope therefore needs:
- a local state replica and bounded local policy;
- store-and-forward message handling;
- monotonic commitment versions;
- idempotent message and reservation identifiers;
- expiry and revocation;
- replay protection;
- conflict detection and reconciliation after reconnection;
- a declared degraded mode;
- an explicit rule for which operations are forbidden while authority or resource state is unknown.

A partition must not create two valid claims over one physical resource. Where exclusive reservation cannot be guaranteed across disconnected peers, the offer must be treated as provisional or partition-local rather than globally committed. The protocol must disclose that limitation instead of pretending that distributed consensus can make disconnected physical capacity simultaneously available.

Local protection and safety functions remain independent of the federation. The federation is a coordination and evidence layer, not a substitute for microgrid controllers, protection systems, utility SCADA/EMS, dispatch systems or qualified operators.

## 7. Essential-service model

Start with three typed service domains:
1. electricity;
2. water and sanitation;
3. telecommunications.

Then add transport, shelter, health, food, fuel/logistics and other dependencies through explicit typed interfaces.

Do not reduce all services to a single scalar "resilience score". A water pump depends on electricity, fuel or stored energy; a communications site may depend on backup batteries and cooling; a hospital has time-critical service floors. The model should express these dependency chains and their time-varying constraints.

For the first service oracle, use a chronological time series:
- delivered service versus required service;
- minimum service floor and criticality class;
- alternative supply and network bottlenecks;
- storage state-of-charge, fuel and consumable depletion;
- restoration milestones and service dependency state.

Report time to restore 50%, 90% and 95% of the declared critical service, unserved-service integral, duration below minimum service floors, p50/p95 and CVaR95 under declared scenarios. Keep the service oracle independent of the schedule generator.

### Fairness before weighted optimisation

A citywide weighted-MWh objective can conceal a neighbourhood or household experiencing prolonged deprivation. Use a constrained or lexicographic objective:

1. satisfy electrical/network safety and legal constraints;
2. preserve declared minimum essential-service floors;
3. minimise worst-served cohort and severe service deprivation;
4. minimise total unserved service;
5. optimise restoration time, cost and resource utilisation.

Publish the weight and priority policy. Do not disguise distributional trade-offs inside a single unexplained score. The programme should measure both total service restored and who remains underserved.

## 8. Security, privacy and governance threat model

Threats include:
- compromised participant or operator keys;
- forged capacity offers or false restoration observations;
- replayed or duplicated commitments;
- stale state advertised as current;
- conflicting reservations during partitions;
- malicious or mistaken authority escalation;
- coercive or non-consensual household participation;
- a dominant company claiming to represent a community;
- denial of service against local coordination;
- sensitive location or consumption data leakage;
- compromised firmware or false device identity;
- malicious supplier substitution or false asset compatibility.

Required controls:
- key rotation and revocation with offline-tolerant verification policy;
- narrowly scoped authority and short-lived delegation;
- signed event/receipt chains and idempotent processing;
- explicit source freshness and uncertainty;
- data minimisation and aggregation;
- separate actor identities and representation records;
- grievance, withdrawal and correction paths;
- independent monitoring of fairness and access;
- local safety interlocks outside the federation;
- fail-closed handling of unknown authority and physical state.

No design can eliminate the need for accountable human institutions. The protocol should make responsibilities inspectable rather than pretend to replace them.

## 9. Benchmark programme

### A. Microgrid function and conformance

Use applicable IEEE 2030.7 controller-function specifications and IEEE 2030.8 controller-test procedures as a baseline. Pin the exact edition and applicable national profile. The 2030.7-2017 and 2030.8-2018 standards are existing baselines; P2030.7 and P2030.8 revision projects are separate and must not be represented as already-published replacements.

### B. Interoperability

Use NIST-style interoperability profiles: choose a narrow, agreed subset of a standard, define supported semantics and create conformance fixtures. Use OpenFMB/CIM/IEC 61850 as existing interoperability ecosystems, not as novelty claims.

Measure:
- asset and relationship identity preservation;
- service, unit and time semantics;
- explicit unsupported/unknown fields;
- authority and ownership separation;
- round-trip loss and canonical receipt integrity;
- stale-data and correction handling.

Target: 100% source-inventory accounting or an explicit incomplete result; zero silent loss of in-scope semantics.

### C. Partition and failure testing

Inject lost, delayed, duplicated, reordered, replayed and conflicting messages across every scope boundary. Measure:
- duplicate or conflicting reservations;
- unauthorised state transitions;
- time to detect stale state;
- time to reconcile after reconnection;
- local service preserved during partition;
- schedule churn caused by reconciliation.

Target: zero duplicate exclusive commitments and zero authority escalation in the frozen adversarial suite. Do not claim zero failures beyond the tested model.

### D. Dependency and capacity integrity

Use independent small-graph oracles with shared upstream feeder, substation, road corridor, port, fuel source, telecom backhaul, depot, battery and maintenance crew. Include common-cause storms and cyber/communications loss. Unknown inventory must never imply independence.

Target: 100% agreement with independently enumerated small cases; zero double-counted capacity; explicit unknowns for incomplete inventories.

### E. Essential-service restoration and equity

Measure:
- critical service restored at 50/90/95%;
- unserved-service integral by service and cohort;
- longest duration below declared minimum service;
- worst-served neighbourhood/site;
- p50/p95/CVaR95 restoration and deprivation;
- cross-service cascade and bottleneck migration;
- effect of each new commitment on all other scopes.

Target: reduce p95 service deprivation under matched scenarios without worsening the pre-registered minimum-service or equity constraints. Do not optimise total MWh by sacrificing a small group.

### F. Governance and human comprehension

Run preregistered, non-hazardous tasks with residents, community representatives, businesses, municipal operators and utility operators. Test whether participants can correctly identify ownership, operator, authority, consent scope, resource state, freshness and withdrawal/appeal process. Report each population separately and include accessibility and language coverage.

## 10. External baseline and novelty boundary

The field already has substantial capability. Power Fabric must integrate with it.

- **IEEE 2030.7-2017 / 2030.8-2018:** existing microgrid controller specification and controller-testing baseline. Revision projects P2030.7 and P2030.8 are active work, not released revisions.
- **NIST Smart Grid Testing and Certification:** describes interoperability profiles as agreed, testable subsets of standards and links those profiles to conformance testing and procurement.
- **OpenFMB:** already offers a peer-to-peer semantic reference architecture using a common data model derived from CIM and IEC 61850.
- **U.S. DOE Community Microgrid Assistance Partnership:** demonstrates a live public programme for technical assistance, planning and delivery of remote-community microgrids. It is a programme comparator, not a measured algorithmic benchmark.
- **ENTSO-E 2026 Transmission Grid Investment and Supply Chain Assessment:** reports at least 36 months typical procurement lead time for 245–420 kV power transformers in the survey's European TSO scope, alongside material and manufacturer-capacity constraints.

The candidate differentiator is the independently verifiable lifecycle across heterogeneous actors and services: requirement → offer → authority → reservation → observation → execution → restoration → reconciliation. Do not claim to have invented microgrids, local islanding, peer-to-peer communications, digital twins, CIM, IEC 61850 or grid interoperability.

## 11. South African jurisdiction profile

Do not hard-code a generic "town controls electricity" rule. Model NERSA, Eskom or another licensed distributor, the relevant municipality, system operator, owner, installer, community participant and emergency authority as distinct roles.

NERSA's 2 February 2026 clarification says that a grid-connected embedded-generation facility below 100 kW is registered with the relevant distributor; a grid-connected facility of 100 kW or more is registered directly with NERSA; and a facility without a grid point of connection is treated differently. This is one dated jurisdiction-specific rule, not a global default. Legal applicability, amendments, municipal conditions, technical standards and connection agreements must be checked at deployment time.

Municipal boundaries and service roles can differ. The protocol should represent the actual licensed distributor, service authority, asset operator and responsible emergency authority independently. This is more accurate than assuming a single municipal hierarchy.

## 12. Phased implementation

**Phase 0 — paper and schema contract**
- freeze actor/scope/authority ontology;
- publish envelope schema and examples;
- document threat model, evidence classes and jurisdiction-profile format;
- independent review before implementation.

**Phase 1 — synthetic federation replay**
- create household/site, company, cooperative, community, municipality and metro actors;
- add electricity/water/telecom dependency graph;
- simulate partition, storm, fuel shock and shared crew/equipment scarcity;
- build independent capacity and service-loss oracles.

**Phase 2 — standards adapter and local-first prototype**
- map one documented microgrid controller profile;
- add one OpenFMB/CIM/IEC 61850 interoperability path;
- maintain local-only operation with no external dispatch;
- test signatures, replay, expiry, revocation and reconciliation.

**Phase 3 — shadow operation**
- read-only, consented, de-energised or non-actuating data integration;
- compare observations and model predictions;
- no command path to real electrical, water or telecom equipment;
- independent privacy, cyber and professional engineering review.

**Phase 4 — bounded field pilot**
- only under applicable approvals, operator supervision, engineering safety case, cybersecurity assessment and participant consent;
- introduce no control function merely because the coordination layer passed its own tests.

## 13. Current qualification status

This document and its initial schema are research artifacts. They do not establish an implemented federation, tested microgrid controller, secure key infrastructure, legal compliance, physical availability, field restoration performance or SOTA result. All synthetic probabilities, capacities and service outcomes must be labelled illustrative. CI or physical qualification has not been claimed by this documentation change.

## Sources

- IEEE 2030.7-2017: https://standards.ieee.org/ieee/2030.7/5941/
- IEEE 2030.8-2018: https://standards.ieee.org/ieee/802.11/6169/
- Active P2030.7 project: https://standards.ieee.org/ieee/2030.7/12022/
- Active P2030.8 project: https://standards.ieee.org/ieee/2030.8/12543/
- NIST Smart Grid Testing and Certification: https://www.nist.gov/programs-projects/smart-grid-testing-and-certification
- OpenFMB introduction: https://openfmb.openenergysolutions.com/docs/openfmb/intro/
- U.S. DOE Community Microgrid Assistance Partnership: https://www.energy.gov/oe/community-microgrid-assistance-partnership
- ENTSO-E Transmission Grid Investment and Supply Chain Assessment Report 2026: https://eepublicdownloads.blob.core.windows.net/public-cdn-container/clean-documents/Publications/Position%20papers%20and%20reports/2026/entso-e_report_investment_supply_chain_assessment_2026_261030.pdf
- NERSA SSEG clarification, 2 February 2026: https://www.nersa.org.za/


## 12. Conditional v0.1 contract gates

The envelope schema now adds conditional structural rules:
- household/individual-site capacity offers require explicit opt-in consent, revocable consent, a non-zero quantity and a bounded time window;
- community/neighbourhood group capacity or commitment messages require a representation reference and a community-mandate participation basis;
- observed-available or locally-dispatchable claims require an observed evidence class and a current observation record;
- observer or observation-only authority cannot be combined with dispatch, curtail, switch, disconnect or restore actions.

Cross-record and temporal checks still require application-level validation: message expiry after issue time, actor/authority scope matching, delegation verification, jurisdiction applicability, key revocation, signature verification, duplicate/replay processing and conflict reconciliation. JSON Schema is not a cryptographic or legal authority verifier.
