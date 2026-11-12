# Subterranean Utility Fabric — Programme Specification v0.1

**Status:** Research specification; not an engineering design, procurement package, safety case, or field qualification.  
**Created:** 2026-11-12  
**Programme issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4910  
**Primary repo:** https://github.com/Luminous-Dynamics/symtropy

## 1. Thesis

Build a **robot-first, modular, selectively underground utility fabric** that makes essential services harder to disrupt, cheaper to inspect and repair, and easier to extend.

The product is not "a tunnel everywhere." It is a portfolio of infrastructure forms selected against the same service requirement and evidence set:

- protected overhead routes where overhead remains cheaper and safe;
- single-utility buried conduits where that is the best fit;
- shallow, partitioned multi-duct corridors where shared installation and future access create value;
- accessible service nodes and small galleries where density, criticality and repeated interventions justify them;
- verified reuse of suitable existing corridors and brownfield assets;
- no-build, targeted hardening or conventional open-cut where alternatives are not eligible.

**Sympoietic** describes the social-technical operating model: people, utility owners, local enterprises, robots and accountable institutions co-design, build and maintain the network. It does not grant autonomous software authority over public infrastructure.

### Primary hypothesis

For selected high-risk, high-density and high-repeat-work corridors, the combined value of lower theft exposure, fewer repeat excavations, faster fault localization, robot-assisted maintenance and shared capacity can offset additional initial construction cost.

This is a testable hypothesis. It is not yet demonstrated for South African conditions.

## 2. Design principles and paradigm shifts

| Conventional framing | Research direction |
|---|---|
| Choose a tunnel or cable and optimize installation | Compare whole corridor architectures against delivered service |
| Underground everything | Select the minimum-cost, risk-adjusted intervention by route and asset |
| Build for human access only | Design access, geometry, nodes and interfaces for both human and robotic maintenance |
| Permanently entomb services | Prefer inspectable, isolatable and replaceable modules where physically qualified |
| Prevent theft with cameras and guards alone | Reduce recoverable exposure, detect tampering, isolate faults, improve response and constrain resale channels |
| Measure metres installed | Measure independently accepted service, uptime, restoration and lifecycle cost |
| AI marks a defect | Sensor quality, localization, confidence, human adjudication and repair evidence form one chain |
| One contractor owns a proprietary digital record | Portable, versioned, interoperable evidence and accountable authority |
| New excavation is the default | Survey and qualify reuse before new underground space |

The first research question is not "what robot should we build?" It is "what corridor geometry, utility separation, access-node design and service-module interfaces make future inspection and repair economically repeatable?"

## 3. Physical architecture

### 3.1 Three infrastructure scales

**Scale A — micro-utility ducts.** Small conduits for fibre, communications, control and other services with a qualified installation envelope. Robot access is primarily through draw-through tools, inspection heads and accessible chambers; do not assume a crawler can traverse every small duct.

**Scale B — robotic service corridors.** Larger partitioned duct banks or non-man-entry service galleries with standardized access nodes, pull paths, isolation points, drainage and sensor interfaces. Small tracked or wheeled platforms may inspect only the geometries for which reach, bend, traction, retrieval and environmental envelopes are demonstrated.

**Scale C — accessible utility galleries.** Human-accessible or hybrid service galleries only for high-density, high-criticality or high-frequency intervention locations where lifecycle economics and fire, ventilation, egress, drainage, structural and access requirements justify them.

These scales are alternatives and complements, not a single mandatory design.

### 3.2 Utility separation

Do not create an unpartitioned common tunnel for every service. Each service has a different hazard, access and operating envelope.

At minimum, model separate compartments / ducts and interfaces for:
- power cables and associated protection / earthing;
- fibre-optic and communications;
- potable water;
- wastewater;
- stormwater;
- instrumentation and low-voltage control.

Any gas, hydrogen, high-voltage or other hazardous service requires a separate competent engineering safety case before inclusion. Co-location must be assessed for fire propagation, electrical faults, flooding, contamination, corrosion, access, structural effects and common-cause failure.

### 3.3 Robot-first maintainability features

Evaluate, rather than assume, the value of:
- consistent access-node spacing and opening geometry;
- known route topology and localization fiducials;
- cable draw paths, bend-radius control and pulling-tension limits;
- removable covers and tamper-evident fasteners;
- replaceable duct / service cassettes;
- inspection ports, drainage and water-ingress sensing;
- safe isolation and prove-dead interfaces;
- standardized docking, charging and retrieval;
- manual fallback and human retrieval provisions;
- modular components that can be fabricated and repaired locally.

