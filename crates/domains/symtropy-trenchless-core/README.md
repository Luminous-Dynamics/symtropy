# Symtropy Trenchless Core

A dependency-light, standalone Rust contract for evidence-qualified screening of pipeline construction and renewal methods.

## Scope

The crate compares known candidate capabilities/predictions against declared requirements and emits one of:

- Eligible: every supplied hard constraint is known and satisfied.
- Ineligible: at least one known hard constraint is violated.
- Indeterminate: no known failure exists, but at least one required fact is unknown or marked not applicable.

It never ranks candidates. It does not calculate geology, jacking loads, settlement, hydraulic performance, structural capacity, cost, safety, or permission to excavate. Those remain with their qualified domain owners.

## Local verification

From the repository root:

    cargo test --manifest-path crates/domains/symtropy-trenchless-core/Cargo.toml

The crate has no external dependencies and declares its own workspace so this first contract can be tested independently of the larger Symtropy workspace graph.

## Evidence boundary

Source references are carried through to results but are not authenticated by this crate. A production composition must bind them to the upstream artifact/custody systems and exact project, model, calibration, method, tool and design revisions.