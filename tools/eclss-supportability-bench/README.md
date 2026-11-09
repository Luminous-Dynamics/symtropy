# ECLSS Supportability Benchmark Validator

Small, CPU-only Rust validator for the frozen ECLSS supportability registries. It is deliberately **not** a reliability simulator, life-support controller, chemistry model, or flight-qualification tool.

## Run

From the Symtropy repository root:

```sh
cargo test --manifest-path tools/eclss-supportability-bench/Cargo.toml
cargo run --release --manifest-path tools/eclss-supportability-bench/Cargo.toml
```

The fixtures are embedded with `include_str!` so the validator checks exactly the versioned files in this commit. Rebuild after changing a fixture.

## Checks

- 30 synthetic case IDs, required keys, enum values, and fail-closed expected dispositions.
- The abstract 98% recovery / finite make-up balance using integer basis points: 60 make-up units required, 40 available, 20 unserved.
- 11 selected ICES-2025-127 component records; preserves MTBF, operational MTTR, crew R&R, preventive R&R, throughput and calendar-limited lives as distinct nullable fields.
- 10 source-bounded ICES-2026-379 operational / plan / applicability records with source locations and claim ceilings.
- Five source-bounded AOGA schedule snapshots and the explicit unresolved variance state.

The binary reports structural and arithmetic fixture validation only. It does not validate JSON Schema conformance with a general-purpose JSON Schema engine, estimate failure probabilities, execute EMAT/RELISSA, simulate physical processes, or qualify hardware.
