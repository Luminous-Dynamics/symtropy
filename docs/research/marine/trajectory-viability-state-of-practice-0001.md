# Marine Trajectory Viability: 2026 State of Practice and Benchmark Position

**Date:** 19 November 2026  
**Track:** Symtropy #6261 and the finite-horizon audit under PR review

## Current state of practice

### Risk-aware weather routing is already strong

Zhang et al., *Risk-aware ship weather routing with ensemble forecasts on adaptive hexagonal grids*, Ocean Engineering 366, article 127661 (15 October 2026), DOI https://doi.org/10.1016/j.oceaneng.2026.127661. The study combines:
- adaptive multi-resolution H3 hexagonal route topology;
- traffic-separation scheme topology;
- edge-level pruning against IMO second-generation intact-stability criteria;
- 50 perturbed ECMWF ensemble members;
- risk-weighted and probability-budgeted routing;
- member-coherent route loss and CVaR assessment.

Its abstract reports that a failed deterministic forecast could make routing more exposed than weather-blind routing, while member-coherent CVaR bracketed realized exposure that marginal summaries missed. It also reports a marginal-equivalence result: two routing objectives were recoverable from precomputed probability fields, so ensemble input did not alter the deterministic path in the tested cases. This means simply adding “ensemble awareness” or “CVaR routing” is not a defensible novelty claim.

### Voyage scheduling and ship energy management are already coupled

Gao et al., *Towards spatio-temporal integration: a coordinated optimization method for voyage planning and energy management for all-electric ship with energy storage system*, Journal of Energy Storage 163, article 122206 (30 June 2026), DOI https://doi.org/10.1016/j.est.2026.122206. Their paper couples ship displacement and power-system scheduling using a day-ahead stage and an intraday DDPG-QP stage. The key comparison target is a two-timescale route/energy optimizer with physical constraints, not just static fuel minimization.

A second baseline is *Adaptive dual-layer MPC for real-time energy management of an inland diesel–LNG–battery vessel on the Yangtze River*, Energy 344 (1 February 2026), article 139792, DOI https://doi.org/10.1016/j.energy.2025.139792. It explicitly addresses the long-horizon optimality versus short-horizon real-time feasibility tradeoff.

### Safety assurance is moving toward invariance under disturbances

The 2026 IEEE/ASME Transactions on Mechatronics survey of safety-critical control under disturbances frames safety as keeping state trajectories inside forward-invariant safe sets under uncertainty and disturbance, DOI https://doi.org/10.1109/TMECH.2026.3666590. This is a useful conceptual target, but it does not mean the present sampled trajectory auditor proves invariance. A genuine viability proof requires validated dynamics, controls, disturbance bounds, inter-sample reasoning and recursive feasibility.

## Competitive boundary

Commercial sailing products already compare multiple forecast-model routes and evaluate motion/comfort variables. PredictWind says its route system produces eight model routes; its comfort settings cover RMS roll, vertical acceleration and slamming. Its help centre also documents an automatic rerun without those limits when no route satisfies them:
- https://help.predictwind.com/en/articles/2884622-weather-routing-change-weather-routing-settings
- https://explore.predictwind.com/features/ai-weather-router

Do not claim novelty for multi-model weather routing, comfort scoring or a route that returns despite constraint infeasibility.

## Proposed differentiator to test

The distinct research target is an independent **whole-voyage service/resource feasibility verifier** shared across weather, propulsion and onboard systems:

1. The planner proposes a trajectory; an independent verifier checks evidence, floors and balances.
2. Essential resources are represented as time-indexed state trajectories, not just arrival estimates.
3. Every required disturbance branch is explicit, with a frozen scenario roster.
4. Energy, freshwater and waste balances reconcile across every adjacent pair of time points.
5. A resource supplier being unavailable prohibits crediting its production in that interval.
6. Common-cause outages are represented as correlated events, not products of independent component probabilities.
7. A single intermediate reserve/service breach forces an explicit `INFEASIBLE/ABSTAIN` result; a later recovery does not erase the breach.
8. Multiple resource axes remain non-compensatory; no weighted scalar can trade potable-water failure against faster arrival.

This is an architecture and evaluation target, not an achieved real-world advantage.

## Benchmark targets

| Benchmark | Research target |
|---|---|
| Scenario completeness | 100% exact required scenario roster and digest match |
| Time coverage | 100% time-grid points present in every scenario |
| Floor safety | Zero accepted conservative-bound breaches at any sampled time |
| Conservation | 100% of energy/water/waste transitions close within preregistered physical tolerances |
| Equipment causality | Zero generation/production credited while the supplying asset is unavailable |
| Common-cause fidelity | Every common-cause branch has one event identity across all dependent failures |
| Initial-state fairness | 100% exact initial-resource equality across scenario branches |
| Decision transparency | First breach time, worst margin per axis and scenario identity always reported |
| Execution performance | p50/p95/p99 evaluator latency reported alongside full evidence output |
| Physical progression | model validation → HIL/PHIL → independently reviewed sheltered-water campaign |

## Important scientific boundary

The finite-horizon tool checks declared sampled states and flow ledgers. It does **not** prove inter-sample safety, recursive feasibility, system stability, a mathematical viability kernel or navigation safety. Before those claims, the programme needs identified state-transition models, admissible control sets, disturbance sets, sensor uncertainty, model validation and an independent assurance argument.

The near-term win is to make infeasible voyages impossible to silently mark as feasible, and to make every accepted feasibility assertion traceable to a time-indexed, scenario-complete resource ledger.
