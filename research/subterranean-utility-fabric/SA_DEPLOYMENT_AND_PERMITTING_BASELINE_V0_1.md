# South African Deployment and Permitting Baseline — SUF v0.1

**Status:** Desk-research baseline for programme design. Not legal advice, a standards compliance matrix, a wayleave approval, or an engineering design.  
**Snapshot date:** 2026-11-12  
**Programme:** https://github.com/Luminous-Dynamics/symtropy/issues/4910

## 1. Why this file exists

A subterranean utility product can be mechanically excellent and still fail commercially because it does not fit the local wayleave, service-owner consultation, road-reserve, construction, testing and handover process.

South African deployment needs a **jurisdiction-specific permitting profile** attached to each corridor. A national design template cannot silently override:
- the relevant road authority;
- municipal wayleave and road-reserve by-laws;
- each utility owner's technical conditions;
- environmental, water, land and servitude requirements;
- applicable SANS / NRS / IEC requirements and project-specific specifications;
- the responsible professional engineer and construction safety process.

The Luminous system may make the process more traceable and less wasteful. It cannot grant permission or replace the authority's decisions.

## 2. National road-authority baseline: COTO

The Department of Transport's COTO Standard Specifications for Road and Bridge Works for South African Road Authorities were approved by COTO on 18 August 2020 as a Draft Standard (DS). The department's page says the October 2020 versions replace the August 2020 versions, and that use in procurement documents advertised from 1 March 2021 was mandatory. It also states that the DS has full legal standing. SANRAL's download page lists the October 2020 chapter set and showed a last-updated date of 12 November 2026 when checked.

Sources:
- Department of Transport: https://www.transport.gov.za/?page_id=1155
- SANRAL COTO downloads: https://www.nra.co.za/sanral-downloads/detail/coto-standard-specifications

**Programme implication:** maintain a pinned COTO profile with exact chapter, edition, procurement / contract applicability, project-specific amendments and authority acceptance. Do not refer to a draft or chapter as the current universal rule without checking the actual road authority and contract.

Chapter 2 (Services) and Chapter 12 (Geotechnical) are especially relevant discovery points for the corridor design, but the exact clauses, project additions and acceptance criteria must be checked against the full controlled document set before any implementation profile is called compliant.

## 3. Municipal wayleave workflow as a first-class engineering interface

### 3.1 City of Tshwane — current digital process exemplar

The City of Tshwane WayleaveCentral portal describes a staged process:
1. **Service inquiry** — preliminary investigation to identify service owners and possible affected services.
2. **Cross-cut** — targeted physical exposure to confirm and pinpoint services identified in the inquiry.
3. **Wayleave** — detailed design using the information obtained in prior phases.

The portal says the process distributes inquiries to registered service owners, uses cross-cuts to confirm service positions, and allows the road authority to coordinate installation, reduce clashes, simplify maintenance and account for future services. The current portal also lists 2026 road-reserve by-law / fee documents and describes WayleaveCentral v5.2 as released 13 January 2026.

Sources:
- WayleaveCentral: https://wayleave.tshwane.gov.za/
- Portal version information: https://wayleave.tshwane.gov.za/home/about/

This is an operational example for the City of Tshwane, not a claim that every South African municipality uses the same workflow.

### 3.2 Proposed Luminous evidence thread

Represent the approval and construction lifecycle as linked, non-interchangeable states:

`service inquiry -> service-owner responses -> investigation / cross-cut -> verified survey update -> detailed design -> wayleave decision -> approved work package -> construction -> as-built submission -> inspection / commissioning -> authority acceptance`

For every transition preserve:
- issuing authority / service owner and the exact decision or response;
- applicant / contractor and their approved scope;
- affected asset references and the data-quality level;
- geometry revision, coordinate reference, date and uncertainty;
- required surveys / cross-cuts and their physical evidence;
- applicable standard, contract and approved drawing revision;
- permitted method, work windows, traffic and reinstatement conditions;
- inspections, tests, deviations, nonconformances and corrective actions;
- as-built and commissioning evidence;
- accepted, rejected, indeterminate or withdrawn disposition.

A Symthaea proposal is not a wayleave. A Mycelix receipt can preserve a decision and its provenance but cannot make the decision valid if the competent authority did not issue it. A robot may support survey or inspection only within its approved role and site method statement.

### 3.3 Johannesburg-specific caution

