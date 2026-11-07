# ShapeBench interoperability plan v0.1

**Status:** scoped research plan; no ShapeBench task has been executed by Symtropy and no cross-benchmark performance claim is made.  
**Parent:** AERO-MDO-INTEROP-001 (#3237).  
**Companion:** `AERO_BENCHMARK_SCORECARD_V0_1.md`, `EXTERNAL_SOLVER_RUNNER_PROTOCOL_V0_1.md`.

## Why this benchmark is strategically important

ShapeBench is a recent aerodynamic shape-optimization benchmark with 103 tasks, eight environments, three broad geometry classes and seven optimization approaches. It combines fast surrogate search with high-fidelity CFD where available. Its published analysis reports mean pairwise Spearman rank correlation of approximately 0.013 among optimizer rankings across tasks. The operational lesson is that optimizer rankings are highly task-dependent: a single-airfoil win is not evidence of general superiority.

Primary references:
- Paper: https://arxiv.org/abs/2605.20763
- Benchmark code and task definitions: https://github.com/KrisshChawla6/ShapeBench
- COCOANet CCA aircraft dataset: https://huggingface.co/datasets/ShapeBench/COCOANet

## Scope: sentinel benchmark first, then full evaluation

Do not start by downloading or running all 103 tasks. Build a small, balanced sentinel panel that tests interoperability, fidelity separation, family generalization and budget accounting. Only scale to the full benchmark after the panel's adapters and source terms have passed independent review.

### Pilot panel

| Pilot | ShapeBench environment | Purpose | Fidelity boundary |
|---|---|---|---|
| A | NeuralFoil 2D airfoil | Start with one or more single-point and multipoint airfoil tasks; geometry/constraint sanity and optimization control | NeuralFoil surrogate is not RANS. The repository identifies XFoil and FEniCS 2D Navier–Stokes as validation / higher-fidelity paths; keep those outputs distinct |
| B | SuperWing 3D transonic wing | Main SU2 adapter integration target; test Mach 0.7–0.9 and multipoint objective behavior on selected tasks | ATsurf_M surrogate proposals are not CFD evidence; only pinned SU2 runs with independently checked outputs and refinement can advance fidelity |
| C | VortexNet delta wing | Third geometry family; test transfer beyond conventional swept transport/airfoil parameterizations | VLM / neural VLM and panel/CFD results are separate model classes; retain the exact high-fidelity pipeline and terms per task |
| D (later) | CERAS aircraft sizing | MDO and system-level integration | FAST-OAD is an external multidisciplinary model; not comparable to an aerodynamic-only reward without matched mission/system scope |

The pilot selection must be frozen by immutable task IDs, operating conditions, parameter bounds, source revisions and reference-output digests before running. The pilot is a method-validation panel, not a replacement leaderboard.

## Optimizer baselines

Run the benchmark's documented methods where they apply: Bayesian optimization (BoTorch), CMA-ES, genetic algorithm / particle swarm, L-BFGS-B, ShapeEvolve, OpenEvolve adapter and ShinkaEvolve adapter. Add simple random/uniform and Latin-hypercube controls. For smooth differentiable subproblems, include a gradient/adjoint baseline when the same derivative information is available to all eligible methods.

Do not compare the methods by raw iteration count. Freeze equal total cost per task, including:
- solver calls at each fidelity and condition;
- failed, rejected, timed-out and repeated runs;
- surrogate training and hyperparameter tuning;
- optimizer tuning, prompt/model tokens and agent wall time where applicable;
- geometry/mesh generation, orchestration, post-processing and independent verification;
- CPU/GPU time, memory and measured energy where available.

The primary metric is validated feasible Pareto quality per total cost. Secondary metrics include regret, feasible yield, false-feasible rate, uncertainty calibration, selective risk/coverage, OOD abstention, fidelity gap and worst-decile performance. Report per task and per geometry family, with family-clustered uncertainty. Do not pool repeated design rows as independent tasks.

## Rust-first interoperability contract

The ShapeBench repository's public entry point is Python-based and its standard output includes `results.csv` (iteration, design, reward, best reward) and `results.json` (full design database and metrics). Do not port the benchmark's Python framework or embed it into the production Symtropy kernel.

Instead:
1. Run ShapeBench as an external, version-pinned benchmark process in its own documented environment.
2. Retain the exact repository commit, benchmark configuration, task definitions, baseline code identity, environment lock, stdout/stderr, output files and run budget.
3. Hash every raw output artifact; preserve `results.json` without rewriting it.
4. Import `results.csv` through a strict Rust adapter into a versioned Symtropy interchange schema. Preserve the original iteration, exact design payload, raw reward, best-so-far reward, task ID, method ID, fidelity label, condition set and cost ledger. Hash the original design string; do not infer geometry equivalence from rounded parameter text.
5. Keep raw ShapeBench result, imported record, independently recomputed metric, and admitted validation claim as distinct nodes in the evidence graph.
6. A surrogate-only row can be used for search accounting, but may not enter the validated Pareto set until its specified high-fidelity confirmation and admission gates pass.

The first importer should support the public CSV interchange and raw-file receipt. Avoid adding a heavyweight JSON dependency to the Rust core until a concrete structured field is required; retain and hash the JSON artifact first, and introduce a typed JSON parser only with explicit schema/version fixtures.

## Licensing and data admission

Licensing is per codebase, dataset, surrogate, solver, and individual high-fidelity environment—not a single benchmark-wide switch.

- The ShapeBench README and paper need to be reviewed together with each selected environment's current license file, model-weight terms and solver/tool terms. A root-level license was not confirmed in this intake review; this is unresolved, not evidence that the project is unlicensed.
- The hosted COCOANet dataset card currently labels the dataset CC-BY-4.0, but the paper's own license appendix and historical dataset revisions must be reconciled against the exact files/revision intended for use. Record the exact dataset commit, card, notice and attribution requirements in the manifest before retaining any bytes.
- Hosted dataset size varies by revision (the inspected Hugging Face revisions reported materially different totals). Do not mirror the entire archive. The first pilot should fetch only the selected case artifacts needed for a frozen task, subject to the license, rate, storage and provenance checks.
- Flow360 is a commercial solver: no assumption of access or license is made. FUN3D access has separate export restrictions. Use an available, licensed alternative when access is not legitimate; record the substitution as a different baseline.
- Some ShapeBench environments use third-party code, pretrained weights, or datasets with non-commercial/share-alike terms. No component may enter a public Symtropy repository until its redistribution rights are confirmed.

## Admission gates for each imported task

- Exact task/configuration identity and source revision are pinned.
- Geometry parameters, units, coordinate frames, operating conditions, objectives and constraints are parsed without lossy conversion.
- Data terms and solver/model licenses are reviewed and recorded.
- Every raw result and input artifact has byte length and SHA-256; originals remain immutable.
- Costs and failed attempts reconcile to the declared total budget.
- The surrogate and high-fidelity model class are explicit; no label promotion by convenience.
- An independent Rust-side importer/oracle reproduces row counts, objective values and best-so-far consistency.
- Negative controls cover altered task IDs, changed input hashes, malformed CSV, non-finite rewards, duplicated iterations, decreasing budgets and missing high-fidelity confirmation.
- Train/tune/held-out tasks are separated by geometry family or source case; held-out tasks remain sealed until final scoring.

## Success criterion

Symtropy should not merely reproduce ShapeBench's leaderboard. The research hypothesis is that an evidence-native, decision-aware policy can improve validated feasible design quality per total engineering cost while maintaining calibrated uncertainty and no increase in false-feasible risk across held-out geometry families.

This is only supported if:
- at least three genuinely distinct geometry families are evaluated;
- at least three independent seeds are run per method/task where stochasticity applies;
- all baselines receive matched budgets and tuning opportunity;
- high-fidelity confirmation is independently replayed;
- the predeclared primary metric clears its uncertainty-adjusted superiority or non-inferiority threshold;
- all failed/abstained results remain visible.

A strong result on NeuralFoil alone, a surrogate reward improvement, or a comparison using unmatched solver budgets is not evidence of SOTA.
