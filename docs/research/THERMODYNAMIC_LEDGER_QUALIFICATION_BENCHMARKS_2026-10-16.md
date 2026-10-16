# Thermodynamic ledger qualification benchmarks

Date: 2026-10-16  
Status: target specification; no benchmark results are claimed.

## Paradigm shift: make energy accounting an auditable protocol

The ledger should evolve from a pair of signed counters into a typed, double-entry flow protocol. Every transfer should name its source, destination, unit, system boundary, and stable event identity. Entity storage changes must be accounted separately from heat and external exports. A numerically balanced local residual is a bookkeeping observation—not evidence that a simulated physical model is correct.

This PR is the first hardening slice. It separates action/consumption cost, regeneration, heat dissipation, and explicitly removed energy; rejects invalid numeric flows; and keeps the legacy residual diagnostic visibly fail-closed after a rejected update. It does not yet implement storage reconciliation, event identity, or complete system-boundary accounting.

## Qualification suite and measurable targets

| Gate | Workload | Acceptance target |
|---|---|---|
| Invalid-domain rejection | For every checked recorder: negative flow, NaN, +infinity, -infinity; invalid Phi domains; accumulator overflow | 100% rejected; zero partial accumulator mutation |
| Fail-closed reporting | Finalize a tick after any rejected update | 100% of invalid ticks report inputs_valid = false, a nonzero invalid count, and an infinite legacy residual; zero invalid tick reported as clean |
| Flow-category isolation | Mixed action, regeneration, heat, and external-sink events | 100% category attribution; no regeneration represented as negative heat; independent oracle confirms the documented local identity |
| Long-run numeric robustness | 100 deterministic seeds × 100,000 ticks with seeded near-capacity, tiny-flow, and large-dynamic-range values | No NaN/infinite accepted accumulator; no negative typed flow; normalized discrepancy against independent recomputation ≤ 1e-9 for the flow totals |
| Replay / duplicate control | Repeat the same recorded event corpus byte-for-byte | Current implementation: deterministic results for the same call order. Next phase target: event-ID deduplication makes replaying an existing event a zero-delta operation |
| Closed-system conservation (future gate) | Frozen, closed-boundary scenario; no unexplained external flows; 1,000,000 ticks; compare dt, dt/2, dt/4 | After storage and all boundary terms are modeled, maximum normalized total-energy drift ≤ 1e-3, with the drift trend consistent with the integrator's expected convergence order |
| Runtime cost | Optimized build, fixed workload and hardware, ledger-on vs ledger-off baseline | ≤ 5% p95 step-time overhead and zero heap allocations per flow event; record absolute latency and hardware rather than comparing unlike machines |

## Reference identities

Until full system accounting exists, the local ledger reports:

local_residual = action_cost + regeneration - heat_dissipation - external_sink

This identity only relates the categories currently recorded by the ledger. It omits entity storage deltas, kinetic/potential energy, unobserved work, and any other sources or sinks. Therefore, it must not be used as a whole-world conservation gate.

The eventual whole-system identity must explicitly state its boundary and include the change in stored energy plus all inputs, outputs, transfers, and dissipative terms. Internal transfers should appear once as a source debit and once as a destination credit, or be reconciled through a single event receipt—not counted as a net external gain.

## Reproducibility contract

Each benchmark report must include:
- exact repository commit SHA and clean/dirty working-tree status;
- Rust toolchain, Cargo profile, target triple, CPU/GPU model, and relevant feature flags;
- fixed seed list and hash of the generated event corpus;
- independent oracle implementation/version;
- totals, rejected-event counts, max normalized residual, p50/p95 step latency, and allocation count;
- exact command lines and raw result artifacts.

Queued checks are not passes. A result applies only to the exact commit SHA whose code and configuration were measured.

## Next architecture milestone

Add immutable flow receipts with stable event IDs and explicit source/destination accounts. Derive each entity's observed storage delta from before/after state, reconcile it with the receipt graph, and make duplicate receipts idempotent. Only after the system boundary is complete should the closed-system conservation benchmark become an acceptance gate.