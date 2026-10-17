---
title: Lunar Resource-to-Production Chain Map
version: 0.1
status: research-roadmap
date: 2026-10-17
scope: lunar surface resource evidence, process chains, and early industrial priorities
evidence_class: mixed-official-research-and-explicitly-labeled-engineering-hypotheses
---

# Lunar Resource-to-Production Chain Map v0.1

## Executive position

Do not model the Moon as a list of resources waiting to be mined. Model it as a set of *site-specific material flows, process capabilities, energy constraints, maintenance dependencies, product specifications, and evidence gaps*. A substance's presence is not a reserve; a theoretically possible chemical conversion is not an operational process; an output stream is not a qualified product; a local-content percentage is not industrial independence.

The USGS lunar resource assessment framework explicitly separates resource type, certainty, and recoverability; only the portion technically recoverable and convertible to a commodity within mission and budgetary constraints qualifies as a reserve. It describes lunar regolith minerals as broadly accessible, while emphasizing that polar water ice still has major unknowns in form, quantity, quality, and distribution pending ground truth. See [USGS Circular 1507 summary](https://www.usgs.gov/publications/assessment-lunar-resource-exploration-2022).

NASA likewise notes that water and volatile resources are not fully characterized in location, form, concentration, distribution, and accessibility, and that many ISRU analog demonstrations have been short duration and under Earth conditions. Current NASA technology priorities connect surface power, oxygen extraction from regolith, in-space manufacturing, and advanced materials rather than treating them as isolated products. Sources:
- [NASA: Overview of In-Situ Resource Utilization](https://www.nasa.gov/overview-in-situ-resource-utilization/)
- [NASA: Lunar Enabling Infrastructure Accelerator, 8 September 2026](https://www.nasa.gov/news-release/nasa-calls-for-proposals-to-accelerate-lunar-surface-technologies/)
- [NASA: Lunar Surface Technology](https://www.nasa.gov/lunar-surface-technology/)

This document separates source-grounded resource context from proposed priority ordering. It is a research map, not a reserve report, mine plan, or economic forecast.

## 1. Evidence ladder: from occurrence to dependable supply

Every resource/product claim should state its highest passed evidence state. Progress is not automatic and may be site-, lot-, product-, and time-specific.

1. **Possible occurrence:** remote sensing, sample analogy, or scientific hypothesis suggests a material may exist.
2. **Site prospecting:** location-specific measurements constrain depth, grade/concentration, form, temperature, illumination, terrain, and uncertainty.
3. **Measured feed lot:** a physical batch is sampled and characterized with provenance and an accepted sampling error.
4. **Recovery demonstrated:** a declared process extracts the material from representative feed and closes mass/elemental balance.
5. **Qualified product:** the output meets a stated purity, mechanical, chemical, pressure, contamination, and safety specification for one named use.
6. **Durable pilot operation:** throughput, energy, maintenance, wear parts, duty cycle, degradation, and recovery are measured over a representative duration/environment.
7. **Reliable delivered supply:** storage, handling, transport, inventory buffers and availability meet a defined customer's service window.
8. **Recoverable reserve / viable chain:** uncertainty, extraction, energy, equipment, maintenance, logistics, environmental constraints, time and budget are evaluated for a specific plan.

A system must not skip states using words such as "available", "local", "ISRU-ready", "closed-loop", or "self-sufficient". Unknown values stay unknown; they are not zero cost or guaranteed capacity.

## 2. Resource families and likely early products

| Resource / input | What is supported | Candidate first products | Major gates and cautions |
|---|---|---|---|
| Solar energy | USGS identifies especially abundant sunlight on some high ridges near lunar poles; site and architecture still matter. | Power for excavation, communications, heating, processing, water systems and manufacturing. | Illumination time series, seasonal/terrain shadow, storage, transmission, thermal rejection, array cleaning/abrasion, mass and repairability. Power is a prerequisite in most chains, not a free input. |
| Bulk regolith | Surface loose-rock material is widespread, but engineering properties are site-specific. | Berms, shielding, landing-area preparation, roads/terrain works, bulk fill; possibly screened construction feedstock. | Excavation, traction, electrostatic/dust management, grain size, abrasion, transport throughput, settlement, sintering/bonding and product qualification. Bulk shielding is easier to qualify than pressure-retaining structures. |
| Oxygen chemically bound in regolith minerals | NASA's current infrastructure programme explicitly targets regolith oxygen extraction; oxygen is a product target, not already free gas. | Breathing oxygen, oxidizer feed and potentially process oxidant, after product/usage qualification. | Energy per kilogram of qualified oxygen, temperature, recovery, reactor consumables, tailings, maintenance, storage, purity and duty cycle. Mixed residues are not automatically refined metals. |
| Polar water / volatile-bearing material | Evidence supports polar volatile interest and likely water ice, but accessible concentration, depth, form, quality, and distribution remain uncertain. | Purified process/life-support water; as a separate allocation, electrolysis feed for hydrogen and oxygen. | Site-specific prospecting, excavation at low temperatures, thermal loss, capture efficiency, purification, extraction energy, cold-region logistics, power, storage and transport. Never treat a polar occurrence map as mineable reserves. |
| Fe/Al/Ti/Si-bearing mineral phases and residues | These elements and potential feedstock phases are relevant to lunar ISRU research. | After extraction/separation and qualification: metal powders, simple parts, alloys, glass/ceramics, wiring or other manufacturing feedstock as supported by the specific process. | Mineralogy, separation chemistry, reductants/electrodes, energy intensity, impurity control, alloying/heat treatment, precision machinery, tooling and product-quality validation. Do not credit these as outputs from oxygen extraction until separately recovered and measured. |
| Glass, ceramics, refractory and sintered material | Regolith-based construction and additive construction are active development areas. | Tiles, blocks, landing/traffic surfaces, thermal-resistant bulk components, radiation-shield structures. | Particle preparation, thermal energy, melt/sinter properties, cracking, dust, thermal cycling, mechanical testing, contamination and repair procedures. Do not label bulk aggregate as structural-grade. |
| Recycled water and consumables | Recycling can reduce imported throughput if recovery, contamination and equipment maintenance are accounted for. | Recovered water, oxygen and selected reusable components/materials. | Recovery fraction, contaminants, filter/catalyst/seal replacement, degradation over repeated cycles, energy use, microbial/chemical control, and waste residuals. A recycled stream is not automatically virgin-grade. |
| Carbon-, nitrogen-, phosphorus-, sulfur-bearing and high-purity speciality inputs | Their site-specific abundance/accessibility and the capability to produce high-purity forms must be established; the chain must not assume they are plentiful because bulk regolith is abundant. | Life-support/biosphere nutrients, polymers/chemicals, catalysts and speciality materials only when feedstock and product evidence support them. | Local concentration, recovery, competing uses, impurity, inventory buffer, recycling closure and supply dependence. Plan selective recycling and resupply until site-specific evidence demonstrates otherwise. |
| Electronics, high-purity semiconductors, sensors and precision control components | They are multi-stage industrial products requiring many specialized inputs and high process control; bulk silicon-bearing regolith does not imply chip manufacturing capability. | Initially, repair/refurbishment, modular replacement, connector/cable repair, enclosure/shielding, and limited simple electronics assembly where parts are qualified. | High-purity feedstock, dopants/chemicals, vacuum/clean processing, precision equipment, metrology, yield, radiation qualification, and upstream tool maintenance. Model them as recurring imports unless the full chain is actually demonstrated. |

## 3. Production chains worth benchmarking first

### Chain A — Site characterization -> power -> bulk regolith utility

**Sequence:** identify a candidate worksite -> characterize illumination, slopes, temperature, and regolith properties -> qualify power and storage -> excavate/screen/move regolith -> produce berm/shielding or landing-area aggregate -> inspect/maintain machinery.

Why first: the product can tolerate lower material purity than metals, electronics, or pressure vessels; the chain also creates useful infrastructure and enables later processes. Its primary metrics are mass moved per energy, verified throughput, power availability, equipment wear, downtime, dust exposure, and accepted product quality—not just kilograms dug.

Do not compare sites using sunlight duration alone. Track where power is available at the hours and locations demanded, energy-storage losses, power distribution, thermal rejection, and maintenance requirements.

### Chain B — Regolith -> oxygen -> qualified use

**Sequence:** site/sample characterization -> excavation/beneficiation -> oxygen-extraction process -> gas capture and purification -> storage/conditioning -> delivery to life support or a propellant customer. Track all non-gas residue and the process's consumables.

The same mass stream may contain candidate Fe/Al/Ti/Si-bearing residues, but the benchmark must not count refined metal as an oxygen-extraction coproduct. A separate separation route must prove its own input/output balance, recovery, purity, energy, tooling and product specification.

High-value metrics: kilograms of *qualified* oxygen per tonne of actual feed; energy per qualified kilogram; recovery interval by feed class; reagent/electrode/consumable usage; residual composition; product purity; availability and replacement interval; and cost per delivered kilogram.

### Chain C — Volatile prospecting -> water or electrolysis products

**Sequence:** geophysical/orbital evidence -> site prospecting and sampling -> excavation -> thermal extraction and vapour capture -> purification -> choose a use: (1) deliver water or (2) electrolysis into hydrogen and oxygen -> condition/store/deliver each product.

The use choice is a shared-resource allocation decision: the same recovered water cannot be simultaneously sold as water and consumed as electrolysis feed. Use an explicit allocation group/inventory ledger.

High-value metrics: concentration/depth uncertainty; accessible kilograms per excavation hour; extraction and purification recovery; thermal/electrical energy per kilogram; losses; purity; storage/boil-off; maintenance; site-to-consumer logistics; and delivered-service value. Run value-of-information calculations to select the next survey or sample that most reduces decision loss—not simply the next easy measurement.

### Chain D — Qualified mineral stream -> simple metal/feedstock -> repair/spares

**Sequence:** characterize residue/mineralogy -> separate a specific metal-bearing stream -> reduce/refine -> test purity and mechanical properties -> form qualified stock (powder, wire, bar, or feedstock) -> manufacture simple parts -> inspect -> return parts to service.

Start with repair-critical, lower-complexity parts whose material specifications are tolerant and whose failure consequences are understood. Do not jump from "metal detected" to "local machine shop"; tooling, heat treatment, quality assurance, seals, bearings, control electronics and machine maintenance may remain imported dependencies.

### Chain E — Waste and used components -> recovered feedstock -> qualified reuse

**Sequence:** collect and segregate waste -> assay contaminants -> disassemble/condition -> recover selected materials -> quality test -> return accepted material to one named process; safely store/dispose of rejects.

Benchmark the full repeated loop over several cycles. Track mass recovery, contamination accumulation, energy, filter/catalyst replacement, degradation, and loss of product grade. A first pass yield is not evidence of a sustainable closed loop.

## 4. Initial prioritization hypothesis

This is a proposed engineering sequence to test, not a claim that every site or architecture should follow it.

1. **Survey and model first.** Improve knowledge of site resources, illumination/power, slope, temperature, geology, process conditions and transport distances. A wrong site assumption can strand every downstream investment.
2. **Exploit bulk utility before high purity.** Start with screening, excavation, berms, shielding, and landing/road works where the accepted product specification is achievable.
3. **Demonstrate one useful commodity end-to-end.** Choose either regolith oxygen or a prospect-confirmed volatile/water pathway based on site, power, mission demand and measured uncertainty. A lab yield alone is not a deployed supply.
4. **Close repair and spare loops.** Establish diagnostics, reversible modular replacements, repair, part recovery, and local manufacture of selected simple components. Measure whether this reduces outage time and Earth-sourced mass.
5. **Add metals, glass and advanced manufacturing incrementally.** Require independently tested purity, energy, mechanical performance, product repeatability, tool maintenance, and yield before counting the products as available.
6. **Defer full semiconductor, sophisticated chemical and biosphere closure claims.** Preserve supply and recycling strategies; promote these capabilities only when their complete upstream toolchain, feedstocks, consumables, and maintenance needs are shown to close for a declared horizon.

The best order depends on the customer service and site. A remote science outpost, propellant depot and construction base have different demand vectors; one universal "best resource" ranking hides those differences.

## 5. Benchmark gates

- **Mass/elemental conservation:** zero unexplained imbalance; explicitly account for process gas, contaminants, rejects, losses, and recycled streams.
- **Evidence integrity:** zero inferred-resource-to-reserve promotions without site, recoverability, and constraint evidence; exact source/site/epoch/model lineage for each input.
- **No double counting:** each feedstock lot has an allocation identity; alternate routes cannot use the same mass twice.
- **Energy feasibility:** process demand plus auxiliaries and storage/thermal losses must fit the site energy budget under required operating periods, not only nominal sunny hours.
- **Product truth:** zero kilograms counted as firm product until process, product spec, and delivery/handling gates pass.
- **Coproduct discipline:** zero metal/refined-material credit until downstream separation, recovery, purity and quality are verified independently.
- **Process robustness:** report distribution/range of recovery, throughput, duty cycle, consumable use, maintenance interval and product quality across representative feeds and operating conditions.
- **Service economics:** compare cost per delivered qualified unit and service availability over a declared horizon, with Earth-sourced replacement mass, outage/recovery cost and end-of-life obligations.
- **Decision value:** measure whether a proposed survey or experiment changes the selected route or reduces expected decision loss; stop paying for measurements that do not change decisions.
- **Reproducibility:** every benchmark result binds exact inputs, assumptions, source/code revision, process version, units, test configuration and independent reference result.

The current Rust reference `tools/space-economy-bench/examples/resource_chain_reference.rs` encodes a small synthetic contract for four alternative routes. Its fixture values deliberately do not estimate lunar grade, reserves, energy intensity, recovery rate or process readiness. See [BENCH-SPACE-008](https://github.com/Luminous-Dynamics/symtropy/issues/1803).

## 6. Destination-specific extensions beyond the Moon

These are candidates for later, separate modules—not inputs to the current lunar mass-balance fixtures. Do not reuse lunar feedstock grades, power assumptions, or recovery values for these bodies.

### Mars: atmospheric CO2 to oxygen

NASA's MOXIE demonstration on Perseverance operated on Mars, producing oxygen in 16 runs and 122 grams total; its best production was 12 grams per hour at 98% purity or better. NASA presents this as technology demonstration evidence, not a full-scale propellant plant. Source: [NASA/JPL, MOXIE completes Mars mission, 6 September 2023](https://www.nasa.gov/solar-system/nasas-oxygen-generating-experiment-moxie-completes-mars-mission/).

Candidate chain: atmospheric intake and dust control -> compression/flow management -> high-temperature electrochemical processing -> oxygen purity/flow verification -> storage/conditioning -> life-support or oxidizer user. Track the carbon monoxide coproduct, thermal/power duty, intake dust, cell wear, and the gap between a demonstrated instrument and industrial-scale throughput. The benchmark should scale demand from grams/hour to a named mission requirement only after measured energy, duty cycle, lifetime, and reliability inputs are available.

### Carbonaceous asteroids: water/volatile and carbon-bearing streams

NASA reports that returned Bennu material contains evidence of carbon and water, and NASA TechPort describes a completed CAVoR concept for thermally extracting water and water bound to clay minerals from carbonaceous chondrite material, potentially producing hydrogen-rich syngas and downstream products. Sources: [NASA on Bennu sample findings](https://www.nasa.gov/news-release/nasas-bennu-asteroid-sample-contains-carbon-water/) and [NASA TechPort: CAVoR](https://techport.nasa.gov/projects/89816).

Candidate chain: prospect/characterize a specific target -> anchoring and sample collection in ultra-low gravity -> thermal extraction -> water/volatile capture -> separate streams by measured composition -> process into water, hydrogen-rich gas or other products only if yield/purity justify it -> store and transfer to a destination with real demand. A laboratory sample establishes composition of that sample; it does not prove a mineable deposit, representative target-wide grade, economic recovery, or a viable return/transfer system. Target-specific navigation, anchoring, thermal control, collection and transfer can dominate a simple "resource content" calculation.

### Cross-body rule

Every body and orbit receives its own geometry, energy, thermal, material, gravity, environmental, transportation, and evidence model. The portable abstraction is the **resource-to-qualified-service chain**; the physical coefficients do not transfer automatically. Current PR #1736 only executes synthetic lunar pathway fixtures and does not model Mars or asteroid operations.

## 7. Exploration as a decision loop: select the next measurement by value

The resource chain must feed back into exploration. Surveying everything at maximum resolution is neither affordable nor necessarily useful. Each prospective assay should be evaluated against a named route decision: *what action can this result change, what loss could be avoided, and is the expected improvement worth the measurement's cost and mission constraints?*

The new reference `tools/space-economy-bench/examples/resource_assay_selection_reference.rs` embodies this contract with exact integer/rational EVSI calculations. The scenario is synthetic, but it checks four core principles:

- The verified direct assay is worth purchasing because a positive result supports a local-oxygen build while a negative result changes the decision to import.
- The lower-resolution and uninformative surveys do not change the preferred action and are not purchased at positive cost.
- An unverified assay's claimed accuracy contributes zero accepted information value.
- Expected value of sample information (EVSI) cannot exceed expected value of perfect information (EVPI) in the same decision model.

This makes resource exploration decision-aware: the best next measurement is not the measurement that produces the most bytes, highest nominal resolution, or broadest map; it is the measurement with verified ability to reduce decision loss, unblock a required evidence gate, or change the optimal route. The calculation must include the alternatives available if no survey is purchased. A survey never has positive decision value if its result cannot change the choice, though it could still have scientific value outside that specific decision model; that separate value should be stated rather than smuggled into the economic calculation.

For real resource planning, candidate measurements might include depth/grade samples, mineralogical assay, volatile concentration, extraction-yield experiments, site illumination/power measurements, or wear/contamination tests. Those observations have different spatial coverage, destructiveness, lead time, uncertainty and ability to affect the production route. A next-stage planner should include measurement cost, sample/lander time, power, location access, site-to-site correlation, false-positive/negative rates, and downstream option value. Use sequential experiment design: after each result, recalculate the best next measurement against the updated posterior and the new decision alternatives.

The toy assay priors, sensitivity/specificity, payoffs and costs are not actual lunar data. Calibration requires independent survey performance, site-specific priors, a documented likelihood model and held-out validation. Tracking issue: [BENCH-SPACE-009](https://github.com/Luminous-Dynamics/symtropy/issues/1813).

### Adaptive follow-up: stop where another sample is not worth it

The one-shot assay contract is now complemented by `tools/space-economy-bench/examples/sequential_resource_exploration_reference.rs` and [BENCH-SPACE-010](https://github.com/Luminous-Dynamics/symtropy/issues/1823). In this synthetic example, a 70%/70% low-cost screen is followed by a 90%/90% assay only when the first result makes that extra test worth buying. The positive branch yields conditional EVSI 13 against cost 20 and stops; the negative branch yields conditional EVSI 47 and continues. The adaptive policy uses 11 expected survey-cost units, versus 20 for the high-resolution assay at every site, and has net expected value 67.5 versus 65 under the direct-assay alternative.

These figures are exact outputs of a deliberately constructed decision model, not real mission economics. The next practical research increment should add independent failure modes for spatial sample bias, shared calibration errors, instrument dependence, access/time/power constraints and a held-out probability-calibration set. Until then, nominal sensitivity/specificity must be treated as fixture assumptions, not empirical evidence. An instrument's information value should be assessed against the production decision and alternatives—not against data volume alone.

### Correlated evidence and spatial representativeness

The separate reference `tools/space-economy-bench/examples/correlated_assay_evidence_reference.rs` adds explicit latent states for shared calibration and an unrepresentative sample patch. The fixture assumes 20% of sampled patches are unrepresentative and yield results uninformative about the site-wide resource; conditional on a representative patch, a pair sharing a calibration domain has a 50% shared-calibration mode. The model emits and checks exact joint probabilities rather than multiplying marginal likelihoods.

For this benchmark, one assay A yields gross expected decision value 61 and net value 57 after cost. Repeating with assay B under the correctly modeled shared/spatial dependence raises gross value only to 66.7; after total cost 10 its net value is 56.7, so it is inferior to the one-assay option. A naïve independence calculation predicts gross value 70.62 and a double-positive posterior of 89.01%, while the explicit joint likelihood yields 81.08%. The naïve output receives no portfolio-selection authority. A complementary assay on a separately modeled patch and calibration domain yields gross value 75.66 and net value 66.66 after cost, so it wins the finite fixture's exhaustive comparison.

The model enforces a two-assay/two-sample/four-power-unit/two-hour budget and fail-closed controls for missing dependence metadata, uncalibrated instruments, unmodeled representativeness, missing results and independence claims contradicted by shared domains. These are synthetic likelihoods and costs, not measured lunar geology or proof that two real instruments are independent. Physical work must estimate sampling bias, common-mode interference, calibration covariance and site-to-site dependence from representative, held-out observations.

Research context: [NIST correlation in measurement uncertainty](https://www.nist.gov/publications/correlation-uncertainty-measurement-discussion-state-art-techniques) and [NIST propagation-of-error guidance](https://www.itl.nist.gov/div898/handbook/mpc/section5/mpc55.htm). Follow-up: [BENCH-SPACE-011](https://github.com/Luminous-Dynamics/symtropy/issues/1824).

### Model uncertainty: choose the policy objective explicitly

The reference `tools/space-economy-bench/examples/robust_exploration_model_uncertainty_reference.rs` adds a three-model uncertainty set (90%/90%, 70%/70%, 50%/50%) and compares three policies. Under this exact synthetic fixture, buying the assay has nominal net value 65 but worst-case value 30. Stopping at the baseline has worst-case value 50. A three-unit calibration gate followed by an assay only in the high-quality regime has value 62 in the optimistic model and 47 in the other two.

Therefore the nominal optimizer buys the assay, maximin stops, and minimax regret chooses calibration-before-commitment. Their maximum regrets are respectively 20, 15 and 3 units. The calibration gate is an idealized perfect model-classification oracle for benchmarking the decision rule, not a physical proposal. Its purpose is to make an important design choice explicit: risk-neutral, worst-case and regret-minimizing objectives can disagree even when they share identical model inputs.

Future site work should replace the discrete fixture with evidence-grounded likelihood bounds or ambiguity sets, propagate uncertainty through production-chain mass/energy/cost constraints, and report how the preferred route changes across that set. Do not silently compress an interval into a midpoint or claim that a robust choice is uniquely optimal without naming the objective.

Research connection: [Robust Bayesian Decision Making under Adversarial Uncertainty](https://proceedings.mlr.press/v337/harikumar26a.html) (UAI 2026) studies experiment design focused on decision stability under plausible model misspecification. The Symtropy implementation is an independent, small finite reference only; it does not reproduce that paper's algorithms or empirical results. Follow-up: [BENCH-SPACE-012](https://github.com/Luminous-Dynamics/symtropy/issues/1837).

### Calibration as an adaptive experiment-design tree

The current reference `tools/space-economy-bench/examples/decision_value_calibration_experiments_reference.rs` treats calibration tests as decision-relevant experiments in their own right. Test A is more informative but costs 1.00; test B costs 0.25 with different ability to distinguish the high/moderate/uninformative instrument regimes. A low-cost uninformative control verifies that measurements which cannot change the production decision should not be purchased merely to add data.

Under the explicit synthetic model weights and downstream assay values, the best finite policy is B first, then A only after B-positive. It achieves net expected value 51.90, against 51.4333 for the reverse A-first, B-after-positive plan. Both yield the same gross decision value, but the B-first branch is less common (40% vs A-positive at 46.67%), so expected calibration spend is lower. In both orders, spending on the second calibration after a negative first result is not useful in the declared fixture because it does not change the optimal downstream action.

The point is a general decision-design rule, not a claim about real space-resource instruments: select the next calibration by its conditional expected improvement to the actual resource-production decision, after likelihood uncertainty and test cost. Recompute posterior-mean, maximin and minimax-regret choices at every branch; report the objective explicitly. NIST notes that uncertainty in a calibration curve and uncertainty in a future corrected measurement both contribute to calibrated-value uncertainty; see [NIST's calibration uncertainty guidance](https://www.itl.nist.gov/div898/handbook/mpc/section3/mpc367.htm). For a relevant experimental-design background, see [Hu, Ao and Mahadevan (2017)](https://www.sciencedirect.com/science/article/pii/S0045782516302274).
