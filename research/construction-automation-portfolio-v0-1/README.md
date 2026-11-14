# Construction Automation Portfolio Core — v0.1

**Status:** synthetic execution-mode selection and Pareto portfolio oracle. It consumes declared schedules, qualification references, resource reservations and workface envelopes. It is not a general scheduler, robot capability authority, safety controller or acceptance ledger.

Parent: [CONSTRUCTION-AUTOMATION-PORTFOLIO-001 #5089](https://github.com/Luminous-Dynamics/symtropy/issues/5089).

Stacked dependency order:
[interop / temporal resource owner #5009](https://github.com/Luminous-Dynamics/symtropy/pull/5009)
→ [accepted-floor ledger #5048](https://github.com/Luminous-Dynamics/symtropy/pull/5048)
→ [space-time occupancy #5049](https://github.com/Luminous-Dynamics/symtropy/pull/5049)
→ [observation planning #5050](https://github.com/Luminous-Dynamics/symtropy/pull/5050)
→ this portfolio adapter.

## Implemented v0.1

- Declares the same required work-unit and acceptance-contract reference across human, assisted, robot and offsite mode options.
- Qualification status is explicit: eligible, ineligible, unknown or stale. Only eligible modes with capability, qualification and authority-scope references enter candidate selection. References are not authenticated here.
- Consumes proposed schedule windows per option; it does not create a second general-purpose scheduler.
- Enforces topological predecessor order and start-after-predecessor constraints.
- Composes the existing ReservationBook and conservative SpatialOccupancyBook; no second resource calendar or geometry checker.
- Enumerates mode portfolios exactly for small declared option sets and returns a three-objective Pareto frontier over projected cost, makespan and skilled crew-minutes.
- Cost is itemized in integer minor currency units; workflow crew-minutes, skilled crew-minutes, energy and mode counts remain separate.
- Baselines include human-only, robotize-all-eligible, fastest-local and lowest-cost-local.
- A bounded search reports `BudgetExhausted`; it never claims infeasible or optimal when search is incomplete.
- Every plan preserves required functional-unit count and acceptance-contract references. Planned units are not accepted units.

## Commands

```sh
cargo fmt --manifest-path research/construction-automation-portfolio-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-automation-portfolio-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-automation-portfolio-v0-1/Cargo.toml --locked --all-targets -- -D warnings
cargo bench --manifest-path research/construction-automation-portfolio-v0-1/Cargo.toml --locked --bench portfolio
```

## Claim ceiling

V0.1 demonstrates only deterministic synthetic portfolio semantics for declared, pre-scheduled options and the frozen resource/space contracts. It does not establish robot ability, physical production performance, safety, independent work acceptance, cost savings, market pricing or a construction time estimate. Actual acceptance and whole-floor actuals remain with #5048.
