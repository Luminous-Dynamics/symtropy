# HoloFrac External Benchmark and Industry Map v0

**Status:** research and evaluation plan; no SOTA or certification claim  
**Related work:** [HoloFrac implementation #5844](https://github.com/Luminous-Dynamics/symtropy/pull/5844), [Phase 2 HLB protocol #5958](https://github.com/Luminous-Dynamics/symtropy/pull/5958), [external qualification issue #6007](https://github.com/Luminous-Dynamics/symtropy/issues/6007), [source/solver receipts #5959](https://github.com/Luminous-Dynamics/symtropy/issues/5959), [budgeted experiment selection #5961](https://github.com/Luminous-Dynamics/symtropy/issues/5961).

## 1. Executive assessment

HoloFrac should not compete on feature count, render quality or generic generative floor plans. Existing commercial systems already combine site planning, BIM/data continuity, layout exploration, engineering simulation and construction coordination.

The defensible research hypothesis is an integrated, evidence-carrying lifecycle loop:

1. represent spatial, functional, temporal, material and component state;
2. propose multiple design/adaptation/repair/reuse candidates;
3. select the next experiment by expected decision value and bounded cost;
4. execute through registered simulation/measurement adapters;
5. bind every result to exact inputs, source revisions, assumptions and applicability;
6. selectively invalidate evidence after mutations;
7. preserve historical states and component identity through operation and second use;
8. allow people to steer trade-offs and override any operational action.

This loop is a hypothesis. The current executable work validates individual synthetic contracts; it has not yet demonstrated end-to-end superiority or operational building performance.

## 2. Industry landscape: compare actual capabilities, not slogans

### Autodesk Forma / Revit / Civil 3D

Autodesk announced on 15 September 2026 that Forma is expanding connected AEC workflows across early site design, detailed design and infrastructure. Civil 3D is becoming a Forma Connected Client; Revit and Forma share increasingly connected data/workflows; AI capabilities include Building Layout Explorer, drawing search, Drawing Change Analysis and Drawing Compliance Review. Its 14 October 2026 AU 2026 update describes a site-to-detailed-design workflow, room/unit-mix exploration and interior daylight analysis. The Building Layout Explorer is expressly described as experimental/research, with outputs intended as starting points rather than dependable finished design.

The 16 November 2026 Forma Construction release notes report more than 60 construction-product updates, spanning AI-assisted project-data visualization, data/model management, operations and preconstruction.

**Implication:** basic project context continuity, AI layout exploration, drawing/change awareness, daylight analysis and construction coordination are not differentiation by themselves.

**HoloFrac response:** benchmark the whole workflow on exact requirements, source/receipt lineage, lifecycle mutations and solver-backed outcomes. Do not claim to replace Revit, Civil 3D or construction data-management platforms.

### nTop

nTop's 5.23 update (18 May 2026) added CFD in its implicit-geometry simulation workflow, operating directly on implicit bodies without body-fitted volume meshes. nTop also offers parametric/implicit geometry and integrated structural, thermal and CFD design exploration.

**Implication:** HoloFrac should not claim superior topology/implicit geometry, CFD, FEA or parametric exploration without matched benchmark evidence.

**HoloFrac response:** target cross-scale semantic/temporal context, exact evidence currentness, lifecycle recovery and value-of-information selection around qualified solvers. Keep Symtropy's physics engine as the authority for numerical simulation.

### TestFit and other feasibility/layout tools

Feasibility/layout products already automate early site, zoning, parking, unit-mix and massing studies. A HoloFrac comparison must therefore use identical site constraints, zoning assumptions, unit programme, circulation/egress conditions, geometry tolerances and evaluation budget.

**HoloFrac response:** compare not only the number of options, but feasible Pareto quality, requirement satisfaction, downstream solver outcomes, rework and the cost of discovering infeasibility.

## 3. External benchmark and standards map

| HoloFrac claim | External anchor | What it tests | What it does not prove |
|---|---|---|---|
| IFC information exchange | buildingSMART IDS 1.0 | Computer-interpretable IFC information requirements and checking | Geometry, clash, dynamic calculations, external data or full engineering compliance |
| First-principles building physics | ANSI/ASHRAE Standard 140-2023 | Technical capabilities and applicability of building/system performance simulation software | Learned forecast-model accuracy, real-building calibration or control safety |
| Operational building control | IBPSA BOPTEST | Matched controller tests on Modelica building emulators, standardized REST control points and KPIs | Physical commissioning, field safety or site-specific performance |
| Design for disassembly/adaptability | ISO 20887:2020 | DfD/A principles, requirements and guidance; measuring performance against declared objectives | Universal numeric performance thresholds or proof that a particular component can actually be recovered |
| Circular early-stage design | BCEF (2026) | Separate GWP, land-use, reused content, disassembly potential, functional adaptation and product/service indicators | Complete lifecycle truth where passports, EPDs, condition or recovery records are missing |
| Operational digital-twin maturity | 2026 scoping review | Evidence maturity across live deployments and quantified outcomes | A performance benchmark for any one model/controller |

### Important scope notes

- IDS v1.0 is a data-delivery specification, not a geometry or physics engine. It should be paired with independent geometry, dynamics and receipt checks.
- ASHRAE 140 applies to appropriate first-principles building-performance simulation engines. Do not use it to claim a learned HDC/CfC predictor is qualified.
- BOPTEST provides a fair external simulation environment for control-strategy comparison. It does not certify a controller as safe to deploy in a real building.
- ISO 20887 supplies principles and guidance, not a universal numeric DfD/A score. HoloFrac must declare measurable case-specific thresholds.
- BCEF's circularity dimensions should remain separately reported. Do not hide hard constraints behind one weighted circularity score.

## 4. Operational maturity: the evidence gap

The 15 October 2026 scoping review *Beyond the dashboard: Architecture, maturity, and performance outcomes in operational building digital twins* screened 2,301 records and retained 37 deployments meeting strict criteria for real-time connectivity, physical instrumentation and operational use. Within the retained sample:
- 48.6% were descriptive;
- 24.3% reached prescriptive, control-driving maturity;
- 27% reported quantitative performance outcomes;
- prescriptive studies were roughly four times as likely as descriptive studies to report quantified outcomes.

This is a small, strictly filtered and education-building-heavy evidence base, so it should not be generalized uncritically to every building type. It nevertheless suggests a useful maturity ladder:

L0 — static BIM/model or synthetic fixture  
L1 — connected observations with source/clock/calibration provenance  
L2 — validated descriptive state and historical replay  
L3 — calibrated forecasts and explicit uncertainty  
L4 — budgeted, independently evaluated decision/experiment recommendations  
L5 — prescriptive control with hard admission constraints, human override and measured outcome reporting  
L6 — lifecycle adaptation/reuse with source-bound evidence applicability across multiple use cycles

A system must not call itself an operational digital twin at L5/L6 merely because it contains a simulator or dashboard. Require physical instrument connectivity, commissioning/calibration evidence, sustained operational use and measured outcome evidence.

## 5. Matched benchmark programme

### A. Information exchange and spatial fidelity

Use versioned IFC fixtures and IDS requirements. Report round-trip preservation of IDs, entity/classification/property/material data, coordinate frames, units and extensions; count unmapped semantics, lost properties, geometry deviations and source-identity collisions. Add independent geometric oracles for what IDS cannot express.

### B. Building physics

Apply Standard 140 to the appropriate first-principles engine. Add separate analytical manufactured solutions, convergence studies, mesh/time-step sensitivity, conservation invariants and cross-solver comparisons. Forecast models must have their own held-out metrics and may not inherit physics-engine qualification.

### C. Forecasting and virtual sensing

Use complete temporal episodes, buildings/years and spatial regions as holdouts. Compare persistence/seasonal, nearest-neighbour, bilinear/RBF/GP, AR/VAR/Kalman, LSTM/GRU, CfC/LTC, static HDC and HDC+temporal dynamics. Report per-channel/per-horizon MAE/RMSE, skill vs persistence, NLL/CRPS, interval coverage/width, calibration, missing/delayed/drift robustness, p50/p95/p99, RSS and energy where measured.

### D. Operations and control

Use the same BOPTEST case, weather, occupancy, actuator limits, sensors, disturbances, warm-up and test horizon. Compare reference schedules/rule-based control, ASHRAE Guideline 36 sequences where applicable, MPC, robust/stochastic MPC where supported, and published learned controllers. Report energy, cost, peak demand, comfort/IEQ violation-hours, control effort, faults, stability, computation and all failures. Require a non-inferior comfort/fault-safety result before claiming lower energy/cost.

### E. Design and experiment selection

Separate candidate generation/optimization from experiment selection. Compare NSGA-II/III, TPE, HypE, RBFMOpt and random/DOE for design optimization as appropriate. Compare random, Latin hypercube/maximin, uncertainty-only, sequential A-optimal and expected-decision-value/EVSI for experiment selection. Use matched high-fidelity solver budgets (25/50/100/250/500 calls), initial information, training data, fidelity, wall-clock and held-out problem families. Report feasible hypervolume, IGD+, time/calls to first feasible candidate, decision regret, realized value of information, constraint violations, cost calibration and resource use.

### F. Lifecycle and circularity

Use ISO 20887 principles and BCEF indicators as the framework. Require component identity/lineage, exact connection/access/disassembly sequence, condition inspection, material/passport provenance, mass-flow closure, actual recovery/second-use evidence and selective evidence invalidation. Compare static single-life design, conventional DfD scoring and source-rich lifecycle approaches using the same boundaries, functional unit, EPD geography/version, service period and replacement assumptions.

### G. Evidence impact

Compare exact dependency-DAG closure to full rebuild/revalidation and manually maintained dependency rules. Include local mutations, common-cause weather/model/material changes, omitted sources, incomplete manifests, wrong solver-input digests, stale build identity and late-arriving corrections. Require 100% independent-oracle agreement on frozen finite corpora, zero false-current evidence and conservative unknown/stale classification for incomplete inventory.

## 6. Prospective targets

These are research goals, not achieved results:
- 100% independent-oracle agreement for finite exact contracts;
- zero false-current receipts and zero admitted hard-infeasible actions;
- 100% component identity/lineage and mass balance on lifecycle fixtures;
- no loss of required IFC IDs/units/frames on declared round-trip tests;
- BOPTEST: non-inferior comfort and fault safety, with a predeclared target of at least 10% lower energy/cost on an applicable case;
- experiment selection: at least 20% fewer high-fidelity calls than the strongest matched baseline, at non-inferior feasible Pareto quality (2% relative hypervolume non-inferiority margin);
- lifecycle: zero unsupported reuse credits and zero false-current evidence after mutations.

Before testing, freeze the KPI, threshold, test case, margin, seed and failure accounting. Do not select the metric or threshold after seeing results.

## 7. Priorities and sequencing

1. Finish exact-head validation of the current HoloFrac synthetic suite.
2. Bind source snapshot manifests and solver input/result receipts (#5959).
3. Add a reproducible BOPTEST adapter/replay case and matched baseline controller.
4. Add IDS 1.0 IFC information checks alongside independent geometry/unit/frame oracles.
5. Apply Standard 140 to the relevant first-principles Symtropy building engine, if/when it exists and is in scope.
6. Reproduce one rights-cleared circular-design case with BCEF/ISO 20887 mapping and real component/passport/condition evidence.
7. Connect HLB experiment/value-of-information proposals through Symtropy admission and immutable solver receipts (#5961).
8. Only after these steps claim any cross-industry superiority.

## 8. Claim ceiling

A pass on IDS is not physical qualification. A Standard 140 result is not learned-model validation. A BOPTEST win is not a field deployment. ISO 20887 alignment is not actual recovery proof. Synthetic lifecycle mass balance is not a real material passport. Report each evidence layer separately and retain unknowns, failures and limitations.
