# Kitchen Station Engineering V1: Sink + Drying as One Resource System

**Status:** research and benchmark specification; no physical prototype or product superiority demonstrated.  
**Owner:** Symtropy engineering programme.  
**Parent:** [KITCHEN-SYSTEMS-001 #2313](https://github.com/Luminous-Dynamics/symtropy/issues/2313).  
**Task:** [KITCHEN-STATION-001 #2324](https://github.com/Luminous-Dynamics/symtropy/issues/2324).  
**Related work:** [KITCHEN-PRESERVATION-ENGINEERING-V1 PR #2316](https://github.com/Luminous-Dynamics/symtropy/pull/2316), [KITCHEN-OPS-001 #2227](https://github.com/Luminous-Dynamics/symtropy/issues/2227), appliance portfolio/lifecycle #1825 and morphology #1893.

## 1. Thesis: optimize a completed task, not a component rating

A sink is usually purchased as a basin/faucet combination; a drying rack is an accessory. Together, they determine how much potable water and hot-water energy a household uses, how much work is repeated, where droplets land, how clean items drain, how much counter space is occupied, and how difficult cleaning and repair become.

The research unit should therefore be **a completed, declared task with a defined endpoint**. A lower faucet flow rate alone does not establish water savings: a user might rinse for longer, reclean a pot or abandon a task. Likewise, an apparently dry plate does not establish that every item in a loaded rack is dry or sanitized.

Report a vector of outcomes—water, hot-water energy, cleaning endpoint, splash deposition, task time, ergonomic burden, drying endpoint, footprint, noise, cost and repair outcome. Do not collapse unlike outcomes into one weighted score that conceals regressions.

## 2. Architecture hypotheses

### A. Sink geometry designed around impacts and reach

Prototype a small family of basin geometries varying basin depth, wall and floor angles, drain placement, faucet height/reach, rim form and regulated spray. Aim to reduce splash onto adjacent clean zones while preserving access to corners and a drain strainer. Test with measured flow/pressure and a safe fluorescent tracer; visual impressions are not enough.

A splash-reducing clinical basin study is a mechanism-level research lead, not evidence that the same design will work for household kitchen tasks. Kitchen soil, utensils, basin use and loading patterns differ.

### B. Task-specific, removable inserts

Candidate inserts include a removable wash tub, draining produce colander, over-basin prep board, pot support and rinse rail. Interfaces should be tool-free where practical, stable under load, fully inspectable and easy to remove for cleaning. They must not create inaccessible seams, hidden water pockets or dead areas.

Physical separation can help organize workflows, but changing an insert or changing its color is not a sanitation step. Any transition among raw food, produce, dirty utensils and ready-to-eat tasks must follow an appropriate cleaning/sanitizing procedure.

### C.1. Commercial pre-rinse sprayers: spray force and soil removal, not flow alone

Keep commercial pre-rinse valves on a separate benchmark track from household faucets. EPA's WaterSense research found that its earlier efficiency test method did not correlate well with user satisfaction and that flowing pressure/spray force materially affected performance. ASTM F2324-13R26 is listed as an active test method covering flow rate and spray force for commercial pre-rinse valves. This is direct evidence for a better experiment: hold soil, target, distance and pressure protocol fixed; measure force and coverage together with whole-task water, trigger time, soil removal and splash. Lower flow alone is not a win if staff rinse for longer or repeat the pass.

- EPA pre-rinse valve research: https://www.epa.gov/watersense/pre-rinse-spray-valves
- ASTM F2324-13R26: https://store.astm.org/f2324-13r26.html
- ASME A112.18.1/CSA B125.1-2024 scope: https://www.asme.org/codes-standards/find-codes-standards/plumbing-supply-fittings-%28with-10-18-errata%29
- Implementation track: [KITCHEN-PRERINSE-001 #2359](https://github.com/Luminous-Dynamics/symtropy/issues/2359)

The valve test method and any applicable commercial certification/compliance process do not replace a matched, real-task soil-removal test.

A complementary test-rig plan is versioned at `docs/research/KITCHEN_PRERINSE_TEST_RIG_V1.md`. It separates valve-method measurements from a supplementary task-level soil-removal test, specifies a 20/40/60/80 psi exploratory matrix, provides stop/go gates, and requires stage-wise reference repeatability and a power/precision plan before confirmatory comparison. It explicitly says the rig has not been built or run and that an internal soil surrogate cannot establish sanitation or microbial reduction.

### Reproducible market reference file

The machine-readable source ledger is `docs/research/KITCHEN_PRERINSE_MARKET_REFERENCE_V1.json`. It separates official U.S. flow/force classes and EPA operating-pressure guidance from the manufacturer's published T&S B-0107 specifications. The latter lists 1.15 US gpm and 7.5 ozf at 60 psi; this programme has not independently measured the product. The file also preserves the WaterSense sunset, target hypothesis, four proposed pressure test points (20/40/60/80 psi), and warnings against treating a datasheet as task performance. Three physical specimens per product is only a starting engineering target; statistical sufficiency must be established separately.

### Current commercial reference envelope

EPA's current pre-rinse page says the WaterSense label for this product category ended on 1 January 2019 after DOE incorporated the efficiency threshold and performance test into federal requirements. Do not describe new products as currently WaterSense-labelled on the basis of that retired programme. EPA lists DOE's maximum flow rates by measured spray-force class: up to 1.00 gpm (3.79 L/min) for force at or below 5 ozf; 1.20 gpm (4.54 L/min) for force above 5 and up to 8 ozf; and 1.28 gpm (4.85 L/min) for force above 8 ozf. These are U.S. regulatory reference classes, not universal South African compliance criteria, and not evidence of completed-task performance.

- EPA pre-rinse spray valves, programme status and class limits: https://www.epa.gov/watersense/pre-rinse-spray-valves
- EPA's WaterSense at Work technical guidance recommends checking pressure regularly and cites a 20–80 psi (138–552 kPa) operating range for expected performance: https://nepis.epa.gov/Exe/ZyPURL.cgi?Dockey=P1017K23.txt
- A current product-side example is the T&S Brass B-0171 pre-rinse unit, which lists a 1.15 gpm (4.35 L/min) spray valve and manufacturer inlet-pressure range. Use its exact model/revision as an optional purchasable reference, then measure it at the fixture; specifications are not independent test results: https://www.tsbrass.com/products/pre-rinse-units/prerinseunits/B-0171

### Proposed staged comparison

1. **Valve bench:** reproduce the applicable standardized flow/spray-force method on at least three randomly selected production specimens per design where feasible. Record unit-to-unit variation and post-cycle performance.
2. **Matched task:** use a standardized soil/plate fixture, fixed distance and angle, measured pressure, randomized valve order and a preregistered removal endpoint. Report water, trigger time, force, spatial coverage, residual soil, splash deposition and operator effort separately.
3. **Operational pilot:** only after the bench gates pass, test across several independent commercial-kitchen stations and operators. A useful design target is at least three sites with several weeks of use at each site, balancing valve order where practical. Record task mix, blocked time, cleaning and maintenance, nozzle clogging/wear, actual water and rerinse events. This is a proposed engineering study design, not a regulatory requirement.
4. **Decision:** report per-site and pooled paired effects with uncertainty; do not let a strong result at one venue hide failures elsewhere. The current PR's Rust harness verifies the data/conditions contract, not statistical sufficiency or physical trial quality.

### C. Efficient water delivery and batch-versus-flow washing

Do not assume the best water-saving change is always a lower-flow faucet. Compare a measured-volume batch-wash workflow with continuous-flow rinsing and lower-flow stream/spray controls using the same load, cleaning endpoint and final-rinse rule. South Africa's CoGTA consumer guidance recommends a plugged sink instead of a running tap for washing dishes and for rinsing vegetables; treat this as a useful behavior/design hypothesis, not measured performance evidence.

A removable wash tub or clearly marked fill reference may make water volume visible and repeatable. Keep wash and final rinse separate: dirty wash water must not be counted as final-rinse water or routed back to potable/food-contact use. Measure the full task, including pre-rinse, batch fill, final rinse, spillage and any rewash, plus the safe lift/drain effort of a loaded tub. Start with a household protocol; do not assume this approach satisfies commercial warewashing or food-premises rules without specific review.

Research reference: https://www.cogta.gov.za/index.php/2020/02/25/play-part-in-saving-water/

### C. Efficient water delivery with useful control

Compare aerated stream, spray, pull-out reach and temporary high-flow pot filling. Test across declared supply pressures; flow rating at one pressure is insufficient. Measure total litres to achieve the same task endpoint, including all pre-rinse, wash, final-rinse and rewash events.

Use published specifications and local plumbing requirements as boundaries, not as a substitute for testing. The U.S. EPA WaterSense Homes technical sheet discusses kitchen faucets around 1.5–1.8 gpm as options versus the 2.2 gpm federal maximum it cites, while noting that WaterSense does not have a kitchen-faucet specification. These values are one U.S. comparator, not a universal target for South Africa or other markets.

Mechanical controls should remain usable without Wi-Fi, mobile applications or mains power. If a temporary high-flow mode is used, test accidental activation and verify that the control returns to its default state.

### D. Drainage-first drying

Treat drying as part of the same station. Candidate features:
- a drip path that returns water to the basin rather than the worktop;
- vertical plate slots that leave surfaces separated;
- cup and bowl supports that avoid sealed contact points and trapped pools;
- a removable cutlery cassette with accessible drainage;
- adjustable or replaceable modules sized around a standardized household item set;
- no airflow path that draws untreated drain air over clean items.

Begin with passive drying. Only add a fan, heater or dehumidification option if testing shows a meaningful benefit without unacceptable electricity use, noise, heat, recontamination or complexity. Do not assume a UV lamp, ozone, fragrance or warm airflow sanitizes an item. Such claims require a separate, applicable, independently validated protocol.

### E. Repairable, accessible service surfaces

Common serviceable items may include aerators/spray heads, hoses, seals, drain strainers, rack feet, latches and modular rails. Record part identity and revision, required tools, hands-on repair time, leakage/flow/drain checks and any remaining fault. A repair is successful only after its post-repair checks pass.

Test reach and control use across user height, grip strength and seated/standing positions. A design that saves water while increasing pain, force, spill risk or task time for a portion of users is not a global improvement.

## 2.5. Existing industry baseline and the differentiation gap

The underlying workstation-sink concept is already commercially established. For example, Pelham & White describes built-in ledges with a colander, roll mat and cutting board; KOHLER lists a workstation sink with cutting board, drainboard, colander, drying rack and soaking bin. These features are useful prior art, so the programme must not present accessories-on-ledges as an invention.

- Pelham & White workstation features: https://www.pelhamandwhite.com/workstation
- KOHLER Strive Smart Divide workstation sink: https://la.kohler.com/en/product-detail/33664-PC
- EPA WaterSense Homes kitchen-faucet context: https://nepis.epa.gov/Exe/ZyPURL.cgi?Dockey=P10180T6.TXT

| Commercial baseline already available | Researchable differentiation target |
| --- | --- |
| Ledges and accessory bundles | Compatibility tolerances, loaded stability, cleaning access, cycle wear, replacement availability and cost |
| Pull-out or multi-mode faucet | Potable litres and hot-water energy per successfully completed task across a declared pressure envelope |
| Roll-up or over-sink drying rack | Dryness by item type and rack position, residual water mass, drip return, cleaning time and occupied footprint |
| Drain grids, corner radii or grooved basins | Measured splash-deposition map and reproducible cleanability, with no hidden residue or transferred contamination |
| Premium materials and product feature lists | Repair-time and post-repair flow/leak/drain evidence, lifecycle cost with explicit assumptions, and accessible use |
| Optional powered drying as a possible variant | A matched passive-first comparison including energy, noise, airflow and contamination-deposition checks |

This is a proposed verification advantage, not a claim that existing brands lack all these capabilities. Public product pages are evidence that features are sold; they are not independent comparative-performance data. Benchmark at least one readily available conventional sink/rack and one workstation-sink reference before choosing a design winner.

## 3. Benchmark protocol

### Freeze the conditions

For every run, record:
- exact candidate/reference, revision, basin/counter/faucet geometry and rack configuration;
- supply pressure, measured flow, cold/hot inlet temperatures and meter calibration/uncertainty;
- water hardness or relevant chemistry where it changes outcomes;
- task recipe, standardized soil/load/item list, cleaning products and amounts;
- operator or user cohort, task order, repetitions, deviations and failures;
- room temperature and relative humidity; airflow conditions for drying;
- instrument identity, raw samples, endpoint rule, and analysis version.

Use matched loads and randomized or balanced crossover order when feasible. Report reference and candidate side by side. Repeatability and baseline-fixture uncertainty must be established before judging small differences.

### Sink and washing endpoints

1. **Water:** potable litres per completed task and per standardized item/load; distinguish pre-rinse, wash, final rinse and repeat washes.
2. **Energy:** hot-water energy per task, with metering boundary and calculation documented. Do not confuse heater nameplate power with measured energy.
3. **Cleaning:** documented standardized soil or tracer removal endpoint, with failures and rewashes retained.
4. **Splash:** deposition map from a safe fluorescent tracer at predefined sensitive surfaces. Report location, area/intensity method and repeatability; do not translate tracer data directly into pathogen or illness claims.
5. **Human factors:** task time, spill/overflow, observed reach/posture, user-rated effort and accessibility failures.
6. **Service:** flow stability, drain performance, leakage, blockage/cleaning time, repair time, part availability and post-service verification.

### Drying endpoints

1. Time to a preregistered endpoint, such as item mass reaching a stated stability tolerance; track visible pooling separately.
2. Residual water mass and fraction of items meeting the endpoint, stratified by item type and location in the loaded rack.
3. Room temperature/humidity and declared airflow; energy in Wh and sound pressure/noise metric for powered variants.
4. Drip/rewetting events and tracer deposition on predefined clean surfaces.
5. Usable capacity and occupied footprint with a standardized loaded set—not empty rack dimensions alone.
6. Cleanability, trapped-residue observations, wear and post-cleaning/post-repair performance.

Run household and food-service evaluations separately. NSF/ANSI 2 covers materials, design, fabrication, construction and performance of equipment used in food handling and commercial service settings; it is a compliance map to investigate, not certification of a prototype. The FDA Food Code is model food-service guidance and must not be described as universal household or South African law. Establish the requirements applicable to the target jurisdiction, installation and intended use before claiming conformity.

## 4. Stretch objectives—explicitly not results

The initial hypotheses are:
- at least **20% less potable water per comparable completed task**, with no worse cleaning endpoint or rewash rate;
- at least **30% less tracer-deposited splash** at predefined adjacent surfaces, without relocation of deposition to another sensitive surface;
- at least **25% faster passive drying** to the same declared endpoint, or improved standardized loaded capacity/footprint without a drying regression;
- zero silent imputation: every required candidate/profile cell is observed or visibly marked not run, missing, blocked or invalid;
- user-serviceable part replacement avoids destructive damage and passes leak/flow/drain checks afterward.

These are stretch targets to test, not literature-derived guarantees, certification thresholds or achieved performance. If the water objective conflicts with time or cleaning, publish the Pareto trade-off. Do not label a product “best” using a single weighted score without publishing raw metrics, uncertainty, weighting and sensitivity.

## 5. Adversarial test catalogue

- Lower-flow mode uses less water per minute but more water per task due to extended rinsing or rewashes.
- Average cleaning endpoint passes while a difficult soil/item subset fails.
- A basin reduces visible splashing at the front but relocates tracer behind the faucet or onto clean dishes.
- Plates meet the drying endpoint while cups or cutlery retain pooled water.
- Powered drying shortens time but increases Wh, noise, heat, aerosol/deposition, or recontamination.
- A fully loaded rack is reported using empty capacity or footprint.
- Water flow, pressure or temperature is absent and silently replaced with a default.
- Invalid trials, failures, or inconvenient users vanish from the denominator.
- A repair appears complete but the joint leaks or post-service flow is outside specification.
- Clean-looking materials are treated as proof of food-contact compatibility or microbial safety.

## 6. Safety and claim boundaries

- Do not recirculate drain or wash water into potable supply, final rinse or food-contact tasks. Any non-potable recovery concept requires a separate plumbing, backflow and jurisdictional review.
- Treat basin/drain interfaces as potential contamination sources. Keep dirty-side drain airflow out of the clean drying path.
- Food-contact materials, chemical compatibility, cleaning access, electrical ingress protection and plumbing compliance are independent acceptance gates.
- No UV, ozone, heat or airflow sanitization claim without a fit-for-purpose validation and applicable requirements.
- A dry appearance or fluorescent-tracer result does not establish microbial safety.
- A household prototype cannot establish food-service compliance or product certification.
- No prototype, measured savings, hygiene improvement or superiority is claimed here.

## 6.1. Current standards and South African deployment boundary

A fresh standards check changes the compliance checklist: NSF's Joint Committee on Food Equipment posted **NSF/ANSI 2-2026** on 25 September 2026 (revision 14). Its stated scope is minimum food-protection and sanitation requirements for food-handling/processing equipment materials, design, fabrication, construction and performance. That is a version to review for commercial food-service designs; this brief is not an interpretation of the paid standard and does not imply certification. Source: https://standards.nsf.org/discussion/nsfansi-2-2026-food-equipment-uploaded

South Africa's National Department of Health currently lists R638 of 22 June 2018 under Food Hygiene and Microbiology / food-premises regulations. Commercial deployment must map the design and test plan to the actual applicable R638 clauses, local Environmental Health Practitioner requirements, potable-water/plumbing rules, and any required approval. Do not assume a US NSF listing or FDA model code substitutes for South African compliance. Source: https://www.health.gov.za/food-control-regulations/

For water-stressed and variable-supply deployment, record pressure and supply interruptions as test conditions, not assumed regional constants. South African domestic water norms/standards describe greywater sources such as baths, showers and hand basins and exclude kitchen-sink wastewater from that definition due to its solids and chemical load. Therefore, this design does **not** propose kitchen sink discharge reuse as ordinary greywater, nor any route back to potable, food-contact or final-rinse use. Any recovery/reuse variant is a separate engineering and legal review, not part of the baseline station. Source: https://www.gov.za/sites/default/files/gcis_document/201709/41100gon982.pdf. The applicability of this document and any later amendments must be checked before installation.

## 6.2. Industry benchmark baseline: feature parity is not the breakthrough

A concrete commercial reference is KOHLER's Prolific 23-inch workstation sink: its current product page lists three graduated accessory ledges, a cutting board, two grated racks, colander and wash bin, plus a cone-shaped bowl/drain geometry. This shows that adjustable accessory arrangements and multi-function bowls are already established product features. It does not supply independent task-water, splash-deposition, drying-time or repair-time evidence.

- KOHLER Prolific 23-inch product page: https://la.kohler.com/en/product-detail/23650
- KOHLER Strive workstation feature example: https://la.kohler.com/en/product-detail/33657-PC

Benchmark at least one mainstream workstation sink and one conventional sink/rack setup. Capture product model/revision, installation geometry, accessory list, published materials and dimensions, parts/warranty information available to the buyer, measured water pressure and measured flow. Public marketing claims are hypotheses to test, not measured outcomes.

## 6.3. Evidence record contract and metric registry

Physical eligibility additionally consults registry-declared categorical `qualifying_values`: for example, a `cleaning_endpoint` measured as `fail` cannot be rescued by a low water-use value or a manually marked passing gate. This only checks internal consistency of the submitted record, not the truth of the measurement itself.

The benchmark is now paired with two machine-readable contracts:
- `KITCHEN_STATION_RECORD_SCHEMA_V1.json` describes the per-trial record, candidate/protocol digests, metric status, uncertainty, raw artifacts and non-compensatory safety gates.
- `KITCHEN_STATION_METRIC_REGISTRY_V1.json` catalogs every primary metric ID in the scenario matrix, its canonical mapping, unit policy, value shape, direction and comparison limits.

The record schema permits structured values for per-surface/per-item results. It does not itself certify that a record is true; raw artifacts and independent recalculation still matter. The metric registry deliberately preserves several unresolved method-specific units as blockers before physical trials, rather than inventing comparability. Any future runner must reject unknown metric IDs and never convert missing or failed observations into zero.

## 7. Implementation sequence

1. Select a conventional sink/rack and one mainstream workstation-sink reference; capture model/revision, size, accessories, available replacement parts and published warranties before selecting the baseline.
2. Assemble an inexpensive, instrumented baseline: known-volume collection or calibrated flow meter, pressure and temperature observations, standardized plate/pot/cup set, safe tracer, scale, stopwatch and ambient thermometer/hygrometer.
3. Reproduce the baseline with repeat trials; declare meter and operator variability.
4. Change one design family at a time (flow delivery, basin shape, insert, rack geometry, passive airflow).
5. Screen for hard-gate failures first, then compare paired outcomes and uncertainty.
6. Only if passive candidates justify it, test assisted drying with power/noise/airflow measurement and contamination controls.
7. Build repair demonstrations and post-repair checks after basic performance is reproducible.
8. Publish raw data, test instructions, invalid/failed runs, deviations and candidate revisions. Do not wait for a CI runner to perform this independent research, but do not confuse documentary review with physical validation.

## 8. Research anchors

- U.S. EPA, WaterSense Labeled Homes Technical Sheet: Kitchen Faucets (July 2023): https://nepis.epa.gov/Exe/ZyPURL.cgi?Dockey=P10180T6.TXT
- FDA, Food Code 2022 and §4-901.11 air-drying provisions for food-service equipment/utensils: https://www.fda.gov/food/fda-food-code/food-code-2022
- NSF, What’s New in NSF/ANSI 2 Food Equipment Standard (14 October 2022): https://www.nsf.org/knowledge-library/whats-new-nsf-ansi-2-food-equipment-standard
- Mihalache et al., “Kitchen layouts and consumers’ food hygiene practices: Ergonomics versus safety,” *Food Control* 131 (2022), 108433. https://doi.org/10.1016/j.foodcont.2021.108433
- Aranega-Bou et al., splash-reducing clinical handwash basin laboratory study, *Journal of Hospital Infection* (2021). https://doi.org/10.1016/j.jhin.2021.04.017

The references establish relevant baselines and hypotheses; they do not validate this station design. Physical performance and applicable local compliance remain open.