A City of Johannesburg document indexed by the official city website is titled **Draft Wayleave By-law** (2022). It describes service-infrastructure drawings, service-owner plans and pilot holes where necessary. Because the retrieved document is expressly a draft, do not treat its clauses as proof of the current enacted by-law. The project must obtain the live City of Johannesburg by-law, current JRA / City Power / Johannesburg Water conditions and the actual wayleave decision for the pilot location.

Source: https://joburg.org.za/documents_/Documents/By-Laws/2022/CoJ_Draft%20Wayleave%20bylaw.pdf

## 4. Municipality-specific examples: useful design evidence, not universal rules

### 4.1 City of Cape Town road-reserve standards

Cape Town's RIM Wayleave Standards state that road crossings are to use trenchless methods (examples include HDD and pipe cracking), while open trenching / microtrenching in a roadway requires written permission. The document also requires a pre-work dilapidation report and, in specified cable / telecommunications open-trench cases, a 160 mm spare duct with draw wire and end caps.

Source: https://resource.capetown.gov.za/documentcentre/Documents/Procedures%2C%20guidelines%20and%20regulations/Road_reserves_Wayleave_Process_and_Standards.pdf

These conditions are specific to the City of Cape Town road-reserve process. They must not be copied into a Johannesburg or Tshwane pilot as if they were national law.

**Design opportunity:** spare-duct policy, route reservation, draw wires, pre/post-condition evidence and reinstatement receipts are natural interfaces for a shared, maintainable duct-bank product. The corridor should be tested for future pull-through capacity, blockage detection, duct proving and as-built location evidence.

### 4.2 City Power contract specification: aluminium and physical protection

A City Power / eTenders document titled *Standard for New Service Connections and Metering Labour Contracts*, reference CP_TSSTAN_144 Rev 0, includes requirements for certain new medium-voltage cable installations to use aluminium conductors. It also describes concrete encasement where cable is to be protected against theft under the applicable City Power guideline. The document specifies particular cable constructions for defined service contexts.

Source: https://www.etenders.gov.za/home/Download/?blobName=5457f91a-bbf2-4d68-b392-c92165f72887.pdf&downloadedFileName=CP_TSSTAN_144_New+Service+Connections+and+Metering+Labour+Contract_REV+0_Upload.pdf

**Interpretation limit:** this is a named contract specification and revision, not proof that aluminium or concrete encasement is mandatory or technically appropriate for every City Power asset, every voltage level, every cable route or every South African utility. Confirm the current controlled specification, design standards, cable rating, jointing, thermal / soil conditions and utility acceptance before treating it as a design input.

**Research implication:** lower-salvage-value conductor selection and physical hardening are not new concepts. Luminous differentiation must come from matched whole-life comparisons, exact configuration and installation evidence, robot-accessible maintenance, fault isolation and exposure-normalized outcome measurement.

### 4.3 Tshwane road-reserve details

The current WayleaveCentral portal links the municipality's standards and design details, including a CTMM standard specification for service ducts. The listed material describes service ducts as supporting easy installation, maintenance and protection of existing, new and future cables and services at road crossings.

Sources:
- Portal standards catalogue: https://wayleave.tshwane.gov.za/page/details/3/
- Service-duct specification: https://wayleave.tshwane.gov.za/document/download/468484/

Treat the linked specification's edition and applicability as project inputs. Verify current revisions and acceptance with the road authority before relying on its detailed dimensions or material requirements.

## 5. Electrical cable standards and data requirements

SANS 10198 is a South African standard family covering selection, handling and installation of electric power cables rated up to 33 kV. Relevant subject areas include statutory requirements, cable selection / installation, earthing, current ratings, soil thermal / electrical resistivity, cable laying, jointing, commissioning, maintenance and fault location. The exact current edition, amendments, scope and normative references for each part must be checked with SABS and the utility owner before a project profile is frozen.

The COTO Chapter 2 services document also references SANS 10198 cable-related parts for relevant energy-service work.

Discovery references:
- COTO Chapter 2 (October 2020 DS): https://www.transport.gov.za/wp-content/uploads/2024/08/CHAPTER2_SERVICES2020.pdf
- SABS Webstore: https://store.sabs.co.za/

