# Habitat Scheduling Micro-Oracle v0.1

**Status:** frozen synthetic test contract; no scheduler implementation or test pass is claimed here.
**Related implementation issue:** [HABITAT-REVALIDATION-006 #1902](https://github.com/Luminous-Dynamics/symtropy/issues/1902)
**Machine-readable cases:** [HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json](fixtures/HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json)

## Paradigm shift

The existing revalidation portfolio planner assigns additive cost and duration units to repair-task sequences. That model is useful for exploring shared evidence repair and Pareto alternatives, but it cannot represent overlapping work, exclusive resources, hard calendars, or duration uncertainty. Do not reinterpret its sum of task durations as construction makespan.

A scheduler must produce a timeline that can be independently validated against the same frozen constraints. Candidate quality and schedule feasibility are different claims: an attractive cost/time score never excuses violating a precedence edge, crew capacity, inspection window, hard gate, or disclosed uncertainty bound.

## Formal conventions

- Time starts at zero and is integer-valued in abstract units.
- Activities are nonpreemptive and occupy half-open intervals `[start, finish)`.
- Precedence means `finish(before) <= start(after)`.
- Makespan is the maximum finish time of all required activities.
- Resource capacities are renewable. Demands exceeding a resource's capacity are impossible before search begins.
- Required gates are hard constraints, not soft costs or optional objective weights.
- Robust feasibility checks the upper-duration scenario against the same hard constraints. Nominal feasibility does not imply robust feasibility.

## Nine exact reference cases

The fixture covers parallel work with distinct crews, parallel work competing for one exclusive crew, a precedence chain, a feasible inspection window, an impossible inspection window, a missing hard approval gate, over-capacity demand, upper-duration uncertainty against a deadline, and a precedence cycle.

The executable Rust reference uses exhaustive integer start-time enumeration, bounded to 12 tasks and a Cartesian candidate-start space no larger than 2,000,000 assignments. Larger instances fail with an explicit oracle-budget rejection rather than an unbounded search. This is intentionally a micro-oracle, not the large-instance solver. The following hand-calculated witnesses provide the first independent oracle for a future Rust scheduler:

| Case | Expected result | Purpose |
|---|---|---|
| Separate crews, durations 4 and 6 | Makespan 6 | Independent tasks should overlap |
| One shared exclusive crew, durations 4 and 6 | Makespan 10 | Resource capacity constrains overlap |
| Chain 2 → 3 → 5 | Makespan 10 | Precedence is hard |
| Build 4 then inspect 1 in window [5, 6] | Makespan 6 | Hard window creates an idle gap |
| Build 4 then inspect 1, start ≤ 3 and finish ≤ 4 | Infeasible | Window conflicts with prerequisite |
| Required safety review unavailable | Infeasible | Hard gate cannot be optimized away |
| Demand 2 from capacity 1 | Infeasible | Resource violation is rejected |
| Nominal durations 4 and 3, upper durations 8 and 7, deadline 6 | Nominal makespan 4; upper scenario 8; not robust | Preserve uncertainty |
| Two-node precedence cycle | Infeasible | Cycles cannot yield a schedule |

Every exact-instance solver should agree with the fixture's expected feasibility and objective. Every emitted schedule should pass a separately implemented validator that recomputes finish times, precedence, windows, resource occupancy, gate satisfaction, and makespan from the materialized schedule—not from the solver's summary fields.

## Beyond synthetic cases: public scheduling benchmarks

The v0.1 micro-oracle is deliberately tiny, so it cannot establish competitiveness. The next external comparison should add an adapter for a well-known resource-constrained project scheduling problem (RCPSP) corpus, with a source-specific provenance manifest and a strict supported subset.

- [PSPLIB (Technical University of Munich)](https://www.om-db.wi.tum.de/psplib/) publishes single-mode, multi-mode, and resource-investment benchmark instances and solution information for evaluating scheduling procedures. Its information page points to the classic Kolisch and Sprecher 1996 library paper.
- The [Ghent University Operations Research & Scheduling project-data library](https://www.projectmanagement.ugent.be/research/data) lists varied instance families for large projects, multi-mode tasks, alternative subgraphs, multi-skilled resources, multi-project portfolios, and an empirical dynamic-scheduling collection with 180+ real project instances described on the page.

These are adjacent evidence for scheduling quality, not a substitute for the novel habitat claims/evidence problem. Do not assume a web-accessible dataset grants blanket redistribution rights. Before vendoring or mirror-hosting any source corpus, inspect its specific terms, preserve attribution, record original instance identifiers and transformations, and pin exact downloaded bytes by SHA-256. A parser must not quietly alter durations, capacities, resource semantics, or instance versions.

A future external-benchmark report should distinguish:
1. **Feasibility:** independent schedule-validator pass rate and resource/precedence/window violations.
2. **Optimality:** objective gap against a published optimum where available; otherwise gap against an explicitly labelled best-known bound/solution.
3. **Runtime:** p50/p95 and memory by dataset family on a disclosed hardware/toolchain.
4. **Robustness:** outcomes under explicitly defined duration/resource scenarios.
5. **Provenance:** source URL, source/version date, instance IDs, exact SHA-256, parser version, transformation receipt, data terms, and whether results are exact or heuristic.

Do not claim all-instance exact optimality where the selected algorithms or known bounds do not support it. Keep the synthetic habitat evidence-graph corpus and standard RCPSP cases as two separate suites and report them separately.

## Release gates

- 100% agreement with the nine micro-oracle cases.
- 100% independent feasibility validation for any returned schedule.
- Zero gate, precedence, window, or capacity violations.
- Zero robust-feasibility claims when an upper-duration scenario violates a hard deadline.
- Zero optimal/infeasible statuses after search budget exhaustion.
- Deterministic outputs and pinned instance bytes for every regression case.
- No performance claim until raw trials, exact workload digests, environment metadata, and incomplete-search counts are published.

The intended novelty is not simply a faster schedule optimizer. It is a connected decision system where a proposed timeline, its assumptions, gates, relevant evidence, uncertainty, and later evidence invalidation remain auditable as one chain—while established scheduling datasets ensure it does not benchmark only against its own invented examples.

## Heuristic scheduling baseline

A separate experimental implementation now lives in [tools/habitat-scheduling-planner](https://github.com/Luminous-Dynamics/symtropy/tree/research/whole-property-habitat-benchmark-v0-1/tools/habitat-scheduling-planner). It tries five deterministic topological list-scheduling priorities and independently revalidates every emitted candidate. It reports `HEURISTIC_FEASIBLE_NOT_PROVEN_OPTIMAL` when it has a valid schedule, `PROVEN_INFEASIBLE_PRECHECK` only for direct hard-constraint/lower-bound contradictions, and `NO_CANDIDATE_FOUND_NOT_PROOF_OF_INFEASIBILITY` if the tried heuristic rules found no schedule.

It evaluates the declared upper-duration scenario separately from nominal durations, and separates a candidate robust schedule from a proven hard-constraint infeasibility. It never promotes a heuristic candidate to a global optimum. A negative-control unit test corrupts a valid schedule so two tasks overlap on one exclusive crew; the independent validator must reject it.

This is a first comparison point for the exact micro-oracle, not an optimal solver and not yet a large-instance benchmark. The oracle, heuristic scheduler, and any future exact/anytime optimizer should remain separately implemented enough that candidate schedules are checked by a validator that does not rely on the candidate generator's own occupancy calculations.
