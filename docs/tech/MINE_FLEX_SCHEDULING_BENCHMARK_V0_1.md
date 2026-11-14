# MINE-FLEX-001 — safety-first flexible mine-load dispatch v0.1

## Claim ceiling

This is a tiny, public, authored synthetic dispatch/accounting oracle. It is **not** a mine controller, an operational savings estimate, a plant model, an environmental assessment, or authorization to change physical operation. All electricity, water, price and carbon quantities are arbitrary integer units.

## Paradigm shift

The decision order matters:

1. Preserve declared safety-critical electric load and protected water reserves as hard invariants.
2. Consider flexible process batches only after those floors are feasible.
3. Couple each period's process plan to renewable availability, import capacity, water above the protected reserve, grid price and carbon intensity.
4. Report a cost/carbon Pareto frontier rather than hiding the trade-off inside one weighted score.
5. Preserve infeasible as a valid result. A scheduler must not make an unsafe or water-reserve-diverting schedule look feasible by silently dropping a constraint.

This fixture does not model ventilation physics, mine dewatering hydraulics, equipment ramp limits, battery storage, ramp/start costs, stochastic weather, water quality, process kinetics, inventory decay, or network power flow. Those require separately owned evidence-bearing models.

## Frozen case

Three time periods each require 4 fixed safety-electricity units. Each process batch uses 2 more electricity units and 1 process-water unit. Protected water reserves are 2 units per period. Renewable power, import limits, unit prices and carbon intensities vary by period.

The target is four batches over periods P0–P2. The fixed comparison schedule is `[2, 1, 1]`; the exact bounded enumerator identifies `[1, 3, 0]` as the minimum-grid-cost schedule.

| Schedule | Arbitrary grid cost | Arbitrary grid carbon |
|---|---:|---:|
| Fixed baseline `[2,1,1]` | 590 | 100 |
| Minimum-cost `[1,3,0]` | 410 | 152 |
| Pareto candidate `[2,2,0]` | 450 | 132 |
| Pareto candidate `[1,2,1]` | 550 | 120 |
| Pareto candidate `[2,1,1]` | 590 | 100 |

On this constructed fixture, minimum-cost dispatch lowers the arbitrary cost total by 30.5% versus the fixed baseline while increasing the arbitrary carbon total from 100 to 152. That is the point: cost-only optimization is not automatically low-carbon. These ratios are fixture outcomes, not field performance.

There are 36 possible per-period batch vectors, eight vectors meet the four-batch target, of which five satisfy every period's power and water constraints. The Pareto frontier preserves all four non-dominated schedules; the fifth feasible schedule `[0,3,1]` is dominated.

## Validation

- Exact corpus schema with unknown-field rejection, bounded period count and explicit exact-search ceiling.
- Exhaustive integer allocation enumeration for the small fixture.
- Per-period electricity demand, renewable use, grid import, cost, carbon, process water and remaining protected reserve in every returned score.
- Independent balance reconstruction from input plus selected batch vector; the verifier does not call the schedule scorer.
- Zero schedules violating process capacity, electrical import caps or protected water reserves.
- Perturbation tests for grid-import shortfall, process-water shortfall, unserviceable safety load, insufficient water for the production target and a violated protected-water reserve.
- Receipt with corpus SHA-256, exact source commit, expected-control disposition and independent hard-constraint check.

## Research anchors

- [Australian Government — Mining sector energy guide](https://www.energy.gov.au/business/sector-guides/mining) notes the large contribution of comminution energy and opportunities around dewatering/process design. It motivates dimensions, not default fixture values.
- [Monteiro, van Laar & Schutte — A Simulation-Based Water Management Strategy for Life-of-Mine Water Planning](https://doi.org/10.7166/33-3-2787), *South African Journal of Industrial Engineering*, argues for integrating service-water and dewatering requirements over a mine life.
- [Mine operations as a smart grid resource](https://doi.org/10.1016/j.mineng.2019.106001) studies flexible demand and process holdup under grid conditions.
- [Wang et al. (2026) — Robust Low-Carbon Economic Dispatching of Coal Mine Integrated Energy Systems](https://doi.org/10.3390/su18126042) and [a 2026 copper-processing microgrid study](https://doi.org/10.1016/j.ecmx.2026.101569) show current research directions in coupled dispatch. Their numerical results are not copied into this corpus.

## Required next tier

To make this industrially meaningful, add pre-registered held-out scenarios for forecast errors, correlated renewable shortfalls, variable water quality, water-network capacities, equipment ramp/minimum-up times, intermediate stockpile bounds, unplanned outage and carbon/water caps. Compare policies on identical conditions, report paired regret and adverse-tail outcomes, and keep safety floors as zero-tolerance hard constraints. Only then connect site-specific, rights-cleared operating data with independent operations/safety review.
