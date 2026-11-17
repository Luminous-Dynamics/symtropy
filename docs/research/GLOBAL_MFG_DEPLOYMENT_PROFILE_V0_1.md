# Global Manufacturing Deployment Profile v0.1

**Programme:** GLOBAL-MFG-DEPLOY-001  
**Owner:** Symtropy  
**Status:** schema and synthetic-corpus seed; no market/site qualification  
**Evidence class:** synthetic fixture  
**Last reviewed:** 17 November 2026

## Purpose

A Global Manufacturing Deployment Profile (GMDP) describes how one globally identified product/process core is realized across different manufacturing sites, destination markets, supply-chain origins, data-processing jurisdictions and service regions.

It is not a generic country scorecard, universal compliance certificate, substitute for PLM/MES/ERP, customs/legal database, second factory ontology or product passport.

The critical design rule is:

> One engineering identity; multiple independently qualified realizations.

A product may be designed in one jurisdiction, manufactured in another, use components from several more, have its production records processed elsewhere, be sold in multiple markets and later be serviced or repurposed in additional jurisdictions. Those are separate dimensions and must not collapse into one `country` field.

## Reuse existing owners

This work composes the existing public Luminous owners:

- Symthaea #12355: cross-industry capability networks and product/process/factory co-design.
- Symthaea #5686: canonical process and capability semantics.
- Symtropy #2354 / PR #2355: low-capital EV factory archetypes and regional site economics.
- Symtropy #1832: regional mobility availability, affordability, infrastructure and serviceability.
- Symtropy #2323: South African import/localization economics.
- Symtropy #3121: composed manufacturing digital-twin VVUQ.
- Symtropy #5743: low-risk FDM manufacturing-tooling pilot.
- Symtropy #5411 / PR #5414: full reconfiguration through first independently accepted output.
- Mycelix #3085: typed process/resource offers and execution lineage.
- Mycelix #6062 / PR #6063: battery-passport projections and lifecycle lineage.

The GMDP stores references to these canonical subjects. It must not reimplement their engineering semantics.

## Seven independent jurisdiction dimensions

Every profile must explicitly model:

1. **Manufacturing site** — where the operation physically happens and which local site, facility, workforce, utilities, permits and emergency arrangements apply.
2. **Destination market** — where the product/component is placed on the market, registered, operated or put into service, and which product/actor obligations apply.
3. **Supplier/manufacturing site** — where a supplier performs its operation. This is not automatically the material's legal origin or the place of substantial transformation.
4. **Material/component origin** — where the material or component legally originates under the relevant destination-market rule; it requires rule-specific evidence and is not inferred from the supplier's address.
5. **Transformation location** — where each legally or technically relevant manufacturing transformation occurs. Multiple transformations may occur in several jurisdictions.
6. **Data processing** — where telemetry, personal data, engineering records or regulated data are stored, accessed or processed, under whose authority and for which purpose.
7. **Service/lifecycle** — where installation, repair, remanufacture, repurposing, recycling or disposal occurs, with its own skills, safety, waste and producer-responsibility obligations.

These dimensions may coincide, but must never be assumed to coincide.
A component record therefore binds supplier site, material origin and each relevant transformation location separately. This is required for origin, tariff preference, local-content and resilience analysis; a supplier's billing address, headquarters or warehouse cannot stand in for the manufacturing plant or transformation sequence.


## Record model

### Global engineering core

Bind exact product family, variant, configuration, revision and digest. Bind canonical material/process/recipe, geometry, inspection and acceptance references. Region profiles must preserve the shared engineering identity unless an explicitly versioned product variant is created.

### Manufacturing-site realization

Bind the exact site/facility/zone, process and machine capability references, tooling, metrology, utilities, environmental envelope, workforce skills, supplier/logistics dependencies, connectivity and emergency arrangements.

Keep measured, sourced, declared, modelled and synthetic inputs distinct. Utility availability is not the same as a utility capacity measurement. Nameplate machine capability is not qualified process capability.

### Jurisdictional obligations

Each obligation must bind:

