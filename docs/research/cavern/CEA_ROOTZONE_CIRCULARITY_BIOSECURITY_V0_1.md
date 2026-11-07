# CEA root-zone circularity and biosecurity benchmark v0.1

**Parent:** [CEA-ROOTZONE-001 #4099](https://github.com/Luminous-Dynamics/symtropy/issues/4099)  
**Composes:** [HYDROPONICS-ACCOUNTING-002 #2277](https://github.com/Luminous-Dynamics/symtropy/issues/2277) for element balances; [WATER-PROC-CLOSURE-001 #2152](https://github.com/Luminous-Dynamics/symtropy/issues/2152) for treatment/residual closure; [MYC-FOOD-001 #5001](https://github.com/Luminous-Dynamics/mycelix/issues/5001) for product lots, handling evidence and recalls; plus [#1649 NUTRI-PORTFOLIO-001](https://github.com/Luminous-Dynamics/symtropy/issues/1649) for accepted food-service portfolio semantics, and #3952, #3957, #4002, #4041/#4057, #4060, #4065 and #4074.

**Status:** source-derived evidence register and benchmark protocol. It is not a crop recipe, nutrient-dosing engine, microbial assay, pathogen-handling method, food-safety certification or water-treatment operating procedure.

## 1. Research basis and state-of-the-art comparator register

The machine-readable comparator registry is `cea_rootzone_research_registry_2026.v0_1.json`; its schema is `cea_rootzone_research_registry.schema.json`.

Eight external comparator families are included:

1. **Langenfeld & Bugbee 2026 zero-discharge lettuce mass balance.** Three long-running deep-water-culture studies report >90% recovery for most measured elements and 25 cycles / 707 days in one study without solution replacement. This is the strongest longitudinal nutrient-accounting comparator; it depends on tissue nutrient analysis and the growth/transpiration basis and is not a multi-facility vertical-farm replication.
2. **ISE-DSS closed-loop cucumber 2025.** Daily Na/K/Ca/nitrate sensing and adaptive fertilizer composition address sodium accumulation from 2.5 mM source water. Reported gains in water productivity and N agronomic efficiency are crop- and system-specific.
3. **Ion-equivalent ratio lettuce 2025.** ISE monitoring and composition-aware dosing outperform an EC-only control on ion-equivalent ratio accuracy in a greenhouse trial. Preserve sensor RMSE and the limited ion panel.
4. **Kenderdine & Raudales 2026 root-zone microbiome.** Reused solution shifted microbial communities across five lettuce cycles and the study investigated Pythium / plant defense. This shows microbiome dynamics are not automatically synonymous with a foodborne hazard.
5. **Dhulappanavar et al. 2026 pathogen persistence.** Salmonella Javiana and Listeria monocytogenes persisted in solution, roots and rockwool in a 21-day recirculating DWC study; edible leaves were negative in that experiment. This is a controlled pathogen-persistence study, not a facility prevalence estimate.
6. **Ayala-San Nicolás et al. 2026 commercial facility monitoring.** Two of 156 samples were L. monocytogenes-positive during the first of three monitoring events at one NFT pak-choi facility: boot covers and drainage nutrient solution. Later events after cleaning/sanitation had no detection; sampling scope still matters.
7. **Kenga et al. 2026 nutrient recovery systematic review.** Summarizes nutrient-rich hydroponic effluents and physical, chemical, biochemical and biological recovery technologies. Recovery product purity and contaminant separation are not optional.
8. **OptiDose 2026.** A five-scenario nutrient-dosing simulator reports large resource-efficiency gains. It is a simulation baseline, not a measured facility saving.

Keep measured greenhouse experiments, commercial environmental monitoring, pathogen-persistence experiments, systematic reviews and simulator scenarios as distinct evidence classes.

## 2. Paradigm shifts

### A. From EC setpoint to element-aware balance

EC is an aggregate conductivity measurement, not a vector of individual nutrient concentrations. Multiple ion mixtures can produce similar EC. Nutrients are not taken up in equal ratios; some are actively absorbed, some passively absorbed or accumulate, and source water can introduce unwanted sodium or other ions.

Compare:
- EC/pH-only conventional management;
- crop- and tissue-informed mass-balance refill;
- ISE / laboratory ion measurement with composition-aware dosing;
- hybrid mass-balance prediction plus targeted assays when uncertainty or residuals grow.

Do not assume the ISE method always wins: sensor selectivity, interference, drift, calibration and sampling cost matter. Do not assume the no-ion-sensor mass-balance approach generalizes across species, substrates, CO2, VPD or source water.

### B. From water recirculation to loop-aware containment

A hydraulic network is also an exposure network. A contaminant or plant pathogen entering a shared return path may affect multiple crop zones. The model must preserve reservoirs, supply branches, return lines, drains, filter/treatment boundaries, sampling locations, isolation valves and batch/crop identity.

No minewater, MTES water, condensate, potable water or wastewater transfer is implied by a generic recycled-water label. Each domain has different chemistry, pathogen, legal and intended-use boundaries.

### C. From sterility to selective biological stewardship

A recirculating rhizosphere is not biologically inert. Some microbial communities may contribute to plant defense or disease suppression; defined plant pathogens and human foodborne pathogens require different evidence and controls.

The system must not:
- label every microbial change as contamination;
- treat a plant pathogen signal as proof of a human-foodborne pathogen;
- equate low total bacterial load with food safety;
- use broad-spectrum sanitization as a default model assumption;
- infer food safety from EC, pH, turbidity, ORP or a model risk score.

Instead it preserves organism/indicator identity, assay method, viability status, matrix, detection limit, time and spatial coverage. Responsible food-safety authorities adjudicate product release.

### D. From zero discharge to verified circularity with safe exceptions

Zero discharge is a resource-management hypothesis, not an absolute rule. Cleaning residues, contaminated solution, filter solids, assay samples, nutrient imbalance and product holds may require removal or treatment. A safety purge can be preferable to a nominally circular but unsafe loop.

Report water/nutrient make-up, plant uptake and harvest removal, element-specific stock and residual concentration, recovered mass and product purity, filter/precipitate/sludge residues, purge/discharge/leak/spill/sample exports, treatment energy and chemicals, contamination isolation and affected-batch disposition.

## 2A. Treatment and product-lot interfaces

The root-zone benchmark composes two existing owners instead of creating duplicate subsystems:

- **#2152 WATER-PROC-CLOSURE-001** owns treatment-train water/material/utility/residual closure. A treatment result supplied to this benchmark must include feed, recovered product, purge, backwash, concentrate, sludge/filter residues, cleaning waste and utility streams. #4099 separately judges whether a recovered product is admissible for the declared crop-water loop, based on source chemistry, treatment quality, microbial evidence and intended-use authority.
- **Mycelix #5001 MYC-FOOD-001** owns ProductProfile, Lot, HandlingEvidence, Fulfillment and recall lineage. Symtropy supplies bounded root-zone / crop-cycle / environmental-sample / isolation event references; Mycelix links those to harvested lots and fulfilled orders while preserving privacy. Do not create a second lot/recall graph here.

A sample result should be scoped to a facility zone, water path, crop cycle, lot/time interval and assay coverage. A positive result may cause a bounded hold/review set based on the evidenced path; it must not globally mark every lot unsafe without a supported exposure path. A negative result cannot clear lots outside its matrix, detection limit and sampling coverage. Symtropy does not adjudicate regulatory food release.

## 3. Canonical root-zone loop contract

Represent each independent water domain and transfer explicitly:
- source water;
- fertilizer stock and dosing;
- solution mixing/reservoir;
- crop root-zone supply;
- drainage/return;
- treatment / recovery;
- condensate;
- potable/domestic;
- minewater / MTES;
- cleaning/sanitation;
- waste discharge/off-site processing.

For every element record exact chemical species / unit basis, source composition, stock solution, additions, inventory, crop uptake, precipitation/adsorption, purge/export and measurement uncertainty. Unknown composition remains unknown.

For microbial evidence record organism / indicator / assay target, matrix, sample site, crop zone and batch, timebase, custody, detection/quantification limit, enrichment/culture/PCR/metagenomic method, viability status, controls and result state. NOT_DETECTED is bounded by assay and sampling coverage, not a universal zero.

## 4. V0.1 benchmark cases

The parent issue contains 44 proposed benchmark cases. The v0.1 machine-readable adversarial fixture now freezes 18 synthetic known-truth cases, including bounded exposure-path, disconnected-zone, chain-of-custody and insufficient-sensitivity controls. The broader planned evaluator corpus should include:
- exact water and per-element mass-balance fixtures;
- nitrate-N / nitrate-ion and elemental/compound unit mismatches;
- equal-EC different-ion mixtures;
- source-water sodium / salinity drift;
- active versus passive uptake, precipitation and adsorption;
- missing tissue or source-water analysis;
- ISE drift/cross-sensitivity and lab assay disagreement;
- zero-discharge vs necessary safety purge;
- high water recovery but unsuitable recovered-product quality;
- reservoir-negative / drain-positive and source-water-positive / treatment-unknown patterns;
- boot-cover/surface positive with crop negative;
- root-zone plant-pathogen evidence distinct from human foodborne hazard;
- PCR positive with unresolved viability;
- single negative sample with inadequate spatial or temporal coverage;
- sanitation record without independent before/after assay;
- one shared return-line contamination affects multiple zones;
- isolation of one zone with unaffected zones continuing only where evidence permits;
- crop harvest meets target but batch remains on hold pending safety adjudication;
- minewater/MTES cross-connection attempt;
- recovered nutrient product with metals/salinity/microbial quality unresolved;
- sensor common-mode or timebase error;
- outage fallback and safe isolation;
- false zero-discharge claim that omits sludge, filter residues, cleaning waste or batch disposal.

No benchmark requires handling or culturing live pathogens. Use published datasets, synthetic fixtures, qualified non-pathogenic surrogates where properly authorized, and accredited laboratory results admitted through exact protocols.

## 5. Metrics and comparison

### Resource circularity
- water recovery fraction, make-up water, discharge and purge by source/quality;
- elemental fertilizer input, plant uptake, remaining stock and output/discharge;
- fertilizer efficiency per accepted dry edible mass and nutritional service;
- recovery product purity and safe reuse status;
- mass-balance closure error and uncertainty per analyte.

### Crop service
- fresh and dry biomass, accepted edible mass, nutritional quality;
- deficiency/toxicity/physiological disorder and plant disease;
- crop-cycle repeatability, cultivar transfer and harvest rejection.

### Biological safety / monitoring
- detection coverage by matrix, site, zone, cycle and time;
- assay detection limit, false-negative uncertainty and turnaround;
- alert-to-isolation / hold time;
- cross-zone exposure in topology fixtures;
- independent post-cleaning/sanitation evidence;
- false-clear rate on frozen negative controls (target zero).

### Resource / operations
- pumping, mixing, treatment, sensing and disinfection electricity separately;
- treatment chemicals, residues, water loss and maintenance;
- time under isolation, lost/held crop batches, continuity of unaffected zones;
- labor and assay/cleaning cost.

Do not collapse yield, water, nutrients, carbon, microbe and food-safety into one score. Hard batch holds cannot be offset by a better water-use efficiency metric.

## 6. Baselines and SOTA strategy

Under matched crop, system type, water source and measurement boundaries compare:
- EC/pH-only plus periodic dump-and-refill;
- EC/pH plus source-water analysis;
- element-specific ISE dosing;
- lab-assay-guided composition control;
- tissue/transpiration mass-balance refill;
- hybrid balance plus trigger-based assay;
- open/once-through reference where appropriate;
- loop-segmented and shared-loop topology under synthetic contamination;
- existing facility's documented environmental-monitoring plan.

To improve on SOTA, do not merely chase the highest percentage of reused water. Demonstrate more accepted edible dry/nutritional output per total water and nutrient input, while preserving balance closure, crop quality and biosecurity. Include source-water salinity and long-duration crop-cycle drift. Match experimental units and independent replication.

## 7. Proposed advancement targets

After source data and physical measurement boundaries are admitted:
- exact synthetic elemental and water closure within declared numerical tolerance;
- zero false-clear and zero unauthorized water-domain cross-connection in the frozen corpus;
- preserve the 2026 mass-balance paper's >90% element recovery only on matched crop/system boundaries, with complete uncertainty and no safety regression;
- improve on EC-only control for measured elemental balance without exceeding assay/sensor cost envelopes;
- no degradation in marketable yield, dry matter, nutritional quality or disease incidence when reducing nutrient discharge;
- isolate an affected loop/batch without contaminating unaffected domains;
- report a qualified safety decision with relevant laboratory result, sample coverage and responsible authority, or return HUMAN_REVIEW / INDETERMINATE.

These are proposed engineering targets, not achieved results.

## 8. Claim ceiling

The registry is source evidence only. A nutrient balance is not food safety. A negative sample is not facility clearance. A sanitation record is not a verified treatment outcome. This package does not certify produce, approve reclaimed water, define pathogen handling or authorize discharge. Any microbiological intervention or food-release decision remains with qualified professionals and responsible authorities.


Accepted-food interface: #1649 consumes only qualified edible-food / lot-level output. Keep harvested biomass, edible/marketable mass, held/rejected batches, released quantities, fulfilled quantities and recall corrections distinct. A root-zone nutrient or microbial evidence record is an input to responsible adjudication, not a Symtropy food-release decision.



## South African food-hygiene and hydroponic-water evidence baseline

These references add a jurisdiction-specific evidence layer. They are **not** a substitute for a site-specific legal review, local-authority determination or accredited laboratory advice.

### Codex CXC 53-2003 — directly covers hydroponic fresh-produce systems

The Codex *Code of Hygienic Practice for Fresh Fruits and Vegetables* (CXC 53-2003; revised 2017) explicitly covers produce grown in protected facilities, including hydroponic systems:
- Official Codex code list: https://www.fao.org/fao-who-codexalimentarius/codex-texts/codes-of-practice/en/
- South African Agriculture / DALRRD hosted copy: https://www.nda.gov.za/images/Branches/AgricProducHealthFoodSafety/food-safety-and-quality-assurance/food-safety/audit-policy/cxc-codes-of-hygienic-practices-for-fresh-fruits-and-vegetables.pdf

Relevant technical guidance:
- Environmental hygiene should review present and previous site uses and adjoining land. The code explicitly lists industry, sewage-treatment and hazardous-waste sites, and names mining extraction as a possible chemical/environmental contamination source. Unknown previous use or identified hazards may warrant contaminant analysis; serious risks can mean a site should not be used for fresh-produce production.
- Hydroponic nutrient solution may enhance survival or growth of pathogens compared with soil irrigation water. Reused hydroponic water should be treated to minimize microbial and chemical contamination; delivery systems should be maintained and cleaned.
- Non-potable systems should be separately identified and should not connect to, or allow reflux into, potable systems.
- If irrigation source water is known or likely to contain human pathogens and connector failures can overspray edible crops or cause localized flooding, a no-harvest zone should be considered.

This is international code-of-practice guidance, not automatically a South African statutory clause. It is an appropriate technical comparator for the underground CEA hazard / sampling design.

### South Africa — R638 of 2018 food premises, handling and transport

The South African Department of Health's *Regulations Governing General Hygiene Requirements for Food Premises, the Transport of Food and Related Matters*, R638 (22 June 2018), is published in Government Gazette 41730:
- Official gazette PDF: https://www.gov.za/sites/default/files/gcis_document/201806/41730gon638.pdf
- Official Government page: https://www.gov.za/documents/notices/foodstuffs-cosmetics-and-disinfectants-act-regulations-general-hygiene-0

R638 requires a valid Certificate of Acceptability for food handling on food premises within its scope, subject to the regulation's exceptions and specific-food rules. The certificate follows local-authority application and inspection, is not transferable between persons or premises, and is valid for the nature of handling specified in the application/certificate. Changes to food-premises conditions governed by regulations 5/6 require advance written notification to the local authority. A certificate is not blanket approval under every other law.

The regulation defines statutory "water" as potable water complying with SANS 241 for domestic supplies. Do not confuse that definition with every agricultural nutrient-solution use: Codex distinguishes source/irrigation, hydroponic nutrient solution, clean water and potable water according to intended use. A potable-water test does not establish a nutrient-loop pathogen outcome, and a root-zone EC or microbial test does not establish potable-water suitability.

**Applicability gate:** determine with the responsible local authority and qualified South African food-safety professionals whether a given grow room, harvest/packing area, storage area, transport vehicle or co-located facility is a food premise / handling activity under R638, which Certificate of Acceptability and handling categories apply, and what additional product-specific rules apply. Do not make this determination from a software field alone.

### South Africa — R692 microbiological standards

The Department of Health's R692 of 16 May 1997, as amended through notices listed in its official consolidated copy, governs specified microbiological standards and methods for the foodstuffs covered by its Schedule:
- Department of Health copy: https://www.health.gov.za/wp-content/uploads/2024/10/Regulations-governing-microbiological-standards-for-foodstuffs-and-related-matters-R692-of-1997.pdf
- Department of Health current food-control regulation index: https://www.health.gov.za/food-control-regulations/

The regulation links analyses for covered foodstuffs to the methods in Annex A and contains product-specific standards. It is **not** a universal microbiological limit for every raw leafy vegetable, hydroponic nutrient solution, irrigation-water stream, floor drain or environmental swab. Before applying a numerical specification, the responsible specialist must confirm that the exact product, intended use, sample matrix and applicable current amendment are covered.

### Benchmark consequences

Add explicit evidence fields and controls for:
- jurisdiction, instrument_id, revision/effective_date, applicability_status, responsible_authority_ref and review_date;
- exact premises, responsible person, food-handling activity and Certificate of Acceptability scope where applicable;
- site-use history including mine extraction, hazardous waste, sewage, industrial operations and adjacent land;
- separate water classes: potable/domestic, clean process water, crop-source water, hydroponic nutrient solution, condensate, MTES/minewater and post-harvest wash/rinse water;
- prevention of physical cross-connection or backflow between non-potable/process and potable systems;
- food product category and intended use linked to the actual microbiological specification / analytical method selected by a qualified authority.

Do not encode a regulation summary as a self-executing food-safety approval. Store the applicable source and human/authority decision as evidence; when applicability or product scope is unresolved, return HUMAN_REVIEW / INDETERMINATE.
