# Textile Process Optimization Benchmark v0.1

**Status:** research/benchmark specification with a first dependency-free Rust contract evaluator; synthetic fixtures only. The evaluator is authored but not yet CI-qualified. No process scheduler, factory digital twin, physical validation, or savings result is claimed by this document.

**Purpose:** define a falsifiable first benchmark for improving textile and clothing manufacturing with Symtropy, Symthaea, and Mycelix. The target is a reusable industrial-process capability, not a fashion recommender or a generic “AI for sustainability” demo.

## 1. The engineering thesis

Optimize for a **qualified functional unit delivered over its useful life**, rather than only cost per kilogram or metres produced.

Examples of a functional unit may be one square metre of fabric meeting a frozen tear/abrasion/colour/wash profile, or one garment delivering a declared number of wear-and-wash cycles while retaining its specified function. The unit and acceptance profile must be declared per case; dissimilar products must not be compared as though they deliver equivalent service.

A cheaper batch is not better if it requires more rework, has a shorter verified service life, misses delivery, produces off-spec effluent, or can only be “recycled” into a lower-grade use. Similarly, a route that reduces freshwater use but violates product requirements or shifts burden into an unmeasured waste stream is infeasible, not a winner.

The result should be a **feasible Pareto frontier plus a traceable explanation of trade-offs**, not a single sustainability score that allows excellent performance on one dimension to conceal a catastrophic result on another.

## 2. Existing ownership: compose, do not duplicate

