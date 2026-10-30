# AERO-CIVIL-001: robust Pareto and model-exploitation benchmark v0.1

Status: proposed research benchmark. The current reference implementation is still first-order cruise screening. All candidate rows in the committed corpus are synthetic and the reported results must remain labelled screening_only.

## 1. Research question

Can an engineering workflow produce candidates that remain useful when inputs and operating conditions change, rather than merely optimizing the nominal model?

The initial hypothesis is not that Symthaea is already better than state of the art. It is that a composable pipeline can be benchmarked on:
- engineering-constraint discipline;
- efficiency of expensive experiment selection;
- robustness to parameter and model discrepancy;
- honest prediction intervals and error breakdowns;
- reproducible candidate ranking;
- manufacturability and physical repeatability.

## 2. The first benchmark distinction

Do not conflate four different tasks:

1. **Code verification:** does the implementation compute its stated equations and reject invalid data?
2. **Solver verification:** are iterative and spatial discretization errors controlled?
3. **Model validation:** does the selected physical model agree with appropriate experimental observations within uncertainty?
4. **Optimization validation:** do optimized candidates retain improvement on held-out conditions and a physical test, without exploiting the model's errors?

A green CI badge answers only a subset of the first question. A converged CFD solution is not by itself empirical validation. A high-scoring optimizer candidate is not physical evidence.


## 3A. Measured-data model-envelope diagnostic (AERO-POLAR-001)

The first implementation adds a deliberately narrow reference-data diagnostic over the pinned NASA TMR Ladson NACA 0012 force observations. It fits `Cd_section = a + b Cl^2` separately for each 80/120/180 grit zone using only `|alpha| <= 8.5 deg`, then reports four error metrics over calibration, an intermediate holdout (`8.5 < |alpha| <= 16.5 deg`) and a high-angle extrapolation bin (`|alpha| > 16.5 deg`).

This is a measured **2D section-model discrepancy** experiment. The fitted `b` is empirical for that section dataset and must not be interpreted as the aircraft induced-drag term or Oswald efficiency. Roughness zones are not pooled. The angle bins are frozen data splits, not operating limits or stall criteria. The NASA TMR case states Re=6 million and recommends M=0.15 for compressible codes with mostly fully turbulent boundary layers; the source also warns about high-angle experimental two-dimensionality and sensitivity of drag to transition/tripping.

Exploratory values on the pinned source are:
- 80 grit: calibration RMSE 0.000216, intermediate holdout 0.003187, high-angle extrapolation 0.182186.
- 120 grit: calibration RMSE 0.000262, intermediate holdout 0.003657, high-angle extrapolation 0.208497.
- 180 grit: calibration RMSE 0.000232, intermediate holdout 0.004131, high-angle extrapolation 0.283891.

These results show a specific model's error can increase by orders of magnitude outside its calibration slice. They do not prove that any alternative model is more accurate. The immediate product requirement is therefore **domain-aware abstention**, not a new optimizer score.

### Model-envelope benchmark gates

Before claiming a new low-order or surrogate model is suitable for a design loop, measure:
1. **In-domain error:** MAE, RMSE, maximum absolute error and signed bias by zone and condition; normalize against published measurement uncertainty where available.
2. **Coverage and calibration:** empirical coverage of nominal 90% and 95% prediction intervals on sealed conditions, accompanied by confidence intervals and sample counts.
3. **Out-of-domain detection:** recall/precision for preregistered domain-shift and geometry/condition excursions; zero false-feasible admissions is the safety-oriented target for the screening gate.
4. **Abstention/escalation:** count and cost of candidates that require higher-fidelity CFD or new measurements; never count an abstention as a feasible design.
5. **Optimizer exploitation:** compare apparent low-fidelity gain with higher-fidelity replay on the same candidates; report rank reversals and the false-improvement rate.
6. **Reproducibility:** exact geometry/data/config hashes, solver and model versions, failed-run accounting, and byte-identical repeated reports.

