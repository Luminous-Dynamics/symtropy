# Culinary Workcell Architecture and Benchmark V0.1

**Status:** research design / unqualified  
**Parent:** https://github.com/Luminous-Dynamics/symtropy/issues/2313  
**Execution issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5618  
**Revision date:** 2026-11-16

This is a product architecture and measurement proposal. It is not a hardware design, pressure-vessel drawing, safety assessment, or evidence of improved physical performance.

## 1. Product thesis

Build a **culinary workcell**: a family of coordinated, repairable kitchen capabilities that improves how food is prepared and cooked. Judge the product on accepted meals, not the number of modes printed on its enclosure.

The workcell integrates:
1. Ingredient handling and preparation.
2. Thermal cooking and food transformation.
3. Workflow, measurement and user interaction.
4. Food storage, portioning, cleaning and material return.

The first version must not assume these functions belong inside one appliance. A single unit may reduce footprint but introduce difficult cleaning boundaries, cross-contamination, mechanical coupling, common-mode failures and expensive replacement. Separate best-in-class appliances may remain superior. The architecture trade is an experiment, not a branding decision.

## 2. Architecture candidates

| Candidate | Description | Main hypothesis | Primary risks |
|---|---|---|---|
| A. Separate reference appliances | Best-in-class processor and multicooker, coordinated by shared workflow | Lowest technical coupling; strongest baseline | Counter space, multiple cleaning and storage interfaces |
| B. Monolithic all-in-one | One enclosure and central mechanism for most operations | Potentially lower footprint and fewer transfers | Complexity, difficult cleaning, single-point failure, compromised specialist functions |
| C. Dockable workcell | Independent thermal and preparation modules with shared workflow and container geometry | Modularity may improve maintenance and use without shared hazardous mechanisms | Docking complexity, interfaces, cost |
| D. Cabinet-integrated system | Removable modules in a compact counter/cabinet arrangement | Better space, ergonomics and workflow | Installation and service constraints |

**Initial preferred hypothesis:** C, with A retained as the control. This is a preference for an independently testable architecture, not a conclusion that modular equipment is inherently better. The shared layer should be software, task identity, measurement, compatible containers and user experience. The thermal/pressure mechanism and high-speed cutting drive should remain physically and safety-functionally independent unless tests demonstrate a safe, simpler alternative.

## 3. Functional decomposition

### 3.1 Thermal module

Potential functions:
- Electric pressure cooking.
- Non-pressure simmering, slow cooking and sautéing.
- Steaming.
- Temperature-controlled processes only where spatial uniformity and outcome are qualified.

Required properties:
- Removable vessel with cleanable interior and inspectable food-contact surfaces.
- Replaceable seals and accessible non-pressure wear items.
- Mechanical pressure containment, lid locking, venting and thermal cut-outs independent of software or AI.
- Deterministic local control, physical controls and a clear manual path.
- Safe behavior under power interruption, sensor disagreement, blocked vents, degraded seals and interrupted cycles.

Do not put a high-speed food processor blade inside a pressure vessel. Do not make the processor motor, network, recipe app or Symthaea a dependency of the pressure boundary or its interlocks.

### 3.2 Food-preparation module

Operations are independent test functions, not one generic “processing” score:
- Chop / mince.
- Slice.
- Dice.
- Grate.
- Shred.
- Purée.
- Emulsify.
- Mix / batter.
- Knead.
- Optional milling or juicing only after separate need and risk review.

Design hypotheses:
- Useful small-batch mode that does not leave ingredients untouched around the bowl edge.
- Full-batch mode with predictable particle size and low fines.
- Easily inspected and cleaned bowl, lid, pusher, seals, blade hub and accessory interfaces.
- Replaceable wear parts and supplier-neutral interfaces where these improve repairability.
- Load-aware motor control may adapt speed/torque, but inferred texture is not observed texture. Verify against measured particle distributions and output mass.

### 3.3 Food and ingredient interfaces

A small set of stackable containers may connect delivery crates, dry/cold storage, weighing, portioning, preparation, serving and leftovers.

A shared container geometry must not imply a universal food-contact rating. Wet, dry, fresh, frozen, hot, acidic, allergenic and pressure applications need separate material and use profiles. A storage container becomes a heated vessel only after exact material, temperature, pressure, migration, closure and cleaning qualification.

## 4. Competitive baseline and task basket

Freeze exact SKU, revision, accessories, firmware, voltage, bowl/vessel capacity, market availability and settings before testing. Consumer Reports' 2026 multi-cooker and food-processor tests provide a transparent public starting basket; they do not cover every desired outcome.

### 4.1 Pressure / multi-cooker comparator shortlist

- Zavor LUX LCD ZSELL02.
- Breville Fast Slow Pro BPR700BSS.
- Instant Pot Duo Crisp 11-in-1 8 qt 140-0021-01.

