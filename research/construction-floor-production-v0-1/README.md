# Construction Floor Production Accounting — v0.1

See [INDUSTRY_AND_BENCHMARK_ALIGNMENT.md](./INDUSTRY_AND_BENCHMARK_ALIGNMENT.md) for the current competitor and benchmark comparison.

**Status:** synthetic Rust integration prototype. Not a construction scheduler, physical inspection system, safety controller, cost estimate or field productivity claim.

- Parent: [HIGHRISE-CLIMBING-WORKCELL-001 #4914](https://github.com/Luminous-Dynamics/symtropy/issues/4914)
- Reference-floor fixture: [#4937](https://github.com/Luminous-Dynamics/symtropy/issues/4937) / draft [#4939](https://github.com/Luminous-Dynamics/symtropy/pull/4939)
- Integration issue: [CONSTRUCTION-FLOOR-PRODUCTION-001 #4957](https://github.com/Luminous-Dynamics/symtropy/issues/4957)
- Reused construction lifecycle and resource core: [draft #3644](https://github.com/Luminous-Dynamics/symtropy/pull/3644)
- Generic schedule oracle: [#1902](https://github.com/Luminous-Dynamics/symtropy/issues/1902)
- Recovery benchmark: [#1843](https://github.com/Luminous-Dynamics/symtropy/issues/1843)
- Accepted-output accounting contract: [#3605](https://github.com/Luminous-Dynamics/symtropy/issues/3605)

## Why this exists

A schedule can be feasible while a floor is not accepted. A robot can finish an executor order while its physical output remains uninspected. A rejected first pass can later become acceptable after corrective work, but the original rework and inspection costs must remain in the floor economics.

The prototype adds a thin ledger above existing owners:

`existing task lifecycle + existing resource reservation + floor functional units + person-time + cost entries -> accepted-production report`

It intentionally does not create another scheduler, resource calendar, fleet protocol, task-state machine or acceptance authority.

## Implemented in v0.1

- A floor binding pins floor ID, site epoch, design revision digest and target frame.
- Required functional units bind a task-specific acceptance-contract reference before any attempt.
- Attempts snapshot the existing `TaskMachine` state, robot-configuration digest and exact task/output/receipt references. Each attempt report retains the functional-unit acceptance-contract reference.
- Rework requires a new task ID, contiguous attempt number and explicit corrective-work authorization.
- First-pass yield and final acceptance are reported separately.
- Only `IndependentlyAccepted` from the existing task machine receives accepted-output credit.
- Person-time slices use half-open intervals on a declared floor labour-time axis and reject overlapping work by the same person.
- Cost entries use integer minor currency units, explicit currency, unique IDs and optional known task IDs.
- Failed/rejected/rework/setup/inspection costs remain in total cost.
- Accepted-floor cost is unavailable until every required functional unit is finally accepted.
- A small adapter function passes floor-bound resource requests to the existing `ReservationBook`; it adds no second calendar.
- Reports contain a disposition row for every required functional unit, including units with no attempted work.
- A paired observation comparison reports the cost and crew-time difference between early and deferred detection without assuming early inspection always wins.

## Local commands

```sh
cargo fmt --manifest-path research/construction-floor-production-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-floor-production-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-floor-production-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

## Important limitations

- The integration crate is stacked on draft #3644. Its path dependency is not available from main until the base PR is merged or the stack is otherwise integrated.
- The reference-floor schedule values remain abstract planning ticks. They are not hours, crew rates or cost inputs.
- The cost and observation examples are synthetic integer arithmetic; they do not encode prices, probabilities, project labour rates or actual savings. The paired observation experiment compares the same declared hidden-fault scenario without assigning a probability distribution.
- The acceptance-contract reference is recorded as an intent pointer; the crate does not inspect or authenticate the external acceptance contract or receipt authority. The ledger checks references and task states exposed by the local lifecycle oracle. It does not authenticate signatures or authorities, independently inspect physical work, verify receipt cryptography, or determine engineering tolerances.
- Currency validation checks a three-letter uppercase code shape only; it does not prove that the code is a currently valid ISO 4217 currency.
- The resource reservation is not a safety permit, clearance or motion authorization.
- No physical trial, construction safety qualification, structural assessment, cost saving, field productivity or code-conformity claim is made.

## Benchmark direction

The next integration stage should run the four #4939 reference-floor cases and ten disturbances against B0 fixed schedule, B1 deterministic rule recovery, B2 evidence-bounded adaptive replanning and B3 a bounded exact oracle for tiny cases. Every strategy must see the same frozen starting state and only observations available at decision time.

Report accepted functional units, first-pass yield, rework, total workflow crew-minutes, resource waiting/occupancy, accepted-floor cycle time and fully burdened cost separately. Do not collapse these into a single score that can conceal hard-gate or worst-stratum failures.

The industry comparison should retain the scope of current products. SYNCHRO already offers 4D planning, human/material/equipment/space resource assignment, resource tracking, cost/earned-value reporting and dynamic clash analysis. The Luminous differentiation to prove is not ordinary 4D planning; it is the operational chain from current task and resource admission through heterogeneous execution, recovery and independent physical acceptance to complete whole-workflow economics.