Targets are aspirational gates, not achieved results: no false-feasible out-of-domain admission in the deterministic negative-control suite; calibrated 95% intervals should achieve coverage compatible with 95% on a genuinely sealed, representative evaluation set; and any claimed cost advantage must beat matched-budget baselines without degrading the pre-registered accuracy, coverage or feasibility gates.

### What "better than SOTA" would actually mean here

- On ShapeBench, use its official tasks and reproducible baselines, not a hand-picked NACA case. Compare matched total cost (surrogate queries, CFD calls, failed solves, tuning and orchestration), report per-task wins/losses and aggregate uncertainty, and reserve an untouched final task/seed split.
- Against SU2 adjoint and conventional optimizers, show a Pareto improvement in objective quality versus total compute/engineering cost on the tasks where the same constraints and solvers apply.
- Against NASA/AIAA validation references, show force and pressure agreement, grid convergence, uncertainty accounting and held-out conditions. A surrogate or optimizer leaderboard does not replace CFD verification/validation.
- In manufacturing, beat conventional processes on cost per conforming unit, first-pass yield, dimensional capability, repairability and energy per accepted part—not merely raw printing speed or CAD novelty.

A system is a stronger engineering method when it is more accurate **and** more calibrated, more reproducible, less expensive at matched quality, explicit about unsupported regions, and physically repeatable. Winning only a nominal simulation objective is not sufficient.


## 3B. Industry-facing 3D drag/aeroelastic validation ladder (AERO-DPW8-001)

The next external target should be the 2026 AIAA Drag Prediction Workshop 8 / Aeroelastic Prediction Workshop 4 (DPW-8/AePW-4), not an invented proprietary-style leaderboard. AIAA describes a joint drag-prediction/aeroelasticity program with working groups spanning static deformation, buffet, high angle, high speed, source of scatter and test environments. NASA's public DPW grid tree now exposes a DPW8 resource directory containing separate Buffet, ONERA_OAT15A, Scatter, Static_Deformation and Test_Environment branches. Treat them as distinct tasks and source families.

A useful staged sequence:
1. **ONERA OAT15A** — retrieve the experimental-data archive and compare surface pressure/force quantities only after condition metadata, units, transitions and applicable license/terms are pinned. Use multiple independently supplied grid families (Cadence, Helden, ONERA-Deck/Rizzi) as cross-code/grid sensitivity, not as interchangeable mesh files.
2. **Scatter / CRM** — use the DPW-8 Test_Case_2 README as a concrete provenance example: its configurations are tied to wing deformation measured at M=0.85 in a low-dynamic-pressure ETW test, with alpha labels from 2.50 to 4.25 degrees. It warns that source geometry included horizontal tails that were removed before gridding and that the farfield geometry was extended during gridding. Therefore geometry provenance and mesh mapping must be explicit; silently mixing the original geometry and gridded geometry invalidates comparison.
3. **Static deformation / FEM** — pair the aerodynamic solution with the exact CRM finite-element model and deformation mapping. NASA's DPW-8 README identifies the full-span wing-body-tail model as coming from the Common Research Model resource and a separate semispan FEM generated for DPW-8/AePW-4. Compare rigid vs deformed loads/shape with interpolation and load-transfer error accounted for.
4. **Buffet / high-angle / high-speed** — keep separate follow-on tasks. These require unsteady or transonic physics and cannot be represented by the current steady level-flight screen or by an isolated 2D polar regression.
5. **Test-environment and scatter accounting** — record facility, balance/pressure system, calibration, support/tare corrections, tunnel state, repeat runs and exclusions. A measured spread across facilities is not simply optimizer noise.

No DPW8 dataset is admitted to the repository by this protocol insertion. Before importing any artifact, pin the exact URL and file bytes, hash, geometry revision, condition/units, dataset ownership and redistribution terms. Workshop use or public visibility is not by itself a redistribution license.

