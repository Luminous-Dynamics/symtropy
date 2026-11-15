# AeroWeft assurance composition oracle

This isolated, dependency-free Rust crate implements the finite composition contract in [AEROWEFT_ASSURANCE_COMPOSITION_001.md](../../../docs/research/aviation/AEROWEFT_ASSURANCE_COMPOSITION_001.md).

It consumes owner-produced dispositions and receipt references. It does not implement PNT qualification, clock conversion, surveillance sensing/fusion, independence counting, aircraft physics, reachability propagation, operational-intent policy, conflict geometry, scheduling, flight authorization, DAA or aircraft control.

The strongest positive state is `READY_FOR_CONFLICT_SCREEN`. Each traffic class is retained as either a qualified observed track or unresolved possible occupancy. A no-report result never yields an empty-sky result.

The 32-case golden synthetic fixture is in `fixtures/aeroweift-assurance-composition/evidence-composition-v0.json`. The Rust test suite also executes 44 single-fault mutations, all 990 unordered fault-mutator pairs (including same-mutator pairs), and 10,000 deterministic seeded two-fault trials. Required traffic classes are bound to a separate profile requirement receipt; a caller cannot weaken the required class set and still pass composition. Each mutated input must fail closed without a partial occupancy list; the separate positive control prevents reject-all solutions from passing. All inputs are fictional; none of these tests provides field, operational or certification evidence.

## Local checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked --all-targets
```
