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

## Five independent jurisdiction dimensions

Every profile must explicitly model:

1. **Manufacturing site** — where the operation physically happens and which local site, facility, workforce, utilities, permits and emergency arrangements apply.
2. **Destination market** — where the product/component is placed on the market, registered, operated or put into service, and which product/actor obligations apply.
3. **Supply-chain origin** — where material and components originate and where the legally relevant transformations occur. Supplier address alone does not prove origin.
4. **Data processing** — where telemetry, personal data, engineering records or regulated data are stored, accessed or processed, under whose authority and for which purpose.
5. **Service/lifecycle** — where installation, repair, remanufacture, repurposing, recycling or disposal occurs, with its own skills, safety, waste and producer-responsibility obligations.

These dimensions may coincide, but must never be assumed to coincide.

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

Bind component identity and exact lot, supplier/manufacturing site, origin evidence, transformation sequence, customs classification candidate, customs-value basis, freight/insurance, tariff/preference rule and local-content calculation method/version.

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
- a local variant silently changing the global product identity.

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
