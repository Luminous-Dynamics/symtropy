# AER-01: industry benchmark and competitive qualification matrix v0.1

**Status:** research protocol and target definition; no SOTA superiority claim.  
**Related work:** #4093 structural/aeroelastic bridge, #4111 reference admission, #4139 read-only NASTRAN intake.  
**Primary references:** NASA Aviary, OpenMDAO, OpenAeroStruct, SU2, NASA NASTRAN/OpenMDAO integration, AIAA DPW-8/AePW-4, BOCoDe and ShapeBench.

## 1. Competitive position: where the industry already is

We should not benchmark against a caricature of the industry. The relevant reference systems already cover substantial parts of the proposed architecture.

| Reference | Demonstrated capability | What AER-01 should do with it |
|---|---|---|
| NASA Aviary + OpenMDAO | Conceptual aircraft design, propulsion/aero/weights/mission routines, conventional and hybrid aircraft optimization, and external subsystem integration | Use as an aircraft-level conceptual sizing and mission baseline; test our contracts against its variable/subsystem interfaces |
| OpenAeroStruct | Coupled VLM and 3-D spatial beam aerostructural optimization through OpenMDAO | Use as a low-/medium-fidelity coupled aeroelastic comparison, not as a high-fidelity FSI substitute |
| SU2 8.6.0 | CFD, adjoint optimization and newly listed experimental FSI+CHT/aerothermoelastic features | Pin its exact build and benchmark the supported solver cases; treat experimental FSI as experimental until the selected case is reproduced |
| NASTRAN + OpenMDAO | NASA has demonstrated static, aeroelastic, flutter and buckling SOL workflows, sensitivities, constraint extraction and integration of aerodynamic tools into OpenMDAO/Aviary | Interoperate with NASTRAN; do not rebuild its solver or claim to supersede its structural capability |
| AIAA DPW-8 / AePW-4 | Public, collaborative test matrix for CFD grid convergence, CRM static FSI, buffet, test-environment effects and data scatter | Reproduce exact workshop cases and preserve the community's conditions, data formats and discrepancy reporting |
| BOCoDe | 307 optimization problems, including 159 engineering tasks, with 31 compared optimizers | Reproduce supported task baselines and add a separate full-cost decision-quality track |
| ShapeBench | 103 aerodynamic tasks across eight shape categories with surrogate search and selected high-fidelity CFD verification | Use to test optimizer transfer and category-level generalization; do not infer universal superiority from a single geometry |

The NASA and community references establish that aircraft sizing, multidisciplinary optimization, beam-level aerostructural optimization, CFD adjoints and NASTRAN integration are existing capabilities. AER-01's intended contribution is the qualification and evidence layer that binds these engines to one exact aircraft/mission/configuration state and makes the engineering decision independently replayable.

### Release and evidence snapshot (7 November 2026)

- **OpenMDAO 3.45.2** was released 27 October 2026; its current documentation identifies a development build updated 2 November. Freeze the exact release/commit and environment for each baseline rather than using a moving latest install.
- **SU2 8.6.0** was released 30 October 2026. Its release notes list FSI+CHT/aerothermoelasticity as an experimental feature. That is a reason to test it on the admitted cases, not evidence that any given AER-01 coupling configuration is already qualified.
- **NASA Aviary** remains an actively developed OpenMDAO aircraft design/optimization tool and supports external subsystems. Its public validation guide is explicitly still under development, so use Aviary as a strong system/MDO baseline while admitting its validation evidence by individual case rather than treating it as a universal reference truth.
- **ShapeBench** reports 103 aerodynamic optimization tasks over eight shape categories and mean pairwise optimizer-ranking Spearman correlation of 0.013 across environments. This makes cross-category validation essential.
- **BOCoDe** reports 307 black-box optimization tasks, including 159 engineering problems, and compares 31 methods across five main problem classes. Use the existing Symtropy BOCoDe interoperability/reproduction protocol for its prescribed budgets, seed counts and all-attempt ledger; do not create a weakened local imitation and call it an official reproduction.

### Verification, uncertainty and dynamic-MDAO reference context

