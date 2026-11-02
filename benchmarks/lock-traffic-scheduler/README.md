# Lock traffic scheduling benchmark v0.1

Dependency-free Rust reference scheduler for Symtropy #3362.

## Run

~~~sh
cargo test --locked --manifest-path benchmarks/lock-traffic-scheduler/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/lock-traffic-scheduler/Cargo.toml --all-targets -- -D warnings
~~~

## V0 scope

- One lock, one vessel per lockage, single service-resource schedule.
- Upstream passage qualification reference is required; ineligible/unknown requests are never scheduled.
- Participant-authorized arrival window bounds are explicit; no implicit demand shifting is inferred.
- FCFS baseline and priority-with-aging/fairness baseline.
- A two-request exhaustive-order counterexample records the FCFS deadline-loss gap; the exact oracle is test-only and makes no field-performance claim.
- Direction-switch setup time, discrete lock availability windows, outages, service duration, latest start and maximum wait.
- Lock-profile currentness must cover the complete reserved service interval; a slot extending beyond profile validity is not scheduled.
- Exact integer-tick schedule and deterministic tie-breaking.
- Reports service wait, worst wait, lock busy ticks, direction switches, unscheduled reason by request.

This is not a multi-vessel chamber packing, multi-lock/multi-chamber route planner, MILP/max-plus implementation, real-time vessel arrival predictor or navigation authority. It is a first reproducible baseline for comparison against those methods.

All test requests and lock windows are synthetic. Scheduler output is a recommendation only; it cannot dispatch vessels, authorize passage or operate gate/pump controls.
