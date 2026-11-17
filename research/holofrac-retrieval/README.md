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

The CLI emits a CSV summary for 2K/4K/8K/16K dimensions and per-query raw outcomes as comment records. Capture stdout as the run artifact; record `rustc --version`, CPU model, OS, exact commit and dirty-tree state alongside it.

## What is implemented

- Typed spatial scales and directed relation kinds.
- Validation for duplicate IDs/facts, empty identifiers, self-relations, unknown endpoints and broader-to-narrower `contains` direction.
- Stable canonical fact keys and exact graph lookup oracle.
- Deterministic role-separated subject, subject-scale, predicate, object and object-scale encoding through `ItemMemory` and bipolar binding/bundling.
- Explicit directionality negative controls using reversed edges.
- Prebuilt semantic index: candidate encodings and canonical keys are computed once; the benchmark verifies the source digest before timing the frozen query batch.
- Separate timings for index construction, query encoding, bipolar search, f32-view search, exact graph oracle and canonical metadata lookup.
- Encoder-contract fingerprint, full encoder-spec compatibility check, exact canonical source-snapshot check and separate non-security source-graph digest.
- An f32 materialization of the same bipolar code to compare storage and similarity cost, not a second encoder family.
- Positive and reversed-edge retrieval metrics at four dimensions.
- Stale-index rejection when the exact canonical source graph changes after index construction; insertion-order changes remain equivalent.
- Rejection of an index queried with a different encoder schema, seed or dimension.
- Separate bipolar-only, f32-only and combined dual-view vector-payload estimates.

## Claim boundaries

This is an initial exact-fact retrieval fixture, not a floor-plan generator, building model, GHRR implementation, partial-context retriever, or proof of fractal geometry. Current bipolar binding is commutative; direction is represented through distinct subject/object role vectors. This is **not** non-commutative matrix binding.

The f32 view is a conversion of the same bipolar code. It does not test learned continuous-HV representations. The current fixture is small and synthetic; its results cannot establish generalization to real buildings or superiority over commercial/academic graph retrieval systems.

The reported memory values are vector-payload estimates, not allocator-inclusive process RSS. The prototype retains both vector views at once, so the dual-view estimate is the currently retained vector payload; single-view figures are hypothetical per-representation comparisons. Timings are local microbenchmarks on a small synthetic graph and must not be treated as SOTA results. The index build is separated from query scoring; source freshness is verified before the timed batch.

The source-graph digest and encoder fingerprint are non-cryptographic regression identifiers. They are not security hashes, signatures, or provenance attestations.

HDC provides candidate retrieval only. Exact typed-graph lookup remains authoritative. A follow-on benchmark tracks partial-context queries, abstention/calibration, update costs, interference/capacity curves and held-out project/topology families: https://github.com/Luminous-Dynamics/symtropy/issues/5867.
