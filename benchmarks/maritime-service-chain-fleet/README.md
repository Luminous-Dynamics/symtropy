# Maritime service-chain fleet oracle v0.1

Dependency-light Rust crate for tiny, deterministic multi-demand scheduling over shared corridor resources. It composes validated `ServiceChainCase` records from `maritime-service-chain-contract`; it does not duplicate vessel, route, evidence, participant or port-call schemas.

## Run

```sh
cargo fmt --manifest-path benchmarks/maritime-service-chain-fleet/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-service-chain-fleet/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-service-chain-fleet/Cargo.toml --all-targets -- -D warnings
```

## Resource semantics

Every resource has one explicit `SharedResourcePolicy`:

- `ConcurrentCapacity`: sum of resource use across all overlapping operations must not exceed capacity at any tick. Half-open intervals are used; an operation ending at tick 10 does not overlap one beginning at tick 10.
- `CumulativeWindowQuota`: each operation must fit wholly inside one declared accounting window, and its consumption is added once to that window's quota.

This version requires `WaterQuota`, `ShorePower`, `TerminalHandling` and `InlandTransfer` to use cumulative-window accounting. The latter two represent tonnes-per-window throughput quotas in this model, not simultaneous occupied tonnes. Channel transit slots, lockage slots and berth slots use concurrent-capacity accounting. The mode is explicit in the workload and checked against the typed resource kind; a shift throughput quota cannot be silently treated as a concurrent slot. A future profile that models cranes or other simultaneous handling equipment must give those resources a distinct typed resource identity/unit rather than reinterpreting `Tonnes`.

All cases in one workload must share a corridor, decision cutoff and tick resolution, bound to a caller-declared `time_basis_id`. If two cases reference the same resource ID, their complete resource declarations and referenced evidence records must be identical. Conflicting capacities, windows, units, revisions or evidence snapshots reject the workload.

## Exact tiny-instance oracle

The oracle:

- validates each one-demand case before search;
- sorts demands and alternatives canonically, independent of input case ordering;
- enumerates integer-tick placements for every declared alternative;
- preserves within-vessel route precedence;
- enforces shared concurrent capacity and cumulative quota limits across demands;
- returns a non-dominated vector of makespan, total delivery ticks, idle wait, water, propulsion energy, auxiliary energy, shore import and inland-transfer energy. Energy meter boundaries remain separate and are not added together;
- distinguishes `ExhaustiveComplete` from `BudgetExhausted`. A partial frontier is never described as proven optimal or infeasible.

An empty frontier is meaningful only when `ExhaustiveComplete` is returned: it means no joint plan exists within the declared finite discrete model. Invalid or stale evidence fails validation rather than being converted into infeasibility.

## Current qualification

The Rust unit-test corpus covers:
- overlapping lockage conflict, half-open handoff and capacity greater than one;
- capacity changes between declared windows;
- cumulative quota overrun, exact-boundary acceptance, independent accounting windows and a use interval that straddles two windows;
- two complete vessel demands sharing lock, berth, terminal, shore-energy and water resources;
- terminal-handling and inland-transfer throughput quota conflicts across sequential (non-overlapping) operations;
- joint infeasibility when shared water, shore-energy, terminal-handling throughput or inland-transfer throughput is individually feasible for each demand but not for both;
- conflicting shared-resource snapshots and invalid accounting modes;
- missing, duplicate and orphan resource policies;
- duplicate demand identity, mismatched decision cutoffs/tick resolutions and missing time basis;
- input permutation replay, zero-budget rejection and explicit search-budget exhaustion.

The current exact-head source declares **32 unit tests and 3 integration tests**. After the two terminal/inland throughput-quota controls were added, the exact-head workflow passed formatting and all tests on 7 November 2026: https://github.com/Luminous-Dynamics/symtropy/actions/runs/42341759255. The independent fleet enumerator and 20-variant sweep are part of the 32 unit-test corpus; the three integration tests cross-check event-sweep and cumulative quota arithmetic against direct references.

The test-only independent fleet enumerator separately explores each demand's alternatives and integer-tick placements, calculates feasibility using per-tick/per-window accounting, reconstructs the objective vector and computes its own Pareto frontier. It is cross-checked against the production frontier on four hand-checkable cases and a **20-variant frozen parameter sweep**. The sweep changes fast/slow alternative combinations, delivery windows, lock/channel/berth/terminal capacity, and cumulative water/shore-power quotas. Every production frontier must match the independently generated frontier exactly.

