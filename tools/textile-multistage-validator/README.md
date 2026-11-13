# Textile Multi-Stage Schedule Validator v0.1

A dependency-free Rust reference validator for proposed multi-stage garment
production schedules. It is a bounded first executable step for Symtropy issue
[#4875](https://github.com/Luminous-Dynamics/symtropy/issues/4875).

## Why a separate verifier comes first

Schedulers must not be trusted to certify their own output. Before comparing
MILP, constraint-programming, dispatch, genetic, evolutionary or learned
policies, the benchmark needs one independent feasibility judge with frozen
semantics. This crate is intentionally separate from any optimizer.

## v0.1 scope

The validator checks:

- exact one-to-one coverage of required operations;
- positive, exact operation durations and half-open time intervals;
- release and hard due-time bounds;
- declared machine capabilities;
- operator skill and certification requirements;
- machine and operator availability windows;
- machine and operator non-overlap;
- explicit sequence-dependent machine setup transitions, including explicit
  zero-duration rules;
- separate setup reservations that bind a positive-duration machine transition
  to a qualified operator, exact time interval and machine availability;
- setup-operator skill/certification, availability and non-overlap with both
  production operations and other setup reservations;
- precedence and minimum-lag constraints;
- deterministic, sorted findings for reproducible comparison.

All time values are integer ticks in one caller-declared common unit. The
reference does not infer minutes, shifts, machine cycle times or setup rules.

## Run locally

~~~sh
cargo test --manifest-path tools/textile-multistage-validator/Cargo.toml
cargo fmt --manifest-path tools/textile-multistage-validator/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/textile-multistage-validator/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/textile-multistage-validator/Cargo.toml
~~~

The known-answer case contains two jobs routed through cutting, sewing,
finishing and inspection. Unit tests mutate the valid case to verify that
malformed or infeasible plans are rejected.

## Deliberate non-goals

This crate does not:

- find or optimize a schedule;
- implement MILP, CP-SAT, genetic algorithms, reinforcement learning or
  dispatch policies;
- model lot streaming, batch splitting, rework or parallel subassemblies;
- model electricity, steam, water, wastewater, peak demand or emissions;
- establish ergonomic exposure or worker wellbeing;
- command machines or act as an ERP/MES/plant controller;
- prove that a schedule was executed or that output passed inspection.

The bounded exhaustive oracle in `src/oracle.rs` independently searches tiny
integer-time instances for the minimum makespan. It does not call the validator
while searching and returns no result when its node budget is exhausted. Its
v0.1 scope is deliberately limited to explicit zero-duration setup transitions;
nonzero setup models are rejected as unsupported. It is an oracle for small
instances, not a production scheduler.

The validator can now check explicit setup-labor reservations, but its bounded
exact oracle deliberately continues to reject nonzero setup durations. It does
not yet search for setup operators or setup intervals. The next benchmark
increment should add time-indexed electricity/steam/water envelopes and then
robust disruption/replanning scenarios. Observed production receipts and
worker-approved constraints remain independent evidence gates.

## Claim ceiling

A passing report means only that the supplied schedule satisfies this
version's declared synthetic constraint model. It is not evidence of factory
productivity, energy/water savings, safe working conditions, product quality,
industrial feasibility or physical execution.


## Route eligibility gate v0.2

The `route_gate` module binds every production job to exactly one upstream
`RouteRequirement` and a matching `RouteReceipt`. A scheduled textile job is
not eligible merely because a schedule can be made feasible.

The gate requires exact matching of:
- physical lot ID and incoming mass;
- route ID and versioned receiving specification;
- receipt ID and exact receipt SHA-256;
- evidence-manifest SHA-256;
- upstream contract version and distinct issuer/verifier IDs.

Only `Eligible` route receipts pass. `Conditional`, `InsufficientEvidence`
and `Ineligible` all block scheduling. `validate_with_route_gates` combines
the ordinary machine/operator/precedence report with route precondition findings.

This layer checks structural identity and digest-string equality. It does not
recompute receipt SHA-256 from artifact bytes, authenticate the issuer/verifier,
or verify physical evidence. The upstream verifier must verify the exact receipt
bytes before a route is supplied here. A mismatch against the pinned digest
makes a changed/stale receipt visible to the schedule gate.

The 18 route-gate unit tests cover eligible multi-lot binding, missing and
non-eligible receipts, digest/lot/route/spec/mass mismatch, duplicate
requirements/receipts, unknown jobs, malformed digests, independent verifier
identity and combined schedule validation.
