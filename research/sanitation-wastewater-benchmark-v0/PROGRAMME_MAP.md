# Water + Sanitation Infrastructure Programme Map v0.1

**Parent programme:** [SAN-WW-BENCH-001 / Symtropy #4902](https://github.com/Luminous-Dynamics/symtropy/issues/4902)  
**Updated:** 2026-11-12  
**Status:** Work decomposition and evidence map. This document is not a deployment approval or a claim that any component has passed physical qualification.

## 1. Programme objective

Develop safe, affordable, maintainable and climate-resilient sanitation, wastewater, stormwater and flood infrastructure. Compare complete services, not isolated products. Every alternative is evaluated for quality, continuity, water, energy, pollutant loads, residuals, maintenance, cost, environmental effects, worker/user safety and equity.

Core service chain:

`need -> toilet/interface -> containment -> collection/emptying -> transport -> treatment -> authorized discharge/reuse -> residuals -> operation/maintenance`

Stormwater and flood paths are coupled through explicit hydraulic interfaces, but remain separate typed flows from potable water and sanitary wastewater.

## 2. Workstream ownership

| Workstream | Owner / current artifact | What it owns | What it must reuse | Status / evidence ceiling |
|---|---|---|---|---|
| Whole-chain benchmark | Symtropy [#4902](https://github.com/Luminous-Dynamics/symtropy/issues/4902), draft [PR #4903](https://github.com/Luminous-Dynamics/symtropy/pull/4903) | Matched service definitions, metrics, T0–T4 evidence tiers, common test contract | Existing hydraulic solvers, resource ledger, asset/work-order owners | Protocol and synthetic T0 artifacts committed; PR draft; no physical claims |
| Toilet + drainline bench | Symtropy [#4919](https://github.com/Luminous-Dynamics/symtropy/issues/4919) | Flush performance, use-water, bowl cleaning, waste extraction, drainline transport, trap seal, clogging and endurance | Effective WaterSense test profile, ASME/CSA fixture method, sanitation service-chain boundary | Bench design issue; no hardware built or tested |
| WWTP process/control reproduction | Symtropy [#4917](https://github.com/Luminous-Dynamics/symtropy/issues/4917) | IWA BSM1/BSM1LT/BSM2 reference runs, input/output pinning, Rust metric adapter, fair controller comparison | Existing physical water-accounting oracle and process/asset owners | Issue opened; no BSM parity or controller result claimed |
| Sewer robot inspection | Symtropy [#4905](https://github.com/Luminous-Dynamics/symtropy/issues/4905), draft [PR #4915](https://github.com/Luminous-Dynamics/symtropy/pull/4915) | Mission completeness, localization, observability, defect metrology, review and safe retrieval | Asset registry, trenchless eligibility, maintenance and GIS owners | Schema + two synthetic cases committed; no robot/dataset/field qualification |
| Flood + urban drainage hydraulics | Symtropy [#2328](https://github.com/Luminous-Dynamics/symtropy/issues/2328), [#2317](https://github.com/Luminous-Dynamics/symtropy/issues/2317), [#474](https://github.com/Luminous-Dynamics/symtropy/issues/474), [#461](https://github.com/Luminous-Dynamics/symtropy/issues/461), Johannesburg cell [#2267](https://github.com/Luminous-Dynamics/symtropy/issues/2267) | One-dimensional reference, urban pipes/pumps, shallow-water floodplain and adaptivity qualification | Existing water stock/flow and spatial-source owners | Existing workstream; preserve separate model/solver qualification gates |
| Asset maintenance + renewal | Symtropy [#3372](https://github.com/Luminous-Dynamics/symtropy/issues/3372) and trenchless [#3765](https://github.com/Luminous-Dynamics/symtropy/issues/3765) | Asset cohorts, condition evidence, maintenance and renewal method selection | Native asset and work-order lifecycle | Existing workstream; robot observations must not mint asset truth or repair authorization |
| Water-quality evidence separation | Mycelix [#6100](https://github.com/Luminous-Dynamics/mycelix/issues/6100), derived assessments [#3233](https://github.com/Luminous-Dynamics/mycelix/issues/3233), instrument metrology [#6034](https://github.com/Luminous-Dynamics/mycelix/issues/6034), condition applicability [#6057](https://github.com/Luminous-Dynamics/mycelix/issues/6057) | Separate raw samples from profile-bound quality/standards assessments | Mycelix EPI, calibration, instrument passport and sample evidence owners | Source-level defect identified; issue opened; no schema migration claimed |
| Service-data and South African evidence | Mycelix [#5431](https://github.com/Luminous-Dynamics/mycelix/issues/5431), [#5434](https://github.com/Luminous-Dynamics/mycelix/issues/5434); Symtropy [#4354](https://github.com/Luminous-Dynamics/symtropy/issues/4354) | Source-vintage, service KPI semantics, rights-cleared empirical corpus | Official DWS, Stats SA, utility and lab sources | Existing source acquisition and definition gates apply |

## 3. External comparison matrix

| Domain | Current external comparator | Defensible comparison |
|---|---|---|
| Tank-type toilets | EPA WaterSense tank-type toilet spec Version 1.2 (currently effective); EPA reports mixed-media flushability, bowl-surface cleaning, drainline transport and waste extraction; ASME A112.19.2/CSA B45.1:2024 | Reproduce the effective test method first. Compare litres per successful functional use, not just L/flush. Aim for at least 20% lower water per successful use only if every functional criterion is preserved. |
| Non-sewered sanitation | ISO 30500:2025, second edition | Compare complete prefabricated front-end + back-end scope; separately account for emptying, transport, external reuse/disposal and O&M outside the standard's product boundary. |
| Community faecal-sludge treatment | ISO 31800:2020 (confirmed current in 2026) | Compare within its prefabricated, community-scale and resource-recovery scope. Keep feedstock, energy independence, treatment capacity and residuals boundary matched. |
| WWTP control/process | IWA BSM1 (14-day control comparison), BSM1LT (609-day horizon with faults), BSM2 (adds sludge treatment); independent WaterTAP BSM2 implementation | Reproduce reference workload/indicators before claiming controller improvement. Separate BSM1, BSM1LT and BSM2 evidence. |
| Decentralized nature-based treatment | 2026 NTSW study: three installations, 2.5 years of water-quality monitoring; reports 0.05–0.17 kWh/m³ and selected effluent metrics | Use as a narrow field comparator, not universal SOTA. Match influent, climate, effluent endpoint and energy boundary. |
| Storm/sewer networks | US EPA SWMM 5.2.x | Cross-check the same rainfall, catchment, conduits, storage, quality processes and green-infrastructure controls. Preserve calibration/validation and model-version boundaries. |
| Urban flood | LISFLOOD-FP 8.2 GPU-MWDG2; existing Symtropy shallow-water reference #2328 | Measure mass residual, depth/velocity/arrival errors, inundation extent and runtime/memory at matched physical error. Published speedups vary by event complexity; do not set a blanket 4.5x target. |
| Sewer inspection robots | Shen et al. (2026): 84.8% mAP@50, 10.2 mm dimensional MAE, 11 fps on embedded Jetson Orin NX in a 0.8 m pipe; Li et al. (2026) review 183 papers | Reproduce the published dataset/split before comparing. Then hold out physical assets, utilities, robot families, operators and time periods; score mission completeness, localization and observability as well as detection. |

## 4. South African service and regulatory design constraints

The 2025 Compulsory National Water and Sanitation Services Standards (Government Gazette 52814, 6 June 2025) provide direct engineering requirements for this programme:
- plan for forecast wastewater generation and adequate sewer collection/treatment capacity;
- determine and forecast faecal-sludge accumulation and provide a safe emptying/transport/treatment/disposal or beneficial-use chain;
- implement climate-resilient sanitation safety plans (SSP) and wastewater risk-abatement plans (W₂RAP), reviewed at least every two years;
- schedule emptying of full on-site containments; identified/reported full containments outside the schedule must be emptied within 10 days to an appropriate treatment facility;
- maintain records of transported faecal-sludge volumes and treatment/disposal facility locations;
- annual sewer and faecal-sludge collection inspections and treatment process audits;
- service sanitation interruptions longer than 48 hours with alternative flushing water or interim sanitation service;
- keep stormwater inlets clear of litter/objectionable substances and prevent stormwater from entering sewerage systems;
- require wastewater plant design capacity to be documented and operated within authorized volume/conditions, with annual condition assessments and a process audit in a three-year cycle.

The 2025 Green Drop assessment reports 396 of 848 municipal wastewater treatment systems (47%) in a critical state. This is a 2025 assessment of 2023/24 performance, not a live real-time service measure. It motivates reliability and maintainability, but does not identify a specific technology as the solution.

## 5. Paradigm shifts with the highest leverage

1. **Whole-chain service economics:** optimize cost, water, energy and labour per safe, accepted sanitation service, not per flush, per module or per m³ alone.
2. **Safety-constrained circularity:** recover water, nutrients and energy only after intended-use quality and health/environment requirements are evidenced.
3. **Distributed modularity with service guarantees:** compare central, community, building-scale, non-sewered and hybrid designs under matched population/load/climate/maintenance conditions; no architecture wins by assumption.
4. **Observability as a design requirement:** treatment process, assets and robots must expose missingness, calibration, uncertainty, bypass, degradation and unobserved intervals.
5. **Predictive maintenance grounded in asset evidence:** join inspection, condition, failure, repair and service outcomes without mistaking a robot/video/AI score for an accepted condition assessment.
6. **Stormwater–wastewater interface by design:** maintain distinct flow authorities, model infiltration/inflow, overflow and flood ingress explicitly, and preserve upstream/downstream consequences.
7. **Polycentric and repairable infrastructure:** vendor-neutral data interfaces, local repairability, parts provenance, operator skills and viable alternatives.

## 6. Evidence and execution sequence

### Now — runner-independent
- Keep the sanitation T0 arithmetic auditor, synthetic positive/negative fixtures and source-boundary schema in the PR branch.
- Recompute fixture arithmetic locally/independently; do not claim Rust tests until a Rust toolchain executes them.
- Freeze source, edition, rights, method and metric definitions for the toilet, IWA BSM and robot benchmark tracks.
- Keep the water-purity raw-observation / derived-assessment fix scoped in Mycelix #6100.

### Next — test infrastructure
- Design toilet rig BOM, instrument calibration and standard-specific fixture configuration; reproduce a certified reference before modifying the geometry or flush pulse.
- Reproduce IWA BSM baselines and indicators; only then compare controllers/process variants.
- Acquire rights-cleared robot CCTV/sensor corpus, split by physical asset and campaign, and set up human adjudication.
- Compose flood solvers with SWMM/LISFLOOD references on matched cases and an explicit observation operator.

### Later — physical qualification
- T1 component bench with clean water and approved test media.
- T2 process pilot with independent laboratory sampling and declared energy/flow/solids boundaries.
- T3 supervised, permitted site demonstration with worker/user safety review and representative seasonal operation.
- T4 replicated multi-site/operator evidence and lifecycle cost; only then make general deployment claims.

## 7. Nonclaims

No product is certified, no toilet prototype is tested, no wastewater process is proven, no robot is field-qualified, no solver is established as SOTA, and no new system has been shown to beat industry. The benchmark and work packages are real engineering artifacts; performance superiority remains an empirical question.