- jurisdiction and jurisdiction level;
- affected actor, object, product class and operation;
- source issuer, instrument, edition/version and official source;
- source publication, entry-into-force, application, transition and expiry dates separately;
- source status: in-force, adopted-not-effective, proposed, guidance, voluntary standard, contractual, superseded, under review, unknown or synthetic fixture;
- applicable conformity/approval route and evidence;
- responsible operator/reviewer;
- current disposition: satisfied, unsatisfied, not applicable with reason, pending evidence, disputed or unknown.

A proposal cannot satisfy an in-force legal obligation. A voluntary standard is not automatically law. A source is not applicable merely because it mentions the same technology. Standards alignment is not a certificate, approval or market permission.

### Supply-chain origin and transformation

For each component bind supplier site, material origin, all relevant transformation jurisdictions, exact lot and the transformation sequence. Bind the destination-specific origin rule, customs classification candidate, customs-value basis, freight/insurance, tariff/preference rule and local-content calculation method/version. Supplier-site membership must reconcile with the supplier-site dimension; material origin with component-origin; and each transformation location with transformation jurisdictions.

Unknown origin, tariff classification, preference eligibility or incentive entitlement remains unknown. Do not award a modeled saving for an unevidenced preferential rate or local-content benefit. Do not count a finished/landed price and then add the same freight or duty again.

### Operating and service realization

Bind dated local currency/FX, tax basis, wages/shifts, energy and water price/reliability, logistics, parts availability, repair skill/tool coverage, language/manuals, accessibility, connectivity/offline operation, cybersecurity/privacy/data-residency requirements and end-of-life routes.

Synthetic operating archetypes are benchmark test conditions, not descriptions of entire countries or populations.

## Status and decision semantics

Do not expose a single `globally_available` or `compliant` boolean.

Keep these states separate:

- concept / benchmark only;
- route identified;
- source evidence assembled;
- local review pending;
- site capability demonstrated;
- process qualified within a declared envelope;
- product conformity evidence complete;
- market approval/registration independently established;
- service and lifecycle route established.

A digital profile audit may report structural defects, stale references or unresolved requirements. It cannot make a legal applicability decision, certify a site, approve a vehicle, grant an incentive or authorize production.

For decision-critical unknowns, the only acceptable disposition is an explicit blocker or abstention. Unknown is not zero, false, compliant, safe, available or accepted.

## Standards and regulatory interoperability

Use standards as versioned external mappings. Preserve scope and applicable edition. Examples include:

- ISO 23247-5:2026 for the manufacturing digital thread;
- ISO 23247-6:2026 for integrated, unified and federated manufacturing digital-twin composition;
- ISO 23247-8 as a work item under development for manufacturing digital-twin VVUQ, not a published International Standard;
- ISO 9001:2026 for quality-management systems;
- ISO 14001:2026 and ISO 45001:2018 where the relevant management-system scope applies;
- ISO 13849-1:2023 for safety-related parts of machinery control systems;
- ISA/IEC 62443 for industrial automation and control-system cybersecurity;
- UNECE WP.29 instruments for applicable vehicle categories/markets;
- ISO/ASTM additive-manufacturing standards where the process and material scope match;
- QIF/ISO 23952, MTConnect and OPC UA mappings where applicable.

For road vehicles, the UNECE 1958 and 1998 Agreements must not be conflated. Under the 1958 Agreement, reciprocal recognition is limited to Contracting Parties applying the same UN Regulation. The 1998 Agreement provides a process for Global Technical Regulations but does not itself provide mutual recognition of approvals. Other jurisdictions use different approval/self-certification routes. Applicability must be evaluated per market, vehicle class, actor, regulation and date.

The profile compiler must not treat the newest publication date as the legal application date. It must preserve transition windows and the latest authoritative source status.

## Synthetic corpus and benchmark

The initial fixture contains four synthetic deployment archetypes:

- dense urban / limited private charging;
- hot, dusty / water constrained;
- cold, high-altitude / weak grid;
- island / import-dependent logistics and sparse service.

They all reuse the same synthetic engineering-core identity to test whether common engineering identity survives differing realization profiles. They are not country profiles and do not contain real legal or cost values.

The negative-control catalogue includes:

