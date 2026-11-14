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


## Machine-readable solver source references

The current and historical SU2 baselines are recorded in `data/solver_reference_manifest.v0.1.json` with exact tag commit SHAs. SU2 8.0.1 is reserved for reproducing the historical DPW8 participant setup; 8.6.0 is the latest-release comparison. Both lanes must use equivalent, independently admitted inputs before comparing numerical results. The manifest pins source only; it does not certify a build, run, solver receipt or validation.


## Risk-proportionate VVUQ

Model credibility effort should scale with model influence and decision consequence, not just model complexity or data volume. The repository adds a qualitative, versioned Luminous requirements matrix inspired by NIST AMS 100-85 (2026). It assigns evidence requirements by model creation type (mechanistic/data-driven/hybrid), logic transparency and artifact access. The output is requirements-only and cannot certify evidence completion or authorize flight/manufacturing. See RISK_PROPORTIONATE_MODEL_CREDIBILITY_PROTOCOL_V0_1.md.


## 9. DPW-8 protocol-conformance audit (2026-11-01)

A new comparison must happen before solver reproduction. The official OAT15A V1a instructions require periodic sidewall boundary conditions, while the pinned IISc participant submission metadata states that symmetric boundary conditions were applied. The same official task defines four grids as required and six as desirable; the participant records four. Therefore the participant record meets the grid minimum but has a documented boundary-condition deviation. No physical equivalence between symmetric and periodic treatments is assumed.

The Rust task-conformance module now reports these separately:
- operating-point values and the five prescribed alpha values conform to the declared V1a conditions;
- sidewall boundary condition is a deviation (symmetric observed, periodic required);
- grid family meets the minimum but not the desirable six-grid target;
- exact geometry, mesh, executable and solver-configuration byte identities remain unknown.

The audit is metadata-only; it does not copy participant bytes or execute SU2. Its aggregate status is deviations_detected_and_incomplete, not a solver pass/fail. See DPW8_TASK_CONFORMANCE_PROTOCOL_V0_1.md and issue #2936.

This changes the reproduction plan:
1. Historical-participant lane: reproduce the participant's actual symmetric-boundary setup exactly, once lawful source access and all geometry/mesh/configuration artifacts are available.
2. Strict-task lane: run the official periodic-sidewall V1a protocol on the prescribed grid family.
3. Compare the lanes only as a declared boundary-condition sensitivity study. Do not combine them into one solver-version comparison.
4. Ask the participant/workshop maintainers whether the symmetric setup was a permitted variant and request clarification before interpreting the discrepancy.

The lesson generalizes beyond DPW-8: protocol identity must be audited independently from result-file integrity and solver numerical verification. A numerically converged result can still be a result for a different problem.


## 10. MDO and solver interoperability programme (2026-11-01)

The next scale-up is not to rewrite every aerospace solver. It is to integrate mature discipline tools behind a Rust-owned, evidence-first coordinator and compete on end-to-end decision quality.

Reference roles:
- SU2 v8.6.0: open CFD and PDE-constrained aerodynamic optimization baseline; retain 8.0.1 as the historical participant lane.
- NASA FUN3D 14.3: independent high-end CFD/adjoint/adaptation comparison when legitimately accessible. Its distribution is export-restricted, so it is not a default dependency.
- OpenMDAO 3.45.x: external MDO framework baseline and interoperability target, not a reason to move Symtropy's production core from Rust to Python.
- Dymos and pyCycle: reference tools for mission/trajectory optimization and thermodynamic propulsion cycle models.
- OpenVSP: parametric aircraft geometry import/export and identity-preserving design-space adapter. VSPAERO is useful for conceptual potential-flow analysis but is not a substitute for transonic RANS validation.
- NASTRAN: structural integration baseline where the exact solver, license and model data are available.

The differentiator to test is system-level evidence: geometry, mesh, CFD, structure, aeroelasticity, propulsion, mission and physical-test claims connected in one traceable evidence graph. Every candidate is evaluated at matched total cost, with failed runs retained, uncertainty carried into decisions, and a blind physical confirmation split.

See AERO-MDO-INTEROP-001 issue #3237 for the adapter contracts, benchmark ladder and acceptance gates.

