# Enzyme-Enabled Textile Care: Wool Anti-Felting and Stain-Specific Treatment v0.1

**Status:** experimental benchmark contract; no formulation, garment or appliance has been physically tested by this programme.  
**Tracker:** [ENZYME-CARE-001 #5569](https://github.com/Luminous-Dynamics/symtropy/issues/5569).  
**Composes with:** laundry service #5471, morphology #5518, professional care #5519, surface chemistry #5527, material passport #5549, garment-ready #5561 and production episodes #5581.

## 1. Paradigm shift: engineer the care envelope, not just the wash cycle

There are two independent propositions:

1. **Manufacturing / durable finishing:** selectively modify wool fibre scales to reduce felting, so specifically qualified dyed wool garments may tolerate a defined aqueous-care envelope.
2. **In-use stain treatment:** select an enzyme class for a specific soil/fibre pair (protein/protease, lipid/lipase, starch/amylase), then use a bounded pretreatment or detergent process only within a validated textile/material envelope.

These are not interchangeable. A shrink-resist finish applied during textile manufacture is not an enzyme detergent. A blood-stain experiment on polyester/cotton does not establish safe treatment of wool, broad stain removal, low-temperature performance or a replacement for professional care.

The new abstraction is a **treatment-and-care passport**: named enzyme preparation, auxiliary chemistry, treatment conditions, textile/garment lot, residuals, validated wash/dry envelope, repeat-cycle durability and evidence class. A care instruction becomes a versioned, evidence-backed material contract—not a generic "machine washable" flag.

## 2. External research results: keep each pathway distinct

### A. Polymer-assisted protease anti-felting (PCDI pre-fill)

Zhang et al. describe a “valley-filling and peak-shaving” approach: a polycarbodiimide (PCDI) pre-fill is used to restrict protease penetration into the fibre interior while favouring surface-scale hydrolysis by compound proteases. The study reports 5.06% felting shrinkage and a 0.27% tensile-strength increase for its tested wool fabric, with original mechanical/colour properties described as preserved.

Reference: https://doi.org/10.1016/j.susmat.2025.e01780

**Classification:** enzyme + polymer auxiliary, not enzyme-only. PCDI identity, residuals, migration, skin contact, wastewater fate, process cost and end-of-life must be qualified independently. The paper's swatch result is not proof of garment-level repeated machine washability.

### B. Protease-only anti-felting (surface-selective hydrolysis)

Zhang et al. report a “wave-riding” dyed-wool anti-felting strategy using compound proteases. Their abstract reports 3.90% ±0.12% dimensional/felting change, 11.39% ±0.61% controlled strength loss, improved air/moisture permeability and hand feel, and colour difference ΔE_CMC below 1.00 under their test conditions.

Reference: https://doi.org/10.1039/D5GC01614B

**Classification:** enzyme-only pathway in the reported treatment. The strength loss is material and must remain visible; the colour result does not erase strength, abrasion, handle or long-cycle uncertainty. A one-time swatch result is not durability qualification.

### C. Protease + reductant anti-felting

Li et al. describe sodium thioglycolate (STG) or glycerol monomercaptoacetate (GMT) pre-reduction with protease, to weaken disulfide bonds in scales before scale hydrolysis. Their study reports less than 4% felting shrinkage with 5 g/L thioglycolate compound and 6 U/mL protease, and describes minimal strength loss / no colour change in its dyed-wool tests.

Reference: https://doi.org/10.1016/j.ijbiomac.2025.141698

**Classification:** enzyme + reducing auxiliary, not enzyme-only. Reductant residues, worker exposure, effluent, chemical cost and treatment repeatability require explicit assessment.

### D. Protease + sodium bisulfite pre-reduction

A separate dyed-wool study reports a two-bath approach using 5 g/L sodium bisulfite and 8 U/mL Savinase Ultra 16XL protease; the abstract reports 4.14% felting shrinkage, 6.58% strength loss and colour difference below 1.1.

Reference: https://doi.org/10.1016/j.ijbiomac.2025.142160 (PubMed PMID 40101820; the same paper is the bisulfite-assisted, two-bath comparator).

**Classification:** enzyme + reducing auxiliary, distinct from the thioglycolate and PCDI processes. Do not pool auxiliary-assisted treatments under “enzyme-only”.

### E. Protease + lipase + detergent for blood stains

Khazi et al. investigate protease/lipase/detergent treatments for blood staining on polyester and cotton. The abstract reports an optimum condition of pH 7.0, enzyme concentration 1 mg/mL and 60°C, with ΔE values of 58.42 ±0.5 (polyester) and 6.60 ±0.5 (cotton) for the combined treatment.

Reference: https://doi.org/10.1016/j.procbio.2025.05.009

This is a substrate- and protocol-specific blood-stain result, not a universal ΔE target, a low-temperature result, a wool result, or a proof of lower whole-cycle resource burden. ΔE alone can be ambiguous: pre-register its reference surface, sign/direction, spectral measurement, stain-free baseline and visual acceptance. Use reflectance/K/S or an established stain-removal index alongside ΔE and blinded visual grading.

### F. Formulation stability and industrial reality

A September 2026 review emphasizes that detergent enzymes face formulation and storage challenges including oxidative stability, surfactant compatibility and storage stability.

Reference: https://doi.org/10.1016/j.nexres.2026.102060

The EU JRC Textiles BREF explains that industrial enzymatic wool anti-felting has often been a subsequent step after chemical or physical pretreatment. It notes that combining enzymes with DCCA may reduce chlorine-agent usage but does not eliminate AOX release, and highlights durability, continuous processing, wash-fastness and operational cost as obstacles to replacement of conventional shrink-resist finishing.

Reference: https://bureau-industrial-transformation.jrc.ec.europa.eu/sites/default/files/2023-01/TXT_BREF_2023_for_publishing%20ISSN%201831-9424_final_1_revised.pdf

This is why “enzyme-based” is not automatically synonymous with enzyme-only, low-impact, residue-free or production-ready.

## 3. Two separately qualified tracks

### Track A — dyed-wool anti-felting finishing

Controls must distinguish:
1. untreated wool;
2. mechanical/chemical sham without enzyme;
3. enzyme-only surface treatment;
4. PCDI pre-fill + protease;
5. thioglycolate/GMT + protease;
6. bisulfite + protease;
7. a qualified commercial shrink-resist reference, only with full process / waste characterization.

Test declared fibre origin/type, merino/other wool classes, yarn/fabric construction, mass, knit/weave, dye class, dark/light shade, finishing, coating and prior treatment. Do not pool all wool or all dyed fabrics as one homogeneous class.

Required measurements:
- dimensional change in warp/weft/area and felting shrinkage separately;
- tensile, tear, burst, abrasion, pilling/fuzz, fibre breakage and surface-scale morphology;
- colour change, wash fastness, dye transfer/bleeding and staining;
- air permeability, water-vapour transmission, wettability, hand feel, drape and elasticity;
- enzyme/protein residual and auxiliary chemical residual/migration;
- wash-water COD/TOC, nitrogen load, ecotoxicity and AOX where relevant;
- total treatment energy, water, enzyme, auxiliary chemical, labour, batch time and yield;
- repeated wash/dry durability at 1, 5, 10, 25 and 50 cycles;
- final garment fit/appearance and user acceptance, not only fabric swatches.

Use standardized domestic wash/dry sequences and record the precise machine type, programme, detergent, ballast, drying method and cycle. ISO 6330:2021 defines domestic washing/drying test procedures; ISO 5077:2007 defines dimensional-change measurement. ISO 105-C06:2010 is a colour-fastness-to-laundering method, not a stain-removal metric.

References:
- ISO 6330:2021: https://www.iso.org/standard/75934.html
- ISO 5077:2007: https://www.iso.org/standard/41877.html
- ISO 105-C06:2010: https://www.iso.org/standard/51276.html

Use garment-level testing after swatch screening. A textile is not machine-washable merely because one treated swatch shrinks less in a single wash.

### Track B — stain-specific enzyme care

Stratify stains by chemistry and age:
- protein: blood, egg, milk/food protein;
- lipid: sebum, oil, grease;
- starch: sauces, food starch;
- tannin/colourant: tea, coffee, wine/plant dyes;
- pigment/ink/particulate;
- mixed/aged stains.

Match enzyme class to the target soil:
- protease for protein-rich soils;
- lipase for lipids;
- amylase for starch;
- cellulase only in a separate, highly guarded textile-surface/pilling/strength experiment (not as a general stain remover).

Controls:
1. water-only;
2. standard detergent at matched dose;
3. enzyme-only;
4. detergent + one enzyme class;
5. detergent + protease/lipase/amylase cocktail where justified;
6. inactive-enzyme control with matched carrier / pH / heat history;
7. bounded localized pretreatment vs full-cycle process.

Match substrate, dye/print, fibre blend, stain recipe/loading, age, humidity, dose, pH, temperature, dwell, agitation, rinse and drying. Include cotton, polyester, polyester/cotton, wool, elastane-containing and coated/printed strata with explicit exclusions.

Measures:
- stain-removal index using declared stain reference and spectrophotometric reflectance;
- ΔE with explicit unsoiled/stained reference and direction;
- blinded visual panel with inter-rater reliability;
- residual protein/lipid/enzymatic activity on fabric;
- colour fastness, dye transfer, fabric strength, dimensional change and pilling;
- water, energy, detergent and enzyme per accepted garment;
- rinse demand, effluent COD/TOC and ecotoxicity;
- enzyme stability, activity decay and storage life for formulations.

The 60°C blood-stain paper is a reproduction anchor, not a low-temperature claim. Explore temperature/time surfaces only after replicating its declared protocol and establishing control variance.

## 4. Benchmark design: do not let one score hide a damaged garment

### Proposed target hypotheses (not results)

| Track | First hypothesis | Stretch hypothesis |
|---|---:|---:|
| Wool anti-felting area shrinkage after 10 washes | <=3% | <=2% |
| Wool tensile-strength loss after 10 washes | <=5% | <=3% |
| Dyed-wool colour difference | ΔE_CMC <=1.0 | <=0.5 |
| Treatment performance retention at 25/50 washes | no breach of preregistered non-inferiority margins | stable through 50 cycles |
| Stain removal on declared target soils | non-inferior to matched detergent control | superior with lower whole-process burden |
| Residual enzyme/auxiliary on garment | below independently justified exposure / analytical limit | lower with maintained efficacy |
| Effluent and resource burden | measured for every arm | lower than matched baseline without quality loss |

These are ambitious hypotheses. Final numeric gates must be set after pilot variance, method precision, textile type and reference-control distributions are known. The 3%/5%/ΔE targets must not be promoted to claims because published study means appear close.

### Statistical design

- Randomize treatment allocation within matched fibre/dye/construction/stain blocks.
- Blind appearance/stain grading where practicable.
- Choose sample size after pilot variance and a pre-registered non-inferiority/superiority margin.
- Use independent lots and garment specimens for held-out confirmation.
- Repeated measurements on one specimen are not independent specimens.
- Report mean, median, SD, confidence interval, failure rate, worst stratum and raw sample counts.
- Use one primary endpoint per study; treat exploratory endpoints as exploratory and correct for multiple comparisons where appropriate.
- Publish all failed/invalid samples and exclusions with reasons.
- Keep external study results in separate reference records, never pooled as our data.

## 5. Safety, worker exposure and environmental gates

Enzymes are proteins that can cause occupational asthma/sensitization when exposure occurs. UK HSE lists alpha-amylases and subtilisins among substances that can cause occupational asthma, including use in detergents and textile processing.

Reference: https://www.hse.gov.uk/asthma/substances.htm

A detergent-worker cross-sectional study found enzyme sensitization and respiratory allergy risks associated with exposure, including spills/leakage during weighing, transport and filling.

Reference: https://doi.org/10.1136/oem.2008.045245

Requirements:
- qualified lab / industrial partner and supplier-identifiable enzyme preparations;
- no open powder handling, spray, nebulization or uncontrolled aerosol generation;
- closed, low-dust dosing and transfer, containment, spill control and exposure assessment;
- current SDS, PPE, occupational hygiene and incident response;
- residual enzyme/protein measurement and validated enzyme inactivation/rinse;
- separate risk assessment for PCDI, thioglycolates, bisulfite, preservatives and formulation auxiliaries;
- wastewater COD/TOC, ecotoxicity and chemical fate; AOX for chlorine-containing comparator routes;
- no live microbial enzyme production in an appliance or home; no uncontrolled enzyme fermentation;
- no claims that an enzyme process is green solely because it contains enzymes.

Symthaea remains advisory-only; physical interlocks and chemical/thermal controls are independent deterministic systems.

## 6. Integration and evidence ownership

- Symtropy: experiment contracts, process windows, compatibility, uncertainty and Pareto assessment.
- Symthaea: advisory stain/material hypothesis ranking and next-test selection; abstain on uncertainty.
- Mycelix: provenance of wool lot, dye, enzyme preparation, auxiliary batch, treatment and garment service history.
- Sol-Atlas: lifecycle assessment against chlorination/Hercosett, wet-care and professional cleaning using qualified data.
- Material passport #5549: current fibre/dye/finish and treatment evidence classes.
- Garment-ready #5561: final appearance, fit, moisture and ready-state acceptance.
- Garment-ops #5581: production process episodes, operator interventions, waste/resource and traceability.

**Claim ceiling:** literature-backed experimental contract only. No enzyme formulation, treatment, wool garment, consumer appliance, stain-removal process or dry-cleaning replacement has been physically qualified by Luminous Dynamics.