- **NASA-STD-7009B** is the active NASA standard for models and simulations. **NASA-HDBK-7009B Version B** (published 3 February 2026) is its public implementation handbook. AER-01 should borrow the disciplined acceptance-criteria, credibility and reporting structure, but must not imply NASA approval or compliance merely by adopting similar fields.
- **ASME V&V 10-2019 (R2025)** is the solid-mechanics V&V reference; **ASME V&V 20** addresses CFD/heat-transfer validation; **ASME VVUQ 20.1-2024** describes multivariate validation metrics for correlated responses. PR #4195 implements a separately disclosed response-vector diagnostic inspired by that problem class; it is not a reproduction or compliance claim for the proprietary ASME standard.
- **Dynamic MDAO** research (Bruggeman et al., Aerospace Science and Technology 175, 111748, 2026; DOI 10.1016/j.ast.2026.111748) demonstrates changing workflow topologies on a mathematical VSDS Goldstein example. This is a valuable architecture-level reference, but not proof of aircraft-scale benefit. AER-01 should compare static fixed graphs with architecture-specific dynamic graphs only after freezing a real engineering case and ensuring every selected graph satisfies the discipline/claim dependency contract.
- **NASA Design for Inspection (DFI)** and the Inspectability Metric provide a further design-space direction: inspectability, defect findability and inspection access should be considered alongside structural mass and manufacturability. Our #4165 work should benchmark against the published methodology without claiming to reproduce a NASA proprietary implementation.





## 2. Paradigm shifts worth pursuing

### 2.1 Aircraft state as the engineering system of record

Make the canonical object a versioned aircraft state, not a collection of files or loosely joined discipline outputs. It binds configuration, geometry, mass/loading state, environment, mission phase, controls, propulsion dispatch, mesh/model identity and uncertainty. Each analysis result is a typed transformation from one state digest to another.

A changed fuel location, payload, control setting, propulsor failure, geometry, mesh, material card or solver build invalidates dependent results unless a bounded and independently checked transform proves applicability.

### 2.2 Evidence-carrying computation rather than optimizer confidence

Every result carries exact input/output digests, source/model identity, units, coordinate frames, convergence state, residuals, uncertainty, applicability domain, resource cost and independent replay. The optimizer may propose and select; it does not certify its own answer.

### 2.3 Fidelity as an adaptive resource

Use a staged fidelity ladder:

1. analytical known-answer checks and exact bookkeeping;
2. low-order beam/wing-box models and source-bound NASTRAN deck intake;
3. airfoil CFD grid verification;
4. 3-D CFD and structural-model reproduction;
5. coupled static FSI and then unsteady/dynamic aeroelastic cases where required;
6. candidate-specific geometry, materials, propulsor interaction and measured physical correlation.

Escalate fidelity when the result could change an architecture or safety decision. Do not spend 33 GB-grid resources to answer a question that a 8 KB beam reference or an airfoil verification case can falsify.

### 2.4 Experiment selection by decision value

Choose the next measurement or solver run by expected reduction in decision regret per full engineering cost, not only by surrogate uncertainty or objective improvement. Compare random, space-filling, gradient/adjoint, Bayesian/multi-fidelity, evolutionary and Symthaea-guided selection on the same physics backends and frozen budgets.

### 2.5 Architecture discovery under hard constraints

Search discrete architecture choices (direct/series/hybrid propulsion, propulsor count/placement, wing arrangement, control topology, structural and maintenance concept) alongside continuous geometry and sizing. Preserve feasible sets and Pareto fronts. Hard safety and mission constraints must not be hidden inside a weighted scalar objective.

### 2.6 Robust service capability, not nominal range

Evaluate distributions and declared contingencies: hot/high, payload/fuel/CG, battery end-of-life, wind, icing where applicable, degraded propulsion, cooling loss, asymmetric thrust and maintenance downtime. Optimize completed mission service and lifecycle cost under declared risk constraints. Keep probability of violation, tail loss, worst-case margin and uncertainty calibration separate.

### 2.7 Let aircraft architecture change the analysis graph—but never the evidence gates

A conventional single-engine aircraft, distributed-electric aircraft and series-hybrid aircraft do not need identical analysis graphs. The graph should be generated from the selected architecture's actual subsystems and coupling paths, so irrelevant solvers are not run and mandatory disciplines are not omitted.

