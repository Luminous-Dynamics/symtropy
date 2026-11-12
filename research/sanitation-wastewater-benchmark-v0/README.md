# Sanitation + Wastewater Systems Benchmark v0.1

**Programme:** SAN-WW-BENCH-001  
**Issue:** [#4902](https://github.com/Luminous-Dynamics/symtropy/issues/4902)  
**Date:** 2026-11-12  
**Status:** Research protocol / design baseline. Documentation-only starter; no physical trials, solver qualification, product certification, or SOTA superiority claim.

## Thesis

We should not optimize a toilet, treatment plant, drainage network, or robot in isolation. Optimize **safe, dignified sanitation service over its complete lifecycle**, with explicit accounting for water, energy, nutrients, pollutants, solids, labour, maintenance, downtime, cost, user experience and environmental risk.

```text
user need
  -> toilet / collection interface
  -> containment
  -> emptying + conveyance
  -> treatment
  -> verified discharge or fit-for-purpose reuse
  -> residuals / sludge management
  -> inspection, maintenance and renewal
```

Stormwater and wastewater are related but distinct systems. They need explicit cross-connection, infiltration/inflow, overflow and flood-ingress interfaces; they must not share an untyped “water” balance or be treated as interchangeable flows.

## 1. Paradigm shifts

1. **From fixture efficiency to whole-chain service efficiency.** A low-flush toilet that blocks the drain, requires repeated flushing, or creates an unsafe sludge chain may be worse than a higher-water comparator. Measure actual litres per user/day and total service costs, not only nominal L/flush.
2. **From one-size-fits-all centralization to context-matched modularity.** Compare centralized sewerage, decentralized community plants, building-scale systems and non-sewered sanitation under equal service, climate, operator, and lifecycle assumptions. Modularity is a hypothesis to test, not an automatic advantage.
3. **From waste removal to safety-constrained resource recovery.** Track water, nitrogen, phosphorus, energy and carbon as recoverable streams, but credit them only when treatment, contaminant controls, quality assurance, market/use requirements and measured delivery are satisfied.
4. **From nominal treatment performance to resilient service continuity.** Include emptying logistics, sludge transport, operator availability, spares, power/water interruptions, wet-weather loads, bypass events and safe fallback.
5. **From sensor dashboards to evidence-qualified operations.** Preserve the distinction between raw observation, calibrated measurement, laboratory result, derived assessment, permit/standard decision, authorization, physical action and observed outcome.
6. **From separated civil-engineering silos to typed infrastructure twins.** Couple treatment and collection hydraulics with stormwater/flood exposure only through explicit interfaces, conservation rules, and qualified models.
7. **From opaque products to repairable, interoperable assets.** Design modules for inspection, cleaning, replacement, local manufacture, standard interfaces, documented BOMs, configuration control and verifiable maintenance.

## 2. Technology families to benchmark

| Layer | Candidate families | Key comparison question |
|---|---|---|
| Toilet / interface | Conventional gravity flush; dual flush; pressure-assisted; vacuum; urine-diverting dry; dehydrating/composting; prefabricated non-sewered systems | Does it deliver equal or better hygiene, usability and conveyance with less total water and lifecycle burden? |
| Containment / collection | Sewer connection; simplified/settled sewer; septic; sealed vault; container-based collection | Are leakage, emptying frequency, worker exposure, transport and end destination controlled? |
| Biological treatment | Activated sludge; extended aeration; SBR; MBR; anaerobic digestion + post-treatment; constructed wetlands; hybrid nature-based systems | What effluent quality and reliability are achieved under the same load, climate and operational boundary? |
| Tertiary treatment | Filtration; UV; chlorination; other disinfection / polishing methods | Does it meet the actual discharge or reuse endpoint, including pathogens and by-products? |
| Solids / faecal sludge | Thickening; drying; stabilization; digestion; dewatering; approved beneficial use or disposal | Is the entire chain safe, traceable, affordable and operationally supported? |
| Stormwater | Inlets; pipes; culverts; detention; infiltration; rain gardens; bioswales; constructed wetlands; permeable pavement; roof storage | What happens to event volume, peak flow, pollutant loads and downstream risk? |
| Flood protection | 1D sewer/drainage + 2D surface flow; pumps; gates; overflow routes; warning/egress systems | Does the model predict the supported observables and does the asset fail safely? |
| Robotics | Sewer crawlers; drain inspection; sampling aids; cleaning/maintenance tools; construction assistance | Does robotic work lower exposure, cost or downtime without creating new hazards? |

No candidate technology is presumed to win. Waterless systems still need a verified emptying and treatment service; membranes still need energy, cleaning and residuals management; nature-based systems need land, seasonal performance, maintenance and pathogen-risk qualification.

## 2A. Reference demonstrator architecture

These are design candidates for engineering study, not finished product specifications or construction drawings.

### Demonstrator A — Building-scale regenerative sanitation cell

Target contexts: a small building, school block, remote facility or demonstrator home.

```text
rainwater capture + first-flush diversion
              |
              v
       non-potable storage ------------------------+
                                                   |
municipal / borehole potable supply -> protected  |
                                   potable network |
                                                   v
toilet candidates -> separate blackwater collection -> treatment candidate train
greywater fixtures -> greywater collection -> treatment / reuse candidate
                                                   |
                         fit-for-purpose testing <-+
                                                   |
                  reuse storage -> metered non-potable service
                                                   |
                    residuals -> sealed storage / approved removal
```

Engineering principles:
- Keep potable, greywater, reclaimed water, blackwater and stormwater physically distinct, clearly identified and protected against cross-connection/backflow.
- Compare an efficient gravity-flush front end with a urine-diverting/low-water alternative rather than forcing one design onto all buildings.
- Make the flush mechanism, seal, trapway and service components replaceable; prioritize accessible cleaning and standard fittings.
- Provide an approved sewer or sealed containment fallback. Unsafe overflow, untreated reuse or uncontrolled discharge is never a fallback.
- Use rainwater as a variable source, not guaranteed supply; log storage level, quality class and source availability.
- Reuse only for an explicitly allowed application after the required treatment and laboratory verification. A dashboard reading is not a reuse permit.

First measurements: effective L/use and L/user/day; successful use and blockages; raw/reclaimed water volumes; BOD/COD/TSS/nutrient and microbial lab results appropriate to intended use; gross energy; sludge/residuals volume; downtime; user and operator burden.

### Demonstrator B — Community-scale decentralized sanitation hub

Target contexts: a school, clinic, community facility or settlement-scale service.

Candidate modules: accessible toilet blocks; conventional or non-sewered front-end options; sealed collection/equalization; screened and grit-managed feed; one or more treatment trains selected from SBR, MBBR, MBR, anaerobic-plus-aerobic, and wetland/nature-based configurations; disinfection where the endpoint requires it; metered reclaimed-water storage; sludge receiving, handling and safe haulage.

The comparison must include central sewer connection as a baseline where technically available, as well as the complete emptying/transport chain for on-site options. The best design may be a hybrid. Provide trained operator access, spare-parts strategy, backup power/water profile, emergency service continuity and a maintenance contract before calling the facility service-ready.

First measurements: cost per person-equivalent/year; availability; sample-based effluent compliance; kWh/m³; sludge logistics; maintenance hours; emptying completion; safe restart; worst-served-user coverage.

### Demonstrator C — Urban catchment water and flood resilience cell

Compose, rather than duplicate, existing Sol-Atlas and Symtropy capabilities:

- Sol-Atlas: source-aware terrain, catchment, network geometry, land cover, asset and exposure data.
- EPA SWMM / matched drainage baseline: rainfall-runoff, sewer, storage, pump, overflow and green-infrastructure comparisons.
- Symtropy qualified shallow-water/flood solver: surface routing and floodplain interactions after numerical qualification.
- Green/blue/grey interventions: roof storage, rain gardens, bioswales, wetlands, permeable surfaces, distributed detention, maintained inlets, pipes and safe overland routes.
- Robotic inspection: stormwater inlet, culvert and sewer observation evidence linked to the exact asset segment and maintenance result.

Do not route stormwater into sanitary treatment as a substitute for catchment design. Measure first-flush pollutant loads, wet-weather infiltration/inflow, overflows, downstream hydrographs, flood depth/arrival and asset downtime. Evaluate climate events and degraded/blocked infrastructure together.

### Candidate toilet product families

**Sewer-connected efficient gravity toilet.** Keep a passive, repairable architecture as the default candidate. Investigate trapway geometry, bowl wash, jet/siphon flow, sealing and flush-volume control with CFD and standardized physical flush/drainline tests. WaterSense's current 1.28 gpf specification (~4.85 L) is a useful comparator. The first competitive goal is lower measured L/user/day with equal-or-better functional performance—not a smaller flush volume at any cost.

**Water-scarce dry or source-separating toilet.** Investigate a user-friendly urine-diverting interface, splash-resistant channels, sealed and replaceable solids container, odor management, vector exclusion and safe lifting/emptying. A viable product includes the emptying, transport and treatment service; a toilet that simply stores waste without a verified downstream chain is not a complete sanitation solution.

**Prefabricated non-sewered sanitation module.** Evaluate a front-end + integrated backend within ISO 30500:2025 scope. Explicitly design the external service chain as well: the standard does not cover downstream transport and external reuse/disposal. A community-scale faecal-sludge unit should be assessed against ISO 31800:2020 where within scope.

For all three families, compare CAD/CFD predictions with standardized component tests, run durability and fault-injection tests, document BOM/material compatibility, service access, local manufacturability and end-of-life. No physical prototype should be connected to a reuse pathway until the required risk assessment, laboratory tests and permissions exist.

## 3. Benchmark tiers and evidence ceilings

| Tier | Scope | Evidence permitted |
|---|---|---|
| T0 | Typed schema, runtime-neutral corpus, deterministic reference arithmetic, conservation and hostile-case tests | Semantic/model conformance only |
| T1 | Controlled component benches: toilet flush/drainline, pumps, valves, sensors, unit processes | Component measurements within tested envelope |
| T2 | Process pilot with measured influent/effluent, flow, energy, chemicals, solids, uptime and maintenance; accredited or otherwise qualified lab methods | Pilot-specific process performance |
| T3 | Supervised site demonstration across representative seasons and disruptions, with permits and independent process/lab audits | Site-specific operational evidence |
| T4 | Replication across sites, operators, seasons and technology families with held-out sites and lifecycle accounting | Conditional generalization within the demonstrated envelope |

A synthetic test is not a physical test. A model output is not an observation. A clean water instrumentation pass does not imply potable water or pathogen safety. A derived score or boolean cannot replace the source measurements and required laboratory methods.

## 4. Required metric set

### Toilet and collection
- Effective flush volume: L/use and L/user/day; flush success and double-flush rate.
- Drainline transport, blockage events per 1,000 uses, cleaning performance, splash/odor/noise and accessibility.
- Non-flush: containment leakage, vector control, emptying frequency, user acceptability, safe handling, collection completion and transport time.
- Measure ordinary operation and defined misuse/foreign-object cases.

### Wastewater and residuals
- Flow-weighted influent and effluent **concentrations and loads**: BOD5, COD, TSS, ammonia/nitrate/total nitrogen, phosphorus and locally relevant contaminants.
- Microbial indicator and pathogen-reduction measurements using named methods; QMRA where required. BOD/COD, turbidity or a model score alone do not establish pathogen safety.
- Compliance by receiving environment and intended reuse class; do not assume a universal effluent threshold.
- Sludge and residual mass, dry solids, stabilization, contaminant status, haulage and final destination.

### Resource and lifecycle performance
- Gross energy and recovered energy separately: kWh/m³, kWh/PE/year, with meters and plant boundary explicit.
- Captured, treated, produced, delivered and actually reused water separately.
- N/P recovered (kg/year or kg/PE/year), quality of recovered product, chemicals/media/membranes, replacement parts and hauling.
- Lifecycle GHG (kg CO2e/m³ and kg CO2e/PE/year), including methane, N2O, grid mix, transport and embodied infrastructure where data supports it.
- Full lifecycle cost per PE/year and per m³ treated, including CAPEX, OPEX, renewal, people, labs, logistics and end-of-life.

### Stormwater / flood / resilience
- Event runoff volume, peak discharge and timing, detention, infiltration, bypass and pollutant loads.
- Sewer inflow/infiltration, wet-weather overload, overflow frequency/volume and receiving-water impacts.
- Flood depth/velocity/arrival-time error, inundation extent, mass residual, asset exposure and warning lead time, only where observations support each metric.
- Track water-service availability during drought, power cuts, floods, pump failures and maintenance.

### Operations and equity
- Availability; planned/unplanned downtime; MTBF; MTTR; maintenance labour; call-outs; spares; operator competency; sludge emptying and repair completion.
- Worst-served group and location, service coverage, affordability, user time, accessibility, dignity, gender safety and worker exposure.
- Keep each metric disaggregated. Do not hide a safety or rights failure inside a weighted aggregate score.

## 5. Fair comparison protocol

Freeze the boundary before comparing:
- service population and person-equivalent (PE) definition;
- usage and influent flow/load distributions;
- temperature, rainfall, groundwater, storm inflow and relevant climate profile;
- effluent quality endpoint and discharge/reuse context;
- power supply, backup, water availability, operator skill and maintenance response;
- process stages, energy meters, sludge/haulage destination and lifecycle horizon;
- sensor identity, units, calibration, uncertainty, sampling, lab method and data rights.

Use matched service conditions and declared operating envelopes. Split validation by site, catchment, season, operator and technology rather than randomly splitting adjacent samples. Preregister calibration and held-out challenge sets. Preserve non-convergence, bypass, missing data, downtime, failed tests and unresolved outcomes in the denominator. Compare full Pareto fronts; only apply weights where the decision-maker has published the weights in advance.

## 6. Industry and SOTA comparison strategy

| Domain | External comparator | What must be matched | Our opportunity |
|---|---|---|---|
| Residential toilets | US EPA WaterSense tank-type specification. Version 1.2 is currently in effect; its 1.28 US gal/flush maximum is about 4.85 L/flush. | Test method, effective flush volume, flushability, bowl cleaning and drainline transport | Improve whole-user water use, repairability, clog resistance, cleanability and lifecycle cost without losing certified performance |
| Non-sewered sanitation | ISO 30500:2025 | Prefabricated integrated NSSS scope, complete input/output and safety conditions | Build interoperable modular service chains with better maintainability and context-specific lifecycle performance |
| Faecal-sludge treatment | [ISO 31800:2020](https://www.iso.org/standard/75633.html) and applicable national rules | Feedstock composition, capacity, sampling and residuals boundary | Better verified emptying logistics, modular treatment, energy/transport and residual product accounting |
| WWTP process/control benchmark | IWA Benchmark Simulation Models BSM1 / BSM2 | Published plant layout, calibrated equations/parameters, influent disturbance, evaluation horizon and performance criteria | Compare controls on a common, cross-platform whole-plant benchmark; add field evidence only after model-to-plant validation |
| Utility service performance | ISO 24511:2024 | Service definitions, indicator calculation, customer/utility boundary and data quality | Extend from utility-level indicators to auditable end-to-end service and equity metrics without inventing one universal threshold |
| Sanitation risk | WHO Sanitation Safety Planning | Entire sanitation chain and hazard-control plan | Bind hazards, barriers, measurements, corrective actions and review evidence into the engineering lifecycle |
| Storm/sewer networks | EPA SWMM | Same rainfall, catchment, network, boundary and calibration/validation sets | Couple 1D network + 2D surface risk, green/gray assets, uncertainty and observed outcomes without duplicating a mature solver |
| Urban flood simulation | LISFLOOD-FP 8.2 adaptive GPU-DG2; HEC-RAS 2D verification/validation and benchmark datasets | Same supported benchmark cases, DEM, forcing, numerical observables, observation operator and compute hardware | Reduce compute at fixed physical-error budget; cross-check independent hydraulic implementations; prove transfer and robustness, not just speed |
| Decentralized low-energy treatment | 2026 NTSW multi-site field paper | Same influent, effluent quality, climate, unit process and energy boundary | Compete on whole-life O&M, land, resilience, verified reuse and reproducibility—not an unmatched kWh/m³ claim |
| Utility performance | ISO 24511 / local regulator and utility reports | Definitions, reporting period, population and service boundary | Measure delivered service, asset condition, repair time, safety and equity beyond a single NRW or plant-compliance number |

Useful published numbers require caution:
- WaterSense's 1.28 gpf is a toilet specification ceiling, not an observed average for every use.
- A 2026 NTSW study reports 0.05–0.17 kWh/m³, BOD5 <3 mg/L, NH4-N <1.5 mg/L and Enterococci <200 CFU/100 mL across three installations with multi-year monitoring. It is a valuable comparator, but three sites do not define a universal SOTA envelope.
- LISFLOOD-FP 8.2 reports roughly 2.0–4.5x speedups for selected dynamic-adaptivity cases; the speedup varies with resolution and event complexity. It is an aspirational comparator for compatible flood cases, not a blanket performance target.
- EU Directive 2024/3019 sets a long-term policy trajectory including energy neutrality for urban wastewater treatment plants by 2045. That is a regulatory/policy benchmark, not proof every process can already achieve net-zero energy.

## 7. Proposed targets (not achieved)

### Hard qualification gates
- 100% of benchmark cases end in an explicit disposition; zero silent skips.
- All declared water, pollutant, energy and solids streams have unit-consistent balances; residuals must fit a preregistered uncertainty budget.
- Zero false-safe outcomes in the frozen safety-negative corpus. Missing, stale, conflicting or out-of-envelope critical evidence is **Indeterminate / no release**.
- Zero inference from synthetic treatment output to physical effluent compliance or pathogen safety.
- Every energy, water-reuse, residuals and lifecycle metric declares its meter and boundary.
- Every bypass, alarm, outage, service interruption, failed repair and missing laboratory sample remains visible in the denominator.

### Competitive improvement goals
- First comparison: at least three treatment-train families and three operating contexts.
- Aim for at least 20% lifecycle-cost **or** resource-use improvement in one preregistered matched regime, while no safety, effluent, uptime, access or equity floor deteriorates. This is a stretch goal, not a promise.
- For low-energy decentralized treatment, investigate <=0.15 kWh/m³ gross electricity only where the process and effluent endpoint are comparable with the published NTSW field case.
- For toilets, demonstrate a measured reduction in L/user/day while maintaining or improving standardized functional tests, rather than simply setting an unsupported universal L/flush target.
- For flood adaptivity, target >=2x compute speedup only on matched cases where a qualified uniform-grid reference and compatible LISFLOOD-FP comparison show it is possible at the declared error tolerance.
- Set service availability and MTTR targets per deployment context before a field pilot; do not universalize one number across household, remote, municipal and industrial sites.

## 8. Mandatory negative controls

At minimum: blocked drainline; repeat/double flush; inadequate flush pressure; water shortage; power outage; stuck, drifting or stale sensor; invalid calibration; duplicate/out-of-order telemetry; clock skew; lab sample mix-up; conflicting lab and proxy measurements; false potability/effluent flag; pathogen breakthrough; industrial/toxic shock load; hydraulic overload; wet-weather inflow; flood ingress; blocked stormwater inlet; pump/blower/valve failure; membrane fouling; unmeasured bypass; missed sludge collection; unsafe emptying; treatment residue leak; forged maintenance completion; stale configuration; unsafe operator shortcut; communication outage; and unauthorized control command.

Every case needs a named expected disposition and an independent oracle. Include positive controls and counterexamples where central treatment or conventional designs perform better.

## 9. Luminous system roles and safety boundary

- **Symtropy:** physical process/hydraulic modelling only where equations and solver are validated; reuse existing water ledger and qualified hydrology solver.
- **Sol-Atlas:** terrain/catchment, GIS and asset spatial evidence, geometry, exposure, source vintage and uncertainty.
- **Symthaea:** read-only anomaly triage, uncertainty-aware design exploration and candidate explanations; no independent water-safety certification.
- **Mycelix:** provenance, lab results, approvals, maintenance and contested records; essential access must never depend on reputation, credits or contribution. Treat existing water-purity fields such as potability scores and WHO/EPA pass flags as submitter assertions unless independently supported by raw measurements, sample custody, method/calibration evidence and qualified laboratory results; a stored boolean is not a certification.
- **Robotics:** inspection/sampling/maintenance assistance within verified envelopes, physical isolation and human override.
- **Luminous platform:** requirements, BOM, revision, suppliers/materials, verification, commissioning, operations and repair lifecycle.

An AI recommendation or model prediction has no effect authority. Physical intervention requires separate scoped authorization, qualified interlocks, site/operator approval, and a tested safe fallback.

## 10. First implementation sequence

1. Reuse Mycelix #3151 runtime-neutral water conformance semantics and Mycelix #5431 Johannesburg service evidence contract.
2. Build T0 typed sanitation, water, solids and energy fixture vectors around the existing Symtropy #3319 / PR #3320 independent accounting oracle; do not create another stock/flow authority.
3. Add a minimal synthetic toilet-to-treatment process train with explicit model scope, unit processes, assumptions and parameter provenance.
4. Compare a conventional baseline and two alternative trains under matched influent/service demand with a multi-axis report.
5. Freeze a bench instrumentation and lab sampling plan before purchasing or building wet hardware.
6. Compose with Johannesburg and national data only after source bytes, definitions, rights, vintage and uncertainty are pinned.
7. Move to T1/T2 only after T0 and safety review; field pilots require local professional engineering and regulatory approval.

### T0 seed artifacts on this branch

- `schema/t0-accounting-case.schema.json`: JSON Schema 2020-12 contract for synthetic accounting-only cases, including explicit mass-balance tolerances and expected dispositions.
- `fixtures/synthetic-accounting-only-v0.1.json`: positive arithmetic fixture covering water, TSS, total nitrogen, total phosphorus, BOD5/COD load arithmetic, gross/recovered energy and effluent reuse/discharge allocation.
- `fixtures/synthetic-water-balance-negative-v0.1.json`: deliberately inconsistent water balance (5 L/day residual against a 0.01 L/day tolerance) with expected `Rejected` disposition.

A one-off local sanity pass parsed the schema and both fixtures and recomputed the declared water/constituent balances, BOD5/COD load-removal arithmetic, energy arithmetic and effluent/reuse allocation (25 checks passed). This was **not** execution of a full JSON-Schema validator, a Rust test suite, hosted CI, treatment-process physics, field test, or safety qualification. The numerical values are invented test vectors, not recommended design parameters or real-world observations.

### Standalone Rust arithmetic auditor

The package at `t0-auditor/` provides a JSON-reading CLI and a reusable library. It recomputes water and constituent closures, calculates BOD5/COD removal from flow-weighted loads (not concentration-only changes), checks gross/recovered/net energy and checks produced/allocated/delivered/stored/discharged water arithmetic. It rejects unsupported evidence ceilings and malformed inputs; it does not infer biological treatment performance, pathogen safety, water quality or discharge compliance.

Commands from the repository root:

```bash
cargo test --manifest-path research/sanitation-wastewater-benchmark-v0/t0-auditor/Cargo.toml

cargo run --manifest-path research/sanitation-wastewater-benchmark-v0/t0-auditor/Cargo.toml -- \
  research/sanitation-wastewater-benchmark-v0/fixtures/synthetic-accounting-only-v0.1.json

cargo run --manifest-path research/sanitation-wastewater-benchmark-v0/t0-auditor/Cargo.toml -- \
  research/sanitation-wastewater-benchmark-v0/fixtures/synthetic-water-balance-negative-v0.1.json
```

Five Rust unit tests are authored, but they have **not** been executed in this environment because a Rust toolchain is unavailable here. A separate one-off JavaScript recomputation of both fixtures matched the expected positive and negative dispositions and all declared arithmetic. That check is not a Cargo test, a full JSON-Schema validation, CI, process-physics validation or a physical test.

## 11. South African design constraints to retain

The Compulsory National Water and Sanitation Services Standards (Government Gazette 52814, 6 June 2025) are especially relevant:
- Sanitation interruptions exceeding 48 hours require alternative flushing water or interim sanitation provision.
- Sanitation safety planning and W2RAP must be climate-resilient and reviewed at least every two years.
- Full or reported on-site containments outside the schedule must be emptied within 10 days to an appropriate treatment facility.
- Annual sewer/faecal-sludge collection inspections and treatment process audits are required.
- Stormwater drains must be maintained to carry water while preventing litter/objectionable substances; stormwater should be prevented from entering sewerage systems.
- The regulations also require service planning to include wastewater generation, collection and treatment capacity, and faecal-sludge accumulation and safe management.

These clauses become testable requirements and evidence obligations, not merely policy citations. Actual site compliance must be established by the responsible authority and qualified professionals.

## References

1. US EPA, [Residential Toilets / WaterSense specification status](https://www.epa.gov/watersense/residential-toilets). Version 1.2 remains in effect; EPA has paused the effective date of version 2.0.
2. ISO, [ISO 30500:2025](https://www.iso.org/standard/87343.html), Non-sewered sanitation systems — Prefabricated integrated treatment units — General safety and performance requirements.
3. WHO, [Guidelines on Sanitation and Health](https://www.who.int/publications/i/item/9789241514705) and [Sanitation Safety Planning Manual (2nd edition, 2022)](https://www.who.int/publications/i/item/9789240062887).
4. Republic of South Africa, [Compulsory National Water and Sanitation Services Standards, Government Gazette 52814 (6 June 2025)](https://www.gov.za/sites/default/files/gcis_document/202506/52814gon6292.pdf).
5. US EPA, [Storm Water Management Model (SWMM)](https://www.epa.gov/water-research/storm-water-management-model-swmm).
6. Chowdhury & Kesserwani (2025), [LISFLOOD-FP 8.2: GPU-accelerated multiwavelet DG solver with dynamic resolution adaptivity](https://gmd.copernicus.org/articles/18/9827/2025/), Geoscientific Model Development 18, 9827–9854, DOI: 10.5194/gmd-18-9827-2025.
7. Ganesapillai et al. (2026), [Beyond the flush: a review of wastewater circular systems](https://www.nature.com/articles/s41545-026-00557-8), npj Clean Water 9, 31, DOI: 10.1038/s41545-026-00557-8.
8. [Decentralized wastewater treatment and water renewal via natural systems: performance evaluation from household to school-based scale](https://doi.org/10.1016/j.jenvman.2026.130241), Journal of Environmental Management (15 July 2026), DOI: 10.1016/j.jenvman.2026.130241.
9. IWA Modelling & Integrated Assessment, [Benchmarking / BSM1 and BSM2](https://iwa-mia.org/benchmarking/). BSM2 extends control comparison to a whole plant including primary clarification and sludge treatment/anaerobic digestion.
10. ISO, [ISO 24511:2024](https://www.iso.org/standard/82489.html), wastewater utility management and service assessment; gives indicator examples but does not prescribe universal thresholds.
11. USACE HEC, [HEC-RAS verification, validation and 2D benchmarking references](https://www.hec.usace.army.mil/confluence/rasdocs/hgt/latest/reference-documents).
12. European Union, [Directive (EU) 2024/3019 concerning urban wastewater treatment](https://eur-lex.europa.eu/eli/dir/2024/3019/oj).
13. South African Department of Water and Sanitation, [2025 Green Drop report findings](https://www.gov.za/news/media-statements/minister-senze-mchunu-releases-2025-green-drop-report-01-apr-2026).

## Nonclaims

This protocol is not an ISO certification, a water/reuse permit, a product design approval, an effluent test, a microbiological risk assessment or an operational flood warning system. Its targets are proposals until supported by frozen tests and independent evidence.
