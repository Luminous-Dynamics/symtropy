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

### 3.4 Power- and grid-aware operation

Treat available electrical capacity as a declared household constraint, not an assumed constant. This is especially useful in markets where appliances may operate on household inverters, generators or constrained circuits.

The scheduler should accept a power envelope supplied by the user / installer: continuous real-power limit, short-duration surge capability, permitted concurrent loads, voltage/frequency range and any known circuit restrictions. These are inputs to a schedule, not values inferred from an appliance marketing wattage.

Research hypotheses:
- stagger the processor and heating module when their combined draw would exceed a declared envelope;
- use retained heat / insulation and sequencing to reduce unnecessary reheating only where the food-quality and food-safety evidence supports the profile;
- expose expected peak draw and duration alongside total energy and time-to-plate;
- preserve local manual control and deterministic appliance safety when network, scheduler or power is lost;
- do not automatically restart a pressure cycle after power restoration unless the exact appliance's manufacturer-defined recovery behavior permits it;
- keep backup batteries/inverters external to the first appliance design; qualify appliance and power-source compatibility independently.

Benchmark a matched multi-dish workflow under a declared power cap: manual scheduling, fixed-rule scheduling and Symtropy scheduling receive the same tasks, appliances, ingredients and power envelope. Record peak real power, duration above threshold, cap violations, total Wh, time-to-plate, user-present time, meal acceptance and interruption recovery disposition. Include waveform/power-quality constraints where an inverter or generator is used. Never intentionally interrupt a pressurized appliance outside a manufacturer-approved safe test procedure.

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

### 4.4 South African reference candidates

Global laboratory comparators must be supplemented with equipment actually sold and serviced in the launch market. Candidate South African baselines discovered from manufacturer / retailer listings:

- Russell Hobbs RHEP7, 6 L digital electric pressure cooker, listed at 1000 W with 16 menus: https://russellhobbs.co.za/products/6l-electric-pressure-cooker
- Russell Hobbs RHEP6N, 6 L electric pressure cooker with enamel removable pot, listed at 1000 W with 17 menus: https://russellhobbs.co.za/products/russell-hobbs-6l-electronic-pressure-cooker
- Russell Hobbs RHPC06, 6 L stovetop pressure cooker with manufacturer-listed 80 kPa operating valve: https://russellhobbs.co.za/products/russell-hobbs-stainless-steel-6l-pressure-cooker
- Kenwood MultiPro Express FDP65.400WH, retail-listed 3 L / 1000 W processor: https://www.pricecheck.co.za/offers/220870801/Kenwood%2BFDP65.400WH%2B3L%2BMultipro%2BExpress%2BFood%2BProcessor
- Magimix 5200XL, locally listed with 3.6 L / 2.6 L / 1.2 L bowls and 1100 W; regional retailer listings vary in stock and price: https://www.newworld.co.za/collections/all/products/magimix-5200xl-food-processor-chrome

These are acquisition candidates, not independent ratings. Record the exact type plate, manufacturing revision, 230 V / 50 Hz compatibility, accessories, local warranty, spares and service route. Retail availability changes; do not treat a search listing as stock confirmation. Keep stovetop pressure cookers in a separate test class and include the hob's measured energy and heat source.

### 4.5 Why feature-count benchmarking fails

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
- For South Africa, classify the finished appliance against the current NRCS compulsory-specification scope. VC 8055 is the relevant starting route to investigate for safety of electrical/electronic apparatus; the NRCS material identifies the SANS IEC 60335 household-appliance series. Confirm the current VC text, accepted SANS editions, scope, required evidence and Letter of Authority with NRCS before manufacture/import/sale. Do not assume an IEC publication by itself is South African market approval.
- IEC 60335-2-15:2024 itself notes that some countries impose additional national pressure-vessel requirements. Its stated household pressure-cooker scope is limited to rated cooking pressure not exceeding 140 kPa and rated capacity not exceeding 10 L; the finished product still needs a product-specific scope and conformity assessment.
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

### Existing food-conditioning research to reuse

