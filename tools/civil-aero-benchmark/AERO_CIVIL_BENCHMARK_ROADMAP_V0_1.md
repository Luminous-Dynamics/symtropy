# Civil aerospace optimization: evidence-first benchmark roadmap v0.1

Status: research protocol and aspirational targets. No solver-performance claim is made by this document.

## 1. The paradigm shift: optimize decisions, not surrogate scores

The proposed differentiator is not another aerodynamic solver and not a claim that a learned model replaces CFD. It is an evidence-carrying design loop:

1. Express the mission and constraints explicitly: payload, range, reserve, operating envelope, manufacturing limits, cost and safety margins.
2. Generate candidate geometries with immutable IDs and exact geometry digests.
3. Use inexpensive analytical and surrogate models only for screening. Record their model version, training domain, uncertainty and applicability domain.
4. Select the next calculation or experiment by expected decision value, including total cost and the possibility of abstaining.
5. Escalate promising candidates through independently audited CFD fidelity levels.
6. Admit a result to the validated Pareto archive only when its solver configuration, mesh, convergence, uncertainty and source lineage are complete.
7. Confirm the final shortlist with physical measurements and manufacturing metrology.

Every candidate should carry a provenance chain:

requirements -> geometry digest -> mesh digest -> solver build/config digest -> convergence & uncertainty record -> force/pressure outputs -> independent audit -> optimizer decision -> physical test record

This creates a falsifiable record of why a design was selected and which evidence supports it. It prevents the optimizer from laundering a low-fidelity prediction into a validated engineering result.

A suitable division of labour is: Symthaea proposes hypotheses and selects information-gathering actions; Symtropy executes physics and geometry; a separately implemented verifier audits results; Mycelix (where integrated) preserves provenance and authorization. These are architectural roles, not claims that these integrations or capabilities are already complete.

## 2. Where the field is today

### Solver capability

SU2 is an open-source multiphysics suite for PDE solution and PDE-constrained optimization, with aerodynamic shape optimization among its primary applications. The latest public release is SU2 v8.6.0, tag commit 7048fd35f56d4bf52e586429d6746a36a10255f8 (released 2026-10-30). It includes GPU-enabled linear-solver updates, a pressure-based incompressible solver, multigrid improvements, and further adjoint / multiphysics capabilities. It is a serious reference baseline, not a straw man. For historical reproduction of the pinned DPW8 participant result, separately retain the reported SU2 8.0.1 build/configuration; do not substitute the latest version and call it a reproduction. The current civil-aero crate is an analytical screening and evidence-audit package; it is not yet a competing production CFD solver.

Reference: https://github.com/su2code/SU2

### Public verification and validation

The 2026 DPW8/AePW4 programme supplies a stronger benchmark ladder than a single generic airfoil objective. Its required OAT15A verification case V1a specifies Mach 0.73, chord Reynolds number 3 million, static temperature 271 K and five angles of attack: 1.36, 1.50, 2.50, 3.00 and 3.10 degrees. The grid family has six members; four are required and six are desirable. Additional cases cover unsteady buffet, wind-tunnel scatter, static deformation, high angle, high speed and fluid/structure interaction.

References:
- https://www.aiaa-dpw.org/TestCases/testcases.html
- https://www.aiaa-dpw.org/

NASA's CFD verification guidance separates programming checks, iterative convergence, consistency/conservation, spatial convergence, temporal convergence and comparison against highly accurate solutions. Merely obtaining similar answers from multiple codes is not sufficient; code-to-code comparison is useful only when the numerical methods and their possible errors are understood.

Reference: https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html

NIST's industrial VVUQ review describes use of Richardson extrapolation and Grid Convergence Index, recommends at least three successive grid levels for estimating discretization uncertainty, and discusses uncertainty in relation to measurement and model-form error.

Reference: https://nvlpubs.nist.gov/nistpubs/ir/2020/NIST.IR.8298.pdf

### Optimizer benchmark evidence

ShapeBench (2026) provides 103 aerodynamic shape-optimization tasks across eight shape categories and multiple optimization regimes. It includes validated surrogates and high-fidelity CFD where feasible, with matched-budget baselines. It reports mean pairwise optimizer rank correlation of approximately 0.013 across its task environments. That finding warns against declaring an optimizer generally superior from one geometry or one benchmark case.

