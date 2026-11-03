# Evidence-native aerospace MDO benchmark v0.1

## Objective

Evaluate whether Symtropy improves *validated engineering decisions* under matched task, fidelity, risk and resource budgets. Do not evaluate only nominal optimizer scores or the number of CFD calls.

This is a benchmark protocol, not a claim that Symtropy currently beats any baseline.

## Benchmark families

### A. Aerodynamic shape optimization: ShapeBench

Use the public ShapeBench task suite as a broad generalization set. The 2026 paper describes 103 tasks across eight categories and reports low mean pairwise rank correlation between optimizers (about 0.013), illustrating that a method's ranking on one task may not transfer to another.

- Freeze the benchmark commit, task manifest, data digests, objective definitions and validation split.
- Preserve geometry-class holdouts; do not randomly split nearby design points and call that out-of-domain generalization.
- Compare the benchmark's supplied baselines, plus a Symtropy-guided policy.
- Where supported, verify finalists with the designated high-fidelity solver rather than reporting surrogate-only quality as a CFD result.

Reference: https://arxiv.org/abs/2605.20763
Repository: https://github.com/KrisshChawla6/ShapeBench

### B. Aircraft multidisciplinary design: Digital Hangar / DLR-F25

Use the Simple Transonic Wing as a lower-cost integration case and the DLR-F25 as a grand-challenge case when the required models/data are admitted.

The benchmark's intended progression includes sensitivity/trade validation, coupled MDO and design under uncertainty. Its disciplines include aerodynamics, structures, propulsion and UQ.

- Start with discipline-level sensitivity verification and trade studies.
- Then compare coupled feasible designs under matched resource limits.
- Add manufacturing, operating-condition and model uncertainty only after baseline sensitivities/coupling are qualified.
- Do not claim access to restricted models or data merely because the benchmark describes them as available on request.

Reference: https://www.digital-hangar.de/collaboration/mdo-benchmark/

### C. CFD / aerodynamic validation: AIAA DPW-8 and AePW-4

