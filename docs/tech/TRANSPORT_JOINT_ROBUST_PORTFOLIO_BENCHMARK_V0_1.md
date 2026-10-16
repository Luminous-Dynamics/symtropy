# Joint Robust Route Portfolios with Shared Physical Capacity v0.1

**Status:** implementation and synthetic Rust tests authored; compilation and execution remain unverified.
**Implementation:** `crates/domains/symtropy-transport-optimization/src/robust_portfolio.rs`
**Dependency chain:** [PR #1718 — per-route resource feasibility](https://github.com/Luminous-Dynamics/symtropy/pull/1718), [PR #1714 — paired scenario minimax routing](https://github.com/Luminous-Dynamics/symtropy/pull/1714), [PR #1701 — evidence-bounded FIFO arrival functions](https://github.com/Luminous-Dynamics/symtropy/pull/1701).

## 1. Paradigm shift: optimize the portfolio, not isolated routes

Two routes can each be capacity-feasible on their own and still be mutually impossible. If two independent shipments each choose the same single-capacity bridge, berth, rail segment, charging station, crane, or terminal resource at overlapping times, evaluating the paths separately misses a network-wide collision.

This slice takes the individually qualified route options for several demands, enumerates one path per demand, checks the selected combination against shared resource snapshots in every paired scenario, and optimizes only among jointly feasible combinations. It separates three claims:

1. **Travel-qualified:** the path can be evaluated under the declared time-evidence and scenario contract.
2. **Individually capacity-qualified:** that path's mapped resources can fit its operation intervals against the supplied capacity evidence.
3. **Jointly feasible:** all selected demand paths can coexist, without double-spending shared physical capacity, in every declared scenario.

A capacity snapshot is keyed by explicit resource kind/ID, evidence ID, provenance, revision, validity, units and capacity limit. Two selected demands may use different records for the same resource only when those records have disjoint validity intervals and each operation window is covered by its own record. Distinct records with overlapping validity intervals—or the same evidence ID with conflicting contents—make the combination invalid rather than allowing the solver to choose the convenient limit.

## 2. Exact small-instance selection contract

Inputs are the per-demand reports from the capacity-qualified route selector. All demands must share the same scenario-set ID and manifest digest, time profile, evaluation instant, and capacity policy. Every demand must have at least one individually qualified option; the caller must provide a positive, explicit objective weight.

For every cartesian combination of one option per demand, the selector:
- checks the combined half-open operation intervals `[start, end)` against each physical resource's shared limit;
- treats end events as occurring before start events at the same timestamp, so adjacent uses do not collide;
- rejects different capacity-evidence fingerprints for one physical resource when their validity intervals overlap; adjacent time-disjoint snapshots may serve sequential jobs;
- scores only combinations that are feasible under *every* declared scenario; and
- retains aggregate rejection counts and up to 32 deterministic representative failure witnesses.

The objective is explicit and lexicographic:
1. minimize the slowest single-demand elapsed time across all scenarios;
2. minimize the worst-scenario objective-weighted aggregate elapsed score;
3. minimize the nominal objective-weighted aggregate elapsed score; and
4. use stable demand and path IDs for deterministic tie-breaking.

Objective weights are caller policy, not inferred physical units. This objective is not a complete social-welfare function and does not guarantee equity for all groups. Later work should expose Pareto frontiers and explicit service floors or fairness policies instead of hiding them in a scalar. The returned report is exact only over supplied per-demand options and the declared scenario set.

A mandatory maximum combination count prevents pretending that exhaustive enumeration scales indefinitely. Exceeding the limit is an explicit error; the function does not fall back to a heuristic and mislabel it exact.

## 3. Synthetic contract tests authored

The Rust suite builds paired nominal/stress scenarios and uses explicit synthetic resource IDs and capacity evidence. Tests cover:
- avoiding a shared single-capacity bottleneck by choosing a feasible alternate for one demand;
- fail-closed behavior when the only combination overbooks a shared resource;
- adjacent, non-overlapping intervals under half-open semantics;
- invariance under demand input order;
- and refusal to enumerate a cartesian product larger than the declared combination budget.

These are designed contract cases, not measured performance. No tests or compilation are claimed as passed.

## 4. Benchmark ladder and targets

| Gate | Benchmark | Target |
|---|---|---|
| PF0 — finite portfolio oracle | Exhaustively enumerate tiny portfolios and compare selected choices/objectives with a separate reference implementation | 100% agreement for every feasible combination in the test domain |
| PF1 — resource capacity | Two or more demands share a resource with capacity (C); sweep assignment units and overlap boundaries | Zero over-capacity portfolios accepted |
| PF2 — half-open interval edges | End/start equal timestamps, one-second overlap, nested intervals and zero-width/reversed windows | Adjacent intervals never conflict; every positive overlap is accounted for |
| PF3 — evidence consistency | Mutate digest, evidence ID, revision, validity, source record, capacity limit or unit between overlapping validity intervals | Zero inconsistent snapshots accepted; disjoint validity windows may serve sequential jobs |
| PF4 — scenario completeness | Make a portfolio feasible nominally but infeasible in one stress/replay scenario | Zero partial-scenario portfolios selected |
| PF5 — determinism | Permute demand, route-option, scenario and evidence-record order | Identical choices, objective values, rejection counts and ordered witnesses |
| PF6 — arithmetic and complexity | Test (u64) addition/multiplication bounds and combinations just under/over the configured cap | Overflow always fails closed; no unbounded implicit search |
| PF7 — solver scale | Measure combinations/s, evaluation calls, p50/p95/max latency and memory on pinned hardware while varying demands/options/scenarios/resources | Publish a measured baseline before setting a performance target; preserve exactness label only for exhaustive mode |
| PF8 — replay validity | Use hash-pinned, rights-qualified time-dependent instances plus explicit synthetic/observed resource constraints; hold out disruption events before tuning | Report each event and every failed case; no claims of future reliability from hand-designed stress cases alone |

Required negative controls: identical shared resource but inconsistent capacity digests; two separate logical edges mapped to one physical resource; overlapping capacity limit 1 assignments; adjacent half-open windows; one-second overlap; future/expired evidence; units mismatch; no individually qualified route; no jointly feasible portfolio; and a search budget exceeded by exactly one combination. The suite must also include feasible cases so a trivial always-rejecting system cannot pass.

## 5. External benchmark context

- The [MAMUT-routing benchmark catalogue](https://github.com/ANR-MAMUT/MAMUT-routing) and its [benchmark documentation](https://mamut-routing.univ-ubs.fr/docs/benchmarks/) cover CVRP/VRPTW and time-dependent VRP/VRPTW with explicit objective contracts and checker-validated best-known solutions. The catalogue snapshot inspected for this work is dated **September 23, 2026** and reports 5,048 instances across 19 families. This makes it a useful travel-time baseline, not a complete shared physical-resource benchmark. Pin a version and verify per-family rights before using its files: the source code license is not a blanket data license.
- Poryos2026 includes paired time-dependent traffic overlays on identical road instances, which is useful for paired baseline comparisons; its vehicle capacity and time windows are not equivalent to a cross-demand terminal/track/berth capacity model. See the [family README](https://github.com/ANR-MAMUT/MAMUT-routing-Poryos2026).
- Figliozzi (2012), [The time dependent vehicle routing problem with time windows: benchmark problems, an efficient solution algorithm, and solution characteristics](https://doi.org/10.1016/j.tre.2011.11.006), describes replicable time-dependent benchmark instances and performance/perturbation measures.
- The 2020 paper [Exact approaches to the robust vehicle routing problem with time windows and multiple deliverymen](https://doi.org/10.1016/j.cor.2020.105062) addresses robust routing under uncertain demand, travel and service times. It is a useful comparison point for later exact algorithms, although the current implementation is only an exhaustive selection layer over supplied route options—not a reproduction of branch-and-cut or branch-price-and-cut.

## 6. What would make the field move

The deeper opportunity is to evolve from “route optimization” to **evidence-backed transport coordination**: every candidate route carries temporal, provenance and resource assumptions; every multi-demand decision is checked for portfolio feasibility; and outcome claims remain bounded by held-out evidence. A longer-term solver can then combine:
- adaptive robust optimization for travel time, energy and disruption sets;
- explicit queue/waiting and schedule semantics rather than converting infeasibility into invented delay;
- shared, versioned resource registries with independently authenticated crosswalks;
- incremental replanning when new evidence arrives, with change-impact receipts;
- multi-objective reporting across service time, operating cost, energy, emissions and declared fairness constraints; and
- independent exhaustive oracles on small graphs before any heuristic is evaluated on larger problems.

## 7. Limits and next steps

This implementation only checks the selected set of individually qualified route options against the capacity receipts already supplied for each option and scenario. It does not account for unprovided pre-existing bookings, infer resource identity, authenticate evidence issuers, fetch/verify crosswalk bytes, model capacity-induced queues, solve a global vehicle-routing problem, or establish operational benefit.

Next gates: an independent exhaustive interval-load oracle; an explicit test that inconsistent cross-demand evidence is rejected; a bounded property-based tiny-instance generator; then a rights-qualified benchmark adapter with source manifests and checker outputs. Only after these gates should Symtropy compare exact small portfolio solutions with heuristics at larger scale.