The graph compiler must consume an explicit architecture-to-discipline dependency contract. A propulsion topology change must trigger the required mass, power, cooling, wiring, structure, CG/inertia, controls and mission analyses. Every skipped discipline needs an explicit reason and must not be counted as assessed. Compare fixed-graph MDO and architecture-specific dynamic graphs on identical frozen cases, budgets and solver fidelities; measure total cost-to-decision, missed dependencies, stale-state acceptance and independent replay.

### 2.8 Co-design the ability to inspect and substantiate the aircraft

Treat inspection access, NDE method, SHM sensor location, sensor-fault discriminability, replacement access and evidence acquisition cost as design variables alongside structure and aerodynamics. This extends #4165 and #4056 rather than creating a new maintenance ledger. A structure that is slightly lighter but cannot be inspected reliably may be inferior over its service life.

## 3. External benchmark ladder and acceptance evidence

| Tier | Case | Primary outputs | Evidence goal | Claim ceiling |
|---|---|---|---|---|
| 0 | Rust beam/wing-box/load-transfer/scalar-coupling corpus | Known answers, conservation residuals, convergence/abstention | Exact arithmetic and software contracts; independent Python replay | Bounded analytical verification only |
| 1 | CRM equivalent-beam BDF intake | Card inventory, topology, source/units provenance | Read-only NASTRAN syntax/topology admission | No structural response claim |
| 2 | DPW-8 ONERA OAT15A Verification 1a | CL/CD/CM polar, Cp as available, six-grid convergence, residuals | Exact steady-RANS method reproduction; four grids required, six desirable | OAT15A transonic airfoil case only |
| 3 | DPW-8 CRM FEM Verification 1b and Static FSI Test Case 2a(E) | Forces/moments, sectional twist/deflection, Cp, separate flow/structural residuals | Structural model and static-FSI reproduction against the admitted workshop model/data | Exact CRM case only |
| 4 | Static FSI Test Case 2b(E), other CRM configurations and suitable dynamic cases | AoA/polar response, mesh sensitivity, loads, deformation and spectra where specified | Generalization across operating points/configurations | Exact admitted workshop families only |
| 5 | Wren 2 and Wayfarer 4 candidate-specific models | Payload-range, runway/hot-high, trim/stability, structural margins, energy/thermal, failure recourse | Candidate-specific analysis only after controlling data and physical correlation | No certification claim |
| 6 | Approved component/aircraft test and certification programme | Calibrated measurement, traceable requirements and authority-accepted compliance evidence | Independent, authority-governed substantiation | Only the approved scope |
| 0A | Correlated response-vector VVUQ diagnostic (synthetic first; CRM after admission) | Raw channel residuals, normalization, uncertainty-source covariance, D², marginal z and local gates | Independent Rust/Python agreement, covariance identity, no silent zero-fill/regularization, no held-out data leakage | Multivariate discrepancy diagnostic only; not physical validation or ASME conformance |
| 0B | Design-for-inspection / sensing observability (synthetic panels then admitted structural cases) | Defect findability, access/tool-path coverage, sensor fault vs damage discriminability, inspect/repair time | Independent detection/uncertainty and cost replay against published DFI principles | Candidate-specific inspectability study only |


### DPW-8 details to preserve

**OAT15A Verification 1a (required):** Mach 0.73, chord Reynolds number 3,000,000, static temperature 271 K, angles [1.36, 1.50, 2.50, 3.00, 3.10] degrees. Six-member grid family; four required, six desirable. Sidewall boundary condition is periodic.

**CRM rigid Configuration B 2a/2b:** the current DPW-8 Test Cases page specifies Mach 0.85, chord Reynolds number 5 million and 100°F (310.928 K). Rigid 2a is alpha=2.50° with grid levels 1–6; rigid 2b uses alpha=[-1.50, 2.50, 2.75, 3.00, 3.25, 3.50, 3.75, 4.00, 4.25] degrees at grid level 3 or 4.

**CRM elastic Static FSI 2a(E) (required):** Mach 0.85, Reynolds number 5 million, Q∞=1,384 psf, CL=0.5000 ± 0.0001 (approximate alpha 2.75°), 120°F (322.04 K), jig/unloaded geometry, and grid levels 1–6. Compare forces/moments, sectional twist/deformation at the six measured span stations, Cp and separate flow/structural residuals.

