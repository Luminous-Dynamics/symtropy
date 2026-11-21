# South African Household Washer Comparator Discovery Register v0.1

**Status:** source-grounded discovery register; comparator shortlist not frozen; no physical appliance has been purchased, inspected, tested or qualified.  
**Snapshot date:** 21 November 2026  
**Owner:** Symtropy APPLIANCE-WASHER-001 #1828, composing KITCHEN-REGIONAL-CONFORMITY-001 #6033 and Mycelix APPLIANCE-EVID-001 #5156.  
**Related crosswalk:** [South African appliance performance and conformity crosswalk](SOUTH_AFRICAN_APPLIANCE_PERFORMANCE_AND_CONFORMITY_CROSSWALK_V0_1.md).

## 1. Why this register exists

The washer architecture programme needs real South African market products as controls, with exact product identity and source lineage before purchase. Retail product pages and manufacturer datasheets are discovery evidence only. They do not establish the exact delivered serial/model revision, market authorization, laboratory-tested performance, or a matched comparison.

This register identifies a first pool of 8 kg front-loading candidates with primary manufacturer sources. It deliberately does not declare a winner or convert non-equivalent published energy labels into a ranking.

## 2. Candidate pool

### Candidate A — Bosch WAN282X1ZA

**Discovery status:** strongest first candidate for a South African procurement enquiry because the official Bosch ZA product page identifies the exact SKU and, on the snapshot date, displayed a price of R10,499 including VAT and “Available for order”. Reconfirm before purchase; dynamic price/stock can change.

Official sources:
- Product page: https://www.bosch-home.com/za/en/product/washersanddryers/washingmachines/frontloader/WAN282X1ZA
- South African product sheet: https://media3.bosch-home.com/Documents/specsheet/en-ZA/WAN282X1ZA.pdf
- Manual: https://media3.bosch-home.com/Documents/9001778979_A.pdf

Manufacturer-published identity/performance data:
- Rated capacity: 8 kg.
- Maximum spin speed: 1,400 rpm.
- Energy class printed on the current product page/spec sheet: A+++ (legacy label scale; do not compare directly with the current EU A–G scale).
- Annual energy: 137 kWh per 220 standard washing cycles.
- Annual water: 9,900 L per 220 standard washing cycles.
- Derived arithmetic only: 137/220 = 0.623 kWh per nominal standardized cycle; 9,900/220 = 45.0 L per nominal cycle. These are arithmetic transformations of manufacturer-declared values, not independent tests and not normalized to kg of accepted cleaned textile.
- Standard cotton programme examples: 60°C, 8 kg = 1.00 kWh / 88 L / 3 h 30 min; 40°C, 8 kg = 0.32 kWh / 88 L / 3 h 14 min. Refer to the manual table for the precise programme and footnote boundaries.
- Spin class B; residual moisture stated as 53%; spin noise 73 dB(A) re 1 pW; wash noise on product page 55 dB(A).

Caveats:
- The product sheet has a previous-generation A+++ label and 220-cycle annual-use basis, which is not directly comparable with current EU Eco 40–60 weighted 100-cycle sheets.
- Verify exact type plate, production date/revision, current price, local warranty, spare-part catalogue and the applicable NRCS LOA before admitting a physical unit.
- Product-page rating/review count is not engineering evidence.

### Candidate B — Miele WWA120 WCS 8kg Active

**Discovery status:** strong documentation candidate because Miele supplies an identifiable South African PDF data sheet and a full set of current-style Eco 40–60 values.

Official source:
- South African product data sheet: https://media.miele.com/downloads/n-/za/FS_12721160_ZAE_ZA-en-ZA.pdf
- Miele South Africa: https://www.miele.co.za/