Official starting points:
- AIAA DPW-8/AePW-4 overview: https://aiaa.org/courses/aiaa-drag-prediction-workshop/
- NASA DPW8 data tree: https://dpw.larc.nasa.gov/DPW8/
- DPW8 Scatter Test_Case_2 README: https://dpw.larc.nasa.gov/DPW8/Scatter/Test_Case_2/README.txt
- DPW8 Static_Deformation FEM README: https://dpw.larc.nasa.gov/DPW8/Static_Deformation/Test_Case_2/FEM_Models/README.txt

This is the meaningful bridge from the current 2D coupon/airfoil software smoke test to an industry-relevant 3D flow-and-structure validation program.



### Interoperable DPW-8/AePW-4 reporting

Do not invent an internal-only output format for the 3D comparison. DPW-8/AePW-4's official postprocessing page specifies Tecplot-readable ASCII submissions with dataset-identifying auxiliary metadata. It publishes templates for convergence, integrated forces and moments, surface cuts/pressure, unsteady time series, spectra, grid metrics, and bending/twist deformation.

The adapter should preserve the official template version and produce schema-compatible exports where each task requires them. Keep Symtropy's richer internal evidence ledger, but make the conversion deterministic and test both directions where a template parser is introduced. Preserve auxiliary run/participant/dataset identity, coordinates, units, and missing-data sentinel conventions; in particular, never turn an optional -999 sentinel into a physical zero.

Pinned template sources (hashes/byte lengths remain pending in the source-discovery manifest):
- https://www.aiaa-dpw.org/postprocessing.html
- https://www.aiaa-dpw.org/Forms/DPW8-AePW4_Convergence_v5.dat
- https://www.aiaa-dpw.org/Forms/DPW8-AePW4_ForceMoment_v5.dat
- https://www.aiaa-dpw.org/Forms/DPW8-AePW4_SectionalCuts_v5.dat
- https://www.aiaa-dpw.org/Forms/DPW8-AePW4_UnsteadyProbe_v5.dat

This is an interoperability requirement, not an assertion that the current screening CSV is already workshop-submission compatible.

### B7 entry point: DPW-8 V1a ONERA OAT15A

The first 3D industrial benchmark is the official DPW-8 V1a verification task, not an unspecified CRM optimization case. The official test matrix specifies a steady CFD/RANS grid-convergence study and polar at Mach 0.73, chord Reynolds number 3×10^6, static temperature 271 K, and alpha = [1.36°, 1.50°, 2.50°, 3.00°, 3.10°]. The committee specifies a six-member grid family: four levels required, six desirable. Use the committee-supplied grids where applicable; if another grid lineage is used, identify it separately and preserve exact mesh provenance.

The first comparison should be **solver verification before optimizer comparison**:
- pin one exact geometry and every grid level;
- run all five prescribed angles across at least four grid levels;
- report CL, CD, CM, Cp distribution, iterative residual/convergence status and resource cost;
- report observed convergence trends and uncertainty rather than selecting only the finest-grid number;
- compare methods only after boundary conditions, turbulence model, transition assumptions, mesh family and convergence rules are identical.

The official NASA/AIAA DPW-8 data tree lists the ONERA OAT15A experimental-data ZIP as 207 KB in its directory listing (last modified 2026-06-25); this directory metadata is not a raw byte count or a digest. The archive is not admitted until downloaded, independently hashed, inspected, and its applicable data terms and condition metadata reviewed.

For the CFD baseline, use the current official SU2 release as a pinned reference rather than inventing a solver: SU2 v8.5.0 "Harrier" (release page identifies 2026-04-27). Its release notes describe significant multigrid improvements, a DES grey-area mitigation strategy, fully functional thermoelasticity coupling and adjoint thermoelasticity. These are useful capabilities but do **not** by themselves prove that a particular DPW-8 aeroelastic coupling case is validated or ready. Resolve the release tag to an immutable commit, record build/compiler flags and test the exact task interface. The older SU2 static-FSI tutorial demonstrates a partitioned flow/elasticity approach; revalidate it on the pinned release before using it as a DPW-8 baseline.

