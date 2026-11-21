# ShapeBench family-aware comparison and cost protocol v0.1

Status: clean-room integration specification. No upstream task, source, model, geometry, weights or result artifacts are copied into Symtropy by this protocol.

## Pinned external reference and scope

- Repository: https://github.com/ShapeBench/ShapeBench
- Reviewed commit: fc86ccfedce00962204105400253b8ec265eebce
- Paper: https://arxiv.org/abs/2605.20763
- README at pinned commit: https://github.com/ShapeBench/ShapeBench/blob/fc86ccfedce00962204105400253b8ec265eebce/README.md

The pinned README describes 103 tasks across eight environments and multiple optimizer families. Its task portfolio includes 2D airfoils, transonic 3D wings, delta wings, supersonic transport, blended-wing-body aircraft, commercial aircraft, drones and automotive aerodynamics. Available high-fidelity paths differ by environment; a surrogate-only task must not be presented as high-fidelity validated merely because another environment has a CFD pipeline.

The ShapeBench results and preprint report that optimizer rankings vary substantially by environment. This makes a family-aware, paired comparison essential; a single airfoil task is a smoke test, not a general optimizer benchmark.

## Licensing and clean-room rule

At the reviewed commit, the repository tree contains no root-level LICENSE file. Some nested components have their own licenses, but those do not grant blanket permission for the entire benchmark, datasets, geometries, pretrained weights, result artifacts or dependencies.

Until the benchmark maintainers clarify repository-wide terms and every selected asset is individually reviewed:
- do not vendor ShapeBench source, task files, data, geometries, pretrained weights, images or historical results;
- do not modify or redistribute upstream benchmark assets;
- use only an independently implemented adapter and user-supplied/lawfully obtained run metadata and outputs;
- record source revision, task identifier, selected asset origin, license/terms status, file/blob identity, byte count and digest for every imported object;
- retain a per-asset terms state; an unknown license is not "open" by default.

This document is an engineering protocol, not legal advice.

## Frozen family mapping

The adapter must preserve the upstream environment identity on every task result. Before any optimizer is tuned, maintain an immutable manifest containing, at minimum:

- upstream commit and task identifier;
- environment identifier and geometry class;
- parent family identifier used for statistical clustering;
- objective names/directions, constraints, ideal/reference points and multipoint conditions;
- surrogate/high-fidelity methods available for that task;
- applicability of every optimizer baseline;
- budget caps and cost-unit basis;
- train/tune/blind-confirmation split;
- license and provenance status for task metadata and every consumed artifact.

Map every task to a family before evaluating policies. Family assignment must not be inferred from task names after seeing results. If two environments share geometry, training data or strongly correlated conditions, group them under a common parent family for the outer bootstrap. The goal is to sample independent design/environment regimes, not maximize the number of apparent samples.

The new Symtropy policy aggregator follows this hierarchy:
1. median across independent seeds within each task and policy;
2. median across tasks within each predeclared family and policy;
3. equal-weight macro aggregation across families;
4. paired policy-minus-baseline deltas and bootstrap resampling of complete family clusters.

It rejects an incomplete task-family map or incomplete task/replicate/policy matrix. A benchmark with 103 tasks across eight environments should report eight family clusters, not claim 103 independent generalization samples.

## Applicable optimizer baselines

Include methods only where the task parameterization and objective make them applicable. Publish "not applicable" rather than forcing an invalid comparison.

| Baseline | Appropriate role | Fairness control |
|---|---|---|
| Uniform/random search | Minimum black-box reference | Fixed seed and identical total budget |
| Latin-hypercube / Sobol space-filling | Coverage and initialization baseline | Same sample count and candidate bounds |
| Bayesian optimization (BO/BoTorch) | Expensive black-box optimization | Freeze surrogate, kernel, acquisition and tuning budget |
| CMA-ES | Derivative-free continuous design search | Freeze population and restart policy |
| GA / PSO | Population-based search | Freeze population, mutation, crossover and stopping |
| L-BFGS-B | Gradient/quasi-Newton comparison when derivatives or declared finite differences are available | Charge gradient/finite-difference calls; do not label it gradient-free |
| ShapeEvolve | Domain-specialist LLM evolutionary baseline | Record prompts, model/version, tokens, retries and orchestration |
| OpenEvolve / ShinkaEvolve | General-purpose evolutionary coding-agent baselines | Pin adapters and count every model/tool run |
| SU2 adjoint / gradient-based | Physics-based design baseline when the task's solver/mesh supports it | Same geometry, physics, constraints, mesh and high-fidelity acceptance |