Manufacturer-published identity/performance data:
- Model: WWA120 WCS 8kg Active.
- Material number: 12721160.
- EAN: 4002516867302.
- Rated capacity: 8 kg.
- Maximum spin speed: 1,400 rpm.
- Weighted energy: 43 kWh per 100 operating cycles.
- Weighted water: 48 L per cycle.
- Eco 40–60 duration at nominal capacity: 219 min.
- Spin efficiency class B; residual moisture and performance detail must be taken from the relevant product information sheet/version.
- Spin noise 72 dB(A) re 1 pW.
- Manufacturer's 20-year test statement describes 5,000 wash loads during product development. Treat this as a manufacturer test claim, not a field-life guarantee, warranty term, or independent reliability qualification.

Caveats:
- The sheet references the EU delegated product-information regime. Confirm exact South African market model/type plate, local sale/stock, price, warranty, service and applicable approvals before purchase.
- The 43 kWh/100 cycle and 48 L/cycle metrics are not directly rankable against Bosch's older 220-standard-cycle annual values without a shared method and programme boundary.

### Candidate C — LG F1222TD5

**Discovery status:** South African manufacturer page with performance fields. Useful candidate for documentation and serviceability research, but the public page is incomplete for modern matched energy comparison.

Official source:
- LG South Africa product page: https://www.lg.com/za/laundry-care/front-loaders-washer/f1222td5/

Manufacturer-published identity/performance data:
- Model: F1222TD5, 8 kg direct-drive front loader.
- Water consumption: 56 L (the public page does not identify the programme/test conditions alongside this value).
- Energy consumption: 1.36 kWh (the public page does not identify the programme/load/test method beside this value).
- Wash noise: 54 dBA; spin noise 67 dBA.
- Energy efficiency class: A+ on the displayed legacy scale; wash performance class A; spin performance class A.

Caveats:
- Do not compare 1.36 kWh / 56 L against Bosch/Miele values until the exact cycle, temperature, load, test method and measurement boundary are known.
- Confirm current local availability, model age, serial/revision, manual, parts catalogue, warranty and approval evidence.

### Regional watchlist — Samsung WW80T554DAX/S3

Official manufacturer page:
- https://www.samsung.com/africa_en/washers-and-dryers/washing-machines/ww5000t-front-loading-eco-bubble-ai-control-add-wash-8kg-platinum-silver-ww80t554dax-s3/

The official Samsung Africa page identifies an 8 kg model with 55 kWh/100 cycles, 48 L/cycle, Eco 40–60 duration 3:38, 1,400 rpm, B spin class and 72 dB spin noise. However, the same page currently displays **“Not for sale”**. It is therefore not admitted to the South African procurement shortlist. Retain only as a regional data point; do not infer local availability or market authorization.

## 3. Why the numbers cannot yet be ranked

These manufacturer specifications span:
- older and newer efficiency-label regimes;
- annualized 220-cycle values versus weighted 100-cycle values;
- unspecified programme values versus defined Eco 40–60 weighted values;
- potentially different detergent, load, conditioning, ambient, water-hardness and test-method editions;
- different model ages, firmware and regional variants.

A convenient ratio such as kWh/cycle is not a fair functional unit if the cycle definitions, cleaning result, load size, rinse quality or spin-drying endpoint differ. The numbers above are evidence for selecting products and designing test questions, not a performance league table.

## 4. Normative/test-method update

### Household washer performance

IEC 60456:2024, Edition 6, is the current IEC household washer performance method identified by the IEC catalogue (published 27 November 2024; stability date 2028):
https://webstore.iec.ch/en/publication/70049

Important changes for our benchmark include:
- separation of test-method result from test-run series/class;
- new standard powder detergent IEC-P and standard liquid/wool detergents IEC-L / IEC-W;
- cleaning-performance rather than relying only on the previous washing-performance formulation;
- wool cleaning, LAS rinsing, gentleness-of-action and low-power-mode measures;
- temperature inside the washload and intermittently recurring functions;
- alternative ambient conditioning corrections;
- larger load descriptions and mixed polyester-cotton loads;
- multi-compartment machine treatment;
- uncertainty/supporting data relocated to the IEC supporting dashboard.

The standard is for household and similar machines; it explicitly does not apply to commercial laundry machines. Commercial laundry performance needs a separate method/profile and should not be silently benchmarked against household units.

### Noise