## 11. External-solver execution, benchmark scorecard, and NACA version qualification (2026-11-01)

A Rust external-solver process runner is now implemented under `src/external_solver.rs`, with the `external_solver_replay_audit` offline CLI and `EXTERNAL_SOLVER_RUNNER_PROTOCOL_V0_1.md`. It launches a pinned executable via argv (no shell interpolation), clears/declares the environment, digest-checks staged inputs, requires a new run directory, bounds output logs, records timeout/nonzero/missing-output outcomes, and produces a byte-replayable receipt. Fake-executable negative controls cover argv, staged inputs, tamper detection, offline replay, timeout, output caps, bad digests, stale directories and path escapes. Its claim ceiling remains process integrity; it does not establish convergence or physical accuracy, and process-group termination/resource isolation remain outstanding.

The v0.1 benchmark scorecard is recorded in `AERO_BENCHMARK_SCORECARD_V0_1.md`. It predeclares an evaluation ladder from analytic known-answer cases through NACA 0012, DPW-8/AePW, ShapeBench, MDO/trajectory/structures/propulsion, and physical test evidence. It fixes matched-budget comparisons, task-family holdouts, optimizer-tuning cost accounting, risk/calibration metrics, independent replay and explicit claim ceilings. The 20% fewer high-fidelity evaluations at non-inferior validated feasible-front quality (30% stretch) is an aspirational hypothesis, not a measured result.

A fresh upstream NACA 0012 provenance review adds a version-compatibility gate: the upstream configuration file header says SU2 6.1.0, while the official tutorial page identifies the example as written for 7.0.0 and revised to 7.0.2. Compatibility with pinned SU2 8.6.0 is unqualified. Therefore historical source reproduction and current-release version-port qualification must remain separate. No automatic rewrite or real SU2 run is claimed; issue #2960 now records the exact artifact identity and the required lane separation.

A deterministic CI failure was traced to the comparison harness capturing Cargo's variable build chatter along with program stderr. The aerospace workflow now uses quiet Cargo invocations for captured command comparisons so it tests application determinism rather than whether a binary was rebuilt or reused from cache.

The exact-head CI for the quiet-logging/scorecard commits is pending; the last fully observed gate before those workflow-only/documentation changes passed Format, tests, Strict Clippy, deterministic report and matched-mission Pareto/exploitation controls, then failed the stress harness only on Cargo stderr noise. The stress harness's fixture-ID assertion was aligned with the canonical `synthetic_baseline` case ID before this quiet-output correction. No SU2 execution or CFD validation claim is made.


## 12. Receipt-bound SU2 history import (2026-11-01)

The external-solver process adapter now has a second-stage output importer: `src/su2_history.rs` and the `su2_history_import` CLI. The caller declares the exact ordered history header, iteration column, selected numeric columns and units because SU2 history output varies by solver and configuration. The importer first replays the process receipt, requires solver identity `SU2_CFD` and successful process status, then verifies the history file against the output length and digest recorded by that receipt. It rejects malformed or mismatched CSV, missing/duplicate columns, non-finite selected values, non-increasing iterations, symlinks/path traversal and insufficient rows. A canonical digest binds the imported observations to the run receipt, executable bytes and history bytes.

The evidence ceiling is `receipt_bound_history_import_only_not_solver_convergence_or_physical_validation`. It deliberately does not infer residual convergence, force stability, conservation, mesh independence or accuracy. The next layer must define solver- and case-specific convergence criteria and then route admitted histories to the existing grid-convergence and validation-uncertainty gates. See `SU2_HISTORY_IMPORT_V0_1.md`.

The section-polar CI guard was also repaired without changing numerical thresholds: its shell AWK parser now strips the surrounding CSV quotes from roughness-zone labels before comparing them. The Rust report correctly quotes those values; the old guard compared them as unquoted raw strings. Its numerical assertions remain unchanged.


## 13. SU2 history interpretation and declared convergence screening (2026-11-01)

The process-to-output pipeline now has three intentionally separate layers:

