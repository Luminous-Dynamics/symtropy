# HoloFrac Cross-Scenario Climate Robustness Protocol v0

**Status:** deterministic synthetic scenario-robustness fixture  
**Implementation:** research/holofrac-retrieval/src/climate_robustness.rs  
**Executable:** research/holofrac-retrieval/src/bin/climate_robustness_sweep.rs  
**Workflow output:** climate-robustness.csv  
**Related:** [#6025 — climate robustness, Pareto persistence and regret](https://github.com/Luminous-Dynamics/symtropy/issues/6025), [#5961 — budgeted experiment selection](https://github.com/Luminous-Dynamics/symtropy/issues/5961), [#6007 — external qualification](https://github.com/Luminous-Dynamics/symtropy/issues/6007)

## 1. Objective

Move from optimizing a design against one representative weather file to measuring how design feasibility and Pareto quality change across multiple declared climate/operational futures.

The initial fixture separates:
- present-day specialist designs;
- future-climate specialists;
- compound heat/grid-stress scenarios;
- universally feasible designs;
- universally Pareto-optimal designs;
- designs on a robust frontier formed from worst-case objective vectors.

This is a finite synthetic benchmark. It does not use CMIP6 data, measured buildings, calibrated weather downscaling or real operational data.

## 2. Scenario and objective contract

The first scenario set contains:
- present_tmy;
- future_hot;
- compound_heat_grid.

Each scenario has an explicit non-negative decision weight, and weights sum to one. The initial fixture uses **AnalyticalDecisionWeights**. These represent declared decision priorities only; they are not probabilities and must not be reported as expected climate frequencies or expected outcomes.

ProbabilityWeights are accepted only when a non-empty provenance/basis identifier is declared. That identifier alone does not validate the probability model; independent justification of probabilities is still required before expected-value language is used.

Three normalized objectives are used, each oriented so that higher is better:
- thermal safety;
- energy efficiency;
- affordability.

Each candidate has a result for every scenario:
- objective vector;
- independently evaluated hard-feasibility flag.

The fixture requires exact one-to-one scenario coverage per candidate. Missing/duplicate scenario rows, invalid weights, non-finite/out-of-range outcomes, or a scenario with no feasible candidate fail closed.

## 3. Calculated robustness metrics

For each scenario:
- feasible candidate count;
- scenario-specific Pareto frontier;
- scenario ideal objective vector.

Across scenarios:
- union of all scenario-specific Pareto frontiers;
- intersection of the scenario frontiers (universal Pareto membership);
- mean pairwise Jaccard similarity between scenario-specific frontiers;
- weighted mean outcome vector for each candidate;
- worst-case objective vector;
- weighted feasibility coverage;
- guaranteed-feasible status (hard constraints pass in every scenario);
- weighted regret by objective;
- maximum regret over every scenario/objective.

The **robust frontier** is calculated from the candidate worst-case objective vectors, but only among candidates that are hard-feasible in every scenario. A candidate with a good average but one hard-feasibility failure is never classified as guaranteed-feasible or placed on this robust frontier.

Weighted feasibility coverage is a weighted summary under the declared scenario weights. Under AnalyticalDecisionWeights it is not a probability or guarantee. Guaranteed-feasible means only that all scenarios in the declared finite set pass their hard constraints.

Regret is measured against the best feasible candidate per objective within each scenario. Weighted regret uses the declared decision weights; maximum regret is the largest objective regret over all scenarios. This is a transparent component-wise regret measure, not a substitute for a fully specified scalar social-utility or loss function.

## 4. Baselines and comparisons

Before making a superiority claim, compare against:
1. present-TMY-only optimization;
2. independent optimization per scenario;
3. union of scenario-specific Pareto sets;
4. intersection/universal Pareto candidates;
5. robust maximin optimization;
6. minimax regret;
7. expected/weighted utility only when scenario probabilities are independently justified;
8. surrogate-assisted NSGA-II/III, MOPSO or Bayesian optimization under matched solver budgets where applicable.

All methods must use the same candidate variables, hard constraints, scenario set, solver fidelity, information, evaluation budget, hardware and stopping criteria. Include failed/infeasible candidates. Hold out scenario families or climates when claiming generalization.

## 5. Synthetic acceptance corpus

The current executable includes:
- a present-day specialist feasible only under the present-TMY scenario;
- a balanced candidate feasible across all scenarios;
- a future-climate specialist;
- a low-cost but brittle design;
- a stable moderate candidate.

The tests verify:
- a present-only specialist cannot be labelled guaranteed-feasible;
- the robust frontier includes only universally feasible candidates;
- scenario-specific and robust Pareto frontiers match separately authored finite oracles;
- analytical weights are not reported as probabilities;
- probability weights require explicit provenance;
- incomplete candidate coverage and invalid weight sums fail closed;
- every declared scenario has at least one feasible candidate.

## 6. Prospective targets

These are research targets, not achieved results:
- 100% independent-oracle agreement for scenario frontiers, robust frontier and regret on frozen finite fixtures;
- zero candidates labelled guaranteed-feasible if any declared scenario violates a hard constraint;
- explicit universal-feasibility and universal-Pareto membership rather than average-only scores;
- quantify frontier persistence, weighted/maximum regret and feasibility-margin degradation across scenarios;
- compare robust methods against the strongest matched scenario-specific and regret baselines under equal solver budgets.

For a future rights-cleared or measured case study, predeclare the climate/time horizons, scenario set, local microclimate assumptions, occupancy/supply stressors, minimum hard constraints, objective scaling and decision weights before running optimization.

## 7. Required real-world extensions

1. Current and future climate files, with source/version and downscaling provenance.
2. Urban microclimate/heat-island and building-specific exposure.
3. Stochastic/culturally calibrated occupancy and manual window/shading/ventilation behavior.
4. Grid outage, demand-response, time-varying tariff and emissions-factor scenarios.
5. Water scarcity and supply restriction scenarios.
6. Component degradation, service-life and maintenance uncertainty.
7. Compound heat wave + occupancy peak + grid stress + degraded cooling.
8. Surrogate error and out-of-domain detection under held-out climates.
9. Matched BOPTEST control studies for operational policy comparisons, plus applicable first-principles simulation qualification.

SSP narratives are not probability distributions by default. Weather, occupancy, tariff, grid and supply scenarios must remain distinct, versioned inputs. Do not merge them into one synthetic probability distribution without independent justification.

## 8. Claim ceiling

Passing establishes only correct finite-scenario metric calculation on the declared synthetic corpus. It does not establish climate resilience of a real building, future-climate probability, energy savings, comfort, safety, net-zero status, code compliance or adaptation certification. Real claims require defensible weather/occupancy/supply scenarios, calibrated physics simulation, held-out validation and independent review.
