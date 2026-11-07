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

### DPW-8 details to preserve

**OAT15A Verification 1a (required):** Mach 0.73, chord Reynolds number 3,000,000, static temperature 271 K, angles [1.36, 1.50, 2.50, 3.00, 3.10] degrees. Six-member grid family; four required, six desirable. Sidewall boundary condition is periodic.

**CRM Static FSI Test Case 2a(E) (required):** initialize CFD/FEM from jig/unloaded wind-off geometry; Mach 0.85, Reynolds number 5 million, dynamic pressure 1,384 psf, CL=0.5000 ± 0.0001, temperature 322.04 K. Comparison metrics include forces/moments, sectional twist/deformation, sectional Cp and separate flow/structural residuals. Measured span stations are η=[0, 0.4286, 0.5546, 0.6773, 0.7954, 0.9150].

**CRM Static FSI Test Case 2b(E) (required):** alpha sweep [-1.50, 0, 1.50, 2.50, 3.00, 3.50, 4.00, 4.50] degrees; grid level 3 in the working-group test sheet.

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

## 7. Current state and gaps

- The bounded Rust analytical oracle and independent Python replay have passed exact-head CI on PR #4110.
- The DPW-8 catalog and hostile consistency checks have passed exact-head CI on PR #4137.
- The CRM equivalent-beam file has only a browser text preview; bytes, terms, units and expected solution are not admitted.
- The OAT15A archive is listed but its member inventory is not inspected.
- No SU2/NASTRAN workshop solve has been reproduced by this programme.
- No CRM static-FSI result or Wren 2 / Wayfarer 4 physical result is claimed.

This document defines the comparison plan, not measured superiority.
