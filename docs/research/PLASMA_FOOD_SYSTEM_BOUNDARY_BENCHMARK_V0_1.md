# Plasma Food System-Boundary Benchmark V0.1

**Status:** research protocol proposal; no physical experiments have been performed and no plasma-food performance claim is established.

**Parent:** Symtropy [PLASMA-FOOD-QUAL-001 #5658](https://github.com/Luminous-Dynamics/symtropy/issues/5658).  
**Companion:** [PLASMA_FOOD_QUALIFICATION_V0_1.md](./PLASMA_FOOD_QUALIFICATION_V0_1.md) and `schemas/plasma-food-trial-v0.1.schema.json`.

## 1. The benchmark should measure accepted food, not plasma activity

The proposed benchmark is **Plasma Food Benchmark (PFB-1)**. Its primary unit of comparison is:

> **Energy, time, cost, water and quality per kilogram (and per serving) of accepted, edible food at a predeclared endpoint.**

A plasma system is not successful merely because it creates a large reactive-species signal, reduces a laboratory plate count, raises water uptake, or shortens one stage. It must improve a useful food outcome against the strongest relevant non-plasma alternative, with the full process boundary visible.

For the first workstream, PFB-1/H1-RICE asks whether plasma-conditioned brown rice can deliver a better consumer product than untreated rice, conventional soaking, and a commercially available quick-cook/parboiled reference, where comparable products exist.

This benchmark deliberately separates three claims:

1. **Ingredient-state claim:** pretreatment reproducibly changes an identifiable food property.
2. **Cooking-performance claim:** the treated ingredient reaches the same accepted endpoint faster or with less household energy.
3. **System-performance claim:** the full upstream + package + household process improves energy, cost, water, throughput or service burden per accepted serving.

Passing one claim does not imply passing the next.

## 2. Why a new benchmark is needed

### 2.1 A published cooking-time result is not an end-to-end win by itself

A low-pressure cold-plasma brown-rice study reported a cooking-stage change from 29.1 to 21.1 minutes after a 5–10 minute pretreatment. The arithmetic matters:

- 5-minute pretreatment + 21.1-minute cooking = **26.1 minutes** of sequential process time, a 3-minute improvement against the 29.1-minute cooking reference.
- 10-minute pretreatment + 21.1-minute cooking = **31.1 minutes**, 2 minutes slower than that reference.

This excludes loading, pump-down, gas, cooling, standby, packaging and other auxiliaries. The household cooking-stage reduction remains potentially useful when conditioning is performed upstream and in parallel with other manufacturing, but it cannot be represented as a 27.5% whole-process time reduction.

The study's full protocol and operating boundary must be retrieved and independently reconstructed before attempting an exact replication. A separate 2016 atmospheric-pressure study is a different modality and must remain a separate evidence record.

### 2.2 Generator settings are not a portable dose

Voltage, frequency and treatment duration are not sufficient for cross-device comparison. The 14 November 2026 review on plasma-induced food-component modification describes how discharge geometry, waveform, gas, humidity, pressure and exposure mode change the reactive species reaching a food. It distinguishes gas-specific energy input from energy delivered per mass or volume of food and notes that reported electrical parameters are often not commensurate.

For each configuration, report at least:

- measured wall-plug power and energy for the entire equipment boundary;
- discharge power / energy where a defensible measurement exists;
- gas-specific energy input, e.g. (SEI=P_{discharge}/Q_{gas}), where flow exists;
- applied energy per mass or volume of food, with the exact power boundary stated;
- feed-gas identity, purity, flow, humidity, pressure and temperature;
- sample geometry, position, spacing, loading density, orientation and exposure uniformity;
- a measured reactive-chemistry profile at the food-contact boundary where validated methods are available;
- post-treatment food-state measurements and their uncertainty.

These quantities are complementary, not interchangeable. Optical emission, ORP, pH, a single oxidant assay, applied voltage, or elapsed time must not be presented alone as a universal food dose.

For PAW, record generation method, water source and composition, pH, conductivity, ORP, relevant long-lived species, storage temperature and age at use. Short-lived species formed during activation must not be assumed to persist through storage and transfer.

## 3. Four process boundaries — report all relevant ones

| Boundary | Includes | Answers |
|---|---|---|
| B1: reactor | Generator, power supply, electrodes, gas delivery, vacuum, cooling, exhaust treatment, controls and idle energy | What does the plasma unit itself consume and produce? |
| B2: processor gate | B1 + ingredient handling, drying/cooling, cleaning, packaging, quality control, rejects and yield loss | What does the producer need per kilogram of saleable conditioned product? |
| B3: household | Ingredient preparation, cooker/pot, water, cooking and holding | What does the consumer experience and consume? |
| B4: gate-to-plate system | B2 + distribution/storage + B3, with declared allocation assumptions | Does the entire product system outperform its best incumbent? |

Do not collapse these boundaries into one number. Report energy carriers separately before any primary-energy or emissions conversion. Any conversion must declare the regional grid factor, year, source and treatment of upstream energy.

### Parallel production versus sequential time

A factory can condition ingredients while a household is doing other tasks; this can reduce consumer time-to-table without reducing the elapsed calendar time from raw commodity harvest to plate. Report separately:

- reactor cycle time and kg/hour at the stated batch fill;
- line throughput, uptime, cleaning/changeover and rejection rate;
- ingredient lead time and storage age;
- household preparation-to-accepted-food time;
- sequential process time when stages truly occur in series;
- critical-path time when stages can occur in parallel.

Never add parallel stages as though they were sequential, and never omit their energy or capacity burden.

## 4. Denominators and accounting

The primary denominator is **accepted cooked edible output**, not raw feed mass, treated surface area or nominal reactor throughput.

For each independent process run, weigh and reconcile:

- incoming dry ingredient;
- added process/cooking water;
- water retained in product;
- drained, spent and evaporated water where measurable;
- package and process consumables;
- cooked edible output;
- rejected, spoiled, burnt, undercooked or otherwise unaccepted output.

Primary energy metrics:

```text
E_system_per_accepted_kg =
  (E_plasma + E_vacuum + E_gas_delivery + E_cooling + E_exhaust
   + E_controls_and_idle_allocation + E_handling_and_drying
   + E_packaging + E_distribution + E_household_cooking_and_holding)
  / accepted_cooked_edible_kg
```

```text
E_system_per_serving = E_system_per_accepted_kg × accepted_edible_mass_per_serving
```

Every term must either be measured, allocated under a disclosed method, or marked unknown. An unknown term must not silently become zero. Report measured electricity separately from fuel or thermal energy. Include standby, pump-down, warm-up, cleaning and changeover at a declared utilization rate.

Other required measures:

- litres of water per accepted cooked kilogram and per serving;
- total cost per accepted serving, including equipment annualization, utilization, maintenance, electrodes, gas, electricity, water, labor, package, testing, distribution and rejects;
- saleable yield and percentage of runs meeting the complete acceptance specification;
- product storage stability and treatment-to-use age;
- sensory acceptance and the distribution of results, not only the mean.

A lower process energy per kilogram of raw material is not an improvement if product yield or acceptance falls.

## 5. H1-RICE experimental design

### 5.1 Qualification sequence

**Stage 0 — protocol reconstruction**

Retrieve and archive the primary-study methods, supplementary information and available raw data. Resolve differences in cultivar, moisture, grain age, starting temperature, cooking endpoint, rice-to-water ratio, vessel, heating source and definition of cooking time. Mark unavailable source fields as unknown.

**Stage 1 — measurement and reactor characterization**

Before food trials, qualify the wall-plug meter, temperature measurement, weighing, moisture assay and relevant plasma diagnostics. Establish empty-chamber, loaded-chamber and sham-process energy. Verify that all auxiliary loads are included. This stage is not a food-safety approval.

**Stage 2 — randomized split-lot screening**

Use one defined brown-rice cultivar initially, with independently sourced lots as biological/process blocks. Randomly split each lot among:

- untreated dry rice;
- conventional soak, with time and water ratio fixed;
- chamber-handled sham without plasma, where compatible with the selected reactor;
- plasma conditioning at protocol-reconstructed exposure settings;
- a matched moisture/thermal pretreatment where scientifically justified;
- a commercial quick-cook or parboiled comparator, if genuinely comparable and available.

Treat the 2012 low-pressure protocol and 2016 atmospheric-pressure protocol as separate experimental families. Do not pool them as interchangeable replications.

Each run—not each grain or instrument read—is the independent process replicate. Grain subsamples and repeated instrument readings are nested technical observations. Randomize run order and cooking order; blind texture and sensory assessors where feasible. Record day, operator, equipment revision and every deviation.

**Stage 3 — endpoint-matched confirmation**

After screening, freeze one or more candidate settings and a complete protocol. Use new lots and at least one holdout cultivar or source. Determine confirmatory sample size from the Stage 2 variance and a predeclared minimum useful effect; do not choose sample size after seeing confirmatory results.

**Stage 4 — process translation**

Only after Stage 3 passes, model a larger batch/continuous line including loading geometry, residence-time distribution, cleaning, line utilization, downtime, product handling, packaging, storage and actual throughput. Laboratory log reduction or treatment uniformity is not proof of industrial throughput.

### 5.2 Cooking endpoint and quality equivalence

A shorter clock time is meaningful only if both products reach the same predeclared endpoint. Freeze:

- rice-to-water mass ratio and water composition;
- cooker, pot, lid, batch size and loading pattern;
- initial ingredient and water temperatures;
- heating protocol and start/stop clock definitions;
- rest/hold time before assessment;
- target cooked moisture, texture range and sensory acceptance criteria.

Measure cooking time-to-endpoint as a curve, not just one selected time. Instrumental texture should include a reproducible hardness and relevant stickiness/chewiness measures. Record cooked yield, grain integrity, water uptake kinetics, moisture and mass loss. Sensory work must use blinded coded samples and appropriate human-subject/food-safety review before tasting.

### 5.3 Food-state and quality endpoints

Measure only parameters linked to explicit hypotheses, using validated methods:

- initial moisture, cultivar, lot, age, storage history and grain geometry;
- water uptake / imbibition kinetics and hydration uniformity;
- alpha-amylase or other enzyme activity only where mechanistically justified;
- starch gelatinization/pasting and thermal properties where they inform cooking behavior;
- texture, cooked yield, sensory acceptance and cooking curve;
- relevant lipid/protein oxidation markers, nutrient composition and volatile/off-flavor changes;
- storage stability across the intended package and realistic storage conditions.

The recent component-focused review reports that plasma can modify starch, protein and lipid structures in directions that depend on source, feed gas and intensity; a desired functional change can coexist with oxidation, aggregation or nutrient damage. Therefore, “more hydrophilic,” “more reactive,” or “more enzyme activity” is not itself a success endpoint.

Microbial reduction is a separate, independently designed and independently reviewed challenge study. It is not inferred from hydration, cooking time, hardness or surface chemistry.

## 6. Frozen hypotheses and performance gates

These are **proposed project targets**, not standards, observed results or industry-leading claims.

| Gate | Proposed target | Required interpretation |
|---|---|---|
| Consumer cook-stage | At least 20% lower cook-stage time than the matched untreated lot | Same appliance and accepted-food endpoint; report upstream process separately |
| Full-system energy | At least 10% lower wall-plug energy per accepted cooked kilogram | Include all B1–B4 terms relevant to the declared product boundary |
| Sensory | No more than a predeclared 0.5-point regression on a 9-point liking scale | Confirm with powered, blinded assessment; include confidence interval |
| Quality / chemistry | No material deterioration beyond predeclared margins | Include texture, nutrition, oxidation, storage stability and package effects |
| Reproducibility | Effect reproduced on new lots and a holdout cultivar/source | Report between-lot variance and worst-lot behavior |
| Economics | Lower or justified cost per accepted serving at realistic throughput | Include utilization, rejects, labor, package and maintenance |

A result may pass the consumer cook-stage gate but fail the full-system gate. That can still be useful as a product insight, but it is not an energy-efficiency victory.

The 20%/10% targets should be frozen before confirmatory work. If pilot variance implies that the planned study cannot distinguish these effects from noise, redesign the study before collecting confirmatory data.

## 7. Competitive baseline: beat the relevant incumbent

Use a layered comparison instead of a universal “plasma versus industry” claim.

| Use case | Required comparators | Fair comparison condition |
|---|---|---|
| Faster brown rice | Untreated, soak, parboiled/quick-cook commercial reference, appropriate pressure-cook method | Same cultivar/lot where possible, same accepted texture and serving |
| Grain functional modification | Hydrothermal or other established pretreatment with similar moisture/thermal history | Same final food function, not merely a chemical proxy |
| Produce surface treatment | Applicable validated wash/sanitizer, UV-C or pulsed light | Same food, microbial target, soil load, surface geometry and assay/recovery |
| Packaged-food CAP | Best relevant existing package/process and untreated package control | Same package barrier, headspace, storage and quality/safety endpoint |
| PAW wash medium | Water-only and the incumbent wash process | Same source water, organic load, temperature, contact time and downstream rinse |
| Direct thermal cooking | Induction, radiant resistance, infrared, flame or pressure/steam equipment as appropriate | Same food, cooking endpoint, wall-plug boundary and batch size |

HPP, PEF, ultrasound and pulsed light are not universal competitors. They should enter only where the product and target endpoint make them technically relevant.

A review's energy value from one food and another technology's energy value from a different paper must never be ranked as a head-to-head result. Obtain primary-study boundaries or perform a matched test.

## 8. Statistical and evidence contract

- Pre-register primary endpoint, comparator, meaningful-effect threshold, exclusion rules and analysis before confirmatory testing.
- Use split-lot randomized blocking. Model lot and run/day variation; do not count technical subsamples as independent replicates.
- Publish effect sizes and confidence intervals alongside means, full distributions and failures.
- Retain null, negative, spoiled, oxidized, off-flavor, out-of-range and incomplete runs.
- Separate exploratory model fitting from held-out confirmatory evaluation.
- Release the protocol revision, raw data or lawful reproducible summary, analysis revision, calibration records and trial-manifest digest.
- Missing source or instrument values remain unknown; no imputation may be presented as a measurement.
- Provenance integrity verifies the record chain, not the truth of a food-chemistry or microbiology claim.

## 9. Stop, pivot and scale rules

**Stop H1** if the effect does not reproduce across independent lots, if quality/safety margins fail, or if the complete boundary cannot meet the predeclared value proposition.

**Pivot H1** if cook-stage time improves but full-system energy does not. Investigate whether upstream batching, a different commodity, conventional hydrothermal pretreatment or a different product proposition can justify the process. Do not rename a failed energy target as a success.

**Advance H2/H3** only for a specific product and validated endpoint, with package/material or water chemistry qualification, by-product assessment, occupational-exposure controls and independent microbiological/food-quality review.

**Keep direct thermal plasma as discovery-only** until it can show a useful heat-transfer or food-quality advantage over induction/IR/flame within a complete wall-plug, safety, maintenance and cost boundary.

Scale only after a successful independent replication and a documented line-capacity, cleaning, reliability, yield and technoeconomic study.

## 10. Paradigm shifts to pursue

1. **From plasma dose to food-state transition.** Model the chain from generator settings through gas/liquid chemistry, transport and matrix-conditioned exposure to measurable ingredient state and final food function.
2. **From raw kilograms to accepted meals.** Make edible yield, cooking endpoint, sensory acceptance, water and full-system energy the common denominator.
3. **From a household plasma appliance to upstream ingredient infrastructure.** Let producer co-operatives or food processors condition ingredients in controlled equipment when the process only makes sense at batch scale; the home appliance should remain ordinary and serviceable.
4. **From average efficacy to robust operating envelopes.** Qualify across lots, moisture, geometry, humidity, treatment position, storage age and realistic fouling/maintenance states. Publish worst-case results.
5. **From single-variable optimisation to causal experiment design.** Use split-lot controls, sham processes, matched thermal histories, randomized run order and factorial tests. Claim synergy only against an explicit additive reference.
6. **From closed demonstrations to an open benchmark.** A useful contribution to the field is a reproducible protocol, machine-readable manifests, common denominators, complete negative-result retention and independent replication—not an unsupported “plasma is better” headline.

## 11. Cross-project responsibilities

- **Symtropy:** deterministic process accounting, mass/energy/time oracle, process scheduling and equipment-scale model.
- **Symthaea:** literature evidence quality, food-state uncertainty and experiment selection once enough primary data exist. No food-safety sign-off or generator actuation.
- **Mycelix:** ingredient-lot, treatment-run, package-revision and claim-evidence lineage. Record integrity does not confer scientific approval.
- **Sol Atlas:** regional electricity, water, logistics, supplier and food-system context, with source, year and spatial resolution attached.

## 12. References and claim ceiling

Research anchors:

- Lee et al. (2012), [Evaluation of physicochemical properties of plasma treated brown rice](https://www.sciencedirect.com/science/article/pii/S0308814612007558).
- Lee et al. (2016), [Evaluation of cold plasma treatments for improved microbial and physicochemical qualities of brown rice](https://doi.org/10.1016/j.lwt.2016.06.055).
- Sarangapani et al. (2016), [Influence of low pressure cold plasma on cooking and textural properties of brown rice](https://doi.org/10.1016/j.ifset.2016.08.009).
- Yang et al. (2025), [Delivery systems of cold atmospheric plasma in the food industry](https://doi.org/10.1016/j.tifs.2024.104831).
- Deng et al. (2025), [Safety assessment of cold plasma technology in food](https://doi.org/10.1016/j.foodchem.2025.146573).
- Food Control (2026), [Cold plasma technology in food systems: mechanisms, multifunctional applications, and pathways to industrial adoption](https://doi.org/10.1016/j.foodcont.2025.111631).
- Boyapati & Muthukumarappan (2026-11-14), [Cold Plasma and Plasma-Activated Water-Induced Modifications of Major and Minor Food Components](https://doi.org/10.3390/plasma9040047).

This document does not establish a prototype, food safety, sterilization, shelf-life extension, nutritional improvement, environmental benefit, commercial viability, regulatory conformity or superiority over any incumbent. Those claims require product-specific evidence and independent qualification.
