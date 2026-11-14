# ShapeBench clean-room adapter plan v0.1

Status: research plan. No ShapeBench tasks or baselines have been executed from this repository yet. This file does not copy ShapeBench source, datasets, pretrained weights or result files.

## 1. External benchmark reference

The reviewed reference is ShapeBench/ShapeBench at commit:

- fc86ccfedce00962204105400253b8ec265eebce
- [Pinned commit](https://github.com/ShapeBench/ShapeBench/commit/fc86ccfedce00962204105400253b8ec265eebce)
- [ShapeBench 2026 preprint](https://arxiv.org/abs/2605.20763)
- [Official README at the pinned commit](https://github.com/ShapeBench/ShapeBench/blob/fc86ccfedce00962204105400253b8ec265eebce/README.md)

The README reports 103 optimization tasks across 8 environments, 3 geometry classes and 7 methods. The environments cover 2D airfoils, 3D wings, delta wings, blended-wing-body aircraft, commercial aircraft, drones and automotive aerodynamics. The paper describes task-level variability in optimizer rankings; conclusions from one task should not be generalized to all classes.

The current repository runner is a plugin system. run_benchmark.py discovers framework, environment and reward packages; methods call an environment's simulate(); outputs include run_config.json, results.csv, results.json and, for some LLM/evolutionary workflows, design lineage. The ShapeBench documentation describes --iterations as total design evaluations. That is not necessarily the same as the number of internal aerodynamic condition evaluations or total compute cost.

## 2. Licensing and asset-provenance gate

**Do not vendor, redistribute, patch, or incorporate ShapeBench source/data/weights into this project until the applicable licensing terms have been established.**

At the reviewed commit, a root-level LICENSE file was not present (root LICENSE request returned 404). Repository search located distinct nested licenses, including MIT licenses for some archived or vendored components and an Apache-2.0 license under tinker/; NeuralFoil's vendored source also has its own license. A nested license is not a blanket grant for the entire ShapeBench repository, all task definitions, training data, model weights, images or result artifacts.

Before any reuse:
1. Obtain maintainer clarification on the repository-wide terms, if any.
2. Inventory every selected task's source, data, geometry, pretrained model and dependency licenses independently.
3. Record exact upstream commit, file/blob identities, source URLs, dataset versions, hashes, byte counts and license texts.
4. Keep external content outside the Symtropy repository unless the rights and redistribution conditions are explicit.
5. Prefer a clean-room adapter that consumes declared result formats and task metadata without copying external implementation code.

The first adapter should therefore be written independently in Symtropy and operate only on user-supplied or lawfully obtained run outputs plus metadata. This plan itself is not a legal opinion.

## 3. Smoke-task selection

Select only two NeuralFoil tasks first, after license and environment review:

### NF-SP-001 — single-point airfoil

Reference task: NeuralFoil ld_ratio, default alpha 5 degrees, Reynolds number 1,000,000, model size large, n_crit=9.

The reference reward is CL/CD. The environment reports an analysis_confidence value and warns when it is below 0.5; the ld_ratio reward implementation does not reject or penalize low-confidence predictions. This yields a useful model-exploitation test:

- The baseline optimizer must be allowed to run as published for faithful reproduction.
- Independently record confidence for every design.
- Report performance both with and without a post-hoc low-confidence exclusion.
- Include a deterministic negative control where a surrogate returns an attractive reward while confidence is below the warning threshold.
- Re-evaluate selected candidates using an independently pinned, higher-fidelity method before any improvement claim.

A low-confidence design may be a search proposal, but it must not be represented as verified feasible.

### NF-MP-001 — multipoint HPA airfoil

Reference task: multipoint_hpa, with lift targets [0.8, 1.0, 1.2, 1.4, 1.5, 1.6], weights [5, 6, 7, 8, 9, 10], Mach 0.03 and the published Reynolds schedule. The reference reward additionally imposes thickness, trailing-edge, moment, monotonic-angle, confidence and shape-wiggliness conditions.

A candidate design triggers six condition-specific aerodynamic evaluations in this task. Counting only one design iteration hides this cost. The adapter must record:
- one proposed design;
- six target-condition evaluations (or the exact actual count if some conditions fail or are unreachable);
- per-condition Reynolds number, angle of attack, CL, CD, CM and confidence when available;
- all penalties, constraint outcomes, unreachable targets and failed calls.

Do not combine this high-lift low-speed HPA task with the NASA TMR Re=6 million, M=0.15 tripped NACA 0012 validation rows as if they were the same regime.

## 4. Baseline matrix

Compare only on tasks where the interfaces, objective, constraints, parameterization and evaluation fidelity match.

| Method family | Baseline | Comparison role |
|---|---|---|
| Random / space-filling | Uniform or seeded Latin-hypercube design proposals | Minimum reference for sample efficiency |
| Bayesian optimization | Official BO/BoTorch or an independently implemented equivalent | Expensive black-box optimization |
| CMA-ES | Official CMA-ES implementation | Derivative-free continuous optimization |
| GA / PSO | Official GA/PSO implementation | Population-based optimization |
| L-BFGS-B / adjoint | Where supported by the task's differentiable or adjoint pipeline | Gradient-based engineering baseline |
| ShapeEvolve | Official LLM-guided evolutionary baseline | Domain-specialist LLM optimizer |
| OpenEvolve / ShinkaEvolve | Official supported adapters | General-purpose evolutionary coding agents |
| Symthaea selector | Proposed candidate/experiment selector | Hypothesis under test; no expected win assumed |

The published methods are not interchangeable. Some apply only to continuous variables or require gradients; some tasks use a surrogate while others expose high-fidelity CFD. Publish per-task comparisons and clearly mark not-applicable methods instead of comparing incompatible aggregates.

## 5. Fair budget accounting

Never collapse all resource costs into one scalar without exposing the underlying dimensions. Report at least three budget views:

### A. Matched evaluation budget
- number of candidate designs proposed;
- total objective calls;
- number of aerodynamic condition calls;
- calls by fidelity tier (surrogate, panel/XFoil, RANS/CFD, physical test);
- successful, failed, timed-out and unreachable evaluations.

### B. Matched compute budget
- wall-clock seconds;
- CPU seconds / core-hours;
- GPU seconds / accelerator-hours and device model;
- peak RAM and accelerator memory;
- solver iterations and convergence status;
- LLM request count, model, input/output tokens, retries and monetary cost where applicable.

### C. Matched engineering budget
- initial integration and environment setup effort;
- optimizer and hyperparameter tuning effort;
- manual repair, failed-run triage and postprocessing effort;
- human review and design-screening time.

A one-point NeuralFoil objective call and a six-point HPA evaluation are not equal work. Parallel execution reduces wall time but does not erase total compute consumption. Failed solver calls retain their incurred cost.

## 6. Metrics and sealed evaluation

Report:
- best feasible objective versus cumulative evaluation and compute budgets;
- area under the best-feasible-so-far curve;
- time and resource cost to a pre-registered target;
- success probability across independent seeds;
- constraint violation magnitude and feasible-run rate;
- Pareto hypervolume for genuinely multi-objective tasks;
- confidence/uncertainty calibration and out-of-domain detection;
- low-fidelity to high-fidelity rank reversals and false-improvement rate;
- per-task and per-condition error, not only an aggregate mean.

Use separate development, tuning and sealed final sets. Split by complete task/condition/geometry family/seed where appropriate. Do not split correlated points from a single multipoint run across training and test. Every final evaluation must be tied to a pinned geometry, task definition, code/config revision and raw output digest.

## 7. Model-exploitation controls

Before optimization begins, define the fidelity escalation rule. At minimum:
- Low-confidence or out-of-domain surrogate predictions cannot be called physically feasible.
- A candidate outside the surrogate's demonstrated Reynolds/Mach/AoA/geometry region must trigger higher-fidelity evaluation or abstention.
- Held-out high-fidelity replay must include the optimizer's apparent winners and selected controls, not only a favorable example.
- Report rank reversals, optimistic bias, false feasible rate, and the percentage/cost of candidates escalated.
- Keep the no-penalty low-confidence ld_ratio baseline unchanged for faithful comparison, then separately measure a confidence-gated variant as a new method.

The goal is not merely to maximize the surrogate's reward. It is to maximize verified improvement per unit of real engineering resource without silently crossing the model's validity boundary.

## 8. Implementation sequence

1. **Clean-room run manifest importer:** define a versioned JSON manifest for exact task/config/environment/model/revision metadata and a Rust parser for ShapeBench-style run_config.json plus results.csv records. Preserve raw output SHA-256, original column names and row identity; do not infer missing cost as zero.
2. **Cost ledger adapter:** map each candidate to its actual condition-call records, failure states and resource measurements. Use the existing matched-budget ledger only after validating one-to-many relationships.
3. **Reproduction harness:** pin two smoke tasks and reproduce a small baseline set under matched evaluation and compute budgets.
4. **Fidelity audit:** independently re-evaluate the selected designs using an appropriate high-fidelity solver and report discrepancies.
5. **Symthaea policy:** add an experiment-selection policy only after the baseline harness is stable; compare against random, BO, CMA-ES and task-applicable specialist baselines.
6. **Broader suite:** expand across task categories only once the first subset is reproducible and licensing is resolved.

## 9. Claim threshold

Do not claim "better than state of the art" unless the method:
- beats matched baselines over a pre-registered task set and seeds;
- improves the feasible objective or Pareto hypervolume at equal or lower total resource cost;
- does not worsen constraint violations, confidence calibration or held-out high-fidelity error;
- generalizes to untouched tasks or conditions;
- has independently reproducible artifacts and a clear license/provenance record.

A strong result may instead be better sample efficiency, lower compute, better calibrated abstention, less model exploitation, or greater cross-task generality. Report the specific axis of improvement instead of a blanket SOTA statement.
