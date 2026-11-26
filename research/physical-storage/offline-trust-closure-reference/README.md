# Offline Trust Closure Reference

This crate provides a dependency-free, fail-closed scoring kernel for the synthetic Offline Cold-Start and Trust-Closure Benchmark (OCSTC-001).

It accepts a trial receipt produced by an independent qualification harness. It does not itself execute a microVM, inspect a filesystem, collect network traces, enforce a sandbox, calculate SHA-256, verify signatures, or prove reader independence. Those inputs must be measured by external, independently controlled tools.

## Qualification dimensions

- Bitstream integrity against a scorer-held expected digest.
- Authenticity/provenance against an out-of-band trust root.
- Interpretability against declared machine-checkable Preservation Objectives.
- Operational closure under a clean offline runtime.
- Independent reader dependency assessment and output agreement.

Any known failure, undeclared runtime dependency, network attempt or sandbox violation blocks a pass. An archive-only digest or trust root cannot independently authorize the archive. Missing or unassessed evidence remains incomplete; all-not-evaluated remains not evaluated.

## Run locally

~~~bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
~~~

The current authoring environment has no Cargo, rustc or rustfmt; the Rust tests have not been executed here. The crate is authored and ready for toolchain verification, not claimed compiled or tested.

## Corpus

The synthetic cases are in ../fixtures/offline_trust_closure_cases_v0_1.json with schema at ../offline_trust_closure_cases_v0_1.schema.json. There are 13 frozen cases covering a trusted clean cold start, missing/archive-only trust roots, payload+manifest substitution, hidden dependencies, network fallback, reader divergence, representation-information loss, unknown revocation freshness, sandbox escape, wrong-but-valid representation, single-reader insufficiency and not-evaluated state.

Fixture outcomes are contract expectations. No cold-start VM, network sandbox, external key ceremony, independent reader or physical archive has been executed.