**CRM elastic Static FSI 2b(E) (required):** alpha=[-1.50, 0.00, 1.50, 2.50, 3.00, 3.50, 4.00, 4.50] degrees, 120°F (322.04 K), grid level 3, same Mach/Reynolds/dynamic-pressure reference and jig/unloaded geometry. This is the schedule in the dedicated elastic Static Deformation PDF and is distinct from the rigid Configuration B 2b schedule on the general DPW8 test matrix page.
The AIAA Static Deformation PDF supplies comparison datasets from NTF197, NTF215, NTF229, ETW ESWIRP and Ames216. Its PDF pages 7–8 are the controlling case sheets for 2a/2b(E); the generic Configuration B page is the separate rigid-CFD description.

Use the AIAA bending/twist template v3 semantics: deflection normalized by reference semi-span. Retain -999 as the documented optional-data sentinel; never reinterpret it as zero.

## 4. Fair industry comparison protocol

### 4.1 Freeze before comparison

Freeze and publish:
- candidate configurations, geometry, mass states and mission cards;
- solver/model versions, executable/build digests, settings and mesh families;
- source/reference dataset and terms status;
- optimization variables, bounds, constraints and objective definitions;
- random seeds, initial designs, training/calibration/test family splits;
- evaluation budgets and full-cost accounting rules;
- primary decision metric and statistical analysis plan.

No test-family measurements may be used for tuning or surrogate fitting.

### 4.2 Baseline participants

For the same admissible cases and equal information/budget, compare:

1. sequential engineering (aero then structure, sizing iterations);
2. deterministic sweeps / space-filling designs;
3. gradient/adjoint MDO where derivatives and solver support are valid;
4. evolutionary multi-objective search;
5. Bayesian and multi-fidelity optimization;
6. OpenMDAO MDF and IDF formulations where applicable;
7. Aviary mission/aircraft sizing with approved external subsystems;
8. OpenAeroStruct coupled beam/VLM analysis;
9. pinned SU2 and NASTRAN backends as physical-analysis engines;
10. Symthaea/Symtropy experiment-selection and architecture-discovery policy over the same physics backends.

Do not compare a high-fidelity workflow with a low-fidelity workflow by objective calls alone. Publish the fidelity and error of each model evaluation.

### 4.3 Score the complete decision process

Report all of the following separately:

**Physical/numerical agreement:** grid convergence, integrated forces/moments, Cp distributions, sectional twist and deflection, structural residuals, force/moment conservation, modal frequency/damping when specified, mesh/model sensitivity and comparison with admitted experimental uncertainty.

**Engineering feasibility:** feasible attainment, worst-decile constraint margin, false-feasible rate, out-of-domain abstention, hard-constraint violation and stale-identity rejection.

**Optimization quality:** feasible hypervolume, normalized regret where a trustworthy reference optimum/bound exists, time-to-first-feasible, evaluation-to-first-feasible, anytime best-so-far, Pareto coverage and sensitivity to budget.

**Total engineering cost:** human setup/review hours, solver and mesh setup, compute/wall time, measured energy where available, memory/storage, failed/timed-out runs, retries, surrogate training, tuning, post-processing, independent audits and experiment costs. Unknown resource fields remain unknown, never zero.

**Evidence quality:** fraction of inputs source-bound, independent replay success, unresolved/unknown inputs, applicability coverage and exact-head reproducibility.

### 4.4 Statistical design

- Use paired seeds for stochastic optimizer comparisons.
- Use at least five independent engineering families for a broad optimizer-transfer claim; hold out geometry/mission families from tuning.
- Report effect sizes and confidence intervals, not only mean ranks.
- Cluster aggregate outcomes by task/family; do not treat multiple seeds or mesh levels from one case as independent aircraft families.
- Preserve all failed/invalid attempts and the expected schedule. A complete-case ranking must be accompanied by a coverage/all-attempt ledger.
- Publish cost-quality curves, not only a single final score.
- For physical comparisons, set tolerances from measurement uncertainty, solver/mesh uncertainty and repeatability. Do not invent a universal percentage threshold.

## 5. Target gates

These are proposed research acceptance targets, not aviation standards or achieved performance.