- proposal represented as enacted law;
- wrong destination market;
- wrong product/vehicle category;
- expired or superseded source treated as current;
- mutual recognition assumed where no applicable recognition exists;
- supplier location mistaken for component origin;
- unknown origin/HS classification treated as qualifying;
- unknown evidence promoted to satisfied;
- data-processing location omitted;
- a local variant silently changing the global product identity;
- unknown customs classification treated as preferential or quota eligible;
- origin preference claimed without the applicable origin rule and transformation evidence;
- responsible economic operator/reviewer mismatched to the scoped obligation;
- requirement or temporary tariff/quota applied outside its effective, transition or expiry window;
- local-content claim made without a named methodology, numerator, denominator and evidence.

The v0.1 corpus freezes **15 adversarial cases** across these themes. Each synthetic case expects rejection or abstention; it does not represent a real legal determination.

### Measures

Report separately:

- jurisdiction/actor/product/operation/date coverage, with denominators;
- false-ready and false-approval outcomes;
- applicability and `not applicable` disposition correctness;
- stale-source and scope-mismatch detection;
- common-core identity preservation across regional variants;
- time and engineering rework to add/update a market profile;
- cost per finally accepted unit with source, local-currency basis and uncertainty;
- supplier concentration, origin exposure, logistics P50/P90 and disruption recovery;
- resource use per accepted product, only where boundary and source evidence permit;
- service/repair coverage, parts lead time and end-of-life route completeness;
- digital-thread completeness, replay and offline/degraded-mode behavior.

### Initial software invariants

These are test targets, not industry performance claims:

- 100% of obligation records identify jurisdiction, actor/object scope, source status and relevant dates, or explicitly declare them unknown.
- 100% rejection/abstention for the frozen adversarial cases.
- Zero false-ready outcomes on the frozen hard-gate corpus.
- 100/100 input-order permutations produce byte-identical canonical outputs.
- Every regional variant preserves the common product/process identity or records an explicit incompatibility.
- No weighted score compensates for a failed legal, product-safety, worker-safety, environmental, quality or traceability gate.
- Independent replay reproduces the same scoped structural disposition.

Only after real baseline measurements exist should deployment-time, rework, cost or coverage improvement percentages be preregistered.

## Implementation boundaries

This v0.1 is a schema/corpus seed. Its first verifier is structural and deterministic. It does not fetch or interpret laws, determine tariff codes, calculate legal origin, validate supplier claims, inspect a physical facility, perform conformity assessment or authorize production.

Production market/site claims require current authoritative sources and competent local legal, regulatory, quality, environmental, occupational-safety and engineering review.

## References checked 17 November 2026

- ISO 23247-5:2026: https://www.iso.org/standard/87425.html
- ISO 23247-6:2026: https://www.iso.org/standard/87426.html
- ISO 23247-8 work item: https://committee.iso.org/standard/94858.html
- UNECE WP.29 framework: https://unece.org/wp29-introduction
- UNECE vehicle-regulations FAQ: https://unece.org/transport/vehicle-regulations/faq
- ISO 9001:2026: https://www.iso.org/standard/9001
- ISO 14001:2026: https://www.iso.org/standard/14001
- ISO 45001:2018: https://www.iso.org/standard/45001
- ISO 13849-1:2023: https://www.iso.org/standard/73481.html
- ISA/IEC 62443: https://www.isa.org/standards-and-publications/isa-standards/isa-iec-62443-series-of-standards
- NIST SP 800-82 Rev. 3 and Rev. 4 draft notice: https://csrc.nist.gov/pubs/sp/800/82/r3/final
- IEA Global EV Outlook 2026, manufacturing and trade: https://www.iea.org/reports/global-ev-outlook-2026/manufacturing-and-trade
- IEA Global EV Policy Explorer: https://www.iea.org/data-and-statistics/data-tools/global-ev-policy-explorer
- EU Battery Regulation 2023/1542: https://eur-lex.europa.eu/eli/reg/2023/1542/ojn
- European Commission battery passport resources: https://single-market-economy.ec.europa.eu/single-market/digital-product-passport/batteries_en

These references inform interoperability and research. They are not blanket certification or legal advice.

## Worked global market-route source panel (17 November 2026)

The first source-discovery panel should include markets with materially different product-conformity mechanisms and manufacturing/trade conditions. The examples below are **source pointers and modelling distinctions**, not completed legal applicability determinations.

### European Union — whole-vehicle type approval

