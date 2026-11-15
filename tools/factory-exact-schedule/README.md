# Exact Multi-Order Factory Scheduling Oracle v0.1

**Owner:** Symtropy FACTORY-EXACT-SCHEDULE-001 #5446  
**Related:** FACTORY-MATCHED-ROUTES-001 #5432 / draft PR #5434  
**Status:** synthetic fixed-route job-shop oracle; no production scheduling claim.

## Model

The bounded instance contains multiple jobs, ordered operations, fixed machine assignments, job release times, unary machine capacity, processing times, and explicit sequence-dependent setup transitions. The exact solver enumerates every permutation of the operations assigned to each machine. For each combination it composes job-precedence and machine-order constraints into a directed graph; cyclic combinations are infeasible. Acyclic combinations are evaluated by longest-path earliest-start propagation. The smallest makespan across the complete enumeration is the exact optimum for this fixed-route non-preemptive job-shop model.

The exact solver has an explicit maximum of nine operations. Unsupported/oversize input returns INVALID_INPUT and cannot claim optimality. The solver does not silently truncate a search.

The SPT comparator is a deterministic serial dispatch baseline over the next ready operation of each job, selecting the shortest processing time with stable ID tie-breaking. Setup is included in machine availability. The baseline is a simple heuristic reference, not an asserted industry SOTA.

## Frozen reference fixture

The reference has three jobs and three machines, nine operations total, and zero sequence-dependent setup. The complete search enumerates:

- 216 machine-order combinations;
- 128 cyclic/infeasible combinations;
- 88 acyclic/feasible combinations.

The exact makespan is 26 synthetic ticks. The SPT baseline makespan is 34 synthetic ticks, an eight-tick gap (307,692 parts per million relative to the exact optimum). This is a small illustrative fixture, not a general performance result.

## Verification

Schedule certificates include machine orders and per-operation setup/processing intervals. The verifier checks complete operation coverage, job precedence, machine sequence, transition setup, operation durations and makespan. Unit tests exercise release times, sequence-dependent setup, malformed/ambiguous inputs, reproducibility and checked arithmetic.

Run:

    cargo fmt --manifest-path tools/factory-exact-schedule/Cargo.toml -- --check
    cargo clippy --manifest-path tools/factory-exact-schedule/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/factory-exact-schedule/Cargo.toml --all-targets
    cargo run --manifest-path tools/factory-exact-schedule/Cargo.toml --bin schedule-oracle

## Claim ceiling and benchmark trajectory

This is an exact oracle for tiny, fixed-route, single-operation-machine-assignment job-shop instances. It is not a flexible job shop, multi-factory transport network, capacity calendar, stochastic/robust scheduler, discrete-event production simulator, MES/MOM, real-time dispatch controller, physical twin or production authorization.

Benchmark progression:

1. Cross-check tiny exact cases with hand enumeration.
2. Add flexible machine eligibility and compare the FJSPLib instance families and best-known upper/lower bounds.
3. Compare standardized time budgets against CP-SAT / CP Optimizer / MIP baselines where available.
4. Add multi-order calendars, sequence-dependent setups, WIP, transport and rolling-horizon disruptions.
5. Add decision-value metrics: first accepted output, reconfiguration/requalification cost, diagnosis value, energy/material use, workforce load and partner dependency risk.

FJSPLib reports 138 flexible-jobshop instances, 264 machine-independent flexible-jobshop instances and 50 flexible-assembly jobshop instances, and compares reference solvers with standardized time budgets and best-known bounds. We should adopt its protocol rather than claim superiority from an unrelated synthetic score.
