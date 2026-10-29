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
