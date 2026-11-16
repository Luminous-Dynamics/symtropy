# Garment-Care Platform Morphology and Process-Routing Study v0.1

**Status:** research architecture; no equipment built, universal process selected, or physical result claimed.  
**Trackers:** [LAUNDRY-CARE-MORPH-001 #5518](https://github.com/Luminous-Dynamics/symtropy/issues/5518), [PRO-CARE-AUTO-001 #5519](https://github.com/Luminous-Dynamics/symtropy/issues/5519), [LAUNDRY-SERVICE-001 #5471](https://github.com/Luminous-Dynamics/symtropy/issues/5471).  
**Composes with:** [MNB #5489](https://github.com/Luminous-Dynamics/symtropy/issues/5489), [microfiber mass balance #5495](https://github.com/Luminous-Dynamics/symtropy/issues/5495), [textile disassembly #4941](https://github.com/Luminous-Dynamics/symtropy/issues/4941).

## 1. Design thesis

Build a **garment-care platform**, not a universal machine. The service objective is to return garments with accepted soil/stain removal for their route, acceptable residue/odour/finish, preserved dimensions/colour/strength, and traceable resource/waste accounting.

Keep operations distinct:
- **Wash:** aqueous soil removal with water, chemistry and mechanical action.
- **Professional wet cleaning:** controlled aqueous cleaning with gentler mechanics and specialist finishing.
- **Dry cleaning:** non-aqueous solvent cleaning in process-specific equipment.
- **Spot treatment:** localized stain treatment; one clean spot does not prove whole-garment cleaning.
- **Refresh:** odour/wrinkle/dust management by air, steam, heat or movement; not automatically soil removal or dry cleaning.
- **Finishing:** shaping, tensioning, steam, pressing and controlled drying.

The paradigm shift is **route selection from evidence-qualified garment properties**, followed by an appropriate specialized process.

## 2. Industry position

**Professional wet cleaning is already commercial.** Electrolux Professional Lagoon Advanced Care advertises a 55-minute dry-to-dry route and delicate garment, leather and shoe applications. These are manufacturer claims that require exact-configuration and independently matched verification. Wet cleaning is a serious baseline, not an empty invention space.  
https://www.electroluxprofessional.com/gb/commercial-laundry-equipment/wet-cleaning/

**Liquid CO2 cleaning is demonstrated but specialist.** TERSUS describes a closed-loop recycled liquid-CO2 platform. A California Energy Commission / U.S. Navy demonstration reported 76% higher operational efficiency, 15% utility savings and 117.5 gallons less water per 150 lb for a specific military-textile application. These results do not generalize to retail garments. CO2 cleaning is a high-pressure process for professionally engineered facilities, not household prototypes.  
https://www.tersussolutions.com/technology  
https://www.energy.ca.gov/publications/2021/carbon-dioxide-based-cleaning-military-textiles

**Solvent alternatives are not automatically benign.** Hydrocarbon processes require combustible-solvent/fire engineering. D5 and dibutoxymethane have textile test methods, but a performance standard is not a safety or environmental endorsement. PCE/perc is a historical comparator only, not a preferred new-machine solvent. U.S. EPA restrictions include a prohibition on PCE in dry-cleaning machines acquired after 16 June 2025 and later staged prohibitions; these are not South African laws.  
EPA: https://www.epa.gov/assessing-and-managing-chemicals-under-tsca/risk-management-perchloroethylene-pce  
ISO 3175-3 hydrocarbon: https://www.iso.org/standard/65155.html  
ISO 3175-5 dibutoxymethane: https://www.iso.org/standard/70812.html  
ISO 3175-6 D5: https://www.iso.org/standard/70813.html

## 3. Form factors

| Form factor | Best role | Opportunity | Main limit |
|---|---|---|---|
| Modular household wash wall | Wash, extraction, heat-pump dry, fibre/water treatment, finishing | Upgradeable serviceable modules and coupled washer/dryer optimization | Footprint, cost and household safety |
| Garment-care cabinet | Refresh, dehumidify, wrinkle relaxation, finishing | Avoid unnecessary full washes where refresh is sufficient | Not a soil-removal or dry-clean replacement |
| Precision spot-care station | Bounded localized stain treatment | Avoid full-garment processing for suitable stains | Stain/fibre/dye uncertainty, extraction and human review |
| Professional wet-clean/finish cell | Delicates and professional garment care | Mature process plus moisture control and tension/steam finishing | Trained workflow and garment-family validation |
| Professional liquid-CO2 cell | Specialist non-aqueous cleaning | Closed-loop recovery for validated applications | High pressure, specialist facilities and cost |
| Dedicated hydrocarbon/solvent cell | Specialist dry cleaning | Established solvent workflows for compatible garments | Fire, exposure, recovery, emissions and hazardous waste |
| Intake/inspection/routing station | Garment identity, care label, stain map, dispatch | Automation and traceability | Composition cannot be inferred from one image or care symbol |
| Textile-care tunnel/service hub | Uniform, hospitality, workwear | Batch scheduling, garment history, finishing | Separate hygiene/PPE and contamination validation |

Deployment:
- **Home:** modular wash wall + refresh cabinet + optional bounded spot station. Keep specialist solvents and high-pressure CO2 out of home appliances.
- **Local professional shop:** automated intake/inspection + wet-clean/finish cell + isolated, media-specific solvent equipment where justified.
- **Central specialist hub:** LCO2 or other dedicated solvent process, full recovery/waste handling and independent pressure/chemical safety.
- **Industrial service:** rail/tunnel handling, batch identity, route scheduling, finishing and garment lifecycle evidence.

Never use one shared fluid circuit that switches between water, LCO2, hydrocarbon, D5 or other solvent families. Their materials, seals, recovery, hazards and waste systems differ.

## 4. Novel invention concepts

1. **Garment-care router:** local garment ID, care label, owner-approved history, fabric/trim uncertainty and stain map produce a conservative route suggestion; unknowns trigger abstention.
2. **Precision stain-care station:** vision proposes a stain region; a bounded head applies approved low-dose chemistry or water/steam; local extraction captures spent fluid; a measurement checks progress.
3. **Refresh-and-finish cabinet:** rail, airflow, heat-pump dehumidification, steam/air wrinkle relaxation, hanger movement and local filtration. Refresh is not cleaning.
4. **Garment-specific tension/steam former:** adjustable torso/sleeve/trouser forms, low-force tension, steam and controlled air drying; protect seams, adhesives and interlinings.
5. **Modular wash wall:** serviceable washer, extractor, heat-pump dryer, greywater/fibre module and finishing station with explicit metering.
6. **Garment-state sensing kit:** local moisture/temperature and bounded optical/odour proxies; no single sensor certifies cleanliness or hygiene.
7. **Microfibre-aware drain/dryer cassette:** low-pressure-drop capture, bypass detection, service-life estimation, mass-balance records and safe emptying.
8. **Automated intake rail:** tag/QR/RFID, privacy-preserving imaging, care-label capture, defect/stain map, route assignment and clean/dirty separation.
9. **Repair-before-replace care station:** inspect seams, buttons, zips, hems, pilling and minor damage; execute only bounded, reversible, user-approved repair. Reuse textile disassembly #4941.
10. **Care-as-a-service hub:** pickup/return lockers, garment identity, professional route dispatch and evidence-backed service history.

Ultrasound, plasma, ozone, UV-C and photocatalysis remain research-only adjuncts. Do not assume they clean, sanitize or preserve garments. Oxidants/irradiation may damage dyes and elastomers and create exposure hazards; each needs efficacy, by-product, exposure and fabric-aging evidence.

## 5. Automated professional-care cell

Workflow: intake → identify → inspect → map stains/defects → estimate material and uncertainty → select supported route → approve or abstain → isolated process cell → rinse/solvent recovery → dry → finish → inspect → return.

Capture garment identity; care-label transcription/confidence; fibre, lining, coating, adhesive, trim and dye estimates; stain class/location/confidence; approved process media and exact machine recipe; operator approvals/interventions; cleaning/rinse/residue/dimensional/colour/finish observations; resource/chemical/solvent/waste records; final inspection/rework.

**Abstain / human inspection** for unknown composition, unstable dye, mixed/bonded materials, leather/fur, embellishments, unverified adhesives, hazardous contamination, PPE/hygiene claims, or unsupported garment/process combinations. Care symbols are not a digital twin. Symthaea may estimate uncertainty but cannot override garment labels, human approval or physical interlocks.

## 6. Benchmarks and methods

Use two functional units:
- **Per garment accepted** for professional cleaning/finishing and garment damage.
- **Per kg accepted dry-equivalent textile** for throughput, energy, water and load economics.

Compare within valid garment/workload envelopes using matched controls, soils, repeat cycles and preregistered margins. Report stain removal by family; soil redeposition; dimensional/colour/tensile/seam change; pilling/finish; lining/trim failure; chemical/solvent residue; energy, water, chemistry, solvent make-up/recovery, waste, emissions, time, throughput; microfiber release/capture; labor, rework, uptime, repair and lifecycle cost; leaks, exposures, pressure excursions, fire/electrical/thermal faults and shutdown.

Method references:
- ISO 3175-1:2017 assessment after professional cleaning: https://www.iso.org/standard/65153.html
- ISO 3175-2:2017 PCE method (historical comparator only): https://www.iso.org/standard/65154.html
- ISO 3175-3:2017 hydrocarbon: https://www.iso.org/standard/65155.html
- ISO 3175-4:2018 simulated wet cleaning: https://www.iso.org/standard/65156.html
- ISO 3175-5:2019 dibutoxymethane: https://www.iso.org/standard/70812.html
- ISO 3175-6:2019 D5: https://www.iso.org/standard/70813.html
- ISO 15797:2017 industrial workwear: https://www.iso.org/standard/65152.html
- ISO 8230-1:2008 commercial PCE/combustible-solvent machine safety: https://www.iso.org/standard/38071.html

These do not qualify LCO2 pressure systems, household appliances, local legal compliance, emissions or worker exposure. Confirm current South African SANS adoptions, NRCS scope and municipal by-laws. South African Hazardous Chemical Agents Regulations require chemical inventory/SDS, competent risk assessment and exposure control; hazardous waste must be classified and disposed of under applicable waste law.

## 7. Luminous integration and priority

- Symtropy: morphology, route contracts, process/resource models and test fixtures.
- Symthaea: advisory route ranking and experiment selection; abstain on uncertainty; no pressure/solvent/heat/motor/safety actuation.
- Mycelix: reuse garment/lot custody, machine/process identity, chemistry batches, service/repair and provenance.
- Sol-Atlas: qualified lifecycle, cost, resource, repair and end-of-life projections.
- Textile attachment removal #4941: reuse disassembly evidence; do not duplicate the recycling ledger.

Priority: freeze home/local-professional/central-hub boundaries; benchmark professional wet cleaning; automate intake/inspection/route abstention/finishing around qualified equipment; evaluate LCO2 with a qualified supplier/lab; consider other solvents only after media-specific safety/environmental/local-law review; add MNB/spot-care only when matched-outcome evidence demonstrates benefit.

No pressure vessel, solvent loop or oxidant-generating prototype is authorized by this architecture. No universal dry-clean replacement, cleaning superiority, garment suitability, safety, environmental or regulatory claim is made.