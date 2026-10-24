# Habitat Scheduling Micro-Oracle v0.1

**Status:** frozen synthetic test contract plus experimental Rust oracle/planner implementations; no Rust compilation or test pass is claimed here.
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

## Ten exact reference cases

The fixture covers parallel work with distinct crews, parallel work competing for one exclusive crew, a precedence chain, a feasible inspection window, an impossible inspection window, a missing hard approval gate, over-capacity demand, upper-duration uncertainty against a deadline, a precedence cycle, and a deliberately adversarial case where five heuristic priority rules miss a feasible schedule.

The executable Rust reference uses exhaustive integer start-time enumeration, bounded to 12 tasks and a Cartesian candidate-start space no larger than 2,000,000 assignments. Larger instances fail with an explicit oracle-budget rejection rather than an unbounded search. This is intentionally a micro-oracle, not the large-instance solver. The tenth case is a key adversarial check: exact search finds a feasible schedule of makespan 19, while the five fast priority rules fail to find a candidate. A bounded topological-order fallback then recovers a validated candidate after 326 expanded states; the priority-only miss remains visible in the report and is not treated as proof of infeasibility. The following hand-calculated witnesses anchor the executable micro-oracle and provide independent tests for future exact/anytime schedulers:

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
| Five exclusive-crew tasks with tight start windows | Exact makespan 19; five heuristic rules find no candidate | Separate valid-feasibility evidence from heuristic search coverage |

Every exact-instance solver should agree with the fixture's expected feasibility and objective. Every emitted schedule should pass a separately implemented validator that recomputes finish times, precedence, windows, resource occupancy, gate satisfaction, and makespan from the materialized schedule—not from the solver's summary fields.

## Beyond synthetic cases: public scheduling benchmarks

The v0.1 micro-oracle is deliberately tiny, so it cannot establish competitiveness. The next external comparison should add an adapter for a well-known resource-constrained project scheduling problem (RCPSP) corpus, with a source-specific provenance manifest and a strict supported subset.

- [PSPLIB (Technical University of Munich)](https://www.om-db.wi.tum.de/psplib/) publishes single-mode, multi-mode, and resource-investment benchmark instances and solution information for evaluating scheduling procedures. Its information page points to the classic Kolisch and Sprecher 1996 library paper.
- The [Ghent University Operations Research & Scheduling project-data library](https://www.projectmanagement.ugent.be/research/data) advertises more than 30,000 artificial instances (with generators able to create nearly four million project instances), an empirical collection with 180+ real project instances, and portfolio datasets totaling nearly 40,000 cases. Its instance families include multi-mode tasks, hard CV instances for exact procedures, alternative subgraphs, multi-skilled resources, multiple projects, and risk/portfolio analyses.

These are adjacent evidence for scheduling quality, not a substitute for the novel habitat claims/evidence problem. Do not assume a web-accessible dataset grants blanket redistribution rights. Before vendoring or mirror-hosting any source corpus, inspect its specific terms, preserve attribution, record original instance identifiers and transformations, and pin exact downloaded bytes by SHA-256. A parser must not quietly alter durations, capacities, resource semantics, or instance versions.

A future external-benchmark report should distinguish:
1. **Feasibility:** independent schedule-validator pass rate and resource/precedence/window violations.
2. **Optimality:** objective gap against a published optimum where available; otherwise gap against an explicitly labelled best-known bound/solution.
3. **Runtime:** p50/p95 and memory by dataset family on a disclosed hardware/toolchain.
4. **Robustness:** outcomes under explicitly defined duration/resource scenarios.
5. **Provenance:** source URL, source/version date, instance IDs, exact SHA-256, parser version, transformation receipt, data terms, and whether results are exact or heuristic.

Do not claim all-instance exact optimality where the selected algorithms or known bounds do not support it. Keep the synthetic habitat evidence-graph corpus and standard RCPSP cases as two separate suites and report them separately.

## Release gates

- 100% agreement with all ten micro-oracle cases.
- 100% independent feasibility validation for any returned schedule.
- Zero gate, precedence, window, or capacity violations.
- Zero robust-feasibility claims when an upper-duration scenario violates a hard deadline.
- Zero optimal/infeasible statuses after search budget exhaustion.
- Deterministic outputs and pinned instance bytes for every regression case.
- No performance claim until raw trials, exact workload digests, environment metadata, and incomplete-search counts are published.

The intended novelty is not simply a faster schedule optimizer. It is a connected decision system where a proposed timeline, its assumptions, gates, relevant evidence, uncertainty, and later evidence invalidation remain auditable as one chain—while established scheduling datasets ensure it does not benchmark only against its own invented examples.

## Exact-vs-heuristic scorecard

The [standalone scorecard tool](https://github.com/Luminous-Dynamics/symtropy/tree/research/whole-property-habitat-benchmark-v0-1/tools/habitat-scheduling-benchmark) runs the exhaustive oracle and the hybrid planner separately on the same ten-case fixture. It compares priority-only coverage with recovery-enhanced coverage, counts candidates recovered by bounded topological-order fallback, tracks fallback states, reports false infeasibility claims and conflicts, and measures objective gaps only for independently validated candidates. The frozen synthetic expectations are: 6 oracle-feasible cases; 5/6 coverage from the fast priority rules; one missed case recovered by a 326-state fallback; 6/6 coverage after fallback; zero false-infeasibility claims; zero oracle conflicts; and 0 ppm conditional relative makespan gap among six returned candidates. No global-optimality claim is made. These are fixture outcomes, not generalized performance claims.

The [scorecard workflow](https://github.com/Luminous-Dynamics/symtropy/blob/research/whole-property-habitat-benchmark-v0-1/.github/workflows/habitat-scheduling-benchmark.yml) checks the exact-vs-hybrid comparison. This is a functional quality gate, not a runtime benchmark; it preserves the fast heuristic's miss as a measured limitation while validating that the bounded recovery phase finds a usable schedule.

## Heuristic scheduling baseline

The experimental [hybrid scheduler](https://github.com/Luminous-Dynamics/symtropy/tree/research/whole-property-habitat-benchmark-v0-1/tools/habitat-scheduling-planner) tries five deterministic topological list-scheduling priorities first, then enumerates prerequisite-valid task orders with a 20,000-expanded-state budget if no priority rule finds a candidate. Every emitted schedule is checked by an independent validator. Priority-only output is preserved separately, so a later recovery does not hide the initial heuristic miss. Fallback candidates use `BOUNDED_FALLBACK_FEASIBLE_NOT_PROVEN_OPTIMAL`; if the fallback budget expires without a candidate, it reports an incomplete status instead of claiming infeasibility. Proven-infeasible prechecks are limited to direct hard constraints or valid lower bounds.

It evaluates the declared upper-duration scenario separately from nominal durations and never promotes a fallback candidate to a global optimum. A negative-control unit test corrupts a valid schedule so two tasks overlap on one exclusive crew; the independent validator must reject it. The tenth micro-case now verifies the recovery path recovers a makespan-19 candidate after 326 search states, while the scorecard still records that the five priority rules alone missed it.

This is an initial hybrid baseline against the exact micro-oracle, not a general optimal solver or large-instance result. The reference oracle, priority rules, fallback traversal, and schedule validator should remain distinct enough that feasibility is checked independently of candidate generation.