The European Commission describes the EU system as Whole Vehicle Type-Approval (WVTA): a vehicle type can be approved in one EU country and marketed across the EU without further tests, subject to Regulation (EU) 2018/858 and applicable regulatory acts. Approval authorities and designated technical services perform distinct roles. The Regulation also supports multi-stage approval for incomplete or completed vehicles, relevant to final-stage assembly and bodybuilders.

- Official Commission overview: https://single-market-economy.ec.europa.eu/sectors/automotive-industry/technical-harmonisation/technical-harmonisation-eu_en
- Regulation (EU) 2018/858, consolidated 2 August 2026: https://eur-lex.europa.eu/eli/reg/2018/858/20260802/eng

GMDP fields to bind: vehicle category, stage of completion, approval authority, technical-service evidence, applicable regulatory acts, conformity-of-production arrangements, manufacturer/importer role, approval number/extension and effective dates. A component certificate, UN approval or factory audit is not automatically a whole-vehicle approval.

### United States — manufacturer self-certification

NHTSA's manufacturer handbook explains that NHTSA does not issue type-approval certificates. Manufacturers of vehicles and regulated equipment sold in the United States certify compliance with applicable Federal Motor Vehicle Safety Standards (FMVSS); NHTSA enforces compliance. Do not represent a US manufacturer self-certification as a NHTSA-issued approval or assume an EU WVTA is the US market-release route.

- NHTSA, *Outline for New Manufacturer Information*: https://vpic.nhtsa.dot.gov/documents/ManufacturerHandbook.pdf

GMDP fields to bind: responsible manufacturer, applicable FMVSS set and revisions, certification label/statement, VIN/manufacturer identifiers, defect/recall obligations, imported-vehicle role and enforcement/recall records. Exact applicability remains vehicle-class, equipment and date specific.

### India — CMVR and notified test agencies

India's vehicle type-approval route is based on the Central Motor Vehicles Rules (CMVR) and notified test agencies under Rule 126. A Ministry of Heavy Industries PM E-DRIVE operational-guidelines document identifies CMVR compliance and applicable AIS/IS standards, including AIS 049 and AIS 156 for BEVs **within that scheme's eligibility context**. Do not generalize this scheme-specific reference into an exhaustive regulatory checklist for every vehicle category or purpose.

- ARAI vehicle certification: https://www.araiindia.com/certification/certification/vehicle
- MHI PM E-DRIVE operational guidelines (30 September 2024): https://heavyindustries.gov.in/sites/default/files/2025-07/operatioal-guidelines-dt._30.09.2024_for_p_e-drive.pdf

GMDP fields to bind: CMVR rule/vehicle category, notified test agency, AIS/IS edition and applicability, test reports, type approval, extensions, domestic manufacturer/importer responsibility and scheme-specific eligibility as a separate obligation.

### South Africa — NRCS conformity and Letter of Authority

The NRCS states that a Letter of Authority (LOA) is required for relevant imported vehicles and for certain vehicles built/manufactured or modified in South Africa; it also describes conformity requirements for locally manufactured and imported vehicles intended for public roads. A foreign certificate may be supporting evidence, but NRCS retains the jurisdiction-specific conformity decision.

- NRCS vehicle imports and conformity: https://www.nrcs.org.za/automotive/importing-vehicles/
- South African Government LOA service: https://www.gov.za/services/register-motor-vehicle/letter-authority-respect-motor-vehicle

GMDP fields to bind: local manufacturer/importer/builder status, vehicle category, applicable compulsory specification, NRCS evidence request, LOA and model/design approval status, local roadworthiness/registration interface, and site permits separately from vehicle approval.

### Brazil — EV CKD/SKD tariff route and time-limited quota

Brazil is a useful example of why customs routes need their own time and eligibility model. MDIC's 26 October 2026 update says the 35% import duty for electrified CKD vehicles is to apply from January 2027, with additional zero-duty import quotas for CKD and SKD for six months, totalling US$463 million. The official Siscomex import page lists the EV CKD/SKD quota, NCM/EX lines 8703.40.00 Ex 019, 8703.60.00 Ex 019 and 8703.80.00 Ex 014, Resolution Gecex 927/2026, Portaria Secex 513/2026 and a company-allocation list. **The official Secex legislation index records Portaria Secex 513 as amended by Portaria Secex 551 of 5 November 2026.** A pointer to Portaria 513 alone is therefore not a complete current rule chain. Quota eligibility and use are not automatic country-wide entitlements.

