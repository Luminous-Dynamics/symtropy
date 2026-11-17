# HoloFrac Spatial Retrieval Lab

Standalone Rust benchmark for the first executable HoloFrac slice: typed, directed architectural-graph retrieval using the existing `symtropy-hdc-core` crate.

## Run

From the repository root:

```bash
HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo test --manifest-path research/holofrac-retrieval/Cargo.toml

cargo clippy --manifest-path research/holofrac-retrieval/Cargo.toml \
  --all-targets -- -D warnings

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml
```

The CLI emits a CSV summary for 2K/4K/8K/16K dimensions and per-query raw outcomes as comment records. Capture stdout as the run artifact; record the compiler version, CPU model, OS, commit, and dirty-tree state alongside it.

## What is implemented

- Typed spatial scales and directed relation kinds.
- Validation for duplicate IDs/facts, empty identifiers and unknown endpoints.
- Stable canonical fact keys and exact graph lookup oracle.
- Deterministic role-separated subject/predicate/object encoding through `ItemMemory` and bipolar binding/bundling.
- Explicit directionality negative controls using reversed edges.
- Encoder-contract fingerprint and separate non-security source-graph digest.
- An f32 materialization of the same bipolar code to compare storage and similarity cost, not a second encoder family.
- Positive and reversed-edge retrieval metrics at four dimensions.

## Claim boundaries

This is an initial retrieval fixture, not a floor-plan generator, building model, GHRR implementation, or proof of fractal geometry. Current bipolar binding is commutative; direction is represented through distinct subject/object role vectors. This is **not** non-commutative matrix binding.

The f32 view is a conversion of the same bipolar code. It does not test learned continuous-HV representations. The current fixture is small and synthetic; its results cannot establish generalization to real buildings or superiority over commercial/academic graph retrieval systems.

The source-graph digest and encoder fingerprint are non-cryptographic regression identifiers. They are not security hashes, signatures, or provenance attestations.

HDC provides candidate retrieval only. Exact typed-graph lookup remains authoritative. A production extension must add held-out project splits, partial-context queries, abstention/calibration, update-cost measurements and independent reproduction before stronger claims are made.
