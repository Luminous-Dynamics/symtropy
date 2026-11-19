# habitat-scheduling-oracle

A small, exhaustive Rust reference oracle for tiny synthetic resource-constrained scheduling cases. It is deliberately not a production scheduling optimizer. Its job is to freeze semantics and provide an independent comparator for a future scheduler.

The oracle enumerates integer start times for nonpreemptive tasks, applies precedence edges, half-open activity intervals, resource capacities, task windows, hard gates, and an optional hard deadline, and computes both nominal and upper-duration makespans. For equal makespans it retains the first schedule in lexicographic start-time search order, making the witness deterministic.

From the repository root:

- cargo fmt --manifest-path tools/habitat-scheduling-oracle/Cargo.toml -- --check
- cargo test --locked --manifest-path tools/habitat-scheduling-oracle/Cargo.toml
- cargo run --locked --manifest-path tools/habitat-scheduling-oracle/Cargo.toml -- docs/research/fixtures/HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json

The fixture is limited to at most 12 tasks per case and bounded integer-time horizons. The engine conservatively caps the Cartesian product of candidate start-time domains at 2,000,000 assignments and returns an explicit oracle-budget rejection instead of attempting an unbounded exact search. It includes distinct/shared crews, precedence chains, feasible/infeasible inspection windows, unavailable approval gates, over-capacity demand, uncertainty against a deadline, and cycle rejection.

All data are synthetic. A passing micro-oracle does not imply a production solver, real schedule quality, construction estimates, permit compliance, or safety certification. The external benchmark plan is documented in docs/research/HABITAT_REVALIDATION_SCHEDULING_ORACLE_V0_1.md.