The official ShapeBench baseline table and task applicability remain the reference. Do not combine results from methods that use different design variables, reward functions, hidden constraints or solver fidelity.

## Evaluation-cost accounting

A single design iteration is not a universal unit of work. The HPA multipoint airfoil task described by the upstream README evaluates six lift-target conditions per design. The run ledger must therefore expose:

1. candidate design proposals;
2. objective/reward calls;
3. aerodynamic condition calls, including target-specific calls that fail or are unreachable;
4. surrogate / panel / XFoil / RANS-CFD / physical-test calls by fidelity;
5. mesh generation, restart and solver setup work;
6. solver iterations, convergence, timeout and failure counts;
7. wall-clock seconds, CPU core-hours, GPU/accelerator-hours, peak memory and measured energy where available;
8. LLM/model calls, model/version, tokens, retry count, external tool calls and monetary cost;
9. human setup, repair, triage, review and audit effort.

Report matched candidate/evaluation budgets and matched compute/resource budgets separately. Parallel jobs reduce elapsed time but do not remove their compute cost. Failed runs are charged for the resources already consumed. An evaluation that calls six aerodynamic conditions must not be counted as one condition call.

## Surrogate exploitation and fidelity gap

The pinned README describes a NeuralFoil ld_ratio single-point reward that warns about low analysis confidence but does not automatically reject or penalize that prediction. Preserve the official baseline unchanged for faithful reproduction, then separately test confidence-gated variants.

Required controls:
- retain confidence for every proposal;
- mark low-confidence and out-of-domain candidates as unverified, never validated-feasible;
- report the fraction and cost of candidates escalated to higher fidelity;
- re-evaluate apparent winners and matched controls through an independent higher-fidelity pipeline;
- report rank reversals and false-feasible rate;
- keep tasks at different Mach, Reynolds, AoA, geometry and transition regimes separate;
- use held-out geometries and flow conditions to measure model exploitation.

The objective is verified improvement per real engineering resource—not maximum surrogate reward.

## Metrics

Per task and family:
- validated feasible objective / Pareto hypervolume versus cumulative evaluation budget;
- objective quality versus CPU/GPU/energy/wall-clock cost;
- number of candidate proposals and number of true aerodynamic-condition calls;
- time/resources to a pre-registered target;
- feasible yield, constraint violation magnitude and failed/non-converged call rate;
- low- to high-fidelity rank reversals and false-improvement rate;
- uncertainty/confidence calibration and out-of-domain abstention;
- worst-family performance, median-family performance, family win/tie/loss and paired family delta.

For multi-objective tasks use a task-specific, frozen ideal/reference box and report validated hypervolume only after the candidate has passed the declared independent admission gate. Surrogate-only objectives must be labelled surrogate-predicted.

## Pre-registered benchmark target

Initial adapter acceptance:
- at least two license-cleared smoke tasks;
- at least three applicable baselines plus random/space-filling;
- at least three seeds;
- complete solver-condition and resource accounting;
- independent oracle and deterministic replay;
- no low-confidence surrogate result promoted to physical feasibility.

Research comparison:
- all eight environments where task and asset rights permit;
- 20+ task configurations and at least five independent seeds each;
- family-clustered statistics and blind geometry/condition holdouts;
- strongest applicable published ShapeBench baselines reproduced under matched budgets;
- at least 20% fewer high-fidelity condition calls at equal or better validated Pareto quality, with no material worst-family regression.

The 20% target is aspirational and not an achieved result. A positive average does not establish superiority when the family-level confidence interval crosses zero or the worst-family loss is unacceptable.

## Relation to DPW8 and SU2

ShapeBench is a broad optimizer benchmark, not a substitute for controlled CFD verification/validation. Keep it separate from DPW8 OAT15A steady RANS, buffet, static-deformation, high-angle and FSI evidence.

Keep the DPW8 participant's historical SU2 8.0.1 reproduction lane separate from the current SU2 8.6.0 release lane. The latter is the latest public release pin observed on 2026-10-31, but a release tag is not a build digest or a successful V1a run. Compare only with identical admitted geometry, mesh family, flow physics, boundary conditions, convergence and cost accounting.

## References

- ShapeBench pinned repository: https://github.com/ShapeBench/ShapeBench/tree/fc86ccfedce00962204105400253b8ec265eebce
- ShapeBench preprint: https://arxiv.org/abs/2605.20763
- AIAA DPW8/AePW4 test cases: https://www.aiaa-dpw.org/TestCases/testcases.html
- NASA CFD verification assessment: https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html
- SU2 releases: https://github.com/su2code/SU2/releases