Treat SU2 as a strong solver baseline and interoperability target, not as a solver to replace prematurely. SU2 v8.6.0 was released on 30 October 2026; its release notes include a fix for the misleading "All convergence criteria satisfied." message on interrupted runs (#2881) and OpenMP support in force calculation (#2870). Pin executable/source/build hashes, compiler, linked libraries, OpenMP settings, thread count and CPU affinity. Keep the historical SU2 v8.0.1 participant reproduction lane separate from the current v8.6.0 lane.

References:
- https://github.com/su2code/SU2/releases/tag/v8.6.0
- https://github.com/su2code/SU2/pull/2881
- https://github.com/su2code/SU2/pull/2870

Use public, protocol-compliant cases such as ONERA OAT15A and NASA Common Research Model when the exact geometry, grid, boundary conditions, solver version, reference data and uncertainty are available.

The 2026 workshop combines drag prediction with aeroelastic prediction and includes working groups on static deformation, buffet, high-angle, high-speed, large-deflection, scatter and test environments.

- Maintain separate strict-protocol and historical participant-reproduction lanes.
- Record every grid, angle, failed run, restart, and exclusion.
- Compare forces and moments, pressure distributions, unsteady spectra/buffet responses and deformation only where the test case defines those outputs.
- Require grid and (for unsteady cases) temporal/sample-window studies before interpreting a nominal result as discretization independent.

References:
- https://www.aiaa-dpw.org/
- https://aiaa.org/courses/aiaa-drag-prediction-and-aeroelastic-prediction-workshop/
- https://dpw.larc.nasa.gov/DPW8/

## Baseline policy matrix

Run all policies from the same immutable task manifest. Record exact method/version/configuration and tuning budget.

| Policy | Purpose | Required comparator |
|---|---|---|
| SU2 native adjoint / gradient-based optimization | Strong solver-native local optimization | Same solver, mesh, geometry parameterization and derivative settings |
| OpenMDAO with analytic/adjoint derivatives | Strong coupled MDO and gradient baseline | DLR-F25 or another shared coupled benchmark |
| DOE + response surface | Conventional engineering exploration | Same sample budget and fidelity allocation |
| Evolutionary multiobjective policy (e.g. NSGA-II / differential evolution) | Derivative-free Pareto baseline | Same total cost, not just population size |
| Bayesian / surrogate-assisted optimization | Sample-efficient black-box baseline | Same high-fidelity-equivalent budget and uncertainty treatment |
| Symtropy-guided policy | Active experiment selection, robust optimization, abstention | No extra hidden evaluations or privileged validation labels |

OpenMDAO's own strength is tightly coupled systems analysis and gradient-based MDO with parallel methods. The correct comparison is interoperability and matched-cost decision quality, not an unsupported claim that a Rust implementation is automatically a better optimizer.

Reference: https://openmdao.org/newdocs/versions/latest/

## Matched-budget rules

The primary budget is total measured resource consumption across every discipline and fidelity, including failed, timed-out and retried attempts.

Report these separately:
- High-fidelity-equivalent evaluation count.
- Solver wall time and CPU core-hours.
- GPU time when used.
- Peak resident memory.
- Energy (J/kWh) and measurement method.
- Human setup/review time where it materially differs between policies.
- Failed-run and retry cost.

A metric is unavailable when its source measurement is unavailable; do not substitute zero or infer energy from wall time. The current Symtropy run bundle measures wall time only and marks the other resource quantities unknown, so it cannot yet support a full compute/energy efficiency claim.

## Terminology clarification: admission is not independent validation

The existing `policy_benchmark` v0.1 scorecard consumes `CandidateObservation.feasible` and an admission receipt supplied by the orchestration caller. Its evidence level is explicitly `caller_bound_admission_receipt_scorecard_not_solver_validation`. Its `validated_hypervolume` field therefore means *admission-receipt-qualified feasible hypervolume under the benchmark's declared policy*, not independently falsified feasibility, ASME V&V 20 validation, experimental validation, or airworthiness evidence.

The separate [policy falsification audit v0.1](./AERO_FALSE_FEASIBILITY_AUDIT_V0_1.md) consumes an exact frame of all admitted candidates and a separate independent-adjudication ledger. It reports:
- a lower bound: known falsified-infeasible candidates / all admitted candidates;
- a worst-case upper bound: (known falsified + inconclusive + unassessed) / all admitted candidates;
- a resolved-only false-feasible fraction, explicitly conditional and potentially selection-biased.

A complete census must adjudicate every admitted candidate. A probability sample or targeted challenge set must bind its sampling frame, selection plan and selection receipt, and unresolved candidates remain in the upper bound. This module does not authenticate verifier independence or prove statistical representativeness.

Until independent adjudication is linked, describe the existing scorecard as an admission-qualified optimization scorecard. Reserve “experimentally validated feasible hypervolume” for a separate scope-matched validation workflow with independent measurement evidence and uncertainty.


## Primary outcome metrics

1. **Validated feasible hypervolume**: compute only from candidates that pass declared feasibility and the relevant numerical/validation gates.
2. **Feasible-design yield**: admitted feasible candidates divided by all attempted candidates, including failed solves.
3. **False-feasible rate**: candidates declared feasible by the policy but rejected by independent constraints, higher fidelity, or physical evidence.
4. **Regret**: objective gap to the best independently validated feasible solution available under the benchmark.
5. **Robust regret / worst-decile performance**: evaluate operating-condition and geometry shifts; report CVaR or worst-decile outcomes, not only averages.
6. **Uncertainty calibration**: empirical coverage and interval width for locked nominal 90% and 95% intervals, stratified by family and out-of-domain condition.
7. **Cost to decision**: total measured resource cost to reach a validated decision of predeclared quality.
8. **Experiment value**: realized improvement in decision utility per measured experimental/simulation cost, compared with random and fixed-sequence experiment selection.
9. **Replay quality**: exact input/output byte binding, deterministic offline replay, independent verifier agreement, and preserved negative controls.

Report per-task distributions, median, mean, worst decile, confidence intervals, all exclusions and all failed trials. Use paired task/seed comparisons and publish the full task-level data.

## V&V and physical evidence gates

NASA's CFD verification assessment identifies programming/code checks, iterative convergence, consistency/conservation, spatial convergence, temporal convergence and comparisons with accurate solutions as separate checks. Do not collapse them into a single pass.

For validation:
- Use ASME V&V 20 pointwise comparisons at specified validation variables and operating points, with simulation and experimental uncertainty.
- Where multiple correlated outputs/locations/times matter, consider ASME VVUQ 20.1-2024's multivariate validation metric rather than selecting only the best-looking scalar. The current claim policy requires a multivariate evidence item for unsteady aerodynamic, aeroelastic-stability and safety-critical claims; the software binds the verifier receipt but does not implement or certify the ASME metric itself.
- Preserve experiment calibration, repeatability, reproducibility, facility corrections, sensor uncertainty and data-processing lineage.
- Keep model-form discrepancy and validation-domain limits visible.

References:
- https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html
- https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty
- https://www.asme.org/codes-standards/find-codes-standards/vvuq201-vvuq-20-1-multivariate-metric-for-validation
- https://www.nist.gov/publications/summary-industrial-verification-validation-and-uncertainty-quantification-procedures

## Pre-registered target ladder

These are proposed research targets, not achieved results. Freeze them before the corresponding blind evaluation.

| Level | Acceptance target |
|---|---|
| Evidence integrity | 100% of accepted run bundles replay offline; zero accepted tampered artifacts |
| Protocol conformance | Zero unexplained deviations in strict protocol lanes |
| Active convergence fields | Every configured steady and time-window field is discovered, imported and independently screened |
| Feasibility integrity | Zero false-feasible admissions in the locked adversarial and blind constraint suite |
| Generalization | At least three held-out geometry families and three distinct operating-condition families |
| Calibration | Nominal 90%/95% interval coverage evaluated on blind families; report width and subgroup coverage |
| Multivariate validation | For correlated variables/locations/times, report the VVUQ 20.1-2024 global metric, covariance/uncertainty assumptions and local pointwise failures alongside V&V 20 results |
| Optimization efficiency | At least 20% fewer high-fidelity-equivalent evaluations at non-inferior validated feasible hypervolume; 30% stretch |
| Robustness | No degradation in worst-decile feasibility or robust regret versus strongest baseline |
| Experimental evidence | Independent calibrated physical measurements at declared validation points, with expanded uncertainty and repeatability |

A 20% evaluation reduction is not meaningful if it is achieved by lowering fidelity, dropping failures, changing the design constraints, or accepting poorer worst-case feasibility.

## What would constitute a credible SOTA claim?

A defensible claim requires:

1. A preregistered protocol, immutable inputs and benchmark commit.
2. Strong, correctly configured baselines and equal total budgets.
3. Multiple tasks/families, independent seeds and blind holdouts.
4. Paired statistics and full failure/cost accounting.
5. Independent numerical verification and experiment-backed validation where claimed.
6. External reproduction by a party not responsible for the optimizer.

Until those conditions hold, use language such as "promising on the evaluated synthetic suite" or "software contract verified"; do not use "better than SOTA."

## Symtropy's differentiated hypothesis

The research hypothesis is that the next expensive computation or experiment should be selected by expected *decision improvement*, conditioned on uncertainty, risk, cost and the current evidence envelope.

The system should be able to:
- Request a finer grid when discretization error dominates.
- Request a new operating point or experiment when model-form uncertainty dominates.
- Select a more appropriate fidelity when high-fidelity evaluation has low expected information value.
- Abstain when the design leaves its validated domain.
- Preserve the reason for every action, exclusion, and final recommendation.

The core outcome is not "more optimizer wins." It is more independently defensible engineering decisions per measured unit of cost, with fewer false-feasible outcomes and no hidden degradation in risk.

## Risk-based model credibility (2026 update)

The October 5, 2026 NIST report, *On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices*, recommends harmonizing ASME computational-model UQ with JCGM metrology practices and scaling credibility work according to model influence and decision consequence.

Symtropy should translate that into claim-specific admission policies:
- Exploratory shape ranking: lighter evidence is permissible, but the report must say it is exploratory and out-of-envelope designs must be flagged.
- Aerodynamic loads used by structural sizing: require stronger numerical uncertainty, model discrepancy and validation support at relevant conditions.
- Flutter, stability, propulsion limits or safety-critical claims: require additional model-specific validation, independent review, physical evidence where appropriate and a strictly narrower domain of use.
- Any missing required evidence produces an explicit unknown/reject/abstain outcome, never a favourable default.

Reference: https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing

## New benchmark gate: stopping-policy completeness

For SU2 cases, the benchmark manifest must record the entire configured stopping protocol, not merely one residual:
- Every CONV_FIELD entry, its exact dry-run recognition and its production-history import.
- Explicit WINDOW_CAUCHY_CRIT enablement.
- Every CONV_WINDOW_FIELD entry and its exact dry-run recognition.
- Window start, convergence start, window-element count, epsilon and window function.
- A post-run independent screen for every active convergence field.
- Time-window fields require terminal mean/range/drift/slope screening; residual-only acceptance is not a substitute for time-average stability.

For periodic/unsteady cases, iterative convergence, windowed time-average convergence, sample-length sufficiency, frequency/spectral convergence and time-step refinement remain distinct gates. A configuration preflight only proves compatibility/recognition; it does not prove the flow is statistically stationary or adequately sampled.

## Cost-to-decision accounting

The benchmark's primary resource unit should be **measured total decision cost**, not a raw count of CFD runs. Maintain a vector rather than hiding cost in one scalar:
- high-fidelity-equivalent solver calls;
- CPU core-hours and GPU-hours;
- peak memory;
- measured energy;
- wall-clock latency;
- human setup/review time;
- physical test and metrology cost.

An aggregate monetary or carbon score may be computed only after unit conversion assumptions, rates, hardware and measurement uncertainties have been declared. Unknown quantities remain unknown. Failed simulations, retries, mesh repair, manual intervention and discarded candidates remain in the ledger.

## Statistical acceptance detail

- Publish task-level paired deltas and confidence intervals, not only a mean over tasks.
- Stratify results by geometry family, objective type, fidelity and flow regime.
- Predeclare the hypervolume reference point, feasibility tolerances, non-inferiority margin and treatment of failed/abstained tasks.
- Evaluate interval coverage with binomial confidence intervals, calibration by family and interval width/sharpness; do not use aggregate calibration error alone.
- A zero false-feasible count is not proof of zero risk. Report a one-sided confidence bound and the size/independence limitations of the qualification corpus.
- Test the decision policy against blind operating-condition shifts and model-form perturbations, not merely random seeds from the training distribution.