A modular cassette is not accepted until insertion, extraction, bend radius, tensile load, jointing, thermal rating, water ingress, fire performance, fatigue and post-installation tests are defined for the exact configuration.

## 4. Reference architectures

Compare these options for the same required service, route, loading, availability, design life and economic boundary:

| ID | Architecture | Role in comparison |
|---|---|---|
| A0 | Observed local practice | Real baseline; capture actual maintenance and security practice |
| A1 | Protected overhead / exposed route | Tests whether targeted hardening is sufficient |
| A2 | Conventional single-utility underground | Standard buried installation and conventional maintenance |
| A3 | Shared, partitioned duct bank | Shared installation with conventional access and maintenance |
| A4 | Robot-first duct bank + modular service nodes | Candidate robotic inspection / replaceable-module architecture |
| A5 | Accessible mini-gallery / multi-utility tunnel | Candidate for dense corridors and repeated works |
| A6 | Existing corridor / brownfield reuse | Reuse after evidence-based suitability screening |
| A7 | No-build / repair-only / targeted hardening | Must remain a valid candidate |

A candidate that fails a hard constraint is ineligible. A candidate with unresolved required evidence is indeterminate, not eligible. Economic preference is computed only after eligibility.

## 5. Luminous system composition

The four systems retain distinct responsibilities:

