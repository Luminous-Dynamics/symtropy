# Exact temporal oracle v0.1

This module is a **test-only exhaustive oracle** for tiny integer-tick service-chain
fixtures. It is not a production optimizer, dispatch planner or physical model.

## Model

The oracle accepts a structurally valid `ServiceChainCase` and exhaustively
enumerates operation start ticks for every already-declared voyage/speed
alternative.

- The first channel-transit interval ends at the conservative latest tick in the
  alternative's half-open arrival window.
- Later route operations may start at any integer tick no earlier than the prior
  operation's end.
- Every operation must fit wholly inside one declared availability window for
  every resource it uses, and the declared capacity must cover the operation's
  use in the exact resource unit.
- The last operation must complete inside the demand's half-open delivery window.
- Waiting is idle time between consecutive operations.
- Water and energy are read from separately declared resource/ledger boundaries;
  the oracle does not infer physical water or energy demand.

## Objectives

The independent objective vector minimizes four values separately:

1. delivery tick;
2. idle waiting ticks;
3. total declared energy (Wh across propulsion, auxiliary, shore and onward-transfer
   boundaries);
4. lock-cycle water (mL).

The oracle returns the non-dominated frontier. It does not hide the trade-off in a
weighted score. The reference fixture intentionally has two non-dominated plans:

| Alternative | Delivery tick | Idle wait | Energy | Water |
|---|---:|---:|---:|---:|
| Fast / high energy | 33 | 3 ticks | 2,800 Wh | 4,000 mL |
| Slow / lower energy | 35 | 0 ticks | 2,300 Wh | 4,000 mL |

These are synthetic fixture outputs, not performance claims about vessels,
locks, ports, rail, or actual waterways.

## Completeness and budget

The caller supplies a maximum count of explored start-state candidates. A result
is either:

- `ExhaustiveComplete`: the finite integer search ended naturally; its frontier is
  proven for this frozen discrete model;
- `BudgetExhausted`: the search hit its budget; the returned frontier is partial
  and **not proven optimal**.

An empty frontier is meaningful only when search completed exhaustively. No
budget-exhausted result may be described as infeasible or optimal.

## Scope limits

The oracle assumes a single demand, one vessel alternative per plan, a single
ordered route, fixed segment durations, integer ticks, and fixed capacity windows.
It does not model concurrent ships, cumulative sharing between multiple demands,
time-varying hydraulic transients, tide/salinity physics, under-keel clearance,
sediment evolution, queue arrival distributions, uncertain duration distributions,
crew/workforce rosters, schedule negotiation, or dynamic replanning.

It reuses the scenario contract for input validation but independently implements
operation-time enumeration and resource-window fit; it does not call a production
optimizer or metric aggregator. The next qualification step is differential
testing against an independent MILP/CP-SAT formulation on matched tiny cases,
then expanding to multiple vessels/demands and serial-lock route segments.