Official references:
- DPW-8 test matrix: https://www.aiaa-dpw.org/TestCases/testcases.html
- NASA DPW8 data directory: https://dpw.larc.nasa.gov/DPW8/
- SU2 v8.5.0 release: https://github.com/su2code/SU2/releases
- SU2 static FSI tutorial: https://su2code.github.io/tutorials/Static_FSI/

Keep buffet Test Case 2 and 3 separate: JAXA describes Case 2 as unsteady CFD on rigid/static structural geometry using measured deformed wings and experimental data; Case 3 adds dynamic wing response/fluid-structure interaction. A steady V1a polar cannot validate either task.

## 3. Fair baseline ladder

All solver-dependent comparisons must pin the same geometry, units, boundary conditions, mesh family or documented mesh mapping, turbulence/transition assumptions, convergence criteria and objective definitions.

| ID | Baseline | Purpose |
| --- | --- | --- |
| B0 | Exact analytical screening model in this crate | Cheap smoke check and dimensional/monotonicity oracle only |
| B1 | Public NASA TMR NACA 0012 prescribed reference results | Verify input/output interpretation against a published solver-validation lineage |
| B2 | Fixed geometry solved with a pinned SU2 build/config | Establish CFD and mesh-convergence baseline |
| B3 | Standard SU2 adjoint/shape-optimization workflow | Engineering solver/gradient baseline on the tasks where it applies |
| B4 | Official ShapeBench task/API and its reproducible baselines | Broad matched-budget comparison across geometry classes/objectives |
| B5 | Decision-aware experiment-selection policy under the same frozen ShapeBench tasks and total cost | Proposed method; no assumed advantage |
| B6 | High-fidelity CFD replay and independent physical-data/coupon validation | Check whether the apparent optimizer win survives a higher-fidelity or physical boundary |
| B7 | AIAA DPW-8/AePW-4 benchmark tasks: ONERA OAT15A, CRM scatter, static deformation, then buffet/high-angle/high-speed | 3D drag prediction, aeroelastic coupling, data scatter and facility effects |

ShapeBench (2026) is a crucial benchmark addition, not something this project should attempt to recreate. Its preprint describes 103 tasks across eight shape categories and baselines including adjoint, L-BFGS-B, PSO, CMA-ES, Bayesian optimization, OpenEvolve, ShinkaEvolve and ShapeEvolve where applicable. It reports optimizer rankings that vary strongly by task and budget (mean pairwise Spearman rho reported as 0.013), illustrating why one easy airfoil result cannot establish general superiority. Exact code/data revisions, task licences, APIs, environments and cost rules must be reviewed and pinned before integration:
- ShapeBench paper: https://arxiv.org/abs/2605.20763
- Official code linked by the paper: https://github.com/ShapeBench/ShapeBench
- Dataset linked by the paper: https://huggingface.co/datasets/ShapeBench

B0 cannot be reported as competing with B2-B6 on physical accuracy. The first-order model lacks boundary layers, transition, separated flow, viscous details, and three-dimensional effects. Use each method only for the task it can support. ShapeBench results also do not imply validation for a new Reynolds/Mach regime or a manufactured article.

SU2 is explicitly designed for PDE-based simulation and aerodynamic shape optimization, including adjoint methods. We should compare against it, not create a new CFD solver to obtain a flattering but weak baseline:
- SU2: https://github.com/su2code/SU2
- SU2 official docs: https://su2code.github.io/
- NASA NACA 0012 reference case: https://tmbwg.github.io/turbmodels/naca0012_val.html

## 4. Experimental design and data leakage

- Freeze a train/calibration set, a model-selection set, and a sealed final evaluation set before optimization begins.
- Group by complete operating condition, geometry revision, tunnel run, specimen/build and acquisition session as appropriate. Never let adjacent time samples or repeated observations of one specimen leak across the split.
- Keep high-Reynolds-number NASA data separate from low-Reynolds-number application data unless the model explicitly handles the domain shift and reports it.
- Pin external source bytes, byte counts, SHA-256 digests, units, source revision, terms/licence and the parser version.
- Preserve raw observations, calibration readings, failed runs and exclusion decisions. Do not delete a difficult data point because it breaks the intended story.
- Do not query the held-out test set during model selection. Every access must be registered; a changed model must use a new untouched test split or label repeated reuse.

