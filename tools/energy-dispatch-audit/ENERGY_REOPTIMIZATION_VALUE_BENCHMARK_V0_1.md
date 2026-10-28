# ENERGY-REOPT-VALUE-001: Event-triggered re-optimization versus fixed cadence v0.1

## Research question

When forecasts update and solver calls are limited, should an energy controller re-optimize at a fixed cadence or only when an observable forecast-residual signal indicates that the current plan may be wrong? Treat compute as a budgeted resource, but never trade critical-service requirements away in an aggregate scalar.

## Frozen synthetic known answer

Four contexts, three intervals, initial/capacity battery SoC 2 quanta, maximum discharge 1 quantum per interval. At t0 the only discretionary action is serving one flexible quantum or holding enough energy for forecasted future essential demand. A context-specific residual is observable before t0; a residual at or above 1.0 triggers an additional solve before the action is committed. Updated forecast data are available at t0.

| Policy | Solver calls | Essential unserved | Flexible unserved |
|---|---:|---:|---:|
| Fixed cadence (three calls per scenario) | 12 | 0 | 1 |
| Event-triggered (initial + one triggered re-solve) | 5 | 0 | 1 |
| No re-optimization | 4 | 1 | 0 |
| Event threshold 3.0 | 4 | 1 | 0 |

On this frozen panel the event-triggered policy uses 58.333% fewer modeled solver calls than the fixed-cadence policy while matching its service outcomes. The no-reoptimization and too-high-threshold baselines miss the large observed residual and incur one essential unserved quantum.

This is a synthetic solver-call count result, not measured wall-clock runtime, general online-regret evidence, an expected savings estimate or a physical-site outcome.

## Required Rust implementation

1. Bind raw panel bytes to SHA-256 and preserve the experiment contract digest in the report.
2. Make trigger threshold, inclusive boundary, residual availability, initial/update forecasts and cadence semantics explicit.
3. Make controller selection depend on observable context and residual—not scenario ID or future demand labels.
4. Reproduce exact policy totals and solver-call counts.
5. Preserve full scenario/interval action, SoC, discharge and service-gap traces, trigger decisions and per-scenario call counts.
6. Test threshold equality, digest mutation, truth-label tampering, context/identity leakage, reordered/missing intervals and unsupported probability weights.
7. Report service and solver calls separately; solver calls are not wall-clock time.
8. Keep physical dispatch audit `NOT_RUN` for this abstract battery fixture.

## Paradigm shift

Treat compute as part of the control problem: decide when to spend another solve, where new information can affect the next action, and when recomputation cannot change a forced action. Trigger on observable *decision-relevant residual*, not a fixed timer alone. But retain critical-service gates as non-compensatory constraints: a saved solver call is not a win if it causes essential load to go unserved.

## Promotion ladder

Exact finite fixture → sweep thresholds, residual noise, update delay and solver-call budgets → evaluate 1,000+ chronological held-out traces → compare fixed cadence, residual-triggered heuristic, MPC and adaptive rolling-horizon approaches on identical hardware, input feeds and time/compute budgets. Report service gaps, regret against a bounded exact oracle, solve count, wall-clock runtime, peak memory and missed-trigger cases independently.

## Related research

- Adaptive Rolling Horizon Optimization for Microgrid Energy Management Under Uncertainty, published 8 June 2026: https://www.mdpi.com/2071-1050/18/12/5868
- Online dispatch of multi-duration energy storage with explainable reference learning, Applied Energy, 1 June 2026: https://www.sciencedirect.com/science/article/pii/S0306261926003776
- Decision-Focused Learning for Power System Decision-Making Under Uncertainty: https://ieeexplore.ieee.org/document/11122623/

The papers motivate the comparison ladder; they do not establish that this synthetic experiment outperforms them.
