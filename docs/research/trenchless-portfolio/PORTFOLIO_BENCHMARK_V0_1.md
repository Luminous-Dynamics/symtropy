# Trenchless Portfolio Benchmark v0.1

**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/3765  
**Status:** initial research and evaluator contract  
**Owner:** Symtropy method-qualification adapter; consumes existing domain authorities  
**Claim ceiling:** synthetic contract verification only until matched physical and independent field evidence exists

## 1. Why this is a distinct work package

Pipeline renewal is not one construction method. A failed host pipe may be rehabilitated; a capacity-inadequate pipe may need replacement; a new crossing may require a guided bore; a dense utility corridor may rule out some trenchless methods; an emergency repair may make open cut the only practical option.

Johannesburg Water's public Pipe Replacement Programme identifies a five-year plan of 428 km of water pipeline replacement and 286 km of sewer replacement. This establishes a local infrastructure-planning context, not proof that any specific route is trenchless-eligible. Source: https://www.johannesburgwater.co.za/about-us/campaigns/pipe-replacement-programme/

The FHWA trenchless utility guidance enumerates horizontal directional drilling, auger/slurry boring, pipe jacking/microtunnelling, impact moling/ramming, and pipe bursting. It also states that trenchless methods are not suitable in every situation; emergency conditions, unsuitable ground/rock and uncertain existing utility locations can require or favour trenching. Source: https://www.fhwa.dot.gov/utilities/utilitycuts/man04.cfm

A 2025 review of pipe-jacking instrumentation identifies a particularly actionable technical gap: jacking force and pipe response depend on ground, lubrication, overcut, stoppages, alignment and operating mode; design practice often relies on empirical relations and comparable historical drives. The review describes earth/pore pressure and axial/hoop strain instrumentation, distributed sensing, and pressure-controlled lubrication as ways to improve understanding and reduce friction. Source: https://doi.org/10.1016/j.undsp.2024.12.003

## 2. Paradigm shift

The goal is not to maximize the use of trenchless technology. It is to minimize the risk-adjusted lifecycle cost and disruption of delivering a specified, independently accepted utility service.

Use this sequence:

1. Define the service outcome: utility type, capacity, pressure or grade, material, reliability and design-life requirements.
2. Characterize the existing asset and route: condition, geometry, cover, ground, groundwater, nearby utilities, structures, access and surface constraints.
3. Generate candidate method/configuration pairs from declared capabilities—not generic method labels.
4. Screen every hard requirement using evidence with units, source, revision, uncertainty and site scope.
5. Preserve all failures and unknowns. An unknown is never a pass.
6. Pass only eligible options to separate accepted-output and lifecycle-cost evaluators.
7. Compare Pareto alternatives under the same service and site boundaries; do not compress safety, service quality, cost, disruption and environmental impacts into one opaque score.

This is a method-portfolio benchmark, not a method recommender that always returns a winner.

## 3. Method families and their applicability boundaries

| Method family | Primary task family | Important evidence gates / failure modes |
|---|---|---|
| Open-cut replacement | New or replacement pipeline | Surface access, traffic and reinstatement, utility congestion, groundwater, trench stability and safe excavation support |
| CIPP / structural lining | Host-pipe rehabilitation | Host geometry/condition, cleaning and preparation, service connections, cure process, chemistry/temperature, structural design and post-install inspection |
| Close-fit lining | Host-pipe rehabilitation | Host dimensions/condition, bends, insertion access, reduction/reversion process, hydraulic capacity after lining and structural design |
| Sliplining | Host-pipe rehabilitation | Host geometry, annular space/grouting, reduced internal diameter, connection details and residual hydraulic capacity |
| Pipe bursting | Replacement along/near existing alignment | Host pipe material and condition, adjacent asset clearance, ground displacement, upsizing ratio, service connections and pulling loads |
| Horizontal directional drilling | New installation/crossings | Entry/exit geometry, bore radius, drilling-fluid regime, ground/frac-out risk, tracking quality, utility clearance and pullback load |
| Auger boring | Usually relatively straight crossings | Launch/receive pits, ground support, alignment control, casing/product compatibility, obstruction risk and spoil removal |
| Slurry microtunnelling / pipe jacking | Guided, controlled-grade installation | Face support, pressure balance, jacking thrust, pipe/joint capacity, lubrication, overcut, steering, shafts, slurry/spoil separation and settlement |
| Pipe ramming / impact moling | Shorter crossings and small conduits where suited | Vibration, ground movement, obstruction risk, alignment, achievable diameter/length and local asset sensitivity |
| Direct Pipe / hybrid method | Specialized guided pipeline installation | Named system configuration, entry/exit footprint, product pipe, steering, face support, slurry/spoil system and verified equipment envelope |

