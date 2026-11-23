# Distributed Appliance Fabrication Profile v0.1

**Status:** research and qualification profile; no household deployment, approved replacement-part catalogue or manufacturing partner is established.  
**Primary issue:** [DISTRIBUTED-APPLIANCE-FAB-001 #6735](https://github.com/Luminous-Dynamics/symtropy/issues/6735)  
**First manufacturing owner:** [APPLIANCE-MICROFACTORY-001 #6603](https://github.com/Luminous-Dynamics/symtropy/issues/6603)  
**Process semantics owner:** [MFG-PROC-000 #5686](https://github.com/Luminous-Dynamics/symthaea/issues/5686)

## Executive decision

**Yes to household participation in production; no to the idea that every household should manufacture complete appliances.**

Build a distributed production network with four capability tiers:

| Tier | Role | Initial permission |
|---|---|---|
| H0 — household | Resident prints or assembles approved items at home | P0 non-functional/cosmetic items only, inside an approved printer/material/ventilation envelope |
| N1 — neighbourhood hub | Trained operator, enclosed material-extrusion printer, material storage, dimensional inspection and repair assistance | Selected P0 and explicitly admitted P1 parts |
| R2 — regional partner | Better-controlled polymer AM, CNC, sheet-metal, moulding and specialist measurement | P2 parts only where exact process, materials and inspection are qualified |
| Q3 — qualified industrial/OEM route | Safety-critical, regulated or high-consequence production | P3-risk parts under product-specific industrial/OEM qualification and conformity |

Part-risk classes are separate: P0 (non-functional), P1 (low-consequence functional), P2 (engineered/service-critical), P3 (safety-critical/high-consequence). The different namespaces avoid confusing part risk with production capability.

A household should not need to own a printer. The most useful product is often local access to a trusted production capability, not another appliance sitting idle in a cupboard.

## 1. Why this is a real opportunity

Additive manufacturing is strongest where geometry, customization, low volume, obsolete spares, short runs or tooling cost make conventional production unattractive. It is not automatically lower-cost or lower-carbon, and it is rarely the best route for every high-volume commodity component.

South Africa has existing infrastructure we should connect to:
- CSIR Advanced Design and Manufacturing Innovation Centre (ADMIC): polymer/powder additive manufacturing, 3D scanning, design and product-lifecycle capabilities for industry and SMMEs. [CSIR ADMIC](https://www.csir.co.za/advanced-design-and-manufacturing-innovation-centre)
- CSIR Collaborative Programme in Additive Manufacturing (CPAM): research, supplier development, design, process development, testing and qualification. Calls are time-limited; this profile makes no assumption that a funding call is currently open. [CPAM](https://www.csir.co.za/call-proposals-collaborative-programme-additive-manufacturing)
- CSIR Future Production: manufacturing capability including metal additive, powder metallurgy, casting and injection moulding. [CSIR manufacturing](https://www.csir.co.za/what-we-do/manufacturing)

These are discovery and capability leads, not confirmed partners, accepted applications or quotations.

## 2. Standards and evidence boundaries

Use the standards as method and process references, not as a substitute for a product-specific safety case:

- [ISO/ASTM 52966:2026](https://www.iso.org/standard/89300.html), published 2 November 2026, defines capability levels for AM in self-sufficient systems, including resources, training, infrastructure, materials and the full part cycle. It is a useful framework for H0/N1/R2/Q3, not certification of this profile.
- [ISO/ASTM 52951:2026](https://www.iso.org/standard/76828.html) describes AM part data packages from design through manufacture and acceptance. Its detailed requirements are based on metal laser powder-bed fusion; this profile borrows its digital-thread concept, not a claim of FDM conformity.
- [ISO/ASTM 52900:2021](https://www.iso.org/standard/74514.html) is the current additive-manufacturing fundamentals vocabulary reference.
- [ISO/ASTM 52902:2023](https://www.iso.org/standard/82056.html) provides test artefacts for AM geometric capability.
- [ISO/ASTM 52924:2023](https://www.iso.org/standard/82959.html) addresses qualification/classification of polymer AM part properties.
- [ISO 27548:2024](https://www.iso.org/standard/83712.html) provides a chamber method for particle and specified chemical emissions from desktop material-extrusion printers; it does not measure every possible emission or automatically predict each real room.
- [UL 2904 Edition 2](https://www.shopulstandards.com/ProductDetail.aspx?productId=UL2904_2_S_20230526) is an active test method for particle and chemical emissions from printers intended for non-industrial indoor environments, including residential use.
- NIOSH [2024-103 safe 3D printing guide](https://www.cdc.gov/niosh/docs/2024-103/) and [Safe 3D Printing bulletin](https://www.cdc.gov/niosh/bulletin/2024/safe-3d-printing.html) prioritize source control, material/process substitution, enclosure/local exhaust/HEPA filtration, maintenance and operating procedures.

A manufacturer’s “low odour” filament claim is not an indoor-air qualification. A generic enclosure or consumer HEPA filter is not proof of adequate source control. The first household pilot must use a documented printer/material/process envelope and a competent safety review; where this cannot be assured, printing stays at a qualified hub. The initial H0 pilot excludes resin/vat-photopolymerization equipment.

## 3. Part admission

A part must be admitted by consequence of failure, operating environment, process/material uncertainty, rights, inspection capability and post-repair requirements—not by whether it fits on a printer bed.

| Part class | Description | Initial route |
|---|---|---|
| P0 | Cosmetic, organizational, packaging, non-functional accessory | H0 or above if rights and domestic printing controls are satisfied |
| P1 | Low-consequence functional item with bounded load/environment | N1 first; H0 only after exact part-specific admission |
| P2 | Engineered function where failure can damage equipment or impair service | R2/Q3, subject to exact qualification and post-repair checks |
| P3 | Safety-critical, regulated or high-consequence component | OEM / Q3 only; no household or unqualified neighbourhood production |

Examples explicitly excluded from home manufacture include mains electrical insulation/enclosures, guards around moving parts, blades/rotors/impellers, pressure or hot-water containment, refrigerant components, hot surfaces, structural/load-bearing parts, fire barriers, food/water-contact surfaces and safety-critical electronics. The list is a screening boundary, not a complete hazard analysis.

### First fan-cell candidates

Use the fan as the learning platform, but do not print fan safety components. First candidates are manufacturing aids: assembly jigs, drilling/alignment aids, non-safety inspection fixtures, packaging spacers and non-functional cosmetic trim. Any fixture used to determine acceptance needs its own verified geometry/reference and must not become an uncalibrated measuring standard.

A replacement clip or cover should be considered only after the exact appliance revision, failure consequence, temperature/UV/chemical exposure, material, orientation, dimensions and post-repair test are frozen. No fan guard, blade, impeller, motor mount or mains enclosure is in the H0 catalogue.

## 4. Part package and digital thread

Do not create a duplicate parts database or manufacturing execution system. Consume canonical product, process and provenance identities from existing Mycelix / MFG-PROC owners. The application-specific part package should reference:
- part identity, exact product/SKU/revision compatibility and supersession;
- part-risk class and allowed production tier;
- design rights/licence, authorship, source and redistribution conditions;
- functional requirements, failure consequences, prohibited use and operating envelope;
- CAD/drawing, critical dimensions, tolerances and inspection datums;
- material grade, supplier/lot, conditioning, drying and storage requirements;
- printer/process capability profile, firmware, nozzle, calibration, slicer profile, orientation and validated parameter envelope;
- approved build-file identity/digest; arbitrary edited G-code is not an equivalent approved build;
- build environment and emission-control requirements;
- witness coupon / test requirement, inspection and acceptance criteria;
- post-processing, installation skill, safe repair preconditions and post-repair checks;
- print receipt with design/material/printer/operator/process/inspection identity;
- evidence source, measurement uncertainty, limitations, date, expiry/review and change triggers.

A digest proves which bytes were recorded. It does not prove the design is correct, the file is licensed, the feedstock is genuine, or the manufactured part conforms.

## 5. Economics: home ownership versus shared access

Compare four routes on the same accepted part and restoration task:
1. OEM spare / official service;
2. conventional local fabrication;
3. neighbourhood additive hub;
4. household additive where H0 admits it.

Full cost per accepted part must include material, failed prints, machine depreciation, electricity, operator time, enclosure/ventilation, calibration, inspection, post-processing, transport, repair, rework and warranty reserve. Include machine idle time and maintenance.

South African retailer price guides observed in November 2026 show entry-level FDM printers around R4,600–R8,500, with realistic mid-range home/small-business budgets extending further; filament examples are around R200–R400/kg. These are dynamic retail-guide estimates, not quotations or a procurement recommendation:
- [Create It 3D — South African price guide, July 2026](https://createit3d.co.za/blogs/news/how-much-does-a-3d-printer-cost-in-south-africa)
- [JLS Computers — South African price guide, 6 November 2026](https://www.jlscomputers.co.za/3d-printing/3d-printer-price-south-africa/)

Illustrative arithmetic only: an R8,000–R11,000 printer amortized over five years and 100 accepted parts per year contributes R16–R22 capital per accepted part before filament, failed prints, electricity, labour, inspection, ventilation and maintenance. At 10 accepted parts/year, the same capital allocation becomes R160–R220 per part. This makes a shared hub plausible for sparse appliance-spare demand, while a household printer may still be useful for education, household accessories and other uses. Actual demand and utilization must replace these assumptions.

Additive should win only where total accepted-part cost, restoration time, availability or customization beats the matched OEM/conventional alternative. It should not be chosen merely because it is local or printable.

## 6. Qualification and benchmark programme

For each selected part compare OEM, conventional, N1 and H0 routes under a matched functional and environmental boundary. Report:
- request-to-conforming-part time and total service restoration time;
- full landed cost per accepted part and cost sensitivity at annual volumes 1/3/10/50/100;
- first-pass yield, reprint/rejection/rework and failed-attempt denominator;
- dimensional error and uncertainty on critical features;
- strength/stiffness by build orientation, creep/fatigue and temperature/moisture/UV/chemical performance where relevant;
- material input, support waste, failed print mass, energy and verified recycled-content fraction;
- traceability completeness, compatibility, design-rights coverage and stale/altered-file rejection;
- post-installation function, safety checks within scope, recurrence and repair downtime.

Initial acceptance hypotheses:
- 100% of catalogue files have rights, exact compatibility, part-risk class, allowed tier and claim limitations before distribution.
- 100% of prints have the process/material/part evidence required by their tier.
- Zero P3-risk designs admitted to H0; zero safety-critical installations without the designated qualified route.
- 100% rejection of seeded wrong-revision, wrong-material, unknown/expired feedstock, incompatible printer/process, altered build file, missing calibration or failed inspection.
- Three independent printer/operator nodes reproduce one admitted low-risk part with declared dimensional/functional acceptance before cross-node reproducibility claims.
- After a baseline is measured, test the stretch hypothesis of at least 25% lower median and P90 restoration time for a selected low-volume/obsolete spare set, without safety, fit/function, recurrence or lifecycle-cost regression.
- Publish all failed prints, missingness, exclusions, rework and uncertainty.

## 7. Implementation order

1. Reuse #6603 Stage 1 FDM for fixtures and controlled fit-checks; no purchase until a frozen process plan justifies it.
2. Define one fixture and its reference/inspection plan for the fan cell.
3. Admit one P0 non-functional accessory and one manufacturing aid as documentation/traceability exercises; no safety-critical replacements.
4. Compare household ownership with a shared N1 service using measured requests, travel, queue time, utilization and total cost.
5. Ask CSIR ADMIC/CPAM and local printing bureaux for current process/material/metrology scope, access, lead-time, quotations and IP/data terms; do not imply relationships exist.
6. After the P0 workflow is stable, consider a single carefully bounded P1 replacement part with designer approval and a qualified post-repair test.
7. Keep P2/P3 production with appropriate professional/OEM routes.

## 8. Claim ceiling

This profile does not establish a deployed network, approved replacement-part catalogue, household printer safety, qualified process/material, licensing rights, supplier relationship, print accuracy, repair performance, lower lifecycle cost or conformity. All numeric cost examples and improvement hypotheses remain illustrative until replaced with dated quotations and measured results.