### Symtropy — scenario execution and independent comparison
- Compose the existing synthetic road–drainage–utility corridor verifier (#1732).
- Reuse trenchless method eligibility (#3765), robot capability matching (#1834), fleet recovery (#1843), sewer inspection (#4905), power resilience (#3418), water maintenance lifecycle (#3372) and brownfield reuse (#1951).
- Evaluate supported robot dynamics, resource budgets, failure/recovery scenarios and deterministic model comparisons.
- Do not claim the generic rigid-body engine is a geotechnical, hydraulic, cable-rating, protection-coordination or structural solver. Integrate named domain solvers through explicit adapters and preserve their limits.

### Sol-Atlas — spatial evidence and asset context
- Own spatial projection, source identity, coordinate reference, time, coverage and uncertainty presentation.
- Preserve the difference between legacy drawing, geophysical indication, surveyed feature, exposed/verified location and as-built acceptance.
- Carry geometry revisions and conflict relationships; do not create a second authoritative asset register.
- Use ASCE 38-22 / 75-22 as a useful interoperability reference, while separately mapping South African legal, utility-owner and project requirements.

### Symthaea — bounded reasoning and investigation planning
- Propose candidate layouts, investigation priorities, anomaly triage and inspection missions.
- Use uncertainty and decision value to decide what to measure next.
- Never convert a model score or recommendation into construction, dispatch, isolation, energization or repair authority.
- Abstain or request a human / domain specialist when observations are stale, out-of-envelope or conflicting.

### Mycelix — authority, accountability and provenance
- Reuse existing construction procurement, lot, inspection and approval provenance.
- Record consent, work authorization, approved roles, material custody, contractor evidence, inspection acceptance and dispute / correction paths.
- Keep asset ownership and work-order authority with the responsible utility / institution.
- A ledger proves an attributable record exists; it does not prove that a physical cable, trench, joint or repair is safe or compliant.

### Existing engineering owners
Geotechnical, hydraulic, electrical, cable thermal, protection, structural, fire, environmental and transport domain models remain authoritative for their own declared outputs. No universal solver is introduced here.

## 6. Evidence and interchange contract

This programme should refer to owner-issued identities and receipts rather than mint duplicate registries.

Every claim-bearing feature / observation / design candidate must be able to resolve:

- stable subject ID and exact revision;
- feature / service type and geometry digest;
- coordinate reference system, datum, axis convention, units and epoch;
- valid time, observation time, publication time and retrieval time where applicable;
- evidence class: measured, surveyed/exposed, operator record, compiled secondary, model output, scenario assumption, vendor assertion or unknown;
- source authority, exact source/version and byte digest when bytes are acquired;
- coverage, resolution, uncertainty, calibration and known limitations;
- provenance / transformation lineage and licence;
- status: accepted, rejected, indeterminate, unsupported, stale, unavailable or conflicting;
- applicable owner and independent verifier reference.

**Unknown is not clear. Not observed is not defect-free. A design is not an as-built. A purchase order is not an installation. An installation claim is not an accepted service.**

### Candidate cross-system references

| Information | Owner / reference |
|---|---|
| Method eligibility | Symtropy #3765 |
| Corridor synthetic truth and finite constraints | Symtropy #1732 |
| Robot capability envelope | Symtropy #1834 |
| Disturbance / recovery evaluation | Symtropy #1843 |
| Robotic sewer observation quality | Symtropy #4905 |
| Spatial evidence | Sol-Atlas #450 |
| Asset / approval / lot provenance | Mycelix #5149 |
| Civil design and investigation value | Symthaea #7697 and #7727 |

No shared schema should be added until the consuming owners compare their current contracts and name the canonical owner for each field.

## 7. South African threat and policy model

### 7.1 Evidence snapshot

- Eskom's FY2025 Integrated Report reports 1,585 theft incidents involving conductors, cabling and network-related equipment, with R77m in direct losses; FY2024 was 2,380 incidents and R120m. This is the most specific retrieved cable/conductor data point used by this specification.
- Eskom's FY2026 results report broader physical-security incidents down 13% and associated losses down 18% to R191m. That broader category includes cable theft, coal, diesel/fuel, vandalism and other crime; it must not be presented as cable-theft-only.
- South Africa's 2025 ITAC export-control guidance describes a price-preference system for scrap exports. A 2024 Second-Hand Goods Act regulation notice invited comments; its proposed measures must not be represented as enacted merely because the consultation was published.

References:
- https://www.eskom.co.za/wp-content/uploads/2025/09/Eskom-integrated-report-2025.pdf
- https://www.eskom.co.za/eskom-reports-second-consecutive-profitable-year-as-turnaround-strategy-execution-strengthens-operational-recovery-energy-security-and-financial-sustainability/
- https://www.gov.za/sites/default/files/gcis_document/202510/53605gen3579.pdf
- https://www.gov.za/sites/default/files/gcis_document/202407/50929gon5037.pdf

### 7.2 Threat model

Assess a range of threats without assuming that a single intervention eliminates theft:
- opportunistic access to exposed conductor / cable;
- forced or unauthorized access to chambers and service nodes;
- tampering with sensors, communications or alarms;
- insider or contractor collusion;
- repeated targeting after repair;
- theft that shifts from copper to aluminium, batteries, transformers or other materials;
- vandalism / sabotage whose purpose is service disruption rather than resale;
- common-cause damage from flooding, fire, excavation or malicious access.

Theft deterrence is a layered portfolio: material selection, physical enclosure, access control, tamper detection, asset segmentation, isolation, rapid response, material provenance and lawful recycling. Quantify the residual risk after each layer.

### 7.3 Material selection rule

Do not select aluminium or another material simply because it has lower scrap value. Compare delivered electrical service and whole-life properties: current rating, conductor sizing, voltage drop, thermal behaviour, jointing, corrosion, installation quality, mass, cost, embodied impacts, repairability, service life and theft attractiveness. All designs require the relevant utility standards, tests, approvals and professional sign-off.

## 8. State-of-the-art comparison

| Area | Public reference / current practice | Luminous comparison strategy |
|---|---|---|
| Subsurface utility engineering | ASCE 38-22 describes investigation / documentation quality; ASCE 75-22 addresses recording and exchange of utility data for digital-twin use | Preserve quality level, uncertainty, provenance and exact coordinate frame through planning, construction and as-built records |
| Sewer inspection coding | NASSCO PACP / LACP / MACP and trained/certified inspectors remain an established practice | Do not replace human coding prematurely; compare automated evidence against blinded, independently adjudicated labels |
| Sewer robotic defect detection | One 2025 RGB-D sewer-robot paper reports 84.8% mAP@50, 10.2 mm overall MAE for defect dimensions, and 11 fps in a 0.8 m pipe demonstration | Reproduce the same task first; then improve held-out asset / hardware performance, critical-defect recall, metrology and uncertainty calibration |
| Robot localization | Hybrid metric-topological localization research targets drift and topology ambiguity in signal-denied pipe networks | Report P50/P95/max position and chainage errors, branch confusion, drift per metre and retrieval, not only successful video capture |
| Trenchless construction | HDD, auger/slurry boring, pipe jacking/microtunnelling, impact moling/ramming and pipe bursting are established method families | Outperform through evidence-qualified method selection, measured ground/utility uncertainty, robotics-ready interfaces and accepted-work economics—not by claiming trenchless is always best |
| Multi-utility tunnels | Published economic analysis finds viability is sensitive to utility density and repeat street works; short-term capex remains a barrier | Build a local break-even model from South African route quantities, tender quotes, repeat-cut rates and service interruption costs |
| Anti-theft operations | Eskom reports intrusion detection, lower-value replacements and intelligence-led enforcement alongside incident reductions | Add physical architecture + exposure-normalized measurements + repair-time and service-outcome evidence; no camera-only or material-only claim |

Primary references:
- ASCE / FHWA SUE: https://www.fhwa.dot.gov/innovation/everydaycounts/edc_8/subsurface_utility_engineering.cfm
- ASCE UESI publications: https://www.asce.org/communities/institutes-and-technical-groups/utility-engineering-and-surveying-institute/publications
- NASSCO inspection quality: https://www.nassco.org/resource/cctv-inspection-quality-in-sewer-cleaning
- RGB-D sewer robot study: https://doi.org/10.1016/j.measurement.2025.119694
- 2026 review of 183 sewer-robotics studies: https://doi.org/10.1016/j.aei.2026.104605
- FHWA trenchless methods and limitations: https://www.fhwa.dot.gov/utilities/utilitycuts/man04.cfm
- Multi-utility tunnel economic study: https://doi.org/10.1016/j.tust.2012.02.001

**Comparability rule:** a paper's mAP, a vendor's metres/day, an HDD estimate and a South African tender price are not directly comparable. Freeze boundary, workload, geology, service output, equipment revision, price year and data rights before ranking.

## 9. Benchmark suite

No single readiness score. Publish a vector with denominators, uncertainty and per-case dispositions.

### B1 — Spatial evidence / conflict discovery
- Horizontal / vertical geometry error against exposed or independently surveyed truth.
- Percent of route with known quality level and percent explicitly unknown.
- Critical crossing detection and false-clear rate.
- Datum / coordinate / unit mismatch detection.
- Uncertainty interval coverage and stale-data detection.
- Time and cost per accepted metre of mapped corridor.

### B2 — Construction / installation
- Accepted installed metres per shift, not merely bored metres.
- Total cost per accepted metre including survey, mobilization, access, traffic, spoil, reinstatement and testing.
- Utility strikes, settlement, rework, service interruptions and environmental impacts.
- Material and energy per accepted service-unit.
- As-built completeness and discrepancy from design.

### B3 — Robotic inspection / intervention
- Mission coverage: observed, unobserved, unusable, not reached and unsupported lengths.
- Position / chainage P50, P95 and maximum error.
- Per-class precision, recall, mAP / IoU where appropriate and critical-defect false-negative rate.
- Defect-size MAE, bias and confidence-interval coverage against independent metrology.
- Calibration (Brier / ECE), abstention and out-of-domain detection.
- Mission completion, recovery, energy per metre, communications interruptions and human review time.
- First-pass repair acceptance, repeat fault, rework, and eligible intervention completed without road excavation.

### B4 — Theft / tamper resilience
- Theft and attempted tamper per km-year and per 1,000 asset-years.
- Direct asset loss, restoration cost and consequential service-hours lost reported separately.
- Detection lead time, false alarm / km-month, tamper localization error and response time.
- Sectional isolation and service restoration time.
- Recurrence after hardening / replacement.
- Sensor / communications availability and behavior during common-cause failure.
- Material salvage value and lawful material recovery traceability.

### B5 — Lifecycle economics / economic development
- Discounted 10 / 20 / 30-year NPV and equivalent annual cost.
- Cost per accepted service-unit-year and per avoided outage-hour.
- Break-even corridor occupancy, utility density and repeat excavation frequency.
- P50 / P90 capex and lifecycle cost; sensitivity to discount rate, theft, outages, repair, demand growth and asset life.
- Local labour hours, wages, supplier spend, SME participation, training completion and retained skills.
- Provider concentration, interoperability, switching cost and repair independence.

### B6 — Assurance / interoperability
- Zero false acceptance of seeded hard constraints in the frozen synthetic corpus.
- Zero unknown-to-clear promotions.
- Zero unapproved state transitions and stale-authority reuse.
- 100% traceability of claim-bearing outputs to input revision and owner receipt.
- Independent verifier agreement with exact oracle on finite test cases.
- Zero silent coverage gaps; explicit Accepted, Rejected, Indeterminate or Unsupported for every candidate / claim.

## 10. Proposed targets and qualification ladder

Targets below are research goals, not current results. Freeze final values before each experiment.

### Gate 0 — offline semantics
- 100% of finite synthetic cases produce deterministic, explainable dispositions.
- 100% detection of seeded hard-constraint violations in the finite oracle corpus; zero false acceptance.
- 100% of critical corridor segments carry an explicit evidence / unknown state.
- No duplicated asset, authority, work-order or physics solver ownership.

### Gate 1 — robot-duct rig
- 100% of route length receives an explicit observation disposition; no silent gaps.
- At least 99 successful safe returns in 100 missions inside the declared laboratory envelope; separately report every fault, manual recovery and unsafe event.
- P95 chainage error target initially 0.5 m or tighter on the declared rig; revise only before the run and justify against maintenance access needs.
- Robot must safely stop and request help on localization loss, retrieval risk, low energy, water ingress, communication loss and unexpected obstruction.
- For the matched RGB-D sewer benchmark, first reproduce the published 84.8% mAP@50 / 10.2 mm MAE / 11 fps result on the same task and setup. Stretch goal: at least 90% mAP@50 and no more than 5 mm MAE without lowering critical-defect recall or hiding abstentions. This stretch goal is only meaningful on the matched benchmark and is not a cable-duct result.

### Gate 2 — controlled field pilot
- 100% of pilot assets linked to approved revision, material lots, installation and acceptance evidence.
- Every counted repair has a separate post-repair acceptance observation.
- Measure detection sensitivity, false alarms, outage duration, restoration and recurrence per asset exposure; set numerical anti-theft effect-size targets only after a valid pre-pilot baseline / matched-control design is agreed.
- Compare total lifecycle cost against the best eligible local baseline; no universal 30-year savings claim.
- Publish P50/P90 results and distributional outcomes, not only the best-performing route.

A robot return rate, defect detector score or digital-twin validation is not a safety certification.

## 11. Initial synthetic campaign

Create three abstract, synthetic site archetypes. Do not invent real coordinates, South African geotechnical properties, utility routes or tender prices.

1. **High-risk urban feeder:** mixed utilities, critical service, high third-party works, constrained access and a defined but uncertain existing-utility survey.
2. **Industrial precinct:** concentrated multi-tenant demand, candidate shared corridor, repeat maintenance and a potential service-capacity lease.
3. **Settlement expansion edge:** staged connection growth, affordability constraints, incomplete legacy records and need for incremental installation.

For each, compare A0–A7 and inject:
- unknown or conflicting utility crossing;
- stale / incorrect as-built and coordinate-frame mismatch;
- unknown rock / groundwater / contamination condition;
- robot cannot turn, traverse or be retrieved;
- power cable overload / excessive temperature;
- flooded, blocked or tampered access node;
- communications and sensor failure;
- malicious access and repeated targeting;
- material substitution with incorrect jointing or installation;
- missing work authorization or expired contractor qualification;
- common-duct fire / water ingress / adjacent-service fault;
- utility / corridor operator insolvency and loss of maintenance provider;
- repair marked complete without independent inspection.

Candidate-generation code and independent reference oracle must be separate. Use finite known-answer cases where possible. Preserve no-build and conventional methods as valid outcomes.

## 12. First implementation slices

1. **Specification and interface map** — this document and issue #4910.
2. **Synthetic corridor composition** — add abstract utility-fabric archetypes to #1732, reusing its independent verifier and unit/frame semantics.
3. **Architecture eligibility** — reference #3765 for method screening and #1834 for robot capability. No new general solver.
4. **Threat / cost evaluator** — a small deterministic evaluator with explicit input factors and uncertainty intervals; economic ranking occurs only after physical and legal eligibility.
5. **Spatial and provenance binding** — consume Sol-Atlas and Mycelix owner receipts; do not store a second asset registry.
6. **Robot prototype** — tabletop replaceable-duct rig before any live electrical / public-space intervention.
7. **Local cost and pilot study** — collect South African quotations, route history, theft/outage exposure and utility-owner requirements before declaring a business case.

## 13. Required South African data before a field business case

- utility-owner asset class, route length, installation date/material and service criticality;
- theft/tamper incident geolocation at a safely generalized level, incident type, attempted/completed status and direct loss;
- restoration work order, material and labour, response and outage duration;
- actual protection / guard / sensor operating expenditure and false alarm burden;
- routine inspection / maintenance cycles and repeat surface-cut history;
- local geotechnical survey, groundwater, contamination and utility-conflict evidence;
- local tender / contractor quotations split by method, route, diameter, ground and restoration;
- wayleave, road authority, municipal, environmental and utility approvals;
- demand / tenancy / service-capacity data and a credible revenue / affordability model.

Data-sharing must be authorized, privacy-preserving and purpose-limited. Sensitive public-asset coordinates and vulnerabilities should not be published in general-purpose datasets.

## 14. Claim ceiling

This programme currently establishes a research direction and benchmark contract only. It does not establish that an underground corridor is safe, permitted, technically feasible, commercially viable, theft-proof or better than overhead infrastructure at any real site. No simulation result authorizes construction, energization, switching, excavation, valve operation, procurement or robot dispatch. Real engineering requires the relevant South African codes, standards, utility-owner requirements, competent specialist solvers, field surveys and independent professional review.

The success criterion is not a spectacular robot demonstration. It is a corridor that delivers a required service at lower risk-adjusted whole-life cost, with independently accepted construction, safe and recoverable maintenance, and measurable local economic benefit.


## Cable-duct robotics is a separate benchmark

Sewer-crawler performance is not automatically transferable to utility cable ducts. The programme now defines a dedicated, non-energized duct inspection and draw-tool qualification in [Symtropy #4913](https://github.com/Luminous-Dynamics/symtropy/issues/4913) and [CABLE_DUCT_ROBOT_BENCHMARK_V0_1.md](CABLE_DUCT_ROBOT_BENCHMARK_V0_1.md). It separates inspection, draw-rope insertion / retrieval and inert dummy-cable pull-through; qualifies exact duct profiles, bend and coupling geometry, tether hazards, localization, observation coverage, force limits, damage and safe recovery. No live cable is used.

## 15. Power-robot standards and the actual robotics baseline

The programme should map terminology and performance categories to the IEC 63439 power-robot family rather than inventing a parallel classification. IEC 63439-1-1:2025 defines electric-power-robot terminology. IEC TR 63439-1-2:2026 is a state-of-the-art and standardization roadmap that covers application classification, mobility, sensing, navigation, communications, testing, performance benchmarks and reliability / environmental stress evaluation.

A land cable-tunnel inspection robot general-requirements work item (IEC 63439-3-31) is in development. It is not a published standard, and its tunnel-inspection scope must not be assumed to qualify small microduct crawlers, draw-rope tools or cable-blowing systems. The programme should track its published revisions and distinguish cable tunnels / walkable galleries from small cable ducts.

Sources:
- IEC 63439-1-1:2025: https://webstore.iec.ch/en/publication/72749
- IEC TR 63439-1-2:2026: https://webstore.iec.ch/en/publication/109098
- IEC 63439-3-31 work item status: https://standardsdevelopment.bsigroup.com/projects/9025-12660 and https://genorma.com/en/standards/iec-63439-3-31-ed1

The current product / research landscape is heterogeneous:
- commercial sewer crawlers demonstrate autonomous travel / return and standardized sewer reporting on particular pipe sizes; they are not cable-duct robots;
- cable-trench inspection research already combines LiDAR / IMU navigation, thermal / visual sensing and environmental measurements, but its test environment is a trench / tunnel, not necessarily a small duct;
- fibre microduct deployment already uses commercial air-assisted cable blowing / jetting equipment, with cable- and machine-specific force, pressure, airflow and slip-test limits;
- the 2026 sewer-robotics review identifies multi-sensor fusion, signal-denied localization, real-time processing, interoperability and communications as active research bottlenecks.

The defensible Luminous advantage is not claiming to invent crawlers, cable jetting, thermal cameras or utility tunnels. It is to qualify a joined-up, interoperable lifecycle: route evidence -> method / tool eligibility -> inspection and uncertainty -> maintenance / retrieval -> accepted as-built -> transparent whole-life and theft / outage outcome. The same exact configuration and task must be compared against the best conventional or commercial method.