IEC 60704-2-4:2025 is the current fourth edition for airborne acoustic-noise test methods for washing machines and spin extractors:
https://webstore.iec.ch/en/publication/90407

It is aligned to IEC 60456:2024, including programme and detergent, multi-compartment machines and drum-speed measurement. This gives a coherent modern household washer performance/noise test family.

### Safety

IEC 60335-2-7:2024 is the ninth edition for washing-machine safety and is used with the applicable IEC 60335-1 edition/amendments:
https://webstore.iec.ch/en/publication/80076

The applicable South African SANS adoption and NRCS product/LOA scope must be determined separately. Low-voltage or external-supply test electronics do not make a mains appliance prototype safe or exempt.

## 5. South African method/accreditation issue — unresolved and important

Public primary-source discovery surfaced a potential method-transition gap:
- The current SABS Appliances page says its laboratory is SANAS ISO/IEC 17025 accredited and performs household-appliance safety, performance and energy-efficiency tests, but the public page does not expose the exact current accreditation schedule/method scope: https://www.sabs.co.za/appliances
- A DMRE/SABS capacity-to-test document refers to washing-machine energy measurement using SANS 60456 / IEC 60456:2003: https://www.dmre.gov.za/Portals/0/Energy_Website/EEE/Projects/EnergyEfficiencyStandardsAndLabellingForHouseholdAppliances/Capacity-to-test-for-newEnergyEfficiencyStandards-SABS-Lab.pdf
- A UNDP historic laboratory tender names SANS 1695 for washing-machine energy testing in a SANS 941 framework: https://procurement-notices.undp.org/view_file.cfm?doc_id=143929
- The IEC now lists IEC 60456:2024 Edition 6 as valid and technically revised. The public records above do **not** prove that South Africa has adopted it as the applicable SANS edition, or that a particular local laboratory is accredited for the 2024 procedure.

Action before protocol freeze:
1. Query the live SABS standards catalogue for the current SANS performance method and edition; capture catalogue status/date.
2. Use the SANAS accredited-facilities database to obtain SABS Laboratory Services Division / Appliances exact certificate and schedule: https://www.sanas.co.za/Pages/website/accreditation/load/latest-accredited-facilities.aspx
3. Confirm whether the washer performance method, noise method and any relevant sample preparation/calibration methods are explicitly in the accredited scope, or whether testing is non-accredited/developmental.
4. Ask SABS whether it can execute IEC 60456:2024 Edition 6 as a customer-specific engineering test, even if that method is not in its current accredited scope; record method deviations and accreditation status.
5. If local capacity is unavailable, identify an ILAC-MRA laboratory with exact method scope for engineering comparisons, and separately ask NRCS what evidence it will accept for any regulatory/LOA purpose.
6. Preserve a strict distinction between research test, accredited test, safety test, energy-label test and NRCS approval.

This is not a claim that local capability is absent. SABS publicly identifies an accredited appliance laboratory. The exact relevant method scope and current edition remain to be confirmed.

## 6. Proposed matched benchmark protocol

### Phase 0 — acquisition and identity
- Reconfirm current quote/availability for each SKU, exact delivery-market SKU and revision.
- Obtain type plate photographs, serial/production code, rated electrical supply and instruction/manual revisions.
- Check LOA/market evidence by exact model and importer; do not use a retailer listing as approval evidence.
- Obtain spare-part catalogue, service manuals, warranty and local repair-provider/parts lead-time statements.
- Record product age/condition; used units are teardown subjects, not interchangeable with new control units.

### Phase 1 — control-only metrology pilot
- Prefer at least three units of the same exact model/revision to estimate unit and measurement repeatability.
- If only one unit is available, explicitly downgrade the result to a case study.
- Verify water and electrical meters, temperature probes, calibrated scales, load conditioning and reference test materials.
- Conduct protocol dry-runs before confirmatory measurements.
- Establish instrument uncertainty and repeatability; do not size confirmatory trials based on a single favorable cycle.

