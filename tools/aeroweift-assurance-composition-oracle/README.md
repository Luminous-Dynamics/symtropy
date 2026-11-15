# AeroWeft assurance composition oracle

This isolated, dependency-free Rust crate implements the finite composition contract in [AEROWEFT_ASSURANCE_COMPOSITION_001.md](../../../docs/research/aviation/AEROWEFT_ASSURANCE_COMPOSITION_001.md).

It consumes owner-produced dispositions and receipt references. It does not implement PNT qualification, clock conversion, surveillance sensing/fusion, independence counting, aircraft physics, reachability propagation, operational-intent policy, conflict geometry, scheduling, flight authorization, DAA or aircraft control.

The strongest positive state is `READY_FOR_CONFLICT_SCREEN`. Each traffic class is retained as either a qualified observed track or unresolved possible occupancy. A no-report result never yields an empty-sky result.

The golden synthetic fixture is in `fixtures/aeroweift-assurance-composition/evidence-composition-v0.json`. All inputs are fictional; the fixture does not provide field, operational or certification evidence.

## Local checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked --all-targets
```