1. `external_solver_run`: execute a caller-pinned binary through the existing receipt-generating runner. Evidence: process exit, declared outputs, staged input/output bytes, environment and logs.
2. `su2_history_import`: verify/replay that receipt and import an exact caller-declared history header/column/unit schema. Evidence: byte-bound history observations, not convergence.
3. `su2_convergence_screen`: check explicitly declared residual final-magnitude/reduction limits and trailing-window monitor ranges. Its canonical screen digest binds the criteria outcomes to the receipt and history import.

The declared screen returns passed, failed and insufficient-row counts separately. It never labels its own result “CFD converged”; the evidence ceiling is `declared_residual_and_monitor_threshold_screen_only_not_complete_CFD_convergence_grid_independence_or_physical_validation`. The limits in the synthetic CI case are fixture thresholds only. No DPW-8 threshold has been proposed or admitted.

The current integration fixture uses a fake shell executable, so it verifies staging, process receipt replay, output byte binding, exact history parsing, threshold arithmetic, digest reconstruction and tamper rejection. It is not an SU2 execution or aerodynamic result. The next physical step still requires lawful case geometry/mesh/configuration, an actual source-built SU2 executable, the official periodic-sidewall lane, all prescribed output channels, conservation checks, and GCI/validation admission.

## 14. Optimizer metric correction and current workflow audit (2026-11-01)

The independent optimizer oracle found a real defect in the analytic ZDT4 exact-front hypervolume denominator. The Rust reference calculation had included the region between (x=1) and (x=1.1) inside the ZDT4 curve-integral branch and then added that region again for all tasks. It reported 0.986666666667 for ZDT4 where the correct reference-point hypervolume is 0.876666666667, equal to ZDT1 because both continuous Pareto fronts are (f_2=1-sqrt{f_1}) on [0,1]. The formula was corrected and a unit regression now asserts the equality. This is an analytic optimizer benchmark correction, not an aerodynamic finding.

Several other CI blockers on the current programme were test-interface defects exposed by independent oracles: CSV quoting in AWK guards, inaccurate names for running partial risk-area columns, a mismatched fine-grid identifier in the validation oracle, and the quoted policy-ID field in the family-aggregate AWK guard. The fixes preserve numerical criteria and expected comparison counts. Every exposed defect has been fixed at its reporting or guard boundary; each needs an exact-head rerun before the aerospace workflow can be called green.

The work reinforces a core design rule: every metric needs one canonical semantic definition, one typed source schema, and an independent implementation that reconstructs it. A green status from a self-referential demo is not sufficient.



## 12. Explicit steady-history and solver-to-evidence lineage (2026-11-01)

The SU2 history path has now been separated into three independently verifiable layers:

1. **Process integrity:** the external-solver runner pins the executable digest, stages the config/geometry/mesh inputs into a fresh run directory, captures bounded logs, enforces timeout, declares expected outputs and writes an offline-replayable process receipt.
2. **Typed history and steady screen:** the history importer binds selected caller-declared columns/units to the exact output bytes. The convergence screen distinguishes log10 from positive-linear residuals, checks per-equation final residual/reduction/terminal span, and checks each declared engineering monitor for terminal range, split-window mean drift and slope. The convergence-screen schema is v0.3.0 and binds solver identity into its canonical digest.
3. **Run-lineage receipt:** the new binding requires the process receipt, history import, screen, three distinct staged inputs (physics configuration, geometry, mesh), and at least one additional force/field output to refer to the same run. It exports one row per artifact and a canonical SHA-256. An independent Python oracle re-hashes the inputs and outputs, checks cross-document run/receipt/history/screen identities, and reconstructs the binding digest.

This is not a complete CFD evidence chain. The process runner does not yet measure CPU core-hours, peak RSS, GPU hours or energy; the executable digest is not a source/build attestation; and the binding is unsigned. The screen accepts only the declared set of equations/monitors. The bridge into the current numerical GCI/admission/evidence-chain APIs remains open, particularly because the legacy admission fields are positive-linear scalars while SU2 commonly reports log10 residuals. No implicit transformation is allowed.