### Phase 2 — common-procedure testing
Where legally, technically and economically feasible, test every candidate using the same version-pinned IEC 60456:2024 protocol, detergent class, test load, ambient conditioning, water hardness, programme and test laboratory/fixture. If a local lab cannot claim conformity with Edition 6, publish the work as a documented engineering protocol, not an IEC-conforming test.

Measure separately:
- kWh/cycle and low-power states;
- litres/cycle and any boundary-specific water input/output;
- cleaning performance across the prescribed soil/load set;
- LAS rinsing performance / residual where applicable;
- gentleness-of-action / textile damage proxy;
- temperature inside load;
- residual moisture after spin;
- programme duration and interventions;
- sound power/noise using the correct method, not phone microphones;
- repeatability, uncertainty, missed runs and invalid runs.

### Phase 3 — repair and lifecycle tests
Use the same exact seeded fault set and controlled work instructions across models:
- time to safe isolation;
- access/disassembly/reassembly time;
- correct fault identification versus false replacement;
- parts/tools/manual availability, order time and price;
- post-repair functional verification;
- recurrence observation window;
- repair cost and material discarded.

Keep serviceability, performance, safety and recurrence as separate outcome families. A repairability score cannot substitute for a verified restoration event.

## 7. New benchmark direction: garment-service quality frontier

The 2024/2025 test-method revisions create an opportunity to move beyond energy-only washer comparison.

We propose a **Garment-Service Quality Frontier** that reports the Pareto surface across:
- cleaning performance;
- rinsing residue / LAS;
- gentleness of action and textile wear;
- residual moisture / downstream drying energy;
- electricity and water;
- elapsed cycle time;
- noise;
- repairability and repeat-failure burden;
- total cost per accepted garment-service batch.

Do not collapse these dimensions into one weighted score. Safety and minimum functional quality are gates; remaining dimensions form an interpretable Pareto frontier.

Potential paradigm shift:
**Optimize for garments returned to a clean, adequately rinsed, low-damage, usable condition over the appliance's life—not for a low-energy cycle in isolation.**

For long-horizon textile-service research, add a controlled repeated-wash textile durability study with fabric-lot control, blinded image/strength/colour assessment, standardized drying and laundering conditions. Do not claim the standard covers full garment lifetime unless the test procedure specifically does so. Separate extrapolation from measured repeated-cycle outcomes.

## 8. Admission decision

Current decision:
- Bosch WAN282X1ZA: **priority procurement enquiry**, based on exact ZA manufacturer listing and a detailed local datasheet.
- Miele WWA120 WCS: **priority documentation/physical candidate**, subject to local availability and exact market identity.
- LG F1222TD5: **secondary candidate**, useful local manufacturer source but current stock/age and the programme/test definition behind public energy/water figures need confirmation.
- Samsung WW80T554DAX/S3: **not admitted for South African procurement** because its official regional page currently says “Not for sale”.

No model is yet selected as the control. Do not buy a three-model set before confirming market availability, exact revision and protocol feasibility.

## 9. Research source register

- IEC 60456:2024 Edition 6: https://webstore.iec.ch/en/publication/70049
- IEC 60704-2-4:2025 Edition 4: https://webstore.iec.ch/en/publication/90407
- IEC 60335-2-7:2024 Edition 9: https://webstore.iec.ch/en/publication/80076
- SABS Appliances Laboratory: https://www.sabs.co.za/appliances
- SANAS accredited-facilities database: https://www.sanas.co.za/Pages/website/accreditation/load/latest-accredited-facilities.aspx
- DMRE/SABS capacity-to-test document: https://www.dmre.gov.za/Portals/0/Energy_Website/EEE/Projects/EnergyEfficiencyStandardsAndLabellingForHouseholdAppliances/Capacity-to-test-for-newEnergyEfficiencyStandards-SABS-Lab.pdf
- Bosch WAN282X1ZA product and specification sheet: sources in Candidate A.
- Miele WWA120 WCS South African product sheet: source in Candidate B.
- LG F1222TD5 South African product page: source in Candidate C.
- Samsung WW80T554DAX/S3 regional product page: source in watchlist.

