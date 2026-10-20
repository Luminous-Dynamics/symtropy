# Commercial Pre-Rinse Test Rig and Study Plan V1

**Status:** bench design, not built; no physical results.
**Programme:** [KITCHEN-PRERINSE-001 #2359](https://github.com/Luminous-Dynamics/symtropy/issues/2359)
**Benchmark package:** [Draft PR #2325](https://github.com/Luminous-Dynamics/symtropy/pull/2325)
**Machine-readable published reference:** `KITCHEN_PRERINSE_MARKET_REFERENCE_V1.json`

## 1. What the rig must answer

Can a candidate remove a defined amount of food soil to a preregistered endpoint with less total water or shorter trigger time than a known commercial valve, while keeping force, coverage, splash, operator burden, wear and service outcomes acceptable?

This is deliberately a task-level question in addition to the valve-level measurements. ASTM F2324-13R26 is the current listed method for flow and spray force. Use the complete applicable standard, including its apparatus and procedure, when claiming a standards test; this document does not reproduce or replace the paid standard.

Reference:
- ASTM F2324-13R26: https://store.astm.org/f2324-13r26.html
- EPA pre-rinse valves and U.S. flow classes: https://www.epa.gov/watersense/pre-rinse-spray-valves
- EPA WaterSense at Work pressure guidance (20–80 psi): https://nepis.epa.gov/Exe/ZyPURL.cgi?Dockey=P1017K23.txt
- T&S Brass B-0107 reference specification: https://www.tsbrass.com/products/parts-accessories/spray-valves/B-0107

## 2. Two linked but separate benches

### Bench A — valve characterization

Measure the valve under a specified method and fixture. Preserve the exact method version and deviations. At minimum, record:
- exact model, serial/revision and specimen;
- flowing inlet pressure and water temperature;
- measured volume flow and uncertainty;
- spray force and uncertainty using the applicable prescribed setup;
- spray-pattern/coverage images from a frozen position, if collected as supplementary data;
- repeatability and any leakage, trigger or control fault.

Do not improvise the ASTM apparatus then call the result “ASTM-compliant.” If the prescribed apparatus or method is unavailable, label the result an internal engineering method and record the deviations.

### Bench B — task-level soil removal

A supplementary task rig answers what the operator is trying to achieve. It must use a fixed target fixture and a defined soil-removal endpoint. It is not a sanitation, microbiological or food-service certification test.

Core setup:
- stable, guarded fixture mounting with a repeatable nozzle-to-target distance and impact angle;
- measured pressure at the valve inlet under flowing conditions;
- calibrated water/flow measurement plus an independent catch-and-weigh or volume cross-check;
- measured incoming water temperature;
- a fixed target plane, removable target coupons/plates and a mapped array of splash collectors;
- camera/lighting or instrumented coverage collection in a fixed position;
- calibrated balance or instrument fit for the selected residual-soil method;
- ambient conditions and any fixture vibration/movement recorded;
- safe drainage and splash containment, backflow protection appropriate to installation, and a means to stop/isolate flow without handling a live electrical or pressurized hazard.

No universal sensor model is prescribed. Choose instruments so their expanded measurement uncertainty is materially smaller than the minimum effect the study intends to distinguish. For a proposed 15% water-use difference, target a metrology plan that can resolve an effect appreciably below 15%, and justify the selected uncertainty budget in the protocol before testing.

## 3. Product references and candidate controls

- Reference product: T&S Brass B-0107, published 1.15 US gpm and 7.5 oz-force at 60 psi. These are catalogue values only, not bench truth.
- Candidate: one clearly revisioned nozzle/valve configuration, with every change recorded.
- If resources allow, sample at least three specimens per product for the exploratory stage. This is a preliminary engineering target for detecting unit-to-unit variation, not a statistical-power guarantee.
- Include any selected conventional/high-flow comparison valve only when its identity and intended comparison are documented. Do not compare a new valve's matched task outcome to an unmeasured assumed legacy flow.

## 4. Pressure matrix

Start with 20, 40, 60 and 80 psi **measured while flowing**, following EPA WaterSense at Work’s recommended operating-pressure range as a U.S. reference. Confirm the target product's rated range and local conditions; do not exceed product ratings. If the target market has a different operating envelope, preregister and justify that profile.

At every pressure point, log:
- setpoint and measured flowing pressure;
- temperature;
- flow rate and uncertainty;
- spray force under the applicable valve method;
- spray coverage/pattern under the supplementary frozen method;
- task water, trigger-on time and elapsed time;
- soil-removal endpoint and any re-rinse;
- splash deposition by fixed receiver;
- faults, interruptions, invalid trials and deviations.

Randomize pressure order within safe operating limits where feasible to avoid order and temperature drift effects. Allow the setup to return to the specified condition between trials.

## 5. Soil and endpoint discipline

Choose a repeatable, documented standardized artificial food-soil method suitable for the target substrate, or preregister an internal surrogate assay with explicit composition, preparation, application mass, drying/conditioning time and removal endpoint.

- Freeze the soil recipe, lot, substrate finish, initial soil mass, elapsed time since application, and conditioning environment.
- Use multiple challenge classes only as separate strata (for example, a readily removable residue and a dried-on residue); do not merge classes in a way that hides a failure.
- Quantify the pre- and post-rinse outcome by the declared measurement method and report residual soil as well as the pass/fail endpoint.
- If the assay is an internal surrogate rather than a validated standardized method, label it exactly that way. No pathogen, allergen, microbial-reduction or sanitation inference is permitted from soil removal alone.
- Use the same load/soil/conditioning rules for reference and candidate.

## 6. Trial design

### Stage 0 — method shake-down

Use non-reportable pilot trials to resolve mounting, leakage, pressure control, endpoint repeatability, camera framing, data-file naming and safe drainage. Do not include these trials in the primary comparison unless the protocol explicitly preregisters and retains them as exploratory data.

### Stage 1 — exploratory repeatability

Run repeated reference trials across the pressure/soil matrix. Retain every trial and invalidation reason. Estimate within-fixture and between-specimen variability. A small pilot is for variance estimation and fixture refinement; it cannot support a broad superiority claim.

### Stage 2 — matched candidate comparison

Before seeing candidate results, freeze:
- primary endpoint and acceptable cleaning margin;
- target water-saving effect of interest;
- non-inferiority margins for soil removal, task time, operator burden and splash;
- specimen and repetition count justified by Stage 1 variability and a stated power/precision goal;
- randomization and block structure;
- missing-data and exclusion rules;
- analysis version and all outputs to publish.

Pair comparable target coupons and task conditions. Randomize or balance product order. The primary water metric is litres for the complete task, including every trigger event, re-rinse and failed attempt. Do not count a task as successful because it consumed little water if it failed the endpoint.

### Stage 3 — durability and repair

Define a representative trigger cycle and water/temperature exposure profile before the test. Inspect for leakage, trigger reliability, clogging, changed flow, force/coverage drift, material wear and control degradation at prespecified intervals. After each service event, repeat the same flow, force, coverage and task-endpoint checks. A work-order close or part swap alone is not a successful repair.

### Stage 4 — operational pilot

Only after the bench has stable outcomes, consider a small multi-site commercial-kitchen pilot. Record site, operating conditions, dish/soil mix, throughput, task times, water/energy boundaries, re-rinses, downtime, operator cohort and maintenance. Use appropriate employer/site approval and privacy practices. An exploratory multi-site pilot is not a substitute for a powered, preregistered evaluation.

## 7. Primary outcomes and comparison

Report these independently:
1. litres per completed matched task;
2. task-endpoint success and re-rinse/failure fraction;
3. residual-soil distribution, stratified by soil class;
4. spray force and coverage over the tested pressure envelope;
5. trigger-on time and elapsed task time;
6. splash deposition on each preregistered surface;
7. operator effort and faults;
8. flow/force/coverage/endpoint drift after endurance cycles and repair.

Present paired candidate-minus-reference differences, raw distributions, denominators, uncertainty intervals, pressure and soil strata, invalid/excluded runs and deviations. A Pareto view may follow, but do not compress force, flow, time and endpoint success into an opaque single score.

### Stretch target (hypothesis, not result)

At least 15% less water per completed task than the measured B-0107 reference or another declared reference, with the preregistered soil-removal endpoint and re-rinse rate non-inferior, and with force/coverage acceptable across the declared pressure range. The reference's published 1.15 gpm specification is useful for procurement and planning, but all actual comparisons must use measured reference performance at the rig.

The task-water target cannot overrule a failed endpoint or safety gate. If 15% is not supported, report the most accurate trade-off and uncertainty.

## 8. Raw records and evidence receipts

Use the Rust harness to validate record shape, candidate/scenario/metric IDs, unit options, complete scenario safety-gate coverage and artifact hashes. Keep:
- the exact fixture/protocol revision and SHA-256;
- exact product model, serial/revision and any substitutions;
- raw flow/pressure/temperature/force/weight/image data and content hashes;
- trial-level records including blocked/invalid runs and explicit reasons;
- a deterministic CLI report with contract and input digests.

A passing software harness establishes record-contract checks only. It does not validate a calibration, a soil assay, an operator study, a standard method, product safety or the veracity of a measurement.

## 9. Stop/go gates

- **Stop:** leakage or unsafe pressure/temperature; unstable mounting; failed backflow/safety review; sensor/calibration out of scope; endpoint cannot be reproduced; missing required gate.
- **Repair fixture/method:** repeated reference runs vary more than the predeclared allowable measurement/within-cell variability; pressure is not stable; soil application drifts.
- **Do not advance to superiority claim:** missing primary metric, unmatched condition, unknown required safety gate, invalid artifact digest, non-comparable units, synthetic-only evidence, or inadequate sample-size/power basis.
- **Advance to candidate comparison:** method is repeatable, reference is measured, safety prerequisites are resolved, primary endpoint and margins are frozen, and enough specimens/repetitions are justified by pilot data.

No rig has been built or run under this document. All specifications, targets and sequence are a proposed engineering plan.