Initial tasks: pork ribs, beef-and-bean chili, rice, steamed vegetables, sautéed onions, slow-cooked stew/chili, a high-starch / burn-prone recipe, and low- and high-fill cases.

Measure complete time-to-plate, including preheat, pressure build, cooking, pressure release, transfer, resting and cleaning. Record edible yield, liquid retention, texture, scorching, temperature distribution, electrical energy, user-present time, noise, steam exposure and failed/restarted cycles.

### 4.2 Food-processor comparator shortlist

- Breville Paradice 9 BFP638.
- Cuisinart Core Custom FP-130.
- Magimix 5200XL.

Initial tasks: uniform chopping of hazelnuts, herbs and onions; cheese grating; puréeing cooked peas, carrots and potatoes; shredding raw carrots and cheese; slicing potatoes; batter; dough; mayonnaise.

Measure output particle-size distribution, coefficient of variation, fines, yield loss, temperature rise, bruising where relevant, throughput, sound, residue and cleaning time. Test both small and large batches; a large advertised bowl capacity is not evidence of small-batch performance.

### 4.3 Integrated-machine comparator

Include a Thermomix TM7 or comparable cooking machine as an architectural reference. Compare only functions it actually performs and retain separate function scores. Do not let its recipe ecosystem or number of automated modes replace matched measurement.

### 4.4 Why feature-count benchmarking fails

The incumbent market already contains strong specialists. A cooker may excel at pressure cooking but offer little time saving for rice; a processor may slice exceptionally well while delivering only average cheese grating. The product must outperform the relevant incumbent on the declared task, not on a summed feature checklist.

## 5. Measurement contract

Every trial has a stable ID and records appliance model/revision, accessory, firmware and test date; ingredient lot, mass, dimensions, moisture and starting temperature; ambient conditions and measured power; exact user settings and interventions; instrument identity, calibration, placement, sample interval and uncertainty; raw time series and endpoint assay references; all failed runs, burn warnings, restarts, spills, blocked-flow observations, broken components and missing records; and final disposition (accepted, partially accepted, rejected, reworked or unknown).

Evidence classes remain distinct:
- **Declared:** user, supplier or label assertion.
- **Modelled:** equation or digital-twin prediction.
- **Simulated:** output from a simulated environment.
- **Measured:** calibrated instrument observation.
- **Assayed:** independent output-quality test.
- **Attested:** reviewer confirms a narrowly scoped method/result.

A hash authenticates bytes, not the truth of a measurement.

### Primary outcome vector

| Domain | Metric | Required denominator |
|---|---|---|
| Cooking quality | Texture, tenderness, moisture, consistency, blinded sensory result | Accepted edible mass / serving |
| Processing quality | Particle-size distribution, CV, fines, bruising | Input and output mass |
| Time | Total elapsed time and user-present minutes separately | Accepted meal / batch |
| Energy | Metered electricity | Accepted edible serving |
| Water | Metered process and cleaning water | Accepted edible serving |
| Waste | Edible trim, transfer residue, spoilage, rejected batch | Input edible mass |
| Cleaning | Parts, steps, labor and elapsed cleaning time | Complete task |
| Reliability | First-pass yield, retries, faults and downtime | Trial and device exposure |
| Craft | Agency, control, learning, creativity, fatigue, accessibility | Practitioner/user cohort |
| Lifecycle | Purchase, energy, water, parts, consumables, repair and end of life | Declared service life |

Never collapse safety, taste, durability, energy, cost and craft into one opaque score. Use hard gates plus a Pareto outcome vector.

## 6. Stretch hypotheses

Targets below are exploratory hypotheses, not standards or achieved results. Freeze the baseline and margin before a confirmatory test.

- At least 10% lower electrical energy per accepted edible serving on a declared task basket without worse taste/texture, safety, cleaning, noise or repair results.
- At least 15% lower user-present time to an accepted meal across a multi-dish workflow; also report total elapsed time.
- At least 25% lower cut-size variation on three or more processing tasks without increasing fines, yield loss or bruising.
- At least 20% lower avoidable edible trim/transfer loss on a matched preparation basket.
- Sensory non-inferiority within a preregistered 0.5-point margin on a 9-point liking scale, with an adequately powered, blinded protocol.

If the reference baseline is already near a physical or practical limit, revise the hypothesis transparently rather than tuning the metric after seeing results.

## 7. Human-centered experience

The cook remains the author. Provide manual operation and optional transparent assistance; explanations of why a process is proposed and what evidence it relies on; clear override, pause and cancellation paths where safe; accessible control placement, grip/force, readable state and audible/visual feedback options; useful operation without camera, microphone, account, cloud or internet; support for varied cuisines, skill levels, bodies, traditions and ingredient availability; and attribution/permissions for practitioner-contributed methods and recipes.