## 5. Metrics: keep them disaggregated

### Physical prediction
- Lift coefficient absolute error and bias by condition.
- Drag coefficient absolute/relative error and bias by condition; state how near-zero denominators are handled.
- Surface-pressure error using a declared interpolation rule, plus leading-edge and adverse-gradient diagnostics where the data supports them.
- Grid-refinement behavior, iterative residuals, conservation/force closure, solver failures and compute cost.
- Domain-coverage table: Reynolds/Mach/transition/geometry/angle conditions tested and those not covered.

### Uncertainty and robustness
- Empirical coverage of declared 50%, 90% and 95% prediction intervals on held-out groups.
- Interval width alongside coverage, so arbitrarily wide intervals do not look successful.
- Error by condition and worst-condition error, not only pooled MAE/RMSE.
- Performance under prespecified perturbations of mass, geometry, surface error, air density, speed, efficiency and measurement bias.
- Separate epistemic model discrepancy from measurement uncertainty and input variability where evidence supports that separation.

### Optimization and experiment selection
- Pareto hypervolume or another predeclared frontier metric with a shared reference point.
- Improvement per high-fidelity solver call and per wall-clock/compute budget.
- Number and severity of hard constraint violations; zero invalid/infeasible candidates promoted as deployable.
- Regret against the best feasible candidate found under the same evaluation budget, without treating a heuristic best-known solution as a global optimum.
- Value of additional information: did the selected expensive experiment change a downstream design decision enough to justify its cost?
- Reproducibility of the exact design and ranking from pinned inputs, model, executable and configuration.

### Manufacturing and ground test
- Nominal-versus-as-built contour error, chord/thickness/span/twist deviation and surface-finish readings with calibration and uncertainty.
- Measured article mass, first-pass yield, scrap/rework, build time, material use, direct cost with currency/as-of date, repairability and energy per conforming coupon.
- Force/pressure repeatability, tare/support correction, facility uncertainty and differences from the registered model prediction.
- Per-specimen results and all failed/aborted runs; do not report only the best build.

## 6. Pareto policy

The first implementation compares only candidates evaluated under the same mission (payload, route, speed, ambient density, energy efficiency and reserve) and evidence class. It first removes candidates failing the explicit energy or lift screening gates, then computes nondominated candidates across estimated route energy (lower is better), total mass (lower is better) and estimated lift/drag (higher is better).

This is deliberately not a scalar weighted score. The three-objective screen is still incomplete: mass is not structural adequacy, lift/drag does not represent dynamic stability, and estimated route energy omits climb, turns, wind, battery derating and motor/propeller maps. Pareto membership is therefore not an aircraft release recommendation.

Next versions should add predeclared uncertainty scenarios, model discrepancy and manufacturing measurements as constraints or objectives—not bake arbitrary weights into a single score.

The new v0.1 stress-screening CLI evaluates a candidate across an explicit CSV scenario set and reports maximum estimated route energy, worst energy margin, lowest lift margin and failure counts. The shipped factors are illustrative synthetic software fixtures, not measured uncertainty bounds or probability weights. Their use is to verify deterministic perturbation handling and demonstrate why nominal feasibility can be fragile. A real stress set must be preregistered from the target application's calibrated measurement uncertainty, validated model discrepancy, manufacturing variation and stated operating envelope. A scenario pass rate must never be called a reliability probability unless a justified probabilistic model and representative evidence support that interpretation.

## 7. Model-exploitation challenge suite

An optimizer must fail closed or be penalized when the candidate is invalid. Add separate fixtures for:

- impossible/non-finite physical inputs and unit mismatch;
- a candidate that reduces estimated energy by violating the lift bound;
- a candidate whose nominal prediction improves but whose held-out-condition error worsens;
- caller-declared uncertainty that is not supported by source evidence;
- a candidate outside the validated Reynolds/Mach/geometry domain;
- corrupt source bytes, wrong hashes, stale geometry, duplicate candidate IDs, non-deterministic output and missing uncertainty;
- a surrogate exploited outside its calibration support;
- a manufacturing tolerance/surface defect that changes measured performance relative to nominal geometry.

Never let a model provide its own approval. Independent checks must recompute key derived quantities and verify source identity and hard constraints.

## 8. Suggested acceptance targets for v0.1

These are software/protocol targets, not aerodynamic performance claims:

- 100% deterministic output for identical inputs and executable.
- 100% of invalid-schema, duplicate-ID, mixed-mission and mixed-evidence negative controls rejected.
- 100% of energy/lift-infeasible rows excluded from the Pareto front.
- A known dominated candidate is rejected in the frozen synthetic fixture.
- The candidate set yields more than one Pareto point when the fixture intentionally creates trade-offs; no single winner is fabricated.
- Every output includes screening_only; synthetic rows never produce validated, qualified or flight_ready.
- No dataset can be marked admitted while source bytes, hash, conditions, units and required review states are missing.

For physical model accuracy, set thresholds after the selected reference experiment and uncertainty budget are frozen. Predeclare those thresholds before examining the holdout result. Do not invent a universal aerodynamic percentage target.

## 9. Research landscape and honest comparison

Modern aerodynamic optimization already uses adjoints, numerical optimization and multi-objective robust optimization. The 2026 ShapeBench suite is now the strongest practical choice for a broad matched-budget optimizer comparison, because it includes heterogeneous tasks and multiple classical and LLM-driven baselines. The project should first reproduce a manageable subset before claiming any new method advantage. Its results also caution that rankings vary across problem classes, so report category/task distributions and fidelity gaps:
- ShapeBench (2026): https://arxiv.org/abs/2605.20763
- ShapeBench code: https://github.com/ShapeBench/ShapeBench

A separate 2026 paper on surrogate-assisted robust airfoil optimization reports a Kriging/EHVI approach and claimed efficiency benefits in its own tasks. It is a relevant comparator, but is neither independently reproduced nor beaten by this project:
- Paper: https://www.sciencedirect.com/science/article/pii/S2666496826000191

The proposed differentiator should be decision credibility per engineering cost: an adaptive workflow that selects informative measurements, preserves model limits, rejects model exploitation, and demonstrates that an optimized candidate survives held-out conditions and a manufactured coupon. That differentiator remains a hypothesis until measured head-to-head.

For manufacturing-digital-twin credibility, track current standards activity and measurement-science guidance:
- NIST Digital Twins Workshops Summary Report (NISTIR 8620, 2026): https://www.nist.gov/publications/digital-twins-workshops-summary-report
- NIST on harmonizing modeling/simulation uncertainty quantification and metrology (October 2026): https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing
- ISO/AWI 23247-8 VVUQ for manufacturing digital twins (work item approved in October 2026; under development, not a completed published standard): https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/09/48/94858.html

## 10. Next implementation slices

1. Finish the Rust evidence-aware Pareto CLI and its test corpus.
2. Ingest and independently hash the exact NASA TMR files; retain raw artifacts and parser tests.
3. Run B2/B3 from a pinned SU2 build and documented configuration; publish mesh/iterative convergence and force/pressure discrepancies.
4. Add a held-out low-Re dataset with its own applicability declaration.
5. Design a Symthaea experiment-selection baseline with a fixed expensive-evaluation budget and compare against random, space-filling and conventional optimizer choices.
6. Only after model/data gates pass, manufacture non-flight-critical coupons and compare as-built geometry and measured forces with declared uncertainty.
7. Do not promote this work to flight operations without separate airframe engineering, structural analysis, independent review and applicable permissions.
