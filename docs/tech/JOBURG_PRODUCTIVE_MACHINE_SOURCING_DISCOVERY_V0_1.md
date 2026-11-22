# Johannesburg Productive-Machine Sourcing Discovery v0.1

**Status:** public-web discovery list only; not an approved supplier list.
**Owner:** [LMF-FDM-300-ARCH-001 #6787](https://github.com/Luminous-Dynamics/symtropy/issues/6787)
**Checked:** 22 November 2026

## Purpose

Identify existing South African workshops and material sources that could support the first Luminous workbench / LMF-FDM-300 prototype. The objective is to use local capability where it is competitive rather than purchase CNC, laser-cutting, bending, welding and extrusion equipment prematurely.

Every entry below is a **discovery lead**. A website description is a supplier-declared claim, not a capability qualification. No quotation, site audit, quality-system review, sample part, lead time, material certificate, price, payment terms or relationship is established by this list.

## Initial routing candidates

| Candidate | Publicly described capability | Potential LMF use | What must be checked |
|---|---|---|---|
| [Directech](https://directech.co.za/capabilities/) — Kya Sand, Randburg | CNC metal cutting/bending, laser cutting, CNC punching, sheet-metal fabrication, milling/turning, powder coating, metrology and mechanical/electrical engineering | Frame plates/brackets, sheet-metal enclosure parts, machined adaptors, possible inspection assistance | Minimum order, machine work envelope, alloy/thickness range, drawing tolerances, metrology traceability, material certs, quote and delivery |
| [Glutek Engineering](https://www.glutek.co.za/) — Strydom Park, Randburg | CNC turning and machining centres, small/medium production, one-off prototypes, aluminium/steel/brass/copper/bronze and engineering plastics | Precision spacers, shafts, bushes, turned mounts, machined bed/motor interfaces | Exact tolerance capability, surface finish, work envelope, material certificates, first-off approval, lead time, cost |
| [Laser Options](https://laseroptions.co.za/) — Strydom Park, Randburg | CNC laser cutting, bending, punching, machining, welding, grinding, powder coating, assembly and design support | Sheet-metal panels, enclosure, brackets and formed components | Gauge/material range, bend radii/tolerance, enclosure sealing, coating specification, inspection records, quote |
| [PWS Engineering](https://pws-engineering.com/) — Roodepoort | CNC milling, turning, routing, surface grinding, general engineering and light fabrication; website describes one-off and small-batch work | Precision faces, plates, router-cut panels/fixtures, machined prototype parts | Actual machine capabilities and calibration, dimensional capability, materials, tolerances, quote and capacity |
| [Faltec Engineering](https://faltec.co.za/) — Jet Park, Boksburg | CNC turning, vertical/horizontal milling, fabrication and managed specialist processes; website describes first-off/in-process/final inspection, calibrated measuring equipment, material/batch records | Precision shafts, brackets, small machined components where first-off inspection and traceability matter | Exact operation scope, inspection report format, accreditation scope if any, material certs, tolerances, subcontracted steps and quote |
| [Talon Works](https://talonworks.co.za/) — Gauteng | Publicly describes FDM printing, CNC milling, laser cutting, CAD/CAM, prototype and custom machine build/upgrade services | Prototype comparison, initial FDM outsourcing, fixture prototypes, possible design-for-manufacture feedback | Exact address/site, printer and material list, process control, machine models, sample results, rights/data terms, quote |
| [Taito 3D Printing](https://taito.co.za/) — Braamfontein, Johannesburg | Structured business search result describes FDM printing and product development/CAD | Local FDM bureau comparator for simple low-risk artifacts | Confirm exact machine/material/process capability and current operating details directly; no website technical qualification reviewed in this pass |

## Material and extrusion discovery

- [Fil X / Inventech](https://www.inventech.co.za/3dprinting): public page describes Fil X as South African filament manufacture with extrusion, tooling, product development and testing, and lists PLA, ABS, PETG, ASA, SBS, TPR and composite materials. Before qualifying a spool, request the exact resin/grade, manufacturer datasheet, batch certificate, diameter/ovality tolerance, moisture/storage guidance, lot traceability, additive composition, recommended process window, emissions information and repeat-lot sample data. Do not treat the country-of-origin claim or marketing range as qualification.
- [3D Printing Works](https://www.3dprintingworks.co.za/): public site describes supply of Fil X material, including SBS and other common filament types. It is a sales/source lead; confirm product provenance and lot data.
- [PrintVeld Filament Finder](https://printveld.co.za/): South African community supplier directory. The current site says automatic supplier collection is paused while permission is confirmed and that listed observations are last verified values. Use it for supplier discovery, not live stock/price or automatic procurement.

## Aluminium extrusion / profile discovery

The first machine may use standard aluminium profiles, but profile family, alloy, temper, section, straightness, cut length and joint loads must be defined before selection.

- [Aluminium Trading](https://www.aluminiumtrading.co.za/) — Sebenza/Edenvale: discovery lead for aluminium profile/metal supply and processing; confirm structural T-slot profile availability rather than assuming architectural extrusion is mechanically interchangeable.
- [Wispeco Aluminium](https://www.wispeco.co.za/) — Alrode, Alberton: extrusion manufacturer/supplier lead; confirm exact standard profiles, alloy/temper, dimensional tolerances, straightness, minimum order and cut-to-length offer.
- [RF Metals](https://rfmetals.co.za/) — Johannesburg South: profile stockist/custom profile development lead; confirm whether required structural T-slot system and compatible joining hardware are available.
- [Aluminium Alloys](https://aluminiumalloys.co.za/) — Spartan, Kempton Park: aluminium supply/processing lead; confirm section/product range, stock state and certification.

Do not assume window/door architectural sections are equivalent to 20-series T-slot or other structural machine profiles.

## Supplier qualification packet

For each candidate, issue the same drawing/BOM/RFQ packet with:
1. exact part ID, revision, neutral CAD, dimensioned drawing, datum scheme and critical-to-function tolerances;
2. material grade/alloy/temper, condition, thickness/profile, finish and required certification;
3. process route requested and permitted substitutions;
4. inspection plan, measurement method, uncertainty/traceability requirement and sample report;
5. first-article quantity and acceptance procedure;
6. price, tooling/NRE, minimum order, lead time, capacity window, payment and delivery terms;
7. rework/return/nonconformance handling, warranty and change-notification process;
8. drawing confidentiality, IP/ownership, derivative rights and retention/security expectations.

Evaluate samples before classifying a supplier capability as verified. For critical dimensions, ask for the actual inspection report and instrument/calibration trace, not simply a statement that the shop has metrology.

## Sourcing strategy by machine assembly

- **Frame/extrusions:** define a profile and joint system only after stiffness and tolerance calculations; ask two or more sources for comparable material and cut-to-length supply.
- **Sheet-metal enclosure:** quote laser/bend/punch route from at least two candidate shops using the same DXF/STEP/drawing revision and finish requirements.
- **Precision plates/shafts/adaptors:** use a CNC shop for first articles with inspection data; do not machine in-house until demand and cost justify it.
- **FDM baseline artefacts:** request matching parts from at least two service providers using the same released STL/3MF, exact material/lot and a declared process profile.
- **Filament:** compare local and imported grades only with datasheets, lot traceability and measured print behaviour. A locally manufactured spool does not automatically have lower emissions or lower cost per accepted part.
- **Electronics/motion components:** preserve a second-source strategy and classify imported dependencies honestly. Avoid proprietary closed components if an equivalent open interface with support is available, but do not sacrifice safety/reliability to achieve local-content optics.

## Claim ceiling

This list does not establish supplier approval, an active business relationship, service availability, current stock, product quality, quote, pricing, lead time, accredited testing scope, local-content percentage or machine build readiness. All entries require direct confirmation and a measured sample route.