Blind sensory testing should use appropriate sensory methodology; ISO 6658 is general guidance, and ISO 11136 addresses consumer hedonic tests in controlled areas. A model-generated taste score is not a substitute for a tasting panel.

## 8. Safety and conformity boundary

Starting references:
- IEC 60335-2-15:2024 for household appliances heating liquids, including pressure cookers only within the scope stated by the standard (rated cooking pressure not exceeding 140 kPa and capacity not exceeding 10 L).
- IEC 60335-2-14:2025 for household kitchen machines including food processors.
- South African food-contact and hygiene requirements, including R638 where applicable.
- Product-specific pressure-vessel, electrical, mechanical, material and market requirements identified with a competent test laboratory and relevant authority.

First physical stage:
1. Buy exact commercial reference products.
2. Use non-invasive instrumentation and/or manufacturer-supported measurement interfaces.
3. Run a shadow-mode observation study; do not change appliance control or safety behavior.
4. Prototype non-pressurized preparation equipment only after the fixture and hazard review.
5. Do not build or modify a pressure vessel for this phase. Any later pressurized prototype needs an independent pressure-system design review and conformity plan.

The learned model never owns a pressure or blade interlock. Loss of sensor confidence, power, network, calibration or identity must not produce an unsafe state.

## 9. Four-project system architecture

- **Symtropy:** physical/process twin, thermal and mass/energy accounting, task dependencies, resource scheduling through KITCHEN-OPS-001, failure rehearsal and architecture/lifecycle trade studies.
- **Symthaea:** chemistry-aware recipe constraints, flavor/ingredient reasoning, experiment selection, uncertainty, recommendation and transfer evaluation. The current toy kitchen state must not be represented as a physical measurement.
- **Mycelix:** existing lot/provenance/repair evidence and supplier/producer collaboration. No second identity, marketplace or repair ledger.
- **Sol Atlas:** regional water, energy, climate, supply, logistics and recovery context only when the data are sourced, rights-qualified and spatially/temporally fit for the claim.

## 10. Execution gates

**G0 — baseline freeze:** exact candidates, purchase availability, task basket, instruments, test recipes, endpoints and comparison protocol.

**G1 — bench fixture:** calibrated scales, power meter, temperature instrumentation, time/event logging, sound measurement and output assays. Verify the measurement system before evaluating algorithms.

**G2 — digital twin:** deterministic task/resource and heat/mass balance model; compare simulated predictions with measured trajectories and preserve model discrepancy.

**G3 — shadow-mode study:** collect real observations on commercially certified products. Compare fixed recipe, simple rules, mechanistic model and Symthaea proposal on identical inputs.

**G4 — non-pressure prototype decision:** start only when a bounded processing task shows a measurable gap and safety/cleaning/repair constraints are defined.

**G5 — pressure-capable product decision:** only after the independent safety/conformity plan, material review and physical qualification budget exist.

Every gate produces an exact revision, input manifest, receipts, failed/unknown counts and an explicit stop/continue/pivot decision. A queued CI result is not a pass; a software pass is not a hardware qualification.

## 11. Research references

- Consumer Reports, [Best Multi-Cookers of 2026](https://www.consumerreports.org/appliances/multi-cookers/best-multi-cookers-of-the-year-a7136971834/), updated 2026-01-01.
- Consumer Reports, [Best Food Processors and Choppers of 2026](https://www.consumerreports.org/appliances/food-processors-choppers/best-food-processors-of-the-year-a7878155198/), updated 2026-10-10.
- IEC, [60335-2-15:2024](https://webstore.iec.ch/en/publication/70372).
- IEC, [60335-2-14:2025](https://webstore.iec.ch/en/publication/80074).
- ISO, [6658:2017 Sensory analysis — methodology](https://www.iso.org/standard/65519.html).
- ISO, [11136:2014 Consumer hedonic testing](https://www.iso.org/standard/50125.html).
- Symthaea, [CULINARY-BENCH-001 #7859](https://github.com/Luminous-Dynamics/symthaea/issues/7859), [CULINARY-TRANSFER-002 #8170](https://github.com/Luminous-Dynamics/symthaea/issues/8170), [COOKWARE-CRAFT-001 #8182](https://github.com/Luminous-Dynamics/symthaea/issues/8182).
- Symtropy, [KITCHEN-OPS-001 #2227](https://github.com/Luminous-Dynamics/symtropy/issues/2227), [APPLIANCE-QUAL-001 #1805](https://github.com/Luminous-Dynamics/symtropy/issues/1805), [APPLIANCE-PORTFOLIO-001 #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825).

## 12. Nonclaims

No hardware prototype, pressure safety, food safety, sensory preference, market demand, savings, service life, repair support, energy improvement or SOTA superiority is established by this document.