- **Symtropy:** deterministic, replayable scenario/process simulation and constraint-aware comparison. The textile domain requires discrete-event scheduling and material/utility flow accounting; it must not be forced into the rigid-body solver or require Φ/consciousness coupling.
- **Symthaea:** generate and compare hypotheses, alternative routes and candidate interventions; help find informative next measurements; detect defects/anomalies when suitable evidence exists. Its advice is not plant authority or a substitute for physical validation.
- **Mycelix:** retain canonical identity, lot lineage, custody, supplier/process claims, temporal validity and evidence admission; coordinate repair/reuse/recovery claims and selectively disclose confidential data. A provenance event alone cannot establish material grade or performance.
- **Existing work to compose:** Symthaea [TEXTILE-HYG-MFG-000 #5984](https://github.com/Luminous-Dynamics/symthaea/issues/5984), [MFG-PROC-000 #5686](https://github.com/Luminous-Dynamics/symthaea/issues/5686), [MFG-LIFE-000 #5705](https://github.com/Luminous-Dynamics/symthaea/issues/5705), and Mycelix [Industrial Alternatives Fabric #4224](https://github.com/Luminous-Dynamics/mycelix/issues/4224). This benchmark does not create a competing material catalog, manufacturing-process authority, lifecycle assessment engine, or supply-chain ledger.

## 3. Paradigm shifts to test

| From | To | What changes in the engineering |
|---|---|---|
| Cost per kg or maximum machine utilization | Cost/resource intensity per **qualified functional service** | Include yield, rejects, rework, declared durability and delivery service. |
| Optimize each machine or stage separately | Co-optimize the route and factory as a coupled system | Account for changeovers, queues, shared heat/steam/water limits, waste treatment and bottlenecks. |
| Fixed recipe and schedule | Evidence-bounded adaptive planning | Re-plan from measured state, but validate every proposal against hard constraints before an operator can act on it. |
| Treat defects as an inspection problem | Prevent defects and learn where variation enters | Distinguish process settings, material lots, machine state and measurement confidence; track false negatives. |
| Linear take–make–discard | Design for repair, reuse and qualified fiber recovery from the outset | Carry composition, blend, finishing, joining and contamination evidence through end-of-life. |
| One opaque “green score” | Feasibility gates + named objectives + Pareto alternatives | Expose trade-offs and uncertainty; do not turn missing evidence into zero impact. |
| One company’s perfect local optimum | Evidence-linked multi-party coordination | Retain the lot/process lineage and confidential boundaries while assessing supply continuity and circular loops. |

## 3. Garment-aware service and care profiles

The process benchmark must not assume all clothing behaves like an everyday T-shirt. Use the versioned profile contract in [Garment Use Profiles v0.1](fixtures/garment_use_profiles_v0_1/README.md) for at least:

- everyday basics and workwear;
- rental and seasonal occasionwear;
- couture/decorative garments with beads, sequins, dense embroidery, metallic threads, appliqué, adhesives or fragile trims;
- bridal, cultural and ceremonial garments that may be altered, passed on or preserved;
- performance/stage costumes whose service unit is a performance run;
- modular decorative garments designed for detachable trims or alternate styling;
- deadstock, runway samples and one-off pieces with unknown blends/finishes;
- historical, archive and exhibition garments whose service may be conservation years or controlled exhibition days rather than wear cycles.

### No frequency penalty

A low use frequency is a descriptor, not a negative score. A garment used once a year may deliver a distinct ceremony, artistic performance, cultural role, collectible value or historical record. The system should not treat any of those as interchangeable with high-frequency utility clothing, nor declare them environmentally superior by assigning arbitrary cultural value.

Each profile must predeclare its intended service, functional unit, quality/appearance constraints, care burden, decoration/material risks, possible circulation paths and whether specialist assessment is required. Compare alternatives only under functionally comparable units, or report the comparison as unresolved. Do not automatically divide a garment's impact by an assumed number of wears.

### Decorative construction and rare-use care

The model should track detachable versus permanently bonded components, material blends and adhesives, repairability, inspection needs, care method, storage/handling risks, and credible next-life routes. More decoration is not automatically bad; a removable module is not automatically recyclable; and a claimed resale/rental route is not equivalent to an observed second use.

For collection-level/heritage items, condition, controlled handling, storage and display exposure can matter more than laundering or wear count. Conservation references are profiles to be assessed by qualified owners, not universal care prescriptions for ordinary clothing. See the Canadian Conservation Institute's [textile and costume guidelines](https://www.canada.ca/en/conservation-institute/services/preventive-conservation/guidelines-collections/textiles-costumes.html) and National Museums Scotland's [textile packing guidance](https://www.nms.ac.uk/our-impact/national-work/training-and-guidance-for-museums/collections-care/object-packing/how-to-pack-textiles).

### Additional synthetic challenge cases

Extend the existing ten-case corpus with at least:

11. **Rare-use couture:** an embellished gown is worn infrequently, with fragile beadwork and specialist storage/repair requirements. Low use frequency must not itself lower its assessed value or trigger a universal penalty.
12. **Rental occasionwear:** compare a rental pathway with owner purchase using actual cleaning, transport, packaging, damage, repair, and use-count data. No universal rental break-even number is assumed.
13. **Modular statement garment:** detachable trims expand possible styling and repair, but material separation and recycling claims remain blocked until the specific modules and output grades are evidenced.
14. **Deadstock/runway sample:** blend, coating or adhesive composition is unknown. Preserve design and provenance value while blocking unsupported material-safety or recycling claims.
15. **Performance costume:** evaluate completed performance runs, fit/movement constraints, quick changes and repair/cleaning burden rather than generic daily wear cycles.
16. **Heritage/archive garment:** use a declared conservation/exhibition service unit, condition observations and specialist review. Do not require the garment to be worn to have measurable service value.
17. **Ceremonial/bridal garment:** compare alteration, lending, inheritance, resale and preservation pathways without collapsing cultural significance into an invented numeric score.
18. **Unsold sample and overproduction:** compare made-to-order/preorder, small batch, forecast-driven production and reuse of an existing sample. Report surplus per accepted functional unit together with lead time, fulfilled demand, stockouts and product acceptance; do not optimize surplus alone.

## 4. Initial process boundary

The first model should be small enough to verify and broad enough to expose real trade-offs. Begin with batch or lot-level operations such as preparation, spinning (if in scope), weaving/knitting/nonwoven formation, dyeing/finishing as abstracted process families, cutting/sewing, inspection, rework, repair, sorting and recovery.

For every operation, represent only fields required for this benchmark:

- process/route identity and version;
- input/output lot IDs and mass, including explicitly unknown losses;
- machine/resource requirements, duration, setup family and availability;
- metered electricity and thermal energy, freshwater withdrawal/consumption and wastewater quantity/quality where actually measured;
- yield, rework, scrap and product acceptance results;
- declared functional requirements and the evidence supporting each gate;
- uncertainty, source, scope, timestamp and validity window for input parameters.

Do not include hazardous chemical recipes, unvalidated process-control sequences or direct write authority to factory equipment. Water, energy, emissions and waste factors require geographic/time scope and source provenance. “Not measured” is a first-class state; it is not zero.

## 5. Optimization contract

### Hard constraints first

A candidate route/schedule is inadmissible if it violates any declared hard requirement: product function/quality, machine capability, material availability, valid process window, delivery commitment when non-negotiable, utility capacity, effluent/waste-treatment admission, worker-safety constraint supplied by the relevant authority, or required evidence/qualification. Missing critical evidence blocks or limits a claim; it never silently defaults to success.

### Report separate objective dimensions

At minimum report:

1. accepted/qualified functional output and first-pass yield;
2. cycle time, makespan, changeovers and on-time delivery;
3. electricity and thermal energy per accepted functional unit;
4. freshwater consumption per accepted functional unit and wastewater burden when measured;
5. greenhouse-gas intensity using a declared factor set and time/location;
6. scrap, rework and downcycling/recovery yields;
7. total cost with stated inclusion boundaries;
8. service-life/durability evidence and verified reuse/recovery grade;
9. uncertainty, evidence completeness and key sensitivities.

A metric should be omitted or explicitly unknown when its measurement boundary is missing. Do not impute a precise lifecycle or toxicity value from an unvalidated proxy. Keep functional quality and safety constraints out of a compensatory weighted score.

### Baselines must be fair

Compare against at least:
- the recorded incumbent plan or a frozen standard dispatch rule, where supplied;
- simple dispatching heuristics (e.g. earliest due date and shortest processing time);
- an exact solver or exhaustive oracle on small instances;
- a strong constrained multi-objective search baseline on larger instances.

Freeze compute budgets, objectives, normalization, reference points and random seeds before comparison. If Symthaea proposes a route, compare it with the same admissibility checks and compute budget. A learned policy should not receive privileged information unavailable to its baselines.

## 6. Frozen synthetic challenge corpus

The first version should cover these adversarial contract cases. All values are synthetic until a public or partner dataset has been licensed, audited and explicitly bound into a later version.

1. **Water/steam/schedule trade-off:** batch grouping reduces reheat/changeover cost but can delay an urgent order or increase water/effluent burden. Return feasible Pareto alternatives; do not silently choose a universal winner.
2. **Weaving defect and rework:** a faster setting raises nominal output but also changes rejection risk. Include false-negative and missing-sensor controls; accepted output, not gross output, is the denominator.
3. **Blend recovery:** recovered blended fiber is available, but composition/contamination or grade is unknown. Block unsupported fiber-to-fiber claims; retain valid lower-grade pathways as different functions, not as equivalent recycling.
4. **Durability/function:** a lower-impact fabric has fewer verified service cycles. Compare only when the functional unit and useful-life evidence permit a defensible comparison; otherwise mark lifecycle ordering unresolved.
5. **Machine outage and delivery:** a high-throughput schedule depends on one machine. Evaluate the makespan/resource frontier under a frozen failure perturbation and preserve delivery/quality constraints.
6. **Missing effluent authority:** energy and water look favorable but effluent-treatment capacity or discharge evidence is absent. The apparent winner must remain blocked.
7. **Partial supply closure:** local sewing is possible but a specialty membrane, yarn, dyeing route or fastener is imported/unqualified. Report partial closure and the exact missing dependency, not “local manufacturing complete.”
8. **Conflicting lineage:** two lots share a nominal material name but differ in supplier claim, recipe/process version, test scope or valid time. They must not be merged into one evidence identity.
9. **Permutation/adversarial input:** reorder independent jobs, resources and evidence records; determinism and semantic results must be invariant. Duplicate IDs, negative quantities, missing objective units and out-of-scope evidence must fail closed.
10. **Unknown impact factor:** a candidate omits a material, electricity or waste factor. It cannot win by treating the missing contribution as zero; return an incomplete assessment and identify the highest-value missing measurement.

Every case should contain a frozen manifest, input hashes, model/solver version, seed, expected admissibility decision, invariant checks and an independent oracle for the smallest instances. Cases must be reviewed so their expected outcome does not merely call the production helper whose correctness is being tested.

## 7. Benchmarks and proposed targets

These are **internal R&D targets**, not industry standards and not measured results.

### Gate A — semantic correctness and determinism

- 100% pass on frozen hard-constraint and mass/lot-flow invariants;
- zero hard-gate violations reported as feasible;
- 100% parity with an independent exhaustive oracle on bounded small instances;
- byte-identical canonical output for identical manifest, solver version and seed;
- permutation invariance for semantically unordered inputs;
- 100% explicit handling of missing/unknown critical evidence;
- no claim of closed-loop recycling when only collection, sorting or downcycling is established.

A failure of any safety/quality/feasibility invariant blocks that candidate regardless of its objective vector.

### Gate B — optimization quality in synthetic cases

With a fixed compute budget and frozen reference point, target at least a **5% relative hypervolume improvement in 70% or more of preregistered larger synthetic instances** against the strongest fair baseline, while preserving all hard constraints. Also publish per-objective changes, dominated-candidate witnesses, run-to-run variance and cases where the method loses. Hypervolume alone is not enough: objective scaling and reference points must be reported.

Target at least 100 seeded instances across several factory sizes, with small instances cross-checked against exact solutions. Report p50/p95 decision latency and memory on a named reference machine; do not claim a speed target before a baseline has been measured.

### Gate C — measurement-grounded evaluation

Before a real pilot, require an independently reviewed data/validation plan, licensed data, a time-separated holdout and an uncertainty budget. A proposed initial threshold is median absolute percentage error at or below 10% for key calibrated resource/throughput predictions on suitable held-out batches; report all exclusions and error distributions, and do not use this threshold where zeros, censoring or inadequate measurements make it invalid.

For an operator-in-the-loop pilot, preregister the functional unit and counterfactual. Aim for at least **5% lower resource intensity on two preselected, measured objective dimensions** with no statistically or operationally meaningful product-quality or on-time-delivery regression. Require confidence intervals, not a point estimate alone. The specific site, time period, product mix, energy factor, water boundary and comparison design must be published with the result. Stop if a hard quality, environmental or worker-safety bound is breached.

### Gate D — operational and circular value

Only after Gates A–C: target 10% lower energy intensity, 10% lower freshwater consumption intensity and 15% lower first-pass rejection/rework in a partner-specific, preregistered trial, with the functional output held constant. Treat these as stretch goals, not promises. For end-of-life pathways, report verified fiber-to-fiber yield and secondary-grade acceptance separately from collection, reuse, open-loop recycling and downcycling.

No benchmark pass alone establishes regulatory compliance, product certification, safe process operation, environmental superiority, or permission to control factory equipment.

### Gate E — rare-use, decorative, and conservation-sensitive garments

These are internal semantic/measurement targets, not a universal definition of garment value.

- **100% profile admission correctness:** every valid profile declares intended service, a specific functional unit, value/function dimensions, care constraints and credible circulation options. No profile is assigned a lower value solely because its use pattern is occasional, event-specific, display-rotation or archive-preservation.
- **Zero unjustified cross-unit ranking:** do not rank everyday wear/care cycles against ceremonial events, performance runs, exhibition days or conservation years unless a documented functional-equivalence rule makes the denominator defensible. Otherwise report the comparison as unresolved.
- **100% decorative-component uncertainty retention:** beads, sequins, embroidery, metallic thread, adhesives, coatings, mixed-material trims and detachable modules must have a known/unknown composition state and material-separation evidence before the system admits a qualified recovery claim.
- **Zero false circularity admissions:** collecting, renting, re-selling, repairing, downcycling, mass-balance accounting or a named recycling event must not alone qualify a material as accepted fiber-to-fiber feedstock. Preserve a distinct source/output-lot and grade path.
- **Profile-specific appearance/condition tests:** for each physical trial, predeclare appropriate metrics such as bead loss, seam integrity, attachment strength, dimensional change, colour change, drape, fit, repairability or conservation condition. An acceptance threshold must be set for that garment and intended service by the responsible product/conservation owner; there is no one numerical appearance threshold for all garments.
- **Fair rental/resale comparisons:** measure cleaning method and frequency, transport, packaging, damage, repair, storage and actual completed uses. Calculate break-even with uncertainty for the specified route and location rather than publishing a universal “rental is greener after N wears” number.
- **No forced-wear assumption:** archive/display profiles can use controlled exhibition days or conservation years. A historical or culturally significant garment need not be worn to have a declared service, and conservation guidance must not be indiscriminately applied to ordinary commercial apparel.

For any physical pilot, the unit, profile, comparison alternatives, measurements and stopping criteria must be frozen before the observation period. Specialist review is required where the profile marks fragile/degrading materials or conservation needs. For production-strategy pilots, additionally target a proposed 10% relative reduction in unsold finished garments/sample stock per accepted functional unit versus a frozen baseline, while keeping on-time fulfillment and functional-profile acceptance within preregistered non-inferiority bounds. This is exploratory, not universal; report confidence intervals and all stockout/lead-time trade-offs. Do not credit inventory donation or reallocation as environmental benefit unless the second use is observed.

## 8. Data and validation path

Useful starting evidence exists, but it must be scoped carefully:

- The open-access **Textile weaving dataset for machine learning to predict rejection and production of a weaving factory** reports 121,148 records and 18 parameters from nine months of daily production reporting. Audit the article, data license, schema, target leakage and split by time/batch before use: https://doi.org/10.1016/j.dib.2023.108995
- A 2025 open-access **dataset for defect detection in textile manufacturing** contains images of two fabric structures collected at a single manufacturer. It may support a bounded visual-defect benchmark after licensing and class/lighting review; it is not evidence of generalization to other looms, fabrics or plants: https://doi.org/10.1016/j.dib.2025.111451
- An August 2026 textile scheduling study reports, for its case study, 2.9% lower energy cost and 10.1% lower carbon emissions with a 1.1% makespan increase. These are study-specific results, not a guaranteed or directly comparable target for this system. Reconstruct baseline/objectives before using it as a reference: https://doi.org/10.1016/j.swevo.2026.102498
- The European Commission's sustainable textiles strategy states that only 1% of material in clothing is recycled into new clothing and frames durable, repairable, recyclable products and Digital Product Passports as strategic goals: https://environment.ec.europa.eu/strategy/textiles-strategy_en
- The European Environment Agency emphasizes that textile recycling—especially synthetic textiles—has technical and economic challenges and depends on accurate collection/sorting and viable fiber-to-fiber routes: https://www.eea.europa.eu/en/analysis/publications/plastic-in-textiles-towards-a-circular-economy-for-synthetic-textiles-in-europe

Public datasets are an initial evidence base, not proof of factory-wide optimization. Do not mix training/test observations from the same time-adjacent batch across splits. Prefer time-separated evaluation and, where possible, a second facility/product family for external validity. Any partner data must be minimized, access-controlled, permissioned and anonymized before entering public fixtures.

## 9. First implementation sequence

1. **Completed in source, awaiting exact-head validation:** freeze the original ten process-contract cases and an independent correctness oracle; add the dependency-free `symtropy-textile-process` crate and versioned garment-use profile fixtures for decorative and rare-use garments.
2. **Implemented but not yet CI-qualified:** typed objective directions/units/scales, evidence-bearing objective values, hard-gate outcomes (`Pass`, `Fail`, `Unknown`), deterministic feasible/incomplete/blocked classification, Pareto frontier and deterministic domination witnesses, exact integer mass closure at one declared process boundary, and garment use/care profiles including specialist decoration risks. A seeded independent exhaustive oracle checks 100 synthetic scenarios × 16 candidates × 4 objectives. A small example makes a hard-gate false-green visible.
3. **Next:** add numerical process-operation and schedule models (durations, setup families, machine eligibility, due windows and utility envelopes) with tiny exhaustive scheduling fixtures. Do not alter Symtropy's core rigid-body semantics or represent missing resource/effluent data as zero.
4. Compare a simple incumbent dispatch rule, earliest-due-date, shortest-processing-time, and exact solutions on tiny instances before multi-objective heuristics. Freeze time and compute budgets.
5. Add a Mycelix-facing assessment receipt referencing immutable lot/process/evidence IDs, uncertainty and objective-contract identity without importing confidential source data.
6. Use Symthaea for offline proposal generation, uncertainty diagnosis and next-measurement selection; independently re-check every proposal.
7. Only after exact replay, oracle parity and data validation, run a no-write shadow evaluation with an operator. Physical trials require their own risk review and process owner.

### Claim ceiling

This benchmark is a specification and synthetic research programme. It currently proves no reduction in textile energy, water, cost, emissions, waste or defects; no actual model accuracy; no circularity, safety or regulatory qualification; and no industrial deployment readiness. A queued CI run is not a pass, and a synthetic result is not factory evidence.
