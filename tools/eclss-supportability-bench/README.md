# ECLSS Supportability Benchmark Validator

Small, CPU-only Rust validator for the frozen ECLSS supportability registries. It is deliberately **not** a reliability simulator, life-support controller, chemistry model, or flight-qualification tool.

## Run

From the Symtropy repository root:

```sh
cargo test --manifest-path tools/eclss-supportability-bench/Cargo.toml
cargo run --release --manifest-path tools/eclss-supportability-bench/Cargo.toml
```

The fixtures are embedded with `include_str!` so the validator checks exactly the versioned files in this commit. Rebuild after changing a fixture. A dedicated GitHub Actions workflow checks formatting, runs tests, validates the exact PR head, and uploads a 90-day receipt.

## Checks

- 30 synthetic case IDs, required keys, enum values, and fail-closed expected dispositions.
- Abstract 98% recovery / finite make-up arithmetic using integer basis points: 60 make-up units required, 40 available, 20 unserved.
- 9 selected ICES-2025-127 component rows plus 133 supplementary Appendix Table 2–10 component rows: 142 component reference rows total.
- 2 selected Table 1 crew-task rows plus 39 supplementary rows: all 41 published crew R&R task estimates are represented. The estimates exclude gathering/stowing and are not complete mission downtime.
- The selected register separates `component_records` from `crew_task_records`. Table 5 component R&R hours are stored as `component_r_and_r_hours`; the three-hour UPA distillation crew task appears only in the Table 1 task register, not duplicated in the Table 3 component record.
- Component MTBF, operational MTTR, Table 5 component R&R hours, Table 1 crew R&R task times, preventive R&R interval/duration, throughput limits and calendar/shelf-life limits remain distinct nullable fields and records. Blank published fields remain null.
- 10 source-bounded ICES-2026-379 operational / planned / applicability records with source locations and claim ceilings.
- Five source-bounded AOGA schedule snapshots; milestone variance remains unresolved rather than treating forecasts as achieved results.
- 13 proof-carrying maintenance-plan cases: one admissible and 12 fail-closed controls for configuration, shelf-life, isolation, service floors, worker skills, post-tests, worker/tool conflicts, spare over-allocation, authority expiry, dependency order, time windows and approved procedures.

Maintenance-plan `ADMISSIBLE` means only that the declared evidence contract is internally satisfied. It is not authority to execute work or control hardware.

The executable checks JSON structure, key sets, selected values, table/record counts and synthetic known answers. It does not validate the files with a general-purpose JSON Schema engine, estimate failure probabilities, extract raw MADS events, execute EMAT/RELISSA, reproduce a stochastic/DRCC optimizer, simulate physical processes, or qualify hardware.
