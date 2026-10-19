# WATER-PROC-CLOSURE-001 executable reference evaluator

This is a standalone Rust crate for the frozen **synthetic** 16-case corpus in the parent PR. It is a benchmark/evaluator, not a treatment plant model, runtime treatment controller, or safety/compliance oracle.

## Run locally

From the repository root:

```sh
cargo fmt --manifest-path tools/water-process-closure-001/Cargo.toml -- --check
cargo test --manifest-path tools/water-process-closure-001/Cargo.toml
cargo run --manifest-path tools/water-process-closure-001/Cargo.toml -- \
  fixtures/water-process-closure-001-inputs-v1.json \
  fixtures/water-process-closure-001-oracle-v1.json \
  artifacts/water-process-closure-001/receipt.json
```

Without `WATER_PROC_CLOSURE_REVISION` (or `GITHUB_SHA`), a matching run is marked `PassedLocalUnpinned`; it is not an exact-revision receipt. The CI wrapper binds `GITHUB_SHA` to the checked-out commit and passes it explicitly.

The receipt includes raw input/oracle SHA-256 digests, the revision supplied by the wrapper, sorted per-case outcomes and oracle differences. It deliberately omits timestamps so identical inputs, oracle, and supplied revision produce deterministic JSON output.

## Semantics

- Water-volume balance, material mass, tracked tracer and lifecycle closure are distinct dimensions.
- Point mass intervals use explicit tolerances. Overlapping uncertain intervals are `PossibleWithUncertainty`, never exact closure. Disjoint intervals are `ImpossibleWithinBounds`.
- Concentration alone cannot imply tracer-mass conservation.
- Missing residual routes or incomplete cycle/utilities do not become zero burden.
- The only positive overall outcome is `AccountingOnly`; it is not treatment efficacy or safety qualification.

This is a small reference evaluator for this fixed corpus. It must not be marketed as a new generic mass-accounting library. For production integration, connect the profile adapter to shared PIE-001 interval semantics and retain Symtropy's water-specific boundary checks.