Reference: https://arxiv.org/abs/2605.20763

## 3. The benchmark ladder

| Level | Benchmark | What must be demonstrated | Evidence ceiling |
|---|---|---|---|
| 0 | Parser/source integrity | Exact bytes, schema, source lines, no duplicate/lost records, sentinel preservation, deterministic parse, negative controls | Data-ingestion correctness only |
| 1 | OAT15A V1a steady RANS | Iterative convergence, conservation, grid refinement, observed order, GCI, CL/CD/CM and Cp comparison | Case-specific numerical verification and validation |
| 2 | OAT15A extended polar/buffet | Time-step convergence, statistically stable time averages, RMS and power spectra, sensor-location matching | Unsteady-flow validation for that test |
| 3 | CRM static deformation | Wind-off/as-built geometry lineage, structural model and boundary-condition audit, displacement/twist and aerodynamic load comparison | Coupled static-deformation evidence |
| 4 | High-angle/high-speed/scatter | Model-form sensitivity, transition/turbulence/boundary-condition matrix, wind-tunnel environment, repeatability | Regime-specific validation |
| 5 | Dynamic aeroelastic FSI | Coupled time integration, structural modes, phase/amplitude, spectra, energy/force consistency | Dynamic aeroelastic evidence |
| 6 | Physical article | Material/build lot, dimensional metrology, surface finish, calibrated rig, repeatability, expanded uncertainty and failure analysis | Test-article evidence only; not automatic type certification |

Do not merge these levels into one score. A steady RANS result cannot validate buffet, and a static FSI result cannot validate dynamic flutter or buffet spectra.

## 4. Fair comparison against industry and state of the art

### Solver comparisons

Use the historical participant build (SU2 8.0.1) for reproduction where exact mesh/config/build inputs are lawfully available, then a separately pinned current SU2 v8.6.0 baseline (tag commit 7048fd35f56d4bf52e586429d6746a36a10255f8). Also use, when licensed and available, a second solver using meaningfully different numerical methods. A commercial solver can be added as an industry reference only when the same geometry, mesh policy, physical model, boundary conditions, convergence requirements and available resources are disclosed. Record exact executable/build hashes and all configuration files.

For every solver run retain:
- geometry and mesh digests, cell/node counts, mesh quality and refinement lineage;
- solver version/build, turbulence/transition model, wall treatment, boundary conditions and discretization options;
- per-equation residual history, force/moment history, mass/energy consistency and stopping criteria;
- CPU/GPU hours, core-hours, wall time, memory peak, energy use, failed/non-converged runs and analyst setup time;
- CL, CD, CM, Cp, sectional/spanwise loads and unsteady outputs where required;
- observed grid/time order, extrapolated solution, GCI and measurement/model uncertainty.

Do not pick a solver because it matches one integrated coefficient. Compare against the relevant experimental uncertainty and inspect pressure distributions and other observables. Do not use agreement between two solvers as a substitute for experimental validation.

### Optimizer comparisons

Compare Symthaea/Symtropy policies against:
- adjoint or gradient-based optimization where derivatives are available;
- Bayesian optimization with a declared surrogate and acquisition rule;
- evolutionary baselines such as CMA-ES or differential evolution;
- a fixed-seed random-search baseline;
- the task-specific and general-purpose baselines supplied by ShapeBench where applicable.

Use identical frozen candidate spaces, constraints, initial data and total budgets. Charge every high-fidelity solve, failed solve, surrogate training run, tuning run, acquisition calculation, human intervention and independent audit to the budget. Report both wall-time and compute/energy cost. Use multiple random seeds and a blind geometry/condition holdout; never tune on the final confirmation set.

### Required metrics

**Verification/validation:** iterative convergence status; conservation defects; observed spatial/temporal order; GCI; CL/CD/CM error and bias; Cp distribution error; sectional-load error; experimental uncertainty; model-form sensitivity.

**Optimization:** best validated objective versus total cost; hypervolume versus cost; feasible-design yield; regret versus the best known validated result; Pareto coverage; median and worst-decile task performance; failed-run rate; wall-clock and energy cost.