| Area | Target |
|---|---|
| Frozen analytical corpus | 100% agreement with independent known-answer oracle |
| Synthetic hostile suite | Zero false-feasible results; every deliberate mutation rejected or explicitly abstained |
| State/evidence identity | 100% of admitted outputs bind configuration, geometry, loading state, mission phase and environment |
| Source admission | 100% of admitted bytes carry SHA-256, retrieval record, terms status and applicability; preview-only source never admitted |
| Unknown handling | Zero missing units, properties, load cases or measurements silently converted to zero/pass |
| Load transfer | Force and moment residuals within predeclared floating-point/mapping error budget |
| CRM static FSI | Report error at all six span stations plus integrated loads, Cp and separate flow/structural residuals; compare against admitted measurement uncertainty |
| CFD grid convergence | Reproduce at least four required grid levels; six desirable; show refinement and non-monotonic scatter, not only final residuals |
| Optimizer generalization | At least five independent engineering families with held-out geometry/mission tests for a broad superiority claim |
| Primary decision quality | Seek ≥10% improvement on a preregistered mission metric without worsening any hard constraint |
| Cost to decision | ≥20% reduction at equivalent/better independent decision quality as a stretch target only |
| Candidate validation | No Wren 2 / Wayfarer 4 superiority claim without controlling data and candidate-specific validation |

A zero-failure finite synthetic suite is not a zero-risk estimate. A successful CRM or OAT15A reproduction is not low-speed candidate-aircraft validation.

## 6. How AER-01 can outperform without replacing industry solvers

The strongest differentiated result would be:

1. Every discipline result is bound to one exact aircraft state and source/model revision.
2. Missing, stale, out-of-domain, non-converged or unlicensed evidence causes explicit abstention.
3. External solvers remain replaceable and independently versioned; parser and adapter outputs are checked independently.
4. Architecture discovery and experiment selection operate over the same physical backends as the conventional baselines.
5. The system identifies when the conventional architecture wins and can show the decisive evidence.
6. A design decision includes a replayable receipt and a full cost record.
7. Held-out families demonstrate transfer rather than a single overfit victory.

The competitive claim should be “more reliable independently reproducible engineering decisions per total engineering cost on held-out tasks,” not “our AI beats NASTRAN,” “we replace Aviary,” or “fewer CFD calls means a better aircraft.”

## 7. Standards and evidence references

Use these as versioned sources for the engineering programme, not as automatic certifications:

| Reference | Intended use in AER-01 | Boundary |
|---|---|---|
| NASA-STD-7009B + NASA-HDBK-7009B Version B | Model/simulation credibility, acceptance criteria, uncertainty and reporting discipline | The handbook guides NASA practice; adopting fields does not make a project NASA-approved or compliant |
| ASME V&V 10-2019 (R2025) | Computational solid mechanics verification and validation | Case-specific structural V&V; not an aircraft certification basis |
| ASME V&V 20 | CFD and heat-transfer validation | Use with appropriate experimental data, uncertainty and numerical verification |
| ASME VVUQ 20.1-2024 | Multivariate response discrepancy and correlated uncertainty | PR #4195 is a transparent diagnostic layer, not a claimed implementation/conformance of the standard |
| AIAA DPW-8 / AePW-4 case sheets | Shared transonic CFD, CRM structural and static-FSI comparisons | Exact case/geometry/data scope only |
| NASA Design for Inspection / Inspectability Metric | Co-design inspection access and defect findability | Published methodology reference; do not claim reproduction of unavailable proprietary internals |

## 8. Current state and gaps

- The bounded Rust analytical oracle and independent Python replay have passed exact-head CI on PR #4110.
- The DPW-8 catalog and hostile consistency checks have passed exact-head CI on PR #4137; the current catalog separates rigid Configuration B 2b from elastic 2b(E) and uses the dedicated elastic PDF schedule.
- The direct HTTPS retrieval attempt for the 8 KB CRM equivalent-beam BDF returned no bytes; this is recorded as a failed attempt, with no fabricated SHA-256. The file remains preview-only, not admitted.
- The NASTRAN read-only parser PR #4142 has a green synthetic-fixture CI result; it has not yet parsed the real NASA deck.
- The covariance-aware response VVUQ PR #4195 has a green synthetic Rust/Python result; it has not yet compared admitted CRM measurement data.
- The OAT15A archive is listed but its member inventory is not inspected.
- No SU2/NASTRAN workshop solve has been reproduced by this programme.
- No CRM static-FSI result or Wren 2 / Wayfarer 4 physical result is claimed.

This document defines the comparison plan, not measured superiority.
