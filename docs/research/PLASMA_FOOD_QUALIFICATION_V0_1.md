# Plasma Food Qualification V0.1

**Status:** research design / not physically qualified  
**Issue:** [PLASMA-FOOD-QUAL-001 #5658](https://github.com/Luminous-Dynamics/symtropy/issues/5658)  
**Parent:** [KITCHEN-WORKCELL-001 #5618](https://github.com/Luminous-Dynamics/symtropy/issues/5618)  
**Branch:** research/plasma-food-qualification-v0-1-20261116  
**Revision:** 2026-11-16

This document is a research and qualification contract. It does not authorize high-voltage construction, direct food treatment, a food-safety claim, or a thermal-plasma appliance prototype.

## Executive decision

Plasma merits a bounded research programme, but the three technology families must not be conflated:

| Family | What it can do | Initial decision |
|---|---|---|
| Thermal plasma | Transfer intense heat to a surface or process stream | Discovery comparator only; no household prototype |
| Cold atmospheric plasma (CAP) | Generate reactive species for surface, enzyme and food-material modification | Research candidate; matrix-specific qualification required |
| Plasma-activated media / in-package plasma | Treat a liquid or package headspace to deliver reactive chemistry | Research candidate; qualify chemistry, food-contact, package and exposure together |

The strongest first experiment is **upstream conditioning of brown rice**, not a plasma burner. Published low-pressure cold-plasma research reported cooking time decreasing from 29.1 to 21.1 minutes after a 5–10 minute treatment, alongside increased water uptake and altered texture. Separate atmospheric-plasma research on brown rice reported altered hydration / hardness and a microbial-count reduction under its own test conditions. These are different reactor configurations and different claims. Reproduce the process and count every minute and watt-hour before adopting either result as a product target.

## 1. Product hypotheses

### H1 — plasma-conditioned quick-cook ingredients

Question: can an upstream process modify a declared grain or pulse so that it cooks faster in ordinary household equipment, while preserving acceptable texture, flavor, nutrition, storage stability and total energy efficiency?

Initial product: one brown-rice cultivar and lot. Do not mix cultivars, harvests or commodity types in the first qualification. Add legumes only after a separate matrix-specific protocol.

Primary comparison arms:
1. Untreated commodity.
2. Conventional preparation (e.g. normal rinse / soak) matched to consumer instructions.
3. A commercially available quick-cook analogue, if the exact product and market version can be obtained.
4. Plasma-pretreated sample.
5. Plasma-pretreated sample followed by ordinary cooking.
6. A relevant non-plasma pretreatment control where it helps isolate the mechanism.

For a replication of the 2012 low-pressure study, capture the exact reactor pressure, power, gas, sample loading, treatment time and moisture state from the primary paper. Do not substitute parameters from the later atmospheric DBD study. A separate atmospheric brown-rice study should be replicated as its own experiment, not merged into the low-pressure dataset.

### H2 — sealed-package CAP

Question: can a defined package + headspace + discharge process reduce a named surface contaminant or alter a declared quality endpoint without unacceptable food oxidation, flavor modification, migration or loss of package integrity?

Treat the package as part of the process. Record material supplier/grade, film thickness, revision, seal geometry, headspace composition, humidity, electrode arrangement and post-treatment integrity. Surface treatment does not establish internal-pathogen inactivation.

### H3 — plasma-activated water / wash media

Question: can a generated water stream improve a declared produce pre-rinse or container-cleaning outcome against water-only and conventional process controls?

PAW is not one standardized chemical. Record generation process, water matrix, pH, ORP, conductivity, hydrogen peroxide, ozone where relevant, nitrite/nitrate and other species selected by a validated analytical plan. Measure concentration and decay at generation, storage and use. Do not advertise PAW as a sanitizer or food-safe treatment until the exact intended use is independently validated and reviewed.

### H4 — food functionality modification

Explore specific outcomes such as starch hydration / gelatinization, flour functionality, protein solubility, emulsification, foaming and drying kinetics. A changed contact angle, enzyme activity or protein structure is only a mechanism-level observation; the relevant cooked or processed food outcome remains the acceptance measure.

### H5 — thermal plasma cooking

A thermal-plasma source can transfer heat, but that fact does not establish a good cooking system. Compare against induction, infrared, radiant resistance and flame on a single well-defined task. A hot plasma core is not evidence of high wall-plug-to-food efficiency.

No household thermal-plasma hardware is in scope until a separate thermodynamic, hazard, maintenance, conformity and matched-performance feasibility gate is approved.

## 2. Brown-rice replication protocol

### 2.1 Freeze the product and recipe

Record exact rice cultivar, supplier, harvest/lot where available, milling / polishing state, starting moisture, grain dimensions, mass, storage history and pre-treatment humidity. Use a single homogeneous lot for paired arms; retain aliquots for independent baseline assays.

Freeze:
- rinse / soak instructions;
- water-to-rice ratio;
- vessel and lid;
- cooker / hob model and revision;
- starting water and grain temperatures;
- heating power profile;
- altitude / ambient conditions where relevant;
- final texture / hydration endpoint and measurement method.

### 2.2 Treat the plasma stage as a unit operation

Record reactor identity, discharge family, pressure, gas and purity, humidity, voltage waveform, frequency, current, actual wall-plug power, absorbed energy if independently measured, electrode gap, treatment time, sample arrangement, fill ratio, sample temperature and process repeatability. Include vacuum pumps, gas supply, cooling, exhaust treatment and standby consumption in the system boundary.

Do not interpret generator nameplate power as energy delivered to the food. If absorbed power is unavailable, declare it unknown rather than fabricate a value.

### 2.3 Measure cooking and quality

Report three distinct clocks rather than one ambiguous “cooking time”:
1. **Household cook-stage time:** from opening the ready-to-cook pack / starting ordinary preparation to the predeclared accepted cooked-food endpoint.
2. **Upstream process time and throughput:** plasma treatment time per batch and per kilogram, including loading, vacuum / gas handling, unloading, packaging and queueing.
3. **Total sequential chain time:** untreated raw commodity through plasma pretreatment and cooking. Report this for a sequential process; a factory batch operation can run in parallel with other production and must not be silently compared with a household elapsed-time measure.

Primary household-service outcome:
- time-to-accepted-cooked-food from the same declared household start point, versus untreated and conventional-preparation controls.

Secondary outcomes:
- pretreatment time / throughput and household cook time separately;
- total sequential chain time, with any overlap / parallelization explicitly declared;
- total wall-plug energy across pretreatment, auxiliaries, storage and cooking;
- water consumption and cooked yield;
- hydration / water-uptake kinetics;
- hardness, chewiness, stickiness and texture uniformity;
- blinded sensory liking / acceptability;
- nutrient and relevant chemical composition;
- oxidation / rancidity markers as appropriate;
- storage stability and package compatibility if pretreated product is to be sold.

The first energy denominator should be kWh per kilogram of accepted cooked edible output. Also publish raw-ingredient and serving-normalized values. Do not count water mass as recovered edible yield.

### 2.4 Adversarial cases

- Plasma treatment shortens the cooking stage but total pretreatment + cooking is slower.
- A vacuum pump or inert gas system erases the energy benefit.
- Surface modification is heterogeneous across the sample bed.
- One sample position receives a higher treatment dose than another.
- Apparent water uptake improves while texture or sensory acceptance worsens.
- A plasma-induced chemical change is misreported as improved nutrition or safety.
- Microbial reduction is measured at one point but not reproduced across lots.
- Plasma-conditioned product degrades during shelf storage.
- Failed / oxidized / off-flavor samples are excluded from the final average.

## 3. Surface treatment and PAW qualification

For H2/H3, start with laboratory-owned, reviewed equipment and a named commodity / organism / endpoint. Do not make generic pathogen, shelf-life or sanitizer claims.

Challenge-study requirements include:
- appropriate independent microbiological laboratory;
- declared organism / strain and initial inoculum;
- realistic product matrix, surface geometry and organic-soil burden;
- validated recovery / neutralization procedure and method detection limits;
- spatial sampling across worst-case positions, crevices and shadowed sites;
- positive and negative controls;
- replicate independent lots and treatment runs;
- confidence intervals and all non-detect / censored-result handling;
- independent food-quality, chemical-by-product and sensory endpoints.

For in-package treatment, also test seal integrity, package barrier / mechanical properties, material migration / extractables, headspace, package aging and post-treatment recontamination. A package treatment cannot be assumed to preserve every polymer or food-contact material.

For PAW, assess actual water chemistry as a function of generation, holding time and food matrix. The concentration at the generator outlet is not automatically the exposure delivered at the product surface.

## 4. Thermal plasma comparator protocol

A thermal-plasma experiment is justified only if it answers a specific cooking question (for example, surface browning on a defined cut / geometry). Compare the plasma system with the best suitable commercial method at the same food load, starting condition and endpoint.

Measure:
- total electrical input, including power conversion and cooling;
- source gas, flow, consumables and electrode wear;
- heat absorbed by food versus exhaust / wall / cooling losses;
- spatial surface temperature and internal thermal trajectory;
- browning / color uniformity, moisture loss, texture and sensory outcomes;
- by-product chemistry and oxidative changes;
- noise, operator exposure, cleaning, fault recovery and maintenance cost.

Stop if a matched incumbent achieves the same result with lower total system burden, or if safe containment makes the product impractical.

## 5. Competitive reference methods

Select the comparator from the purpose of the treatment:

| Application | Required reference methods |
|---|---|
| Grain / pulse cooking | Untreated preparation, conventional soaking / cooking, quick-cook commercial product, pressure / induction / resistance cooking as appropriate |
| Surface microbial treatment | Validated product-specific wash / sanitizer, UV-C or pulsed light where applicable, CAP |
| Nonthermal bulk processing | HPP, PEF, pulsed light, ultrasound and conventional thermal processing where the endpoint and matrix permit |
| Direct heat / searing | Induction, radiant resistance, infrared and flame |

Do not rank results pulled from different organisms, food matrices, detection limits or process boundaries. Report all exact test conditions and study limitations. Review-level values are literature triage, not performance baselines.

One 2025 review reports 3–4 log CFU/g E. coli reductions on spinach in a cited five-minute CAP configuration, but also reports 0.5–2 kWh/kg for some CAP cases and identifies energy / scale-up limits. The pulsed-light values in the same review use different studies and boundaries. Treat these figures as reasons to design a matched experiment, not as a leaderboard.

## 6. Cross-project system responsibilities

- **Symtropy:** unit-operation model, electrical / thermal / mass balance, batch geometry, process scheduling, equipment reliability and failure cases.
- **Symthaea:** experimental-design suggestions, source / evidence appraisal, uncertainty and food-response modelling when sufficient independent data exist. It does not issue a food-safety clearance or actuate the plasma generator.
- **Mycelix:** connect exact ingredient lot, packaging design revision, treatment event and independent claim evidence using existing provenance and packaging-loop work. A ledger authenticates record integrity, not scientific truth.
- **Sol Atlas:** only add region-specific power, water, supply, food-category or logistics context where source provenance and spatial / temporal resolution are adequate.

## 7. Benchmark targets and decision gates

All targets below are hypotheses, not established standards or results.

### H1 brown rice

After the baseline is reproduced:
- >=20% lower household cook-stage time versus the matched untreated lot at the same accepted-food endpoint; report upstream treatment time and plant throughput separately;
- >=10% lower total wall-plug energy per accepted cooked kilogram after including plasma equipment, vacuum / gas / cooling auxiliaries and household cooking;
- publish total sequential chain time and any factory/household parallelization explicitly; do not claim end-to-end time savings from a reduced cooking stage alone;
- no more than a predeclared 0.5-point reduction in a 9-point sensory-liking score;
- no material deterioration in texture, nutrition, oxidation, storage stability or food-safety handling.

A study can disprove the hypothesis. If pretreatment saves boil time but increases total time or energy, it is not a successful kitchen product.

### H2 / H3

Set a commodity- and organism-specific log-reduction target only after a competent lab confirms assay range and an appropriate challenge protocol. Evaluate the process against an equal-endpoint incumbent with energy, chemistry, sensory, packaging and exposure outcomes included.

No uncharacterized process by-product or occupational exposure pathway may remain at a proposed operating condition.

### H5

No assumed improvement percentage. First establish a complete baseline and safety feasibility. Continue only if plasma offers a useful, reproducible food outcome that justifies its full system burden.

### Gate sequence

1. Primary-paper / prior-art ledger.
2. Data-rights and protocol audit.
3. Deterministic trial manifest and energy / mass accounting.
4. Brown-rice replication design reviewed by food-science and process specialists.
5. Independent laboratory / electrical-safety / chemical-exposure review.
6. Controlled laboratory trial, with every failure retained.
7. Decision on a sealed-package or PAW follow-up based on H1 learning and specific product need.
8. Thermal-plasma continuation decision only after matched feasibility evidence.

## 8. Research references

- Lee et al. (2012), *Evaluation of physicochemical properties of plasma treated brown rice*, https://www.sciencedirect.com/science/article/pii/S0308814612007558
- Lee et al. (2016), *Evaluation of cold plasma treatments for improved microbial and physicochemical qualities of brown rice*, https://doi.org/10.1016/j.lwt.2016.06.055
- Sarangapani et al. (2016), *Influence of low pressure cold plasma on cooking and textural properties of brown rice*, https://doi.org/10.1016/j.ifset.2016.08.009
- Bezerra et al. (2023), *Cold plasma as a pre-treatment for processing improvement in food*, https://doi.org/10.1016/j.foodres.2023.112663
- RSC (2024), *Cold plasma technology for sustainable food production*, https://doi.org/10.1039/D4FB00209A
- Sasikumar et al. (2025), *A comprehensive review on cold plasma applications in the food industry*, https://doi.org/10.1039/D5FB00148J
- Deng et al. (2025), *Safety assessment of cold plasma technology in food: From molecular modification to toxicological analysis*, https://doi.org/10.1016/j.foodchem.2025.146573
- Food Control (2026), *Cold plasma technology in food Systems: Mechanisms, multifunctional applications, and pathways to industrial adoption*, https://doi.org/10.1016/j.foodcont.2025.111631

## 9. Claim ceiling

This research design does not establish an appliance prototype, food safety, sterilization, shelf-life extension, nutrient improvement, environmental advantage, commercial viability, product superiority or regulatory conformity.
