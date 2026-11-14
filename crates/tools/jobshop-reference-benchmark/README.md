# Job-shop reference benchmark

This offline, dependency-free Rust crate starts the INDUSTRIAL-SCHED-REF-001
benchmark track. It pins two classic job-shop (JSP) instances, validates their
input format, computes workload lower bounds, and provides a bounded exhaustive
makespan oracle for tiny cases. The oracle emits operation-level start/finish records
and audits precedence, machine exclusivity, assignment, duration, and makespan. Three
deterministic dispatch baselines (shortest processing time, most work remaining, and
most operations remaining) generate audited feasible schedules for the larger pinned fixtures.

## Run

cargo test --locked --offline --manifest-path crates/tools/jobshop-reference-benchmark/Cargo.toml

cargo run --locked --offline --manifest-path crates/tools/jobshop-reference-benchmark/Cargo.toml -- summarize crates/tools/jobshop-reference-benchmark/fixtures/ta01.txt

cargo run --locked --offline --manifest-path crates/tools/jobshop-reference-benchmark/Cargo.toml -- solve-exact crates/tools/jobshop-reference-benchmark/fixtures/tiny_cross_2x2.txt
cargo run --locked --offline --manifest-path crates/tools/jobshop-reference-benchmark/Cargo.toml -- dispatch-mwkr crates/tools/jobshop-reference-benchmark/fixtures/ta01.txt

The exact solver has hard limits (12 operations and 2,000,000 expanded nodes).
Larger inputs are parsed and summarized but are not solved by this oracle. A
node-budget exit is an explicit lack of an optimality result, not a feasible
schedule or a benchmark score.

## Deliberate claim ceiling

- This is a classic fixed-route JSP only. It does not parse FJSP alternative
  machines, setup times, releases, due dates, calendars, failures, or online jobs.
- The exact oracle is a small-instance reference, not a production scheduler.
- Dispatch baselines construct feasible schedules but are not optimal solvers. Their
  output is marked `optimality=not_proven`; `lower_bound_gap` is makespan minus a
  workload lower bound, not an optimality gap.
- Workload lower bounds are not schedules and are not presented as best-known
  solutions for external instances.
- No plant execution, throughput gain, energy saving, or economic value is
  measured by these fixtures.

Fixture sources, upstream commit IDs, blob hashes, and licensing are in
fixtures/SOURCES.md.
