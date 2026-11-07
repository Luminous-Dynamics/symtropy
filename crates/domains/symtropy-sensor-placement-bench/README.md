# Symtropy Sensor Placement Bench

Exact small-set search for Pareto sensor layouts using the fault-pair observability contract from #3993/#4005.

Implements [issue #4006](https://github.com/Luminous-Dynamics/symtropy/issues/4006).

## Run

    cargo test --manifest-path crates/domains/symtropy-sensor-placement-bench/Cargo.toml

## Input

The optimizer accepts:
1. an existing sensor-observability case (same state taxonomy, response intervals, channel independence groups and critical-pair set); and
2. a placement profile with candidate sensor lifecycle cost, maintenance burden, unavailable time, cost provenance, maximum channel count/budget and declared outage scenarios.

## Exact search

v0.1 enumerates all non-empty subsets for at most 20 candidate sensors. It rejects larger inputs with SEARCH_SPACE_LIMIT rather than silently using a heuristic. It evaluates each subset nominally and under every declared channel/common-principle outage scenario.

A layout is feasible only when every critical pair is identifiable in the nominal case and every required outage scenario. OOD and non-critical pairs are still reported; they are not forced into a known diagnosis.

## Pareto objectives

All are retained as separate dimensions:
- lifecycle cost (minimize);
- selected sensor count (minimize);
- maintenance hours/year (minimize);
- planned unavailable hours/year (minimize);
- worst-case number of all required pairs identifiable (maximize).

No weighted sum or single winner. The output is a deterministic Pareto set.

## Evidence boundaries

The supplied placement costs are synthetic cost points, not currency, vendor quotes or mine economics. Channel signatures are inherited from the #4005 fixture and are synthetic response intervals. Feasibility therefore demonstrates algorithmic behaviour only, not an actual recommended sensor layout.

Real use requires calibration-domain qualification from #3991, actual lifecycle cost sources, common-mode failure analysis and independently measured fault response envelopes.
