# Kitchen Preservation and Appliance Engineering V1
## Cold storage, dry pantry, cooking, cleaning, and repair as one household capability system

**Status:** research/design brief; no refrigerator, freezer or pantry prototype has been built or physically tested under this programme.  
**Parent:** [KITCHEN-SYSTEMS-001 #2313](https://github.com/Luminous-Dynamics/symtropy/issues/2313)  
**Workstreams:** [APPLIANCE-COLD-001 #2314](https://github.com/Luminous-Dynamics/symtropy/issues/2314), [DRY-STORE-001 #2315](https://github.com/Luminous-Dynamics/symtropy/issues/2315)  
**Existing work to reuse:** [APPLIANCE-QUAL-001 #1805](https://github.com/Luminous-Dynamics/symtropy/issues/1805), [APPLIANCE-PORTFOLIO-001 #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825), [APPLIANCE-MORPHOLOGY-001 #1893](https://github.com/Luminous-Dynamics/symtropy/issues/1893), [KITCHEN-OPS-001 #2227](https://github.com/Luminous-Dynamics/symtropy/issues/2227).

## 1. Thesis: engineer food-safe capability, not appliance features

A kitchen should be evaluated as a connected service system:

`food acquired → stored appropriately → prepared → cooked → served → cleaned → scraps/packaging recovered`

The strongest outcomes depend on the full chain. A high-efficiency refrigerator is not a win if it fails to keep product temperatures within the declared envelope during routine use. A sealed dry-food bin is not a win if the incoming food is too damp, the closure traps residue, or users cannot inspect and clean it. A dishwasher energy saving is not a win if sanitation, drying, reliability or repair cost is degraded.

The product's true unit of value is **reliably available, safe, usable food per total resource and lifecycle burden**, with food safety and other hard constraints kept separate from economic trade-offs.

This is a design/measurement principle, not a universal scalar score. Preserve the underlying metrics and disagreement between needs.

## 2. Industry baselines: what they measure and what remains open

### Refrigerators and freezers

ENERGY STAR says certified refrigerators are about 9% more efficient than the US federal minimum, and identifies compressor efficiency, insulation and temperature/defrost mechanisms as key elements. Its public criteria currently require standard/compact refrigerator and freezer categories to use 10% less measured energy than the relevant federal minimum. Those are useful minimum/label reference points; a serious development effort should also compare strong commercially available products of comparable volume and intended climate/use.

- Consumer guidance: https://www.energystar.gov/products/refrigerators
- Criteria: https://www.energystar.gov/products/refrigerators/key_product_criteria
- US DOE test procedure: https://www.energy.gov/cmei/buildings/refrigeration-products

A standardized energy test alone does not describe every household duty cycle. The experimental matrix should add high ambient temperature, door-opening disturbance, warm-load pull-down, defrost, degraded seals and power interruption as separately declared cases. Never combine these with the official procedure into a made-up certification result.

### Thermal storage / power resilience

A 2025 review describes phase-change materials (PCMs) as a possible route to store renewable energy and shift domestic refrigeration loads, but discusses adoption barriers and the need for consistent comparison criteria. A 2025 refrigerator study reports energy savings up to 7.4% for a particular heat-storage material in a natural-draft condenser and does not recommend that configuration for variable-speed compressors. This is a valuable warning that the best add-on depends on the base architecture, not evidence that PCM is universally beneficial.

- Review: https://doi.org/10.1016/j.applthermaleng.2025.125421
- Specific heat-storage study: https://doi.org/10.1016/j.ijrefrig.2025.02.007

Test PCM as a distinct candidate variant. Measure energy, temperature excursion, holdover time, recharge time, usable-volume penalty, added thermal resistance, mass, corrosion/containment risk, maintenance and cost. Thermal holdover and efficiency are different endpoints: a design might improve outage resilience while consuming more energy in normal operation.

### Dry food and staple storage

Southern African stored-product literature highlights insects and other pests, and moisture-associated microbial risks, as continuing staple-storage concerns. Hermetic bins/drums/bags are promising options for properly managed dry staples. A meta-analysis found substantial reductions in several pest/damage/weight-loss outcomes in hermetic compared with conventional grain-storage bags, while noting that mycotoxin outcomes were not assessed often enough to generalize strongly.

- Southern African stored-product protection: https://doi.org/10.4324/9780429401701-16
- Hermetic versus conventional storage meta-analysis: https://doi.org/10.1002/jsfa.12934

The key boundary: airtightness does not dry wet grain, prove mycotoxin safety, make all food safe, or suit every fresh produce item. The container must match the food, its starting condition and use pattern.

### Food-temperature safety

The US FDA uses refrigerator temperature at or below 4°C and freezer temperature at or below -18°C as consumer references, emphasizing measurement using appliance thermometers. These are reference points for the research protocol, not a substitute for the actual food category, market jurisdiction or relevant professional food-safety requirements.

- https://www.fda.gov/consumers/consumer-updates/are-you-storing-food-safely
- https://www.fda.gov/food/buy-store-serve-safe-food/refrigerator-thermometers-cold-facts-about-food-safety

Do not infer a food-safety outcome from average cabinet-air temperature, a label, a model prediction, or the fact that an appliance remains powered.

## 3. Refrigerator/freezer architecture: candidate families, no presumed winner

### Candidate A — mature high-efficiency vapor compression

Optimize insulation continuity, gasket pressure/uniformity, door geometry and leakage, heat exchanger effectiveness, condenser airflow, compressor/control envelope, fan parasitics, defrost control and sensor placement. Treat cabinet geometry and usable volume as part of efficiency, not only nameplate compressor efficiency.

Use a well-defined commercial benchmark from the strongest suitable local/market reference set, not a weak baseline chosen to make the proposal win.

### Candidate B — zoned temperature and humidity control

Compare single-loop/single-evaporator, dual-evaporator or other compartment-control arrangements where suitable. Measure spatial temperature, humidity, fan/valve/control energy, defrost interaction, cost, noise, diagnostic complexity and fault consequences. Extra thermal zones can improve independent control but may add components, leakage paths and failure modes.

### Candidate C — thermal buffer / cold battery

Evaluate PCM or other thermal buffers at a specified, professionally assessed location. Model and test latent sensible capacity, charge/discharge rate, containment, thermal conductivity, volume and temperature range. If the product is designed for outages, measure time-to-threshold with controlled ambient, initial load, starting temperature, door state and sensor placement. Report the entire scenario; never advertise a single universal holdover hour figure.

### Candidate D — grid/PV/microgrid-aware operation

Optional control can shift compressor load toward declared energy availability while maintaining the required product-temperature envelope. The fallback is standalone local thermostat control. A connected network, subscription, account, cloud inference, or proprietary application must not be necessary to keep food cold.

Compare the controller against a simple fixed set-point baseline. Include forecasting error, stale signal, sensor faults, loss of network, unexpected door openings and power restoration. Do not optimize grid cost in a way that exceeds the food-temperature safety boundary.

### Repair-first construction

Create service boundaries for door gaskets/hinges, drawers/shelves, accessible fans, lighting, temperature sensors, control panels and power/control modules. Show exact isolation, diagnosis, part compatibility and post-repair verification. Avoid destructive bonding or embedded wear parts when a replaceable assembly is technically justified.

Sealed refrigerant circuits, mains-live power electronics, pressure systems and combustion systems are hazardous service domains. Ordinary-user tasks must remain clearly separated from tasks requiring qualified technicians and applicable recovery, leak-checking and electrical-safety procedures. This project does not prescribe DIY sealed-loop repair.

## 4. Refrigerator/freezer benchmark protocol

Freeze candidate identity, revision, cabinet volume, usable volume, compartment design, intended climate class, load profiles, test procedure, meter boundary, setpoint and randomized ordering before testing.

**Required profiles:**
1. Reproduce the applicable standard/label test as closely as accessible equipment, method and scope allow. State all deviations; do not claim certification equivalence.
2. Stable ambient, empty and representative loaded state.
3. Door-opening disturbance with a fixed duration/frequency/angle and controlled recovery condition.
4. Warm-load pull-down, defined by load mass/type/starting temperature and safe test method.
5. High-ambient operation at justified points for the intended market.
6. Defrost and recovery over representative cycles.
7. Declared power interruption/recovery with exact starting state and no door opening, then separately declared realistic-use door events.
8. Degraded but serviceable states, such as controlled gasket leakage or fan/filter obstruction, where safe and reproducible.
9. Fault-injected sensor/control cases using an isolated safe fixture or simulation; no unsafe live manipulation of the appliance.

**Metrics:**
- energy use and peak demand at a declared boundary;
- usable storage volume, not merely gross cabinet volume;
- temperature distribution and warmest-point exposure duration;
- load pull-down/recovery time and defrost behavior;
- outage time-to-declared temperature threshold under exact conditions;
- energy per declared useful storage/service profile, with transparent normalization;
- noise, vibration, condensation and maintainability;
- component and repair cost, diagnosis and hands-on time, waiting time, downtime and recurrence;
- post-repair functional, thermal and energy verification;
- lifecycle/resource claims only with stated scope, source evidence and uncertainty.

Use calibrated thermal loads/probes and a documented sensor layout. Air temperature, product-simulant temperature, food-core temperature and microbial outcome are different evidence levels. A temperature trace by itself does not prove microbiological safety.

## 5. Dry-store architecture: a family of containers, not one magic box

### Profile 1 — suitable dry staples

For correctly dried grains, rice, legumes and similar staples: pest barriers, a repeatable hermetic closure where suitable, removable gasket, cleanable refill port, inspection path, stack strength, stable base, repairable latch and visible stock/lot identity.

### Profile 2 — flour and powders

Use a cleanable, dust-contained interface; reduce spill and residue; evaluate dosing repeatability, clumping and moisture ingress. Avoid internal corners or mechanisms that retain old stock or make cleaning impossible.

### Profile 3 — aroma/light/moisture-sensitive ingredients

For coffee, tea, spices and selected ingredients, evaluate barrier type and turnover; opaque containers or separate cartridges may help depending on the food. Do not assert a universal benefit for vacuum or oxygen absorbers without product-specific trials and safe-use guidance.

### Profile 4 — fresh produce

Use airflow and storage conditions appropriate to the produce. Produce-specific respiration, moisture and ethylene behavior differ, so the dry-staple hermetic profile must not be applied indiscriminately.

### Profile 5 — seed or emergency reserve

Separate this from everyday food use where appropriate. Define whether germination/viability, long-term nutrient/quality preservation, pest protection, or only household rotation is the intended endpoint. A “reserve” claim needs a duration and condition boundary.

### Ergonomic and repair design

Use modular volume sizes and a common cabinet/shelf interface where this reduces cleaning/repair effort without compromising seal integrity. A container should be removable for cleaning and inspection. Consider a large reserve bin plus a smaller daily-use decanting vessel so each meal does not expose the full reserve to humid air. Offer simple labels, stock-rotation windows and tactile/mechanical indicators. Sensors are optional; the basic store should work unpowered.

## 6. Dry-store benchmark protocol

Freeze food category, initial moisture/condition, lot and mass, container material and revision, container volume/fill fraction, ambient temperature/humidity, open-close schedule, closure procedure, storage duration, cleaning procedure and endpoint before testing.

Test:
- seal integrity and repeated open/close cycles;
- moisture ingress or product-moisture change under declared humidity/temperature cycles;
- pest exclusion using an appropriately contained, ethical and safe testing method;
- structural durability under handling, drops and stacking relevant to the intended use;
- refill/dispense loss, spills and portion repeatability;
- cleanability and residue traps after a declared cleaning cycle;
- access, opening force, reach, readability and stock rotation for representative users;
- relevant product outcomes such as mass loss, visible pest/physical damage, quality, seed germination if relevant, or competent laboratory analysis where making a mycotoxin claim.

Do not deliberately distribute infested or contaminated materials. Use appropriate containment, safe handling and waste-disposal procedures. Synthetic fixtures can qualify schema logic but cannot prove real-world pest exclusion, shelf life or food safety.

**Metrics:** moisture change, seal failure rate, infestation/damage, mass loss, product-specific quality, cleaning time, residual contamination, useful capacity per footprint, refill/spill loss, opening force, gasket/latch service time, replaceable-part availability and full lifecycle cost. Keep outcomes separate; airtightness alone is not a food-preservation score.

## 7. The rest of the kitchen: high-leverage engineering portfolio

### P0 — preservation
Refrigeration/freezing plus food-profile-aware dry storage; initial rationale is direct food loss reduction, resilience and everyday utility. Validate the magnitude with household-specific baselines instead of assuming it.

### P1 — cooking and extraction as a coupled system
Compare induction/electric cooking, oven/convection/steam/pressure options for declared tasks. Measure energy to a defined meal outcome, time, cooking quality, usable capacity, cleanability, repair and ergonomics. Design the hood/extractor and make-up air together: better capture can also affect room heat loss, drafts, noise and cooking comfort. Keep installation, electrical and ventilation compliance under competent review.

### P1 — washing, sanitation and drying
Benchmark dishwashers and sink workflows against the same cleaning/hygiene result, with water, energy, detergent, drying, cycle time, sound, maintenance, residue and repair events. Explore heat recovery only when whole-process measurements show benefit and the design preserves applicable sanitation requirements.

### P2 — food preparation
Accessible, cleanable work surfaces; scales, cutters, mixers, grinders and food processors with replaceable wear parts; ergonomics, acoustic load, task outcome, spare availability and repair time. Match the equipment to household and professional tasks, not to feature count.

### P2 — drying and fermentation
Modular dehydration, solar-assisted drying where climate/process allow, temperature-controlled fermentation and proofing, and humidity/time control. Separate food-specific operating recipes from actual measurements and competent food-safety authority; drying does not automatically eliminate toxins or make every product shelf stable.

### P2 — water and material flows
Point-of-use filtration only for an identified water-quality need; filter-life/flow/contaminant claims require testable evidence. Design waste separation for food scraps, packaging, oil/grease and compost where appropriate. Measure preventable edible waste separately from unavoidable residues; do not call waste “recovered” until a real collection or processing path exists.

## 8. Whole-kitchen benchmarking

Use household and professional kitchen profiles separately. Freeze household size/use pattern, meals and batch sizes, climate, electricity/water reliability and tariffs where relevant, appliance set, food mix, accessibility, maintenance capacity and intended life. For each candidate, report:

- food safety/quality outcomes as their own qualification dimensions;
- edible mass wasted and causes, with a consistent denominator;
- kWh, water, consumables and time per defined meal/service;
- food preservation by category and storage conditions;
- maintenance, repair attempts, waiting for parts and downtime;
- total cost boundaries and sensitivity;
- noise, heat, access, ergonomics and user agency;
- uncertainty, failure cases and situations where a simpler product is better.

Do not invent savings from synthetic digital-twin output. A simulator's value is to select experiments and expose infeasible designs before fabrication; field/bench measurement establishes real-world performance.

## 9. Proposed acceptance targets (not achieved results)

1. 100% of measured values have units, protocol/profile ID, source/evidence class and stated uncertainty or an explicit missing-uncertainty flag.
2. 100% of expected comparison cells are measured or marked missing, not applicable, unsupported or failed with a reason.
3. Zero silent missing-value imputation and zero safety gate averaged into a composite score.
4. Report independent physical unit count separately from repeat runs, door events and repeated time points.
5. A proposed appliance can only be called superior after preregistered primary outcomes, adequate sample-size rationale and uncertainty analysis show an advantage against strong matched references with no unacceptable regression.
6. A repair is counted as successful only after the declared post-repair functional test passes; a repair-order status is not physical evidence.
7. No “hours of outage protection”, “years of life”, food-safety, waste-reduction or environmental claim without the conditions, outcome, method and evidence required for that claim.
8. Publish negative results and the tasks/profiles where conventional appliances remain the better option.

## 10. Work sequencing and evidence gates

1. Start with requirements, stakeholder tasks, market-available comparators and the measurement protocol.
2. Build a low-cost non-hazardous fixture or dry-storage demonstrator first; qualify its measurement method before ranking products.
3. Use synthetic models to screen refrigeration geometries, insulation, control or PCM variants; validate finalists with a physical bench under matched profiles.
4. Build refrigeration prototypes only with qualified engineering support for electrical and sealed refrigerant hazards; independently review food-contact/material choices, electrical/mechanical protection and relevant product-market requirements.
5. Run limited field trials only after bench gates pass, with explicit participant consent, privacy-minimizing instrumentation and a failure-response plan.
6. Link component replacement and restore-cost evidence through the existing Symtropy repair/lifecycle stack and Mycelix provenance where useful. Do not clone those systems in this programme.

## 11. Cross-project ownership

- **Symtropy:** appliance/layout architecture, bounded thermal/resource simulation, operating-profile orchestration, maintainability and repair-task integration.
- **Symthaea:** independent experiment design, evidence-class enforcement, uncertainty-aware comparisons and claims ceilings. Model predictions never substitute for measured temperatures, energy or repair outcomes.
- **Mycelix:** optional permissioned supplier/lot provenance, replacement-part identity, repair/contributor attribution and service-history governance. It is not a substitute for physical qualification.
- **Sol-Atlas:** regional climate, electricity reliability, water availability, tariffs, service capacity, supply chains and material/end-of-life routes where sourced regional evidence changes the design decision.

Reuse the existing lifecycle, repairability, EVSI, provenance and kitchen-scheduling owners. Do not introduce a second universal optimizer, work-order system, provenance ledger, refrigerant service controller or food-safety authority.

## 12. Decision principle

The winning kitchen is not the one with the most connected appliances. It is the one that reliably preserves ingredients, supports excellent cooking, cleans effectively, uses less total resource for the declared service, remains repairable and accessible, and preserves the user's decisions and craft. The programme must be capable of concluding “no meaningful improvement demonstrated.”
