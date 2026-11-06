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

### Primary manufacturer sources

1. Cessna Skyhawk official product page: https://customer.cessna.com/sitecore/content/cessnaweb/home/piston/cessna-skyhawk?sc_lang=en
2. Diamond DA40 official technical specification: https://www.diamondaircraft.com/en/flight-school-solution/aircraft/da40/tech-specs/
3. Diamond DA40 NG 2025 product folder: https://www.diamondaircraft.com/fileadmin/diamondaircraft/documents/da40/DA40_NG_Product_Folder_2025_SCREEN.pdf
4. Tecnam P2010 180 HP official page: https://tecnam.com/aircraft/p2010-180hp/

## 3. Advanced architecture references — different product classes

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

## 4. First observations — not yet design conclusions

1. **Useful load is a central constraint.** A nominally four-seat aircraft can have roughly 400 kg total useful load before occupants, baggage and fuel are allocated. The energy-system architecture must therefore be evaluated against payload remaining after usable energy and reserve are included.
2. **Cruise efficiency does not imply short-field performance.** The DA40 NG product page's 590 m takeoff-over-obstacle figure and 154 ktas maximum cruise are different mission dimensions. AER-01 must report both and cannot trade away payload or reserve to win the runway metric.
3. **The blown-wing field is already advanced.** Electra is the direct eSTOL reference. Our differentiation needs a smaller owner-operated mission, transparent lifecycle economics, hot/high performance, serviceability and evidence quality—or measured aerodynamic/propulsive improvements beyond the published competitor architecture.
4. **Modularity is not automatically beneficial.** Removable modules add attachment structure, connectors, sealing, cooling, interlocks, inspections and configuration-control requirements. The cost and turnaround reduction must exceed those penalties.
5. **High-aspect-ratio wings are not free efficiency.** Structural mass, aeroelasticity, flutter, gust loads, ground handling, hangar span and inspection must be priced into the mission result.
6. **Public web specs are insufficient to freeze design requirements.** Obtain exact POH/AFM, type-data, operating limitations, option weights and applicable manuals before any performance or airworthiness comparison.

## 5. Missing source acquisitions

- Exact current POH/AFM or approved performance data for each frozen reference variant.
- Weight-and-balance envelope and option effects by configuration.
- Ground-roll and obstacle-distance definitions, runway assumptions and environmental correction method.
- Maintenance schedules, labor-hour data, overhaul intervals, parts prices and availability.
- Dispatch reliability / fleet utilization evidence with actual denominator and operating context.
- Manufacturer warranty and battery replacement assumptions where electric/hybrid aircraft are evaluated.
- South African registration, design approval, flight-test and operation pathway reviewed against current SACAA guidance and the intended aircraft category.

## 6. Next analysis

1. Freeze Wren 2 / Wayfarer 4 mission cards and a common reserve policy.
2. Normalize C0 reference missions from approved aircraft manuals, not headline web specs alone.
3. Build mass, payload, fuel/energy, reserve and CG ledgers before comparing propulsion concepts.
4. Add Electra / DLR / NASA data as differently scoped research comparators.
5. Only then calculate a mission-normalized Pareto comparison.

**Claim ceiling:** This source screen is not an independent aircraft performance database. The values above are published manufacturer figures with their source limitations preserved.