The table is a taxonomy for benchmark adapters, not a construction design guide. The task-specific competent engineer remains responsible for method selection and design.

## 4. First implementation contract

The initial Rust crate is at:

- `crates/domains/symtropy-trenchless-core/`
- Issue: https://github.com/Luminous-Dynamics/symtropy/issues/3765

The evaluator receives candidate method/configuration records bound to a non-empty versioned constraint-profile reference, the full required constraint-ID inventory, and evidence-bearing numeric constraints. It rejects omitted or undeclared checks. Each candidate/requirement side must also declare an explicit evidence policy (AnyKnown or an allowlist); known values outside that policy remain unresolved rather than passing. It checks:
- exact required-constraint inventory coverage;
- explicit numeric unit equality;
- finite numeric values and non-empty source references;
- unique method IDs, profile constraint IDs and accepted evidence classes;
- candidate capability >= required capacity, or candidate predicted impact <= allowed impact;
- conservative outcome semantics.

Status precedence is:
- Ineligible if at least one known hard constraint fails;
- Indeterminate if no known failure exists but one or more facts are unknown/not applicable;
- Eligible only if every supplied hard constraint is known and passes.

The evaluator does not authenticate profile/source references, decide whether the authoritative profile is complete, decide whether a constraint is relevant, invent missing numerical envelopes, compute engineering predictions or rank methods. The upstream adapter must construct the complete, method-specific check set and preserve evidence provenance. An empty constraint set is rejected.

## 5. Required scenario inputs

Each frozen work package should identify:

### Service and asset
- Potable water, wastewater, stormwater, conduit or other service.
- Required internal diameter/capacity, pressure or gravity gradient, material, connection requirements, reliability and design life.
- Existing asset ID, material, age/condition evidence, defect morphology, alignment and inspection coverage.
- Whether the task is repair, rehabilitation, replacement, upsizing, new installation or crossing.

### Route and ground
- Route geometry, length, depth/cover, curvature, vertical grade, tolerance and coordinate frame.
- Ground/rock class, strength/abrasivity where tested, discontinuities, boulders, fill, contamination, voids and uncertainty.
- Groundwater elevation/head/pressure and seasonal variation where relevant.
- Settlement/heave limits, adjacent building/foundation sensitivity, traffic/surcharge and other third-party constraints.
- Utility-survey quality, date, coverage, confidence, known conflicts and unresolved zones.
- Surface access, launch/receive areas, shaft/pit footprints, right-of-way, traffic and reinstatement.

### Method and machine
- Exact method family and named equipment/configuration.
- Tool/cutter, pipe/liner product, diameter and material.
- Firmware/software and calibration revision where relevant.
- Declared length, diameter, curve, depth, thrust/pullback, torque, pressure and environmental limits as relevant to that method.
- Manufacturer declaration vs measured capacity vs independently derived prediction must remain separate.
- Maintenance/service requirements, mobilization, specialist operators and consumables.

### Construction and service outcome
- Excavated/spoil volume, swell assumptions and custody/disposition.
- Water, slurry, lubricant and energy boundaries.
- Predicted/observed settlement and ground response.
- Independently measured line/grade and as-built geometry.
- Pressure/flow/leakage/infiltration or gravity-flow acceptance.
- First-pass/final acceptance, rework, service interruption and restoration.

All facts bind units, time, frame, source reference, evidence class, uncertainty where available, and exact upstream artifact identity. Unknown fields remain unknown.

## 6. Benchmark design

### V0: deterministic synthetic conformance

Create a frozen corpus where the expected outcome is specified by the contract:
- simple eligible candidate with all checks known;
- one hard failure in otherwise eligible candidate;
- all missing ground evidence;
- missing utility-location clearance;
- stale or conflicting source fact;
- mixed units;
- NaN/infinity;
- duplicate IDs;
- missing equipment envelope;
- rehabilitation option with missing host-pipe condition;
- predicted settlement above the limit;
- jacking capacity below required load;
- eligible and ineligible candidates in the same portfolio;
- no candidate eligible;
- one method's missing evidence must not affect another method's result.

Negative controls must prove that missing values cannot be interpreted as zero, unsupported vendor declarations cannot become measured data, and an apparently high confidence prediction does not override a hard physical limit.

### V1: public-data and literature reproduction

Use published cases only when source data and metric definitions are available. Do not scrape or invent unavailable field telemetry. Reproduce the study's stated target, input set, split and metric before any cross-study comparison.