A separate integration-test oracle compares the production concurrent-capacity event sweep with a direct per-tick implementation and compares cumulative quota accounting with independent window sums over exhaustive tiny operation pairs.

## Research comparisons

See [`industry-baselines.md`](industry-baselines.md) for the current 2026 industry/research comparison, matched baseline matrix, qualification targets and explicit gaps. The solver-backed reference is tracked in [MARITIME-FLEET-MIP-001 #3833](https://github.com/Luminous-Dynamics/symtropy/issues/3833).

## Deliberate limits

This is an exact tiny-instance reference oracle, not a production optimizer. It requires all demands to be jointly served in a candidate plan; partial-service allocation, participant fairness policies, stochastic arrival/duration distributions, dynamic replanning, multi-chamber vessel packing, hydraulic transients, water-level/tide physics, sediment evolution, AIS/LPMS ingestion, resource reservation and live dispatch remain out of scope.

The implementation is a bounded exhaustive enumerator over its declared finite integer-tick search space. A second test-only implementation now cross-checks the complete fleet frontier on a small set of frozen hand-checkable workloads; this is useful differential evidence, not yet a comprehensive independent qualification corpus or a reproduced external MILP/CP-SAT solver. Expand the cross-check beyond the existing 20-variant two-demand sweep to three-, five- and ten-demand cases, then compare larger matched instances against the solver-backed HiGHS MILP reference tracked in #3833 and the reproduced serial-lock/multi-objective literature baselines.

The production search budget counts operation-start candidate evaluations, not every recursive call or memory allocation; future benchmark reports must preserve that exact definition.

The oracle assumes fixed segment durations, fixed capacity windows and one ordered route per demand. Its `total_idle_wait_ticks` metric only sums idle gaps between consecutive route operations for the same demand; it excludes anchorage/queue wait before the first operation, berth dwell, and end-to-end cargo dwell. Those must be added by the recovery and service-chain owners before comparing door-to-door logistics performance.

It does not establish that upstream evidence is authentic, that a vessel is physically qualified, that a resource operator has authorized use, or that real water/energy availability matches the synthetic profile. It never controls a lock, berth, pump, vessel or shore-power asset.

## Exact multi-vessel chamber packing primitive

The crate also exposes `chamber_packing::exact_chamber_packing`, a separate
geometric reference primitive for already-qualified vessel footprints. It assigns
each axis-aligned vessel rectangle to one of a set of rectangular chambers, using
integer millimetre coordinates and a conservative per-vessel/per-wall clearance.
Each hull rectangle is inflated on all four sides by the declared clearance;
inflated rectangles may touch but may not overlap. Rotation is not permitted.

The search explores every remaining-vessel ordering and stable corner placements
at chamber boundaries or edges of already-placed inflated rectangles. Its state
budget counts candidate (chamber, x, y) placements. A complete placement witness
is returned only on Feasible; no partial placements are exposed on Infeasible or
BudgetExhausted. The geometric abstraction is deliberately separate from lock
passage eligibility and does not establish safe clearance, mooring, hydrodynamics,
chamber flow or navigation approval.

An independent integer-grid integration oracle exhaustively compares the edge-
candidate search against every integer-millimetre vessel origin on frozen tiny
cases. The current suite includes more than 6,000 two-vessel chamber layouts, a
multi-chamber reference, and an exact full-chamber grid fill. The grid oracle is a
correctness reference for this discrete geometry model, not an operational
hydraulic or naval-architecture model.

## Industry and state-of-the-art comparison matrix

Treat the following as distinct comparator layers. None is a substitute for another:

| Comparator | What it already covers | What this programme must reproduce or add |
|---|---|---|
| Liao et al. (Ocean Engineering, 1 May 2026), FSR vs ESR | Lock queue fairness and efficiency; average waiting, chamber-area load factor and inversion rate | Reproduce both rules and report each metric by participant/service class; do not transfer the paper's site-specific arrival-rate threshold |
| Xu et al. (Systems, 3 May 2026), NSGA-II / NSGA-III / MOEA/D / SPEA-II | Multi-objective lock scheduling with waiting, utilization, energy, arrival rescheduling and berth-overcapacity objectives | Matched data, exact decision variables and tuning budgets; add shared water quota, multi-demand physical fit, distributional fairness and disruption recourse only as separately measured axes |
| Segovia et al. (Ocean Engineering, 15 Jan 2026), switching max-plus → MILP | Serial-lock routes/waypoints, timing, order, multi-chamber assignment and 2D vessel placement | Reconstruct the published case; compare feasibility, route/time decisions, chamber assignment, placements, waiting and solver effort. The new chamber module is only a discrete geometry primitive, not a reproduction of the full model |
| Ji et al. (EJOR, 16 Sep 2026), bi-objective MILP | Joint serial-lock schedule design and vessel sailing-speed optimization for emissions reduction | Reproduce the exact formulation and compare arrival/speed decisions, lock schedules, emissions boundary, waiting and decomposition runtime |
| DCSA Port Call Standard | Shared, standardized operational port-call data and event/API semantics across carriers, terminals and service providers | Consume owner-authoritative events and preserve ETA/ETB/ETD, planned vs actual, event issue/record/valid time and revision semantics; do not invent a competing event vocabulary |
| StormGeo + Portchain (acquisition closed 30 Oct 2026) | Commercial integration of voyage/speed intelligence with shared berth plans, terminal resources and carrier-terminal data exchange | Do not claim JIT or berth alignment as novel. A fair comparison must include voyage decision, berth readiness, cranes/workforce, disruption updates, and terminal data access on matched inputs |
| World Bank / S&P Global CPPI 2025 | Global port-level benchmark using actual container-vessel time in port, including waiting and berth time | Reconcile the port-call boundary to the chain model and report the CPPI-compatible measure separately from lock delay, inland transfer, and door-to-door freight service |

Official and primary references:

- Liao et al., FSR/ESR lock scheduling: https://doi.org/10.1016/j.oceaneng.2026.125004
- Xu et al., multi-objective ship-lock optimization: https://doi.org/10.3390/systems14050507
- Segovia et al., switching max-plus multi-chamber placement: https://doi.org/10.1016/j.oceaneng.2025.123414
- Ji et al., joint serial-lock scheduling and speed optimization: https://doi.org/10.1016/j.ejor.2026.01.038
- DCSA Port Call Standard: https://dcsa.org/standards/port-call
- StormGeo acquisition announcement (30 Oct 2026): https://stormgeo.com/insights/stormgeo-acquires-portchain-to-advance-just-in-time-voyage-optimization-at-scale
- World Bank CPPI 2025 and methodology: https://www.worldbank.org/en/topic/transport/publication/cppi

StormGeo/Portchain's release describes a network of more than 200 terminals and over 40% of global container-terminal capacity. Treat this as the company's reported network coverage, not independently audited market share. Portchain's own site also advertises 6–14% bunker/emissions savings; treat that as a vendor claim until it is independently validated on matched cases.

### What would constitute a credible improvement

1. **Reproduction first:** exact or solver-proven agreement on the papers' tiny instances, with source/constraint parity and no silent omitted assumptions.
2. **Matched baselines:** FCFS, the published FSR/ESR definitions, NSGA-II/III, MOEA/D, SPEA-II, a reproduced MILP/max-plus reference, and operational JIT/berth-alignment baselines where data and semantics are available.
3. **System-level service:** retain lock delay, chamber occupancy, port time, inland transfer and door-to-door elapsed time as separate stages. Report delivered/eligible service, unserved demand and p50/p95/max delay by cohort.
4. **Resource integrity:** zero physical placement, time-capacity or cumulative quota violations; separate water, propulsion, auxiliary, shore-import and onward-transfer meter boundaries.
5. **Resilience and fairness:** compare recovery time, missed commitments, plan churn, contingency-resource use, displacement and worst-served cohort outcomes under identical disruptions.
6. **Statistical discipline:** paired scenarios and seeds, predeclared objectives/reference points, held-out scenario families and confidence intervals. Do not tune and score on the same event set.
7. **Claim discipline:** beating one weak heuristic is not beating SOTA; improving one scalar objective while harming water, fairness or service is not system superiority.

Current status: the fleet oracle has a synthetic exact-head green record; the chamber-packing slice is still under qualification. There are no reproduced industry/academic results, source-cleared operational data, measured service improvements, or better-than-SOTA claims yet.