**Uncertainty and robustness:** coverage of nominal 90% and 95% prediction intervals on blind data; calibration error; uncertainty versus observed residual; performance under specified out-of-domain shifts; abstention quality; false-feasible rate. Scenario perturbations must not be described as probabilities unless independently calibrated.

**Manufacturing/physical:** dimensional deviation, surface finish, repeatability, test-rig calibration, expanded uncertainty, build yield and observed failure modes.

## 5. Targets to pursue

These are proposed acceptance targets, not achieved results.

| Area | Minimum acceptance target | Stretch target |
|---|---|---|
| Data ingestion | Zero lost/duplicated canonical records; 100% paired-view agreement; 100% source-line traceability; zero sentinel-to-zero coercions | Independently parse multiple licensed official submissions and reject all one-field mutations |
| Reproducibility | Deterministic report and exact source/build/config digests | Independent third-party replay from a clean environment |
| CFD verification | At least three suitable systematically refined grids; explicit convergence and uncertainty; no silent non-converged result admission | Complete six-level V1a grid studies with uncertainty trends that satisfy the case's asymptotic consistency checks |
| Validation | Compare each required observable against the correct experiment and its uncertainty | Generalize across OAT15A, CRM and at least one distinct geometry family |
| Uncertainty | Nominal interval coverage measured on a blind confirmation set; OOD abstention measured explicitly | 95% empirical coverage for declared 95% intervals without destroying decision utility |
| Optimization | Matched total budgets, multiple seeds, blind task family; publish all failures and tuning cost | 20–30% fewer high-fidelity evaluations to reach the same validated Pareto quality as the strongest baseline, without degrading worst-decile feasibility |
| Physical evidence | One independently measured non-flight test article with full metrology and uncertainty record | Replicated physical results across build lots and operating conditions |

The optimizer stretch target is intentionally a decision-efficiency target rather than a claim of beating SU2's solver accuracy. It should be pre-registered and attempted only after the benchmark suite and confirmation split are frozen.

## 6. Immediate execution order

1. Complete and verify the DPW8 V5 parser: dual-view reconciliation, mesh identity, source-line retention, optional-sentinel semantics, byte digest and independent negative controls.
2. Build an uncertainty-aware grid-convergence auditor for V1a. Require explicit characteristic grid spacing, ordered mesh family and at least three members; calculate observed order and GCI only when the convergence assumptions are met. Reject oscillatory/non-monotonic sequences rather than producing a misleading single number.
3. Acquire official grids/configurations and exact historical solver inputs under clear licensing/terms. First reproduce the participant's historical solver version/configuration where possible, then compare a separately pinned current solver build.
4. Establish a common solver baseline matrix and a frozen ShapeBench/DPW8 optimizer evaluation split.
5. Add a decision-value benchmark in which EVSI is evaluated against realized reduction in expensive solves and final validated design quality, not merely synthetic arithmetic.
6. Only then advance candidates to physical coupons and calibrated test facilities.

## 7. Next evidence-integrity gap

The current CFD admission receipt is a summary-level contract: solver exit code, iteration count, one aggregate initial/final residual pair, a six-sample scalar monitor tail, conservation defects and resource totals. Its SHA-256 fields are caller-supplied strings whose syntax is checked, not hashes recomputed over actual binary/config/mesh/output files. The next step is to bind actual artifacts (solver executable/build, configuration, geometry, mesh, raw output, residual history and force history) by exact byte length and digest, preserve per-equation residuals and execution metadata, then independently verify the artifact bundle. Until then, a numerical admission pass is not an authenticated solver receipt.

## 8. Evidence policy

- A queued or running workflow is not a pass.
- A synthetic fixture proves code-path behaviour, not aerodynamic physics.
- A parser pass proves file consistency, not that the source author or simulation is correct.
- A solver pass proves neither experimental validity nor airworthiness.
- A low-fidelity screen never authorizes manufacturing or flight use.
- Every failure, excluded run and deviation remains in the record with a reason.


### Current solver baseline pin

As of 2026-10-31, SU2 v8.6.0 is the latest public release. Immutable tag commit: 7048fd35f56d4bf52e586429d6746a36a10255f8. Record the local executable digest, compiler/build flags, dependencies, hardware and exact configuration in every run receipt. Keep the historical SU2 8.0.1 participant reproduction lane separate from the latest-release performance lane.
