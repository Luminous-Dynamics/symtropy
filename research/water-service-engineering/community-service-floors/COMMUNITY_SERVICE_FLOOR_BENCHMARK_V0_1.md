# WATER-COMMUNITY-FLOOR-001 — Non-compensatory Community Service Floor Benchmark v0.1

**Tracking:** Symtropy [#5027](https://github.com/Luminous-Dynamics/symtropy/issues/5027)  
**Related:** pump service [#4959](https://github.com/Luminous-Dynamics/symtropy/issues/4959), sparse-sensor control [#5018](https://github.com/Luminous-Dynamics/symtropy/issues/5018), Symthaea climate-mobility service contract [#11832](https://github.com/Luminous-Dynamics/symthaea/pull/11832).

## Research question

Can a water-network control or assessment method improve total delivered volume while pushing a required community below its declared service floor—and would an aggregate metric hide it?

The benchmark treats service as a vector of required, non-substitutable area-level obligations. An aggregate score is diagnostic only and cannot override any one area's minimum.

## Paradigm shift

From a single network average to a **service-floor vector**:
- each required area has its own floor identity and revision;
- each observation has a conservative lower/upper bound, coverage and validity;
- missing, stale, partial or conflicting evidence remains indeterminate;
- a clear local breach remains a breach even when other areas have surplus;
- a known breach plus a separate unknown area is reported as both, not collapsed into a single optimistic or pessimistic bit.

This is a benchmark of evaluation semantics, not a rule for deciding which neighbourhood should receive water.

## Frozen v0.1 synthetic corpus

The companion files contain 12 cases, 3 required areas per scenario, a Draft 2020-12 schema and separate expected-disposition oracle.

They cover:
- all service floors met;
- a weighted average of 98.43% that hides one area's 25% delivery;
- a floor breach while the aggregate remains above the floor;
- missing, stale and partially covered evidence;
- a measurement interval crossing the floor;
- known breach plus unknown evidence;
- missing policy identity, unit mismatch and time-grid mismatch;
- exact inclusive boundary at the floor.

The synthetic profile uses a 0.80 minimum delivery fraction, three abstract DMAs and synthetic weights. **These are not South African service standards, Johannesburg Water data, real utility operating setpoints or household measurements.**

## Disposition semantics

For an individual area:
- `meets_floor`: valid, current, complete evidence and conservative lower bound >= floor;
- `breached_floor`: valid, current, sufficiently complete evidence and conservative upper bound < floor;
- `indeterminate`: missing/stale/conflicting/partial evidence or an interval that crosses the floor;
- `invalid`: unsupported policy, metric, unit or time-grid contract.

For the complete required vector:
- `meets_all_floors`: every required area meets;
- `breached_floor`: one or more areas breach and none are indeterminate;
- `indeterminate`: no known breach, but at least one area is unknown;
- `breached_with_indeterminate`: at least one known breach coexists with at least one unknown;
- `invalid`: one or more required records are structurally unsupported.

Never resolve a mixed breach/unknown state using an average.

## Comparator families

- Weighted average delivery: report as an intentionally lossy diagnostic baseline.
- Independent area threshold: basic hard-floor comparator.
- Lexicographic / max-min service-floor constrained control: optimise service margins without trading away a protected floor.
- CVaR / tail unmet-demand objective: supplementary risk metric only; it does not replace the floor.
- Controller families from #5018: run on identical hydraulic traces, sensor budgets and actuator constraints.

Do not compare methods with different population/area weights, different required-area sets, or different evidence coverage.

## Metrics

Report each area's:
- lower/upper service margin to floor;
- breach degree-time and maximum continuous shortfall where time series exist;
- evidence coverage and missingness;
- abstention / indeterminate disposition;
- recovery time after disturbance.

Report separately:
- minimum service margin across all required areas;
- count/fraction of areas meeting, breached and indeterminate;
- network weighted mean as a non-authoritative diagnostic;
- worst-area unmet-demand bound and optional CVaR.

A scalar fairness score, city average or total volume must never erase an area-level breach.

## Proposed qualification targets

Targets, not achieved results:
- 100% exact matches on frozen known-answer semantic cases;
- zero aggregate-average passes if any required, fully observed area is below its floor;
- zero positive floor claims with missing, stale, incomplete, conflicting or unsupported evidence;
- zero omission of a required area from the evaluation set;
- 100% of breach-plus-unknown cases retain both conditions;
- in matched control comparisons, seek >=10% reduction in worst-area unmet-demand bound without any required floor regression or lower evidence coverage.

## Integration boundary

- Symtropy owns physical service-area/network scenario construction and benchmark evaluation.
- Symthaea consumes qualified per-area results for scenario comparison and abstention; it does not choose migration or allocation policy.
- Mycelix owns source and measurement provenance; publisher notices are not telemetry.
- Sol-Atlas owns authoritative geometry and area-boundary revision.

No allocation, rationing, shutoff, enforcement, relocation or pump/valve actuation is authorized.

## Nonclaims

A synthetic fixture match does not establish real community access, pressure, delivery volume, water quality, legal adequacy of a service standard, health effect, fairness, utility performance or public-sector deployment readiness. Real floors must be supplied by a competent authority or explicitly approved research policy; source and observation coverage must be qualified independently.