- MDIC announcement, updated 26 October 2026: https://www.gov.br/mdic/pt-br/assuntos/noticias/2025/julho/gecex-delibera-sobre-cronograma-de-elevacao-tarifaria-para-carros-eletricos-e-hibridos-importados/
- Siscomex EV CKD/SKD quota, NCM/EX lines and company allocation: https://www.gov.br/siscomex/pt-br/informacoes/importacao
- Secex legislation index (records Portaria 513 amendment by Portaria 551, 5 November 2026): https://www.gov.br/siscomex/pt-br/legislacao/secex

GMDP fields to bind: NCM/EX tariff line, product form (CBU/CKD/SKD), origin, importer/company allocation, current underlying resolution, current portaria and all amendments, allocated amount, quota balance, application and utilisation window, customs value, rate outside the quota, and each official source's last-updated/retrieval date. The tariff schedule and quota are not vehicle type approval and must not be blended into a single compliance score.

### Cross-market vehicle software and cybersecurity obligations

For connected or updateable EVs, product conformity must also preserve cybersecurity and software-update management as distinct obligation families. Do not merge them into a generic "cyber compliant" field or infer applicability from the fact that a vehicle has an OTA feature.

- **UN Regulation No. 155 (R155)** addresses vehicle cybersecurity and the Cyber Security Management System (CSMS). Its scope covers vehicle categories L, M, N and O when fitted with at least one electronic control unit, subject to the applicable series, approval route and national/contracting-party application. The regulation requires lifecycle-oriented risk management across development, production and post-production, including monitoring and response to threats and vulnerabilities. Official consolidated EU text: https://eur-lex.europa.eu/eli/reg/2025/5/oj/eng
- **UN Regulation No. 156 (R156)** addresses vehicle software updates and the Software Update Management System (SUMS), including update identity, compatibility, integrity and safe/update-management processes. Bind the applicable vehicle category and whether software updates are within the vehicle's approved configuration. Official UNECE resource: https://unece.org/transport/documents/2021/03/standards/un-regulation-no-156-software-update-and-software-update
- The VCA's 22 May 2026 guidance distinguishes the categories and explains audit-based CSMS/SUMS assessments. It lists M/N/O and some other categories for R156 where software updates are permitted; exact applicability must be reconciled against the legal instrument and target market, not copied wholesale from a summary page: https://www.vehicle-certification-agency.gov.uk/connected-and-automated-vehicles/cyber-security-and-software-updating/

GMDP should represent these as separate, actor- and market-scoped obligations binding:
- vehicle category, electronic architecture / update capability and exact vehicle type/revision;
- applicable UN Regulation, amendment series/supplement, destination-market adoption and transition dates;
- manufacturer's CSMS/SUMS authority and relevant certificates/assessment scope;
- suppliers and software-component provenance, vulnerability intake, response and patch dependencies;
- software identity, version, integrity data, compatibility, update authorization, installation outcome and rollback/recovery evidence;
- production conformity and post-production monitoring responsibilities;
- data access, privacy, consent and cross-border telemetry processing jurisdictions.

A CSMS certificate is not a vehicle type approval; SUMS process evidence is not proof that a specific update is compatible or safely installed; software-update capability does not establish that a destination market applies R156 to every vehicle category. The responsible manufacturer remains accountable for the applicable approval and lifecycle obligations. These are separate evidence joins, not a single tick-box.

### Cross-market implications

These examples establish a useful initial market-route benchmark:

- EU: type approval with multi-stage route and conformity-of-production.
- US: manufacturer self-certification against FMVSS and NHTSA enforcement.
- India: CMVR/notified test-agency route; scheme-specific AIS references remain scoped.
- South Africa: NRCS conformity and LOA requirements.
- Brazil: time-bound customs tariff/quota route, distinct from product approval.

The market profile should therefore be a graph of actor-specific obligations and evidence, not a checklist that says a country is either supported or unsupported. Keep approval, factory/site permission, customs treatment, incentive eligibility, import licensing, registration, production quality and after-sales/service obligations separate.

