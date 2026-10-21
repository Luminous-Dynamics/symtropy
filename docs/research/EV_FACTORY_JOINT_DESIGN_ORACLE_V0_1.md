# EV factory joint-design exact oracle v0.1

**Issue:** [EV-FACTORY-DESIGN-001 #2526](https://github.com/Luminous-Dynamics/symtropy/issues/2526)  
**Evidence class:** synthetic response surface; no measured plant data  
**Purpose:** verify joint-design search methods against exact enumeration on a small, auditable design space

## Paradigm shift

The single-intervention frontier asks what happens if one station receives an extra server. A real factory decision can combine equipment, yield/rework, buffers and changeover improvements. Their effects may interact, and hard limits on capital, labor, power and space can make an otherwise attractive combination infeasible.

This v0.1 benchmark enumerates all binary combinations of five options (32 designs), computes a finite robust objective, and exposes the full design set and Pareto frontier. It compares the exact robust best with no change, the best feasible single intervention, and a greedy marginal-gain policy.

## Response model and objective

Each option has a scenario-specific main effect. A sparse table adds scenario-specific pairwise interactions for selected pairs. Missing interaction rows mean a zero pairwise interaction by this fixture contract. The model has **no higher-order interaction terms**; the fixture states this limit explicitly.

For option set \(D\) and declared synthetic scenario \(s\):

\[
G(D,s)=\sum_{i\in D}g_i(s)+\sum_{\{i,j\}\subseteq D}h_{ij}(s).
\]

The primary decision objective is set-based robust good-unit gain:

\[
G_{\min}(D)=\min_{s\in S}G(D,s).
\]

No scenario probabilities are assigned. The exact oracle maximizes \(G_{\min}\) over feasible designs; ties are broken by lower CAPEX, energy and labor, followed by a stable design ID. The report also exposes the Pareto frontier for robust good-unit gain and minimized CAPEX, staffed labor, energy, emissions and footprint. The tie-break rule selects a reproducible benchmark winner; it does not replace or collapse the Pareto frontier.

A design is feasible only if it respects every declared hard constraint on CAPEX, staffed labor hours per shift, active power and floor area. Infeasible designs are never members of the reported Pareto frontier. Unknown safety or release readiness is not represented as a soft score; production authorization remains false.

## Baseline comparison

- **No change:** hold capital and accept zero incremental output.
- **Best single intervention:** select the best feasible design with exactly one option.
- **Greedy marginal worst-case gain:** repeatedly add the feasible option producing the largest positive improvement in the current worst-case gain; stop when no single addition improves it.
- **Exact oracle:** enumerate every binary design and choose the feasible design maximizing the declared robust objective.

The synthetic fixture deliberately creates a complementarity case: the best single intervention and the greedy design are inferior to the exact oracle. This is an oracle test, not evidence that the greedy policy or any specific factory intervention behaves that way in practice.

## Fail-closed contract

- Strict schema; synthetic evidence class and claim ceiling required.
- Unique scenario and option IDs.
- Complete one-row-per-option/per-scenario main-effect catalog.
- Finite main effects, interactions and non-negative resource inputs.
- Canonical option ordering for each pairwise interaction; duplicates and unknown IDs rejected.
- Exact search capped at 20 binary options (1,048,576 combinations) to avoid accidental combinatorial explosion.
- Every design receives explicit feasibility reasons; hard-limit violations exclude it from the frontier and search recommendations.
- Source fixture SHA-256 and byte-identical deterministic report on repeated runs.
- Negative controls for duplicate main effects, non-canonical interactions, invalid resources and the exact-search size limit.
- No scenario probabilities, expected value, ROI, purchasing approval or production release.

## Targets for subsequent versions

This v0.1 response-surface oracle is a structural test harness, not a calibrated production digital twin. The next version should add a runner adapter so feasible portfolios are evaluated in the discrete-event simulator with common random numbers, then compare search algorithms using equal simulation budgets and frozen holdouts.

On small spaces, exact enumeration agreement must be 100%. For larger spaces, report median, 90th-percentile and worst-case regret against the exact oracle on held-out instances; also report constraint violation rate (target: zero among designs labelled feasible), Pareto-front coverage, dominated outputs, simulation calls, wall time, memory, and uncertainty in candidate effects. Benchmark greedy, best-single, random feasible search, and at least one established heuristic/optimizer before claiming an advantage.

A credible industry comparison also needs to distinguish:
1. exactness of the optimizer on its supplied response model;
2. correctness of the discrete-event simulation;
3. calibration of the simulation to a physical factory;
4. validity of the capital, staffing, energy and carbon inputs.

Passing the first two does not establish the latter two.

## Industry and research context

AnyLogic's documented optimization experiments provide parameter constraints, post-simulation requirements and replicated evaluation for stochastic models: https://anylogic.help/anylogic/experiments/optimization.html. The ranking-and-selection literature treats statistically sound design selection as its own problem; a South African Industrial Engineering review surveys single- and multi-objective simulation ranking and selection: https://doi.org/10.7166/28-2-1732.

These are references for establishing strong baselines, not claims that this synthetic oracle outperforms AnyLogic or any commercial optimizer.

No MES/PLC/plant traces were collected; no factory, integrator or supplier was contacted; no quote was requested.
