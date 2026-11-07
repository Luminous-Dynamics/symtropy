# Aerospace benchmark scorecard v0.1

**Status:** benchmark protocol and target thresholds; no SOTA result is claimed.  
**Scope:** Symtropy civil-aero screening, solver orchestration, verification, uncertainty, and multidisciplinary decision quality.  
**Companion work:** CFD-REF-001 (#2894), AERO-QUAL-001 (#2960), AERO-MDO-INTEROP-001 (#3237).

## Core paradigm: optimize decisions, not solver output

A design optimizer is not successful because it finds a low objective value. It is successful only when it produces a better *decision* under fixed resources, quantified uncertainty, explicit constraints, and independently replayable evidence.

Every benchmark row therefore carries five linked records:

1. **Design proposal:** geometry/mission/material/configuration, design-variable bounds, and constraint definitions.
2. **Analysis event:** solver identity, executable/build, exact inputs, argv/environment, outputs, costs, failure state, and run receipt.
3. **Numerical credibility:** iterative history, conservation/consistency, mesh/time refinement, observed order, numerical uncertainty and model-form limits.
4. **Validation claim:** observable, reference source, coordinate/normalization convention, experimental uncertainty, discrepancy, and validity domain.
5. **Decision consequence:** feasibility, objective trade-off, risk exposure, uncertainty, abstention, and value of the next experiment.

A result that cannot traverse this chain may still be useful for exploration, but it cannot silently enter the validated Pareto set.

## Benchmark ladder

| Tier | Benchmark family | Purpose | Minimum comparison |
|---|---|---|---|
| 0 | Analytic/known-answer tests | Verify math, parsers, receipt logic, optimizer metrics | Closed-form or independently computed oracle; adversarial negatives |
| 1 | NACA 0012 incompressible SA-RANS | Qualify one fixed 2D solver case and the analysis-to-evidence pipeline | SU2 tutorial reproduction; TMR Cp; CFL3D Cf as code-to-code only; compatible tripped forces |
| 2 | DPW-8 V1a/V1b and AePW | Multi-grid, multi-condition RANS validation and protocol adherence | Official boundary conditions and grid families; independent participant results; no mixed protocol lanes |
| 3 | ShapeBench | Generalization across aerodynamic shape classes and optimization formulations | 103 tasks / eight categories; same task split, compute budget, fidelity access and optimizer-tuning budget |
| 4 | MDO/trajectory/structures/propulsion | Qualify the engineering digital thread beyond single-discipline aerodynamics | OpenVSP/VSPAERO geometry baseline; SU2 flow; OpenMDAO/Dymos/pyCycle adapter baselines; structural and propulsion checks |
| 5 | Hardware/flight-representative evidence | Test model-to-hardware transfer and real decision utility | Instrumented non-flight coupons, wind-tunnel/rig measurements, and independently reviewed test protocols before any flight claim |

A lower tier is not evidence for a higher tier. In particular, synthetic optimizer wins do not validate aerodynamic physics, and a single airfoil case does not establish optimizer generality.

## Baseline policy

- **CFD:** compare the same case, geometry, mesh family, physical model, convergence policy and output definitions. SU2 is an independent solver baseline, not truth. Add FUN3D only where access/export conditions permit; record exact release/build and licensing constraints.
- **Geometry / conceptual design:** use OpenVSP and VSPAERO as a separate potential-flow/geometry baseline. Do not compare potential-flow output to RANS as though the models have equal fidelity.
- **MDO / trajectory:** use OpenMDAO and Dymos as external baselines for system optimization and trajectory optimization; pyCycle where cycle models are in-scope. Keep the Symtropy engineering kernel Rust-owned and use narrow file/CLI adapters for Python ecosystem comparisons.
- **Optimization:** include at least one deterministic gradient-based method where derivatives are available, one derivative-free method, one evolutionary/population method, and a simple uniform/random or Latin-hypercube baseline. Use identical feasible design bounds and total compute budgets.
- **Human baseline:** measure a documented engineering workflow and analyst time on a frozen task. Do not claim productivity improvements from solver runtime alone.

Baseline versions, commits, binaries, options, tuning budgets and data splits must be pinned before evaluation. A baseline can be excluded only with an explicit, reviewable reason.

## Metrics — do not compress into one headline number

### 1. Numerical and physical credibility

Report separately for each observable and task:

- Iterative convergence: residual slope/plateau, force/moment history stationarity, conservation residuals, and termination reason.
- Spatial/temporal verification: mesh/time family, refinement ratios, observed order, Richardson estimate, GCI/error band, and applicability diagnostics.
- Validation discrepancy: dimensional and normalized L1/L2/L-infinity profile errors, peak location/value error, integrated-load error, and experimental/reference uncertainty.
- Model discrepancy: turbulence/transition/geometry/boundary-condition sensitivity, source disagreement, and validity domain.
- Reference admissibility: license/source/hash, test conditions, calibration, measurement uncertainty, and comparability.

Do not pool Cp, Cf, CL, CD, moments, or different trip/transition states into a single accuracy score.

### 2. Optimizer quality at matched cost

Report:

- Hypervolume (fixed, predeclared normalization and reference point), IGD+ and additive epsilon where reference fronts are defensible.
- Feasible hypervolume and feasible-point count; false-feasible count remains a separate safety metric.
- Evaluation counts by fidelity and discipline; failed and rejected attempts count toward cost.
- CPU/GPU core-hours, wall time, peak memory and measured energy where available.
- Total budget including training, surrogate fitting, hyperparameter tuning, failed calls, retries, post-processing and verification.
- Replicate distributions and task/family-clustered confidence intervals, not only best seed or mean over pooled rows.

Target claim to test: **at least 20% fewer high-fidelity condition evaluations at statistically non-inferior validated feasible hypervolume**, on held-out task families, at equal total compute/energy and equal tuning budgets. Stretch target: 30%. This is a target, not an achieved result; it must be tested across the benchmark ladder, not one NACA case.

### 3. Decision and risk quality

- False-feasible rate, with numerator, denominator, and one-sided confidence bound.
- Selective risk-versus-coverage and an explicit abstention rate.
- Prediction interval coverage and width at each nominal level, split by geometry/flow/domain.
- Out-of-domain detection: recall, precision, calibration, and false in-domain acceptance.
- Decision regret under predeclared mission utility and constraints.
- Value of information / expected value of sample information for each expensive next analysis or test.
- Worst-case and distributional robustness are separate; never infer probabilities from illustrative stress scenarios.

For a zero-failure claim, report the sample design and confidence limit. As a rough independent-Bernoulli planning rule, 0 events in 300 independent trials only bounds the one-sided 95% failure rate to about 1%; it does not establish zero risk. Clustered or correlated cases require an appropriate effective sample size.

### 4. Evidence integrity and reproducibility

Required for every admitted result:

- 100% exact input/output provenance and immutable run receipts.
- Independent offline replay of byte bundles.
- Solver source/build identity, executable digest, compiler/toolchain, linked dependencies, hardware/driver, and resource accounting.
- Deterministic repeated output where determinism is expected; documented tolerance and stochastic seed policy otherwise.
- Failed, timed-out, unconverged and rejected cases retained in the ledger.
- Independent parser/oracle and tamper/negative controls.
- Separate process-integrity, numerical-admission, validation, and engineering-release states.

A SHA-256 receipt alone is not an attestation or a physical-validation result. A caller-supplied solver version string is not a verified solver identity.

## Predeclared comparison protocol

1. Freeze task definitions, objectives, constraints, geometry families, reference point/scales, task splits, and reference data.
2. Publish solver/build pins and all baseline tuning budgets before running.
3. Allocate an equal total budget per method; normalize by actual discipline-call costs, including failed runs and repeated evaluations.
4. Separate training/validation/held-out test sets by geometry family or source case to prevent leakage.
5. Run multiple predeclared seeds and report all of them. Use task/family-clustered uncertainty, not pseudo-replication from repeated points within one geometry.
6. Use a blind, independent evaluator to recompute metrics from immutable raw outputs and receipts.
7. Publish a complete failure ledger, not only the Pareto winners.
8. Make superiority claims only when the primary metric's confidence interval clears a predeclared non-inferiority/superiority margin and all safety/evidence gates pass.

## What “better than SOTA” would mean

A credible claim requires all of the following, simultaneously:

- Equal or stronger physical/numerical credibility than the strongest applicable baseline.
- Better validated feasible-front quality or equal quality at materially lower total cost, across multiple held-out geometry families.
- No increase in false-feasible risk; uncertainty calibration and OOD behavior are reported.
- Reproducible exact-head/source/build evidence with independent replay.
- A third party can reproduce the result without privileged access to the original author’s machine or memory.

Until those gates pass, describe the system as a **research prototype with a growing evidence and benchmark infrastructure**, not a superior aerospace design platform.

## Paradigm-shift hypotheses worth testing

1. **Evidence-native engineering:** make every optimization objective traceable to exact analysis, numerical uncertainty, validation domain and decision consequence.
2. **Fidelity allocation as a decision problem:** select the next mesh, solver, measurement or experiment by robust value of information, not fixed low/high-fidelity schedules.
3. **Claim-scoped digital thread:** each coefficient, geometry region and flow regime has its own validity domain; no global “validated” boolean.
4. **Adversarial design search:** actively search for optimizer exploitation, surrogate blind spots, false feasibility and protocol deviations.
5. **Cross-family generalization:** evaluate on many tasks and held-out geometry families; the optimizer itself is part of the hypothesis under test.
6. **Physics-to-hardware closure:** fit models to measurements, quantify sensor/fixture uncertainty, then test whether decisions remain correct on held-out physical experiments.

These are research hypotheses, not claims of novelty or achieved superiority.