Do not duplicate the plasma / food-conditioning workstream inside the appliance prototype. Compose with:
- Symtropy [PLASMA-FOOD-QUAL-001 #5658](https://github.com/Luminous-Dynamics/symtropy/issues/5658) for separate cold-atmospheric plasma, plasma-activated media, ingredient-functionalization and direct thermal-plasma hypotheses.
- Symtropy [PFB-ORACLE-001 #5683](https://github.com/Luminous-Dynamics/symtropy/issues/5683) and its [isolated Rust oracle](https://github.com/Luminous-Dynamics/symtropy/tree/research/plasma-food-qualification-v0-1-20261116/research/plasma-food-oracle) for fail-closed wall-plug energy accounting across reactor, processor-gate, household and gate-to-plate boundaries.
- Symtropy [PFB-1 system-boundary benchmark](https://github.com/Luminous-Dynamics/symtropy/blob/research/plasma-food-qualification-v0-1-20261116/docs/research/PLASMA_FOOD_SYSTEM_BOUNDARY_BENCHMARK_V0_1.md).

The appliance programme can compare the final accepted food outcome, but must not absorb a plasma generator into a kitchen appliance by assumption. If pretreatment is upstream, report household cook-stage time separately from sequential process time and full system energy per accepted edible output. Reuse PFB-1's existing accounting semantics; never turn missing upstream energy into zero.

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
- Russell Hobbs South Africa, [RHEP7 product listing](https://russellhobbs.co.za/products/6l-electric-pressure-cooker), [RHEP6N product listing](https://russellhobbs.co.za/products/russell-hobbs-6l-electronic-pressure-cooker), [RHPC06 product listing](https://russellhobbs.co.za/products/russell-hobbs-stainless-steel-6l-pressure-cooker).
- South African NRCS, [Compulsory Specifications overview](https://www.nrcs.org.za/compulsory-specifications/about-compulsory-specifications/) and [Letter of Authority administration procedure](https://www.nrcs.org.za/Documents/Electrotech/LOA%20Admin%20Procedure/LOA%20ADMIN%20Revised%2012_06%20May%202019.pdf).
- Kenwood local candidate discovery, [FDP65.400WH retailer comparison](https://www.pricecheck.co.za/offers/220870801/Kenwood%2BFDP65.400WH%2B3L%2BMultipro%2BExpress%2BFood%2BProcessor).
- Magimix local candidate, [5200XL product listing](https://www.newworld.co.za/collections/all/products/magimix-5200xl-food-processor-chrome).
- Symtropy, [KITCHEN-OPS-001 #2227](https://github.com/Luminous-Dynamics/symtropy/issues/2227), [APPLIANCE-QUAL-001 #1805](https://github.com/Luminous-Dynamics/symtropy/issues/1805), [APPLIANCE-PORTFOLIO-001 #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825).

## 12. Nonclaims

No hardware prototype, pressure safety, food safety, sensory preference, market demand, savings, service life, repair support, energy improvement or SOTA superiority is established by this document.



## 14. Benchmark alignment: recipe grounding, temporal planning and power-aware execution

Our benchmark should be comparable with external research, not merely with Luminous's own synthetic fixtures.

### External benchmark anchors

- **Recipe2Plan (Findings of EMNLP 2025):** evaluates multi-dish concurrent cooking plans with action durations and temporal intervals between steps. Its central lesson is that aggressive local parallelization can violate downstream timing constraints. Reproduce its public dataset and evaluation protocol before comparative claims: https://aclanthology.org/2025.findings-emnlp.229/ and https://github.com/WilliamZR/Recipe2Plan
- **A Benchmark for Recipe Understanding in Artificial Agents (LREC-COLING 2024):** provides recipes, procedural action representations, qualitative and quantitative kitchen simulators, and a standardized evaluation of mapping recipe language into executable actions: https://aclanthology.org/2024.lrec-main.3/

Recipe understanding is not schedule optimality; schedule feasibility is not physical food quality; simulator success is not hardware qualification.

### Comparison ladder

1. Human practitioner reference plan.
2. Serial topological baseline.
3. Existing earliest-gap resource scheduler.
4. Power-envelope verified scheduler.
5. Recipe2Plan-compatible temporal multitasking evaluation.
6. LREC-style recipe grounding evaluation.
7. Exact solver on small instances for proven lower bounds and optimality gaps.

A solver timeout or feasible-but-unproven result must not be reported as an optimality proof.

### Benchmark tiers

| Tier | Workload | Required evidence |
|---|---|---|
| T0 | Synthetic task graphs, 1–500 tasks | Deterministic fixtures, independent verifier, runtime and failures |
| T1 | Recipe2Plan-compatible recipes | Temporal-window satisfaction, success rate, makespan and intervention count |
| T2 | LREC-style natural-language recipe grounding | Action grounding precision/recall, graph validity, practitioner corrections and rights |
| T3 | Household power-envelope schedules | Peak and sustained real power, overloads, surge windows and background demand |
| T4 | Instrumented cooking sessions | Time-to-plate, user labor, energy/water, edible yield and blinded food-quality outcomes |

Report schedule-feasibility rate over all submitted plans. Report makespan gap only where a proven lower bound exists. Include replan latency, schedule stability under perturbation, peak draw, energy, human corrections and quality outcomes. Keep safety/feasibility, recipe grounding, efficiency, physical food quality and human craft as separate dimensions rather than one opaque score.

### Claims boundary

These are benchmark targets and comparison families, not results. No superiority over Recipe2Plan, LREC, CP-SAT, human practitioners or appliance incumbents is established. Freeze dataset rights, split, mapping, protocol and verifier before comparison.


### Power-aware scheduling beyond all-or-nothing fallback

The current power-constrained planner explores three stages: (1) existing parallel earliest-gap scheduling; (2) an event-boundary heuristic that delays only the task whose overlap violates the declared envelope; and (3) a serial topological fallback. Candidate starts are drawn from scheduled task/profile boundaries and surge-duration offsets rather than stepping through every millisecond.

A deterministic synthetic case uses two independent 10 ms tasks drawing 700 W and 600 W, a 1,000 W continuous limit, a 1,500 W peak limit and a 5 ms maximum excursion. Fully parallel execution creates 10 ms above the continuous limit and fails. A 5 ms stagger creates exactly 5 ms of allowed overlap and has a 15 ms makespan, versus 20 ms for full serialization. This is a unit-fixture target that now has code and a test; it must be confirmed by exact-head CI before being reported as a pass. It does not prove global optimality.

### Exact-solver and industrial scheduling comparison

Google OR-Tools documents interval variables, no-overlap scheduling and cumulative constraints for resource capacity profiles. The power-envelope problem is naturally a resource-constrained project scheduling problem (RCPSP) with precedence, exclusive equipment and a cumulative real-power resource. For a frozen small-instance subset, implement a CP-SAT reference using intervals plus cumulative limits; record optimal/feasible/unknown status, time budget, hardware and lower bound. Never treat solver timeout or feasible-but-not-proven status as optimality.

Use standard job-shop/flexible-job-shop benchmark infrastructure only as an algorithmic reference; it is not a direct kitchen workload. Compare on the exact same kitchen task graphs, resource profiles and envelope. Track the power-aware heuristic's makespan gap to a *proven* lower bound and savings over the serial baseline. The objective is not merely to beat a solver on toy inputs; it is to remain safe and useful on real kitchen constraints while maintaining predictable compute cost.


### 15. Commercial appliance comparison and South African acquisition protocol

The workcell is evaluated against function-specific appliance classes, not an undifferentiated “smart kitchen” score. Current comparator candidates already include pressure multicookers, food processors, an integrated cooking machine, and South African-market pressure cookers and processors. The exact regional SKU, type plate, accessory pack, firmware/revision, warranty and local service availability must be verified before acquisition. Retail discovery pages are not sufficient evidence of technical specifications.

A May 2026 independent comparison of seven cooking food processors lists Bosch Cookit MCC9555, Kenwood Cooking Chef KCL96, Vorwerk Thermomix TM7, WMF Avantgarde, Thermomix TM6, Kenwood Cooking Chef XL and Silvercrest Monsieur Cuisine Smart. The reported nominal power values range from 1,200 W to 1,750 W across those models. These are a useful procurement/comparator shortlist, not a matched performance benchmark: power ratings alone do not establish energy per accepted serving, heat-up duty cycle, cooking quality or household compatibility.

For South African comparisons, retain local models as separate regional SKUs: Russell Hobbs RHEP7/RHEP6N pressure cookers and Kenwood FDP65.400WH food processor are listed as candidates, while their current exact revision and availability remain unverified. Acquire manufacturer datasheets and physical type-plate readings before treating the listing specifications as ground truth. Stovetop pressure cookers must be measured as a complete cooktop-plus-vessel system and not compared to electric appliance wattage alone.

The commercial benchmark is not “has more programmes.” Compare each function against the best specialist tool and the integrated cooking machine: accepted output quality, task time, active labor, cleaning burden, durability, consumable/accessory costs, energy, water, repairability and user autonomy. A modular workcell only wins when matched-task trials demonstrate a better outcome or meaningful option value without shifting hidden burden to setup, cleaning, maintenance or user supervision.


## 16. Recipe action ontology and integrated kitchen robotics evaluation

### Six action-core hierarchy

The recipe-grounding lane now includes an optional `ActionOntologyBindingV1` with six cores—cutting, pouring, mixing, preparing, pick-and-place, cook-and-cool—and specific action groups. Each binding carries its ontology version, deterministic or human-reviewed mapping method, provenance, typed parameters, tool-capability references, precondition references and declared/modelled/simulated evidence class. The field is optional for backward compatibility; no binding means no ontology enrichment was performed. `Other` stays unmapped by default.

The taxonomy is aligned with the 2025 *Everything robots need to know about cooking actions* study. The paper analyses Recipe1M+ (1,028,692 recipes / 10,767,598 instructions) and reports about 54% verb coverage from direct matching and 76.5% after cosine-similarity matching against its six cores and action groups. It describes a further neuro-symbolic/LLM step for remaining verbs and limitations around less common actions, simplified parameterization, and reliance on simulation/selected demonstrations rather than broad real-kitchen qualification. Its 76.5% is the paper's corpus/protocol result, not a Symtropy measurement and not a precision score. Reference: https://www.frontiersin.org/journals/robotics-and-ai/articles/10.3389/frobt.2025.1682031/full

Our action-ontology benchmark therefore records coverage, group precision/recall, unmapped verb fraction, human override rate and downstream task-graph feasibility separately. It also tests whether ambiguous verbs remain unmapped rather than being force-fit for a headline coverage gain. Compare to the paper only after matching the corpus, rights, language coverage, normalization, similarity threshold and evaluation split. Kneading is categorized as Preparing in line with that taxonomy; verbs whose meaning depends on context remain explicit unknowns.

### Kitchen-R as a three-layer robotic benchmark

Kitchen-R (Kachaev et al., *Mind and Motion Aligned*, 2025) uses an Isaac Sim digital twin and 500+ complex language instructions for a mobile manipulator. It provides a VLM-based task-planning baseline and a diffusion-policy low-level control baseline, and separates three evaluation modes: planning-only, low-level control-only, and integrated whole-system execution. The paper describes roughly 2,700 mobile-manipulation trajectories. Reference: https://link.springer.com/article/10.1134/S1064562425700450

This suggests a useful evaluation boundary for Symtropy:

1. **Planning:** recipe/action grounding, task decomposition, resources, timing windows and plan validity.
2. **Low-level control:** manipulation/navigation success, recovery, perception robustness and timeouts.
3. **Integrated:** complete episodes with the planner and control policy connected, counting failures and human takeovers.

Our first Kitchen-R adapter is a simulation evaluation target only. Kitchen-R's mobile-manipulation benchmark does not certify food processing, heating, pressure-vessel operation, hygiene, taste or food safety. A simulated scene reaching its goal is not an instrumented meal-quality result or real appliance qualification. Planning, control and integrated success rates must be reported separately, on the same frozen episode IDs and environment configuration; no component may be assumed to execute perfectly.

### Benchmark gates

- Action grounding: exact core/group precision and recall, coverage, unmapped fraction, human overrides, language/locale stratification and rights/provenance.
- Planning: independent task-graph validation, temporal-window violations, resource conflicts, schedule feasibility and replan latency.
- Low-level policy: subtask success, collision/contact violations, navigation/manipulation completion, failure recovery and timeouts.
- Integrated: episode success, recovery, human takeover, execution time and reproducibility under fixed seeds.
- Physical kitchen: separately instrumented energy, water, temperature, accepted food yield, blinded texture/sensory quality and qualified food-safety procedures.

Do not collapse these dimensions into one score. The objective is to improve the whole chain without trading precision for coverage or simulation success for unsupported physical claims.


### Kitchen-R episode receipt and denominator rules

The benchmark now defines `KitchenRobotEpisodeReceiptV1` with a strict mode discriminator (`planning_only`, `low_level_control_only`, `integrated`). It binds each receipt to a source/rights status, instruction ID, deterministic seed, simulator and asset-manifest revisions, robot revision, initial-state digest, perturbations, planner and controller outcomes, recovery attempts, human takeover and final scene digest. The schema has synthetic fixtures for all three modes plus a negative case that rejects a planning-only episode that claims controller execution.

Every attempted episode stays in the relevant denominator. A plan that succeeds but whose low-level controller fails is not an integrated success. Human takeover, timeouts and unrecovered perturbations are retained. Scene restoration, collision incidence and recovery latency are separately reported. The receipts are explicitly simulated and cannot support a physical robot safety or food-quality claim.