For machine-learning predictions (for example, settlement/jacking load/cutter wear):
- compare rules/engineering formulas where applicable, persistence/mean baselines, linear and tree models, then temporal/GNN models only when data support them;
- split by project, drive, asset, machine or geology, not random adjacent rings/segments;
- preserve chronology where forecasting;
- report calibration, interval coverage/width, severe-event recall and out-of-domain behaviour;
- separate prediction accuracy from downstream decision utility.

### V2: physical matched trials

A future matched study should compare eligible methods on comparable work packages with independent measurement and acceptance. Stratify by service type, diameter, depth, ground, groundwater, route length, access/shaft need, traffic/surface sensitivity and existing-utility congestion. Record site preparation, shaft construction, mobilization, utility survey, specialist labour, spoil, reinstatement, service interruption and downstream inspection—not only machine advance.

A second site and independent measurement chain are required for transfer claims. Shadow-mode observations come before any machine integration; no new control path is authorized by this benchmark.

## 7. Scorecard

Never collapse all dimensions into one scalar. Report:

| Dimension | Primary outputs |
|---|---|
| Eligibility | False-eligible/false-ineligible, indeterminate, reason coverage, constraint evidence completeness |
| Installed output | Independently accepted service-ready metres, first-pass/final acceptance, geometry and quality |
| Service | Capacity/pressure/grade, leakage/infiltration, reliability, interruption/customer-hours |
| Ground/surface | Settlement/heave, damage, traffic/lane-hours, reinstatement defects |
| Workflow | Total elapsed hours, critical-path duration, specialist labour, setup/shaft time, spoil and logistics |
| Resources | kWh per accepted metre, water/slurry/lubricant, consumables, cutter/liner/pipe loss |
| Lifecycle | Cost per accepted service-ready metre, maintenance, rehabilitation/replacement interval, service delivered over declared life |
| Transfer | Performance by held-out site/geology/machine/contractor and calibrated uncertainty |

The primary economic denominator is not merely a metre of tunnel. It is an independently accepted metre of pipeline delivering the declared service capacity and quality over a stated lifecycle boundary.

## 8. Prospective targets, not claims

Initial protocol-quality gates:
- 100% correct outcome on frozen synthetic conformance cases.
- Zero false-eligible outcomes from missing, stale, conflicting, malformed or unit-inconsistent required facts.
- 100% of results retain exact constraint IDs, source references and evidence classes.
- Deterministic output for frozen inputs and evaluator revision.
- No ranking when the evidence is incomplete or no method is eligible.

Initial field hypotheses:
- Test whether a 10–20% lifecycle-cost reduction is achievable on a defined repeatable work-package class at matched accepted service quality and risk. Do not make it a universal promise.
- Test whether instrumentation plus pressure-controlled lubrication reduces jacking-force peaks, stoppages and pipe/joint stress on comparable drives; the 2025 review supports studying these mechanisms, but does not provide a universal percentage target.
- Test whether a method selector reduces method-ineligible mobilizations, variation orders and avoidable rework.
- Test whether utility-survey uncertainty is reduced enough to widen the set of methods that can safely be considered.

## 9. Ownership and integration

This work package is deliberately thin:
- #3372 owns water-asset renewal and service feedback.
- #1834 owns robot/tool capability-envelope matching.
- #1588/#1603 own material-aware excavation and cell-mask/spatial-delta integrity.
- #1595 owns shared digital-twin robotics scenarios.
- #1933 owns underground mine campaign systems.
- #3305 owns TBM ring-level ground/machine/tool/energy/wear benchmarking.
- #3767 owns instrumented pipe-jacking measurements, distributed pipe response and lubrication experiments.
- #3605 owns accepted output and production accounting.
- #3631 owns lifecycle cost per accepted functional unit.
- #3566 owns permission-to-evidence chain integrity.
- #3591 owns metrology reproducibility and cross-laboratory comparison.
- #3706/#3644 own mixed-fleet interoperability and construction-task semantics.

Symthaea contributes bounded geological inference and active sensing; Symtropy contributes scenario evaluation and physical-model interfaces; Sol-Atlas contributes spatial/site/network evidence and service context; Mycelix contributes authorized provenance, custody and decision receipts. No layer inherits another's authority.

## 10. Claim boundary and next steps

This benchmark does not prove that a method is safe, design a tunnel, authorize excavation, select equipment for a live site, certify compliance or control machinery. Engineering and statutory approval remain external requirements.

After this standalone contract is reviewed and its tests are executed:
1. add a method-adapter for the existing Johannesburg water-maintenance scenario without duplicating its asset/service model;
2. connect capability-envelope evidence from #1834;
3. connect accepted-output and lifecycle cost records from #3605/#3631;
4. add a small frozen, clearly synthetic method-screening corpus;
5. only then consider physical trials, with a competent utility partner and independent measurement.