**Engineering implications for the robot-first corridor:**
- Cable thermal rating must include installation geometry, grouping, loading, ambient and soil thermal conditions, not just nameplate current.
- A duct-bank capacity model must account for mutual heating, spare ducts, future occupancy, pulling tension, bend radius, joint / termination access and replacement sequence.
- A sensor can provide condition evidence but cannot substitute for the applicable cable test, commissioning regime or qualified fault-location process.
- Aluminium conductor selection must include connection hardware, surface preparation, jointing process, creep / thermal-cycle performance and inspection acceptance.

No cable sizing, thermal limit, clearance dimension or test voltage is specified by this programme document.

## 6. Standards and permit profile required before a pilot

For each candidate pilot corridor, create a versioned **JurisdictionProfile** (initially a document / manifest, not an autonomous permit engine) with:

| Profile field | Required evidence |
|---|---|
| Road authority | Identified authority, road class, road-reserve limits and live permit process |
| Wayleave / servitude | Current by-law, application route, decision, conditions and expiry |
| Service owners | Named utility owners, consultation responses, accepted connection / crossing conditions |
| Subsurface data | Existing records, quality level, investigation method, cross-cut / exposed verification and coordinate reference |
| Construction methods | Method statement, contractor competence, equipment identity, ground envelope and risk assessment |
| Electrical | Applicable SANS / NRS / IEC and utility-owner specification revisions; approved cable design, thermal study, protection and commissioning requirements |
| Water / wastewater / stormwater | Applicable owner standards, pressure / gravity and separation requirements, contamination and access controls |
| Public safety | Occupational health and safety, traffic management, excavation / confined-space / electrical controls and emergency response |
| Environment / land | Watercourse, groundwater, contamination, heritage, ecology, land access and other approvals as applicable |
| Quality / handover | Inspection and test plan, nonconformance process, as-built, asset-owner acceptance and maintenance responsibility |

Every field has an explicit status: known, partial, stale, conflicting, unavailable, not applicable or unknown. A missing permit profile is a stop condition for field work, not an invitation for the planner to infer permission.

## 7. New benchmarks derived from the local process

Add a separate **B7 — Wayleave and utility-coordination performance** track. Do not combine it into a single score with robot detection or engineering feasibility.

Measure:
- elapsed time from complete submission to service-owner responses, cross-cut plan, wayleave decision and authorized start;
- number of affected service owners identified before excavation;
- cross-cut yield: conflicts confirmed, ruled out and still unresolved per investigation;
- percentage of critical crossings with evidence quality sufficient for the declared construction method;
- number of design iterations caused by previously undocumented services;
- utility strikes, near misses, damage and reinstatement defects per accepted route kilometre;
- permit submission correction / rejection rate and reasons;
- time from work completion to accepted as-built record;
- percentage of future-work applications that can reuse accepted prior evidence without treating stale evidence as current;
- repeat road cuts / route interventions avoided, verified against work-order history;
- contractor and community access to the process, including cost and time burden.

**Key outcome:** shorten the path from an honest service inquiry to an accepted, safe, maintainable installation—not bypass the authority process.

## 8. Implications for the Luminous programme

1. **Sol-Atlas:** map route and asset evidence with coordinate frame, source quality, observation date, coverage and conflict status. Keep proposed alignments separate from approved route and as-built.
2. **Symthaea:** prioritize investigation by expected decision value; recommend cross-cuts or additional survey where uncertainty affects eligibility. Do not infer missing asset clearances.
3. **Symtropy:** compare eligible construction methods, simulate robot access and failure / recovery, and preserve the source model's physical claim ceiling.
4. **Mycelix:** record the actual authority, consent, contractor qualification, material lot, inspection and acceptance evidence using existing construction provenance. Do not make the ledger an issuer of permits.
5. **Economic model:** include approval delay, survey and cross-cut cost, repeated applications, service-owner coordination, traffic / reinstatement, future route access and rejected-work cost—not only trenching and cable materials.

## 9. Next local research tranche

Before a Johannesburg pilot is proposed:
- obtain live controlled City of Johannesburg / JRA / City Power / Johannesburg Water wayleave requirements;
- obtain current City Power specifications and cable design acceptance rules, including the status of CP_TSSTAN_144 and its referenced specifications;
- confirm applicable COTO chapters and exact contract conditions with the road authority;
- verify current SANS 10198 parts / editions directly through SABS and the utility owner;
- obtain anonymized route work-order and wayleave cycle data;
- identify a willing industrial / institutional landowner and utility authority for a non-energized demonstration first.

No real corridor has been selected or approved in this research tranche.
