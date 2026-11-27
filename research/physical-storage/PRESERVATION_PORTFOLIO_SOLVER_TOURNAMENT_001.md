# Preservation Portfolio Solver Tournament — v0.1 roadmap

**Parent programme:** [PRESERVATION-INTERVENTION-PORTFOLIO-001 #7673](https://github.com/Luminous-Dynamics/symtropy/issues/7673)  
**Reference implementation:** preservation-intervention-portfolio-reference  
**Purpose:** compare exact, independently implemented Pareto-front algorithms on the preservation action-selection problem.

## 1. Why this tournament is necessary

The current Rust planner is an exact reference for small instances, not a state-of-the-art optimizer. It enumerates every non-empty subset and applies pairwise dominance filtering. The current 12-action bound limits enumeration to 2^12−1 = 4,095 subsets and the worst-case pairwise filtering to roughly 16.8 million comparisons.

A recent, directly relevant 2026 contribution is **PAUGMECON**, which combines the SAUGMECON epsilon-constraint method with a Pareto global constraint and archive-based pruning/reuse. Its authors compare PAUGMECON against SAUGMECON and MOBAB-CP on multi-objective knapsack, N-Queens, resource-constrained project scheduling and satellite-image selection.

The reported results are problem-dependent:
- PAUGMECON improves on SAUGMECON across the tested MUKP objective counts; it has the strongest profiles for two and three objectives, while MOBAB-CP is best for four and five.
- MOBAB-CP is strongest on the tested multi-objective N-Queens set and completes all instances within the paper's time limit, while PAUGMECON misses ten and SAUGMECON misses 25.
- PAUGMECON is best on the paper's MORCPSP HV/IGD+ comparison and finds fronts on 22 of 70 instances, compared with 20 for SAUGMECON and none for MOBAB-CP.
- On SIMS, PAUGMECON has the strongest reported HV and completes 26 of 48 instances, compared with 24 for SAUGMECON and 22 for MOBAB-CP.

The study reports severe degradation for PAUGMECON and SAUGMECON beyond three objectives, with MOBAB-CP preferable in the tested four-/five-objective cases. These are results for the paper's benchmark instances, Choco implementation and experimental setup—not measured results for preservation portfolios.

## 2. Preservation problem formulation

The portfolio planner is a 0/1 multi-objective combinatorial optimization problem with:
- binary action selection variables;
- all-alert coverage constraints;
- prerequisite evidence and approval-availability constraints;
- dependency closure constraints;
- destructive/source-discarding exclusion;
- hard budgets for cost, energy, duration and downtime;
- five minimization objectives: cost, energy, duration, downtime and action count.

This is related to multi-objective knapsack, set covering and resource-constrained project scheduling, but it is not identical to any one of those benchmark families. The action dependency graph and preservation trust/authority conditions must remain explicit in the formulation.

## 3. Required solver baselines

1. **Rust exhaustive reference** — current exact small-instance oracle (12 candidates).
2. **PAUGMECON** — primary modern exact multi-objective CP comparator.
3. **SAUGMECON** — epsilon-constraint baseline.
4. **MOBAB-CP** — Pareto global-constraint branch-and-bound baseline.
5. **OR-Tools CP-SAT epsilon-constraint adapter** — independent integer/Boolean modeling baseline. The adapter must implement the Pareto enumeration procedure itself; CP-SAT's OPTIMAL status for one scalar objective is not proof that the full Pareto front is complete.
6. **AUGMECON2 or another exact multi-objective MIP baseline** — optional where a reproducible implementation and licence permit.

The Choco-based published algorithms should be consumed as an external benchmark dependency or linked source baseline. Do not silently port or modify upstream code and then label the result as the published implementation. Record exact repository commit, solver version and licence.

OR-Tools CP-SAT is a suitable independent Boolean/integer baseline because its model is integer-based and it exposes distinct OPTIMAL, FEASIBLE, INFEASIBLE, MODEL_INVALID and UNKNOWN outcomes. Use those statuses honestly: FEASIBLE is not an optimality proof; UNKNOWN is incomplete; a full Pareto pass requires a complete enumeration/proof protocol.

## 4. Experimental design

### A. Exact small instances

Use the 19 frozen v0.1 cases and add generated cases with 2–12 actions. The Rust reference establishes exact frontiers and contract outcomes. Every solver must match:
- feasible/infeasible/evidence-incomplete/invalid-evidence classification;
- exact action-set Pareto frontier;
- hard budget and dependency enforcement;
- all excluded-candidate reasons;
- deterministic output after canonicalization.

### B. Scale-up instances

Generate frozen action portfolios with 16, 24, 32, 50, 100 and 250 candidates, varied alert counts, dependency depth, coverage density, objective count and budget tightness. The exhaustive reference is not expected to solve these; use exact solver proofs where available and independent cross-checks.

Stratify by structure:
- sparse versus dense coverage;
- shallow versus deep dependency DAG;
- single- versus multiple-action coverage;
- low versus high Pareto-front cardinality;
- tight versus loose budget caps;
- dominated, duplicate-objective and adversarially incomparable actions;
- many optional actions with no direct coverage but required as dependencies.

Do not infer solver performance from candidate count alone; front cardinality and constraint propagation can dominate.

### C. Metrics

Report:
- time to first feasible portfolio;
- time to first Pareto point;
- time to prove the complete front;
- exact-front completion rate by timeout;
- number of Pareto points;
- hypervolume and IGD+ when exact front is unknown;
- memory and peak archive size;
- explored search nodes / branches / epsilon regions where available;
- number of infeasibility proofs and reused regions;
- time spent in dominance filtering;
- model construction/translation overhead;
- objective-space dimension and front cardinality.

For approximate fronts, use one frozen normalization and reference point per instance. Do not compute normalization from each solver's own output. When the exact front is known, compare against it. When unknown, publish the union-reference approximation and clearly label it.

### D. Matched conditions

- Pin CPU model, core count, memory, OS, runtime, solver commit/version and build flags.
- Run single-threaded primary comparisons; report parallel results separately.
- Use the same time/memory limits for all methods.
- Include cold-start and warm-start runs separately.
- Preserve raw logs and front outputs.
- Run repeated trials and report median, p95 and dispersion; no cherry-picked best run.
- Use independent fixtures/instance seeds and hold back a test set.

The 2026 CP paper used single-core runs on an AMD EPYC ROME 7H12 64-core host, with 1.5-hour timeouts on MUKP/MN-Queens and 3-hour limits for MORCPSP/SIMS. Its results are useful methodological context; direct numerical comparison requires reproducing the same instances and hardware/solver configuration.

## 5. Research hypotheses

1. PAUGMECON-style archive pruning and region reuse should outperform all-subset enumeration as candidate sets grow.
2. Dependency-heavy portfolios may reduce the benefit of epsilon-grid enumeration and favour branch-and-bound or CP propagation.
3. The five-objective setting may amplify the degradation seen in the CP-2026 study beyond three objectives; use ablations with 2, 3, 4 and 5 objectives.
4. Approval/evidence/source-retention constraints may create strong propagation opportunities that generic knapsack benchmarks miss.
5. A dedicated multidimensional dominance archive may reduce the current pairwise filter bottleneck independently of the search method.

These are hypotheses; they are not claims of current performance.

## 6. Prospective success criteria

- 100% exact frontier agreement on the 19-case corpus and all generated small instances.
- Zero hard-budget, dependency, approval, source-retention or destructive-action violations.
- At least one exact solver completes all 50-action instances under the published time limit; choose and disclose a realistic limit after a baseline pilot.
- Publish scaling curves rather than one best runtime.
- Demonstrate reproducible frontier equality across at least two independent solver implementations.
- Any claimed improvement must name the exact workload family, solver version, hardware, objective count and time limit.

Do not claim “better than SOTA” from the v0.1 Rust exhaustive reference. The goal of this tournament is to discover which exact multi-objective method is best for this domain, then improve it with evidence-specific constraint propagation and provenance-preserving pruning.

## 7. References

- Combarro Simón, Talbot and Bouvry, “Combining an ε-Constraint Method with the Pareto Global Constraint,” CP 2026, DOI: https://doi.org/10.4230/LIPIcs.CP.2026.14
- Published implementation branch (Choco solver): https://github.com/mancs20/choco-solver/tree/CP-2026
- Google OR-Tools CP-SAT documentation: https://developers.google.com/optimization/cp/cp_solver
- OR-Tools MIP/CP-SAT comparison: https://developers.google.com/optimization/mip
