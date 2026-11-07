# AER-01 Baseline Screening — v0.1

**Date:** 2026-11-06  
**Status:** Preliminary source acquisition; not a normalized performance comparison  
**Parent:** [Symtropy #3921](https://github.com/Luminous-Dynamics/symtropy/issues/3921)  
**Related brief:** [research/aero-01/README.md](./README.md)

## 1. Purpose and rules

This file records what the public manufacturer pages actually say, the conditions they disclose, and what remains unknown. It is an acquisition register, not a ranking.

Rules:
- A manufacturer's product page is a source for its published specification, not independent validation.
- Do not compare ranges unless altitude, power setting, speed, payload, fuel/energy state, reserve and weather assumptions are compatible.
- Do not compare takeoff/landing distances unless runway surface, slope, wind, temperature, pressure altitude, mass, obstacle height and performance definition match.
- Treat empty weight as aircraft-configuration-specific. Options and installed equipment change payload.
- A value missing from a public page stays unknown; no default reserve or assumed payload is allowed.
- Every baseline must eventually be pinned to model/variant, publication revision, serial/configuration when available, data source and exact mission assumptions.

## 2. Preliminary four-seat references

| Reference | Seats | Useful load | Full-fuel payload | Cruise / range | Field performance | Notes |
|---|---:|---:|---:|---|---|---|
| Cessna Skyhawk (current product page) | 4 maximum occupants | 878 lb / 398 kg | 560 lb / 254 kg | 124 ktas max cruise; 640 nm max range | 960 ft ground roll; 1,630 ft takeoff distance; 575 ft landing ground roll; 1,335 ft landing distance | Page lists 53 US gal / 201 L usable fuel, 14,000 ft service ceiling and 730 fpm max climb. Exact reserve and range-condition basis are not stated on the page. |
| Diamond DA40 NG | 4 | 407 kg | Approximately 289 kg from 407 kg useful load minus stated 118 kg usable fuel mass | 154 ktas max cruise at 16,000 ft / MCP; 934 nm at FL160 / 50% power, including climb, explicitly no reserves | 397 m ground roll / 590 m takeoff distance over obstacle; 318 m landing ground roll / 650 m landing distance | Standard equipment, MTOM 1,310 kg, empty weight 903 kg, max operating altitude 5,000 m. Manufacturer warns specifications are approximate and may change. |
| Tecnam P2010 180 HP — hold for reconciliation | 4 | 395 kg | Not derived until exact fuel-density and loading definition are confirmed | Page headline gives 137 kt; detailed table gives 140 kt TAS. Max range listed as 720 nm. | 336 m takeoff run / 595 m takeoff distance; 237 m landing run / 521 m landing distance | Do not freeze the speed datum until the product-folder/POH revision reconciles the page's 137/140 kt figures and confirms performance conditions. |
| Sling TSi (current manufacturer page) | 4 | 430 kg | Not derived; exact fuel/mass convention and mission reserve need approved source data | 155 ktas typical cruise at 9,500 ft; 1,000 nm maximum range; 28 L/h economy-cruise fuel burn | 330 m takeoff over 50 ft obstacle; 350 m landing over 50 ft obstacle; 940 fpm sea-level climb | Manufacturer publishes 520 kg standard empty mass and 950 kg MTOW, 160 hp Rotax 916 iS. Verify exact aircraft build/registration/airworthiness category and current manual revision; do not assume all examples share one approval basis. |
| Sling High Wing (current manufacturer page) | 4 | 490 kg | Not derived; exact fuel/mass convention and mission reserve need approved source data | 145 ktas typical cruise at 9,500 ft; 900 nm maximum range; 28 L/h economy-cruise fuel burn | 810 m takeoff over 50 ft obstacle; 550 m landing over 50 ft obstacle; 679 fpm sea-level climb | Manufacturer publishes 630 kg standard empty mass, 1,120 kg MTOW (VLA), 160 hp Rotax 916 iS turbocharged. Tricycle and taildragger variants exist; freeze variant and weight convention before comparison. |

### Primary manufacturer sources

1. Cessna Skyhawk official product page: https://customer.cessna.com/sitecore/content/cessnaweb/home/piston/cessna-skyhawk?sc_lang=en
2. Diamond DA40 official technical specification: https://www.diamondaircraft.com/en/flight-school-solution/aircraft/da40/tech-specs/
3. Diamond DA40 NG 2025 product folder: https://www.diamondaircraft.com/fileadmin/diamondaircraft/documents/da40/DA40_NG_Product_Folder_2025_SCREEN.pdf
4. Tecnam P2010 180 HP official page: https://tecnam.com/aircraft/p2010-180hp/
5. Sling TSi: https://slingaircraft.com/aircraft/sling-tsi/
6. Sling High Wing: https://slingaircraft.com/aircraft/sling-hw/

### Interpretation

Sling TSi and Sling High Wing are especially relevant to the Wayfarer 4 benchmark because they are locally developed/manufactured South African designs with four-seat touring/utility positioning. Their published numbers do not prove superiority and should not be compared directly against the Skyhawk, DA40 NG or P2010 until payload, runway definition, atmosphere, fuel/reserve, configuration and certification/airworthiness basis are normalized. Use them as serious market/engineering comparators, not as a reason to copy their architecture.

The High Wing's 160 hp turbocharged Rotax 916 iS is advertised to retain full takeoff power to 15,000 ft. This is a relevant engine-level hot/high claim, not proof of obstacle-clear performance at a specified hot/high mass, runway or temperature. AER-01 must benchmark the complete mission at declared density altitude rather than claim hot/high capability as a novelty.

## 3. Preliminary two-seat references — Wren 2 lane

These are useful acquisition targets, not a normalized ranking. They differ in category, propulsion, intended operation, energy reserve and evidence maturity.

| Reference | Published configuration / mass | Published mission figures | Evidence boundary / unresolved items |
|---|---|---|---|
| Sling 2 | Two seats; 700 kg MTOW; 420 kg standard empty mass; 280 kg useful load; Rotax 912iS, 100 hp / 75 kW | 117 KTAS typical cruise at 9,500 ft; 830 nm published range; 330 m takeoff over 50 ft; 260 m landing over 50 ft; 875 fpm climb; 15 L/h economy fuel burn | Manufacturer figures. Confirm exact Sling 2 variant, equipment basis, fuel/reserve and applicable South African build/registration/airworthiness path. Treat separately from type-certified aircraft and obtain current POH/AFM/build configuration before comparison. |
| Evektor SportStar RTC | Two seats; official page lists approximately 351–355 kg empty mass and 600 kg MTOW; Rotax 912-series, 100 hp class | 115 KTAS headline cruise; 700 nm headline range; 128 m takeoff and 169 m landing figures on the official page; 78 km/h stall figure | Manufacturer page notes data may change. A separate official overview has quoted 255 kg usable load, which must be reconciled against the exact equipment/variant and detailed empty-weight definition. Do not compare short field numbers until ground-roll/obstacle definitions, conditions and revision are confirmed. |
| Pipistrel Velis Electro | Two seats; EASA type certificate EASA.A.573; 428 kg empty mass with batteries; 600 kg MTOW; 172 kg published useful load; 57.6 kW maximum takeoff power | Approximately 88–90 kt cruise depending on page/definition; 50 min typical endurance plus VFR reserve; official EU page reports 453 m grass / 409 m asphalt takeoff over 50 ft obstacle | Type-certified electric trainer; an important battery-electric operational comparator, not a direct long-range touring comparator. Verify exact manual revision, battery state/temperature, reserve policy and charging turnaround. Do not mix the 88 KTAS and 90 KCAS values from different pages. |

### Primary manufacturer sources

1. Sling Aircraft — Sling 2: https://slingaircraft.com/sling-2/
2. Evektor — SportStar RTC: https://www.evektor.com/sportstar-rtc
3. Evektor — SportStar overview: https://www.evektor.com/sportstar
4. Pipistrel — Velis Electro (EU): https://www.pipistrel-aircraft.com/aircraft/velis-electro/
5. Pipistrel — Velis Electro (US): https://www.pipistrel-usa.com/velis-electro/

### Interpretation

- **Sling 2 is strategically relevant** because it is a South African manufacturer and a practical two-seat reference. It is not automatically a type-certified peer; its exact regulatory category and local operating pathway must be established for the comparison.
- **SportStar RTC provides a more conventional certified-trainer comparison**, but its headline payload/range/runway data need a single revision and consistent definitions.
- **Velis Electro supplies a real electric-aircraft operational reference** and illustrates the distinction between battery-electric training missions and longer owner-touring missions. It should not be judged on ferry range alone; compare completed training sorties, charging time, battery life and dispatch availability.
- Do not use any table value as an AER-01 target until its source basis and mission boundary have been recovered.

## 4. Advanced architecture references — different product classes

| Reference | Published / reported architecture | What it can benchmark | What it cannot establish |
|---|---|---|---|
| Electra EL9 | Hybrid turbogenerator + battery, eight distributed electric propulsors, blown wing, fly-by-wire | eSTOL lift augmentation, phase-aware energy architecture, propulsor-wing integration, certification-basis planning | AER-01 performance; certified service performance; same-class ownership economics |
| DLR HYDRA | Research testbed concept with interchangeable hydrogen fuel-cell / turbogenerator modules, battery support and DEP | Modular test architecture, hybrid-system experimental programme, integration questions | Production readiness, certified operation or economic superiority |
| DLR PomoΔoro | Research concept with high-aspect-ratio structurally supported wing, energy module alternatives, DEP and morphing/boundary-layer technologies | Configuration hypotheses and subsystem coupling | Flight-tested performance or validated structural/aerodynamic claims |
| NASA X-57 | All-electric experimental aircraft programme with battery, motor, controller and distributed-propulsion work | Systems-engineering lessons, component and wind-tunnel test lineage | Successful first flight; NASA explicitly records programme closure without one |
| NASA truss-braced wing experiments | Structural and aerodynamic investigations of higher-aspect-ratio wings | Structural methods, test design, aeroelastic and load-path questions | AER-01 margins or geometry-specific strength |

Sources:
- Electra EL9: https://electra.aero/technology
- Electra FAA certification-basis announcement: https://electra.aero/news/electra-achieves-faa-certification-milestone-for-el9-ultra-short-aircraft
- AOPA design reporting: https://www.aopa.org/news-and-media/all-news/2026/july/flight-training/electras-blown-wing
- DLR HYDRA: https://www.dlr.de/en/latest/news/2026/hydra-aircraft-concept-takes-first-place-in-the-dlr-design-challenge-2026
- DLR PomoΔoro: https://www.dlr.de/en/latest/news/2026/hydra-aircraft-concept-takes-first-place-in-the-dlr-design-challenge-2026/pomodoro-tu-munchen
- NASA X-57: https://www.nasa.gov/x-57-maxwell/
- NASA structural wing experiment: https://www.nasa.gov/centers-and-facilities/armstrong/nasa-pushes-new-wing-design-to-find-structural-limits/

## 5. First observations — not yet design conclusions

1. **Useful load is a central constraint.** A nominally four-seat aircraft can have roughly 400 kg total useful load before occupants, baggage and fuel are allocated. The energy-system architecture must therefore be evaluated against payload remaining after usable energy and reserve are included.
2. **Cruise efficiency does not imply short-field performance.** The DA40 NG product page's 590 m takeoff-over-obstacle figure and 154 ktas maximum cruise are different mission dimensions. AER-01 must report both and cannot trade away payload or reserve to win the runway metric.
3. **The blown-wing field is already advanced.** Electra is the direct eSTOL reference. Our differentiation needs a smaller owner-operated mission, transparent lifecycle economics, hot/high performance, serviceability and evidence quality—or measured aerodynamic/propulsive improvements beyond the published competitor architecture.
4. **Modularity is not automatically beneficial.** Removable modules add attachment structure, connectors, sealing, cooling, interlocks, inspections and configuration-control requirements. The cost and turnaround reduction must exceed those penalties.
5. **High-aspect-ratio wings are not free efficiency.** Structural mass, aeroelasticity, flutter, gust loads, ground handling, hangar span and inspection must be priced into the mission result.
6. **Public web specs are insufficient to freeze design requirements.** Obtain exact POH/AFM, type-data, operating limitations, option weights and applicable manuals before any performance or airworthiness comparison.

## 6. Missing source acquisitions

- Exact current POH/AFM or approved performance data for each frozen reference variant.
- Weight-and-balance envelope and option effects by configuration.
- Ground-roll and obstacle-distance definitions, runway assumptions and environmental correction method.
- Maintenance schedules, labor-hour data, overhaul intervals, parts prices and availability.
- Dispatch reliability / fleet utilization evidence with actual denominator and operating context.
- Manufacturer warranty and battery replacement assumptions where electric/hybrid aircraft are evaluated.
- South African registration, design approval, flight-test and operation pathway reviewed against current SACAA guidance and the intended aircraft category.

## 7. South African regulatory decision gate

Do not select a development or prototype route by assuming an amateur-built pathway will support a future commercial aircraft.

SACAA's published guidance lists distinct engineering materials and application processes for Type Certification, Design Organisation Approval, Experimental Certificate of Airworthiness and Amateur-Built Aircraft. Its public FAQ describes amateur-built projects as construction/assembly by natural persons for their own education, recreation and experimental purposes, and notes operational limitations that include no commercial or remunerated activity. This makes the intended end product and applicant structure a first-order programme decision.

Before physical design freeze, request a pre-application discussion with the SACAA Engineering Office and qualified South African certification specialists. Bring a concise concept description, intended aircraft category and mission, propulsion/energy novelty, applicant/manufacturing intent, development/test sequence, and the proposed basis for design assurance and continued airworthiness. Obtain the authority's written direction on the applicable route and scope.

Official starting points:
- SACAA Airworthiness / Engineering guidance and listed Technical Guidance Materials: https://www.caa.co.za/industry-information/airworthiness/
- SACAA public FAQ, including amateur-built aircraft purpose and limitations: https://www.caa.co.za/information-for-the-public/frequently-asked-questions/

This is not a regulatory determination. The competent authority determines the applicable requirements and approvals for the actual product and applicant.

## 7A. External solver / MDO comparison versions

These are software comparators for the design method, not aircraft performance baselines. Exact build and task identity must be frozen before any measured comparison.

| Tool | Current reference pin | Intended use | Important boundary |
|---|---|---|---|
| OpenMDAO | 3.45.2 stable (27 October 2026) | MDO workflow, derivative propagation and coupled optimization comparison | Keep the 6 November development documentation distinct from the stable executable package; count adapter/workflow setup and human effort in total cost |
| SU2 | 8.6.0 (30 October 2026) | Pinned CFD / adjoint backend and solver reference | Use a source build for discrete-adjoint comparisons: official binary executables omit adjoint functionality. Keep historical 8.0.1 replication separate |

Reference acquisition, release notes, licences and build restrictions are collected in [external-benchmark-reference-manifest-v0.1.json](./external-benchmark-reference-manifest-v0.1.json). These are current reference candidates, not evidence that any SU2/OpenMDAO AER-01 task has been run or independently reproduced.

## 8. Next analysis

1. Freeze Wren 2 / Wayfarer 4 mission cards and a common reserve policy.
2. Normalize C0 reference missions from approved aircraft manuals, not headline web specs alone.
3. Build mass, payload, fuel/energy, reserve and CG ledgers before comparing propulsion concepts.
4. Add Electra / DLR / NASA data as differently scoped research comparators.
5. Only then calculate a mission-normalized Pareto comparison.

**Claim ceiling:** This source screen is not an independent aircraft performance database. The values above are published manufacturer figures with their source limitations preserved.
