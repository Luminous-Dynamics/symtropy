# habitat-scheduling-benchmark

A reproducible, ten-case scorecard that runs two separate implementations over the same frozen synthetic scheduling fixture:
- the bounded exhaustive micro-oracle, which supplies exact reference outcomes only for tiny instances;
- the deterministic five-priority heuristic planner, which emits independently validated candidates but never claims global optimality.

The report separates feasibility coverage from conditional solution quality. For a reference-feasible case with a returned candidate, it reports absolute and relative makespan gap against the exact optimum. For a feasible case where the heuristic finds no schedule, it counts a **missed candidate** rather than recording infeasibility. Any heuristic candidate contradicting the exact oracle is counted as a reference conflict. It pins the original input digest and the canonical digests of both reports.

From the repository root:

- cargo fmt --manifest-path tools/habitat-scheduling-benchmark/Cargo.toml -- --check
- cargo test --locked --manifest-path tools/habitat-scheduling-benchmark/Cargo.toml
- cargo run --locked --manifest-path tools/habitat-scheduling-benchmark/Cargo.toml -- docs/research/fixtures/HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json

The micro suite is a functional quality comparison, not a performance benchmark. The current hand-checked counterexample demonstrates why the metrics are separated: exact search finds a feasible schedule with makespan 19, but all five heuristic priority rules fail to construct one. The correct output is "no candidate found; not proof of infeasibility." The scorecard's small-sample coverage/gap values are not claims about general RCPSP performance.

All costs, durations, resources, and gates are synthetic. No reported schedule means work was executed; no scorecard establishes safety, permits, code compliance, or a construction estimate.