Files added or extended:
- SU2_HISTORY_IMPORT_V0_1.md
- SU2_RUN_LINEAGE_BINDING_V0_1.md
- src/su2_history.rs
- src/su2_convergence_screen.rs
- src/su2_run_binding.rs
- scripts/verify-su2-history-import-independent.py
- scripts/verify-su2-convergence-screen-independent.py
- scripts/verify-su2-run-lineage-independent.py

The next gates are: exact-head format/tests/Clippy; successful CI run of the composed fake-solver lineage fixture and independent oracle; bridge a verified run bundle to a grid-study record without losing raw residual encoding; then add measured resource telemetry with explicit unknown states. A real, lawfully sourced and pinned SU2 run remains a separate next milestone; the synthetic CI fixture is not an SU2 simulation.


## 13. Exact-solver field preflight and boundary-marker identity (2026-11-02)

The SU2 path now has a separate dry-run receipt type and preflight gate. A process-only receipt is restricted in the library API to solver ID `SU2_CFD_DRY_RUN`, exactly `-d CONFIG_PATH`, and a staged config. It is not a generic “successful process with no outputs” escape hatch.

The dry-run preflight replays that receipt, checks the exact invocation/config/mesh digests, parses `SOLVER`, `CONV_FIELD` and `HISTORY_OUTPUT`, and verifies that the pinned executable advertised the configured convergence field and every required later history field for that exact solver/configuration. This responds to SU2's troubleshooting guidance: fields depend on the solver, and an unrecognized `CONV_FIELD` can be ignored, leaving convergence checking disabled. The output ceiling is field-discovery/configuration compatibility only.

A second audit now reconciles native SU2 mesh marker tags with config boundary assignments. Each marker must be assigned exactly once through an explicitly declared option schema; config references absent from the mesh, duplicate assignments, malformed parameter tuples and unclassified `MARKER_*` options are rejected. Non-boundary marker lists (monitoring/plotting/designing/analysis) are separately declared and checked against the mesh. Tuple widths and marker-name positions are caller-declared so option layouts are not guessed. A periodic tuple schema must be checked against the pinned SU2 solver/version before use.

See `SU2_FIELD_PREFLIGHT_V0_1.md` and `SU2_BOUNDARY_MARKER_AUDIT_V0_1.md`. The synthetic CI fixture tests structural mapping and negative controls; it does not validate boundary-condition physics.

## 14. Lineage-bound multi-grid GCI study composition (2026-11-02)

The previously separate SU2 run-lineage and grid-convergence paths now compose through `su2_grid_family.rs`. A study requires at least three individual run lineages, unique run IDs and mesh digests, coarse-to-fine level/spacing order, one solver executable digest/version claim, one physics-configuration digest/path and one fixed geometry digest. Each member's process/history/convergence/lineage evidence is replayed before its quantity is derived from the same passing terminal monitor window.

GCI is therefore no longer fed arbitrary caller-entered force coefficients in this path: the selected scalar is computed from the imported history monitor values. The study preserves every sliding coarse/medium/fine GCI triplet in a separate CSV; its member-level CSV binds each run lineage and the complete GCI-report digest.

Limitations remain explicit:
- the 2D lane now derives characteristic spacing as (h=\sqrt{A_{total}/N_{cells}}) from exact mesh bytes, restricted to linear triangles/quadrilaterals; total domain area and mesh marker-tag set must remain invariant across the family within a declared relative area tolerance;
- 3D volume-based spacing, systematic/topological nesting, anisotropy, skewness, orthogonality, wall-distance/y+ and boundary-layer-resolution checks remain future work;
- coordinate units and the protocol-manifest digest remain caller declarations, not independently source-authenticated;
- the convergence screen only evaluates explicitly declared equations and monitors;
- passing a terminal monitor screen is not equivalent to all solver equations, conservation laws, or time-dependent behavior being converged;
- GCI is not experimental validation or model-form validation.

The end-to-end Rust fixture builds three distinct nested triangular meshes on the same unit-square geometry, with one executable/config/geometry identity and deterministic fake solver histories. It checks a known second-order GCI sequence, lineage replay, and coarse-to-fine rejection. It is a synthetic software integration test, not a CFD result. See `SU2_GRID_FAMILY_STUDY_V0_1.md`.

