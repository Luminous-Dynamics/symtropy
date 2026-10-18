# Organ Transport Q0 Reference Kernel

A runtime-dependency-free Rust reference for the first analytic transport benchmark described in [ORGAN_ENGINEERING_TRANSPORT_BENCHMARKS_V0.md](../../docs/research/ORGAN_ENGINEERING_TRANSPORT_BENCHMARKS_V0.md). A test-only `serde_json` dependency lets an executable integration test consume the pinned JSON corpora without adding a runtime dependency to the numerical library.

## Scope

- Analytic Hagen–Poiseuille resistance/flow for a rigid circular tube under declared steady, laminar, Newtonian assumptions.
- Signed junction-flow residuals for a precomputed set of edge flows.
- Unit tests for a pinned analytic value, radius/viscosity/length scaling, flow direction, invalid inputs, numerical-range failure, and junction conservation.

This is **not** a network pressure solver, vascular growth model, oxygen transport model, biological simulator, or clinical tool. No cell-specific constants are present. The function does not independently establish the laminar-flow assumption; the caller/benchmark must declare whether the assumptions apply.

## Run

```sh
cargo test --manifest-path research/organ-transport-q0/Cargo.toml
```

The crate owns a tiny workspace so it is not implicitly added to the production Symtropy workspace. Review licensing and domain ownership before product integration.

## Shared evidence artifacts

- [`benchmark-receipt.schema.json`](benchmark-receipt.schema.json) defines versioned receipts with context of use, evidence tier, exact code/config/input identity, assumptions, metrics, uncertainty, validation split, and an explicit claim ceiling. `PASS`, `FAIL`, `BLOCKED`, and `NOT_RUN` are distinct.
- [`tube-reference-cases.v0.json`](tube-reference-cases.v0.json) pins baseline, pressure-reversal, zero-pressure, and negative-control examples.
- [`network-reference-cases.v0.json`](network-reference-cases.v0.json) pins three small prescribed-pressure networks: equal-resistance series flow, a two-outlet bifurcation, and a two-inlet merge. The verifier checks analytic edge flows and mass conservation at each declared internal junction. Node pressures are specified inputs; Q0 does not solve for unknown network pressures.

These are intended to make benchmark results comparable without pretending unlike contexts (for example, flow-only and barrier-function assays) are interchangeable. Future receipt producers must bind the digest to exact input bytes and must never promote a synthetic result into physical or biological validation.

## Executable corpus verifier

Run the pinned tube and small-network specifications against the Rust reference kernel with:

```sh
cargo test --manifest-path research/organ-transport-q0/Cargo.toml
```

The `tests/fixture_corpus.rs` integration tests parse the checked-in JSON files, execute every tube fixture, compare network edge flows with the pinned expected values, and check junction mass balance with declared tolerances. The negative controls must be rejected with the specified error. This is an analytic/software gate only; the fixture JSON describes expected cases while the exact-head test receipt records whether verification actually ran.

The dedicated `Organ Transport Q0` workflow checks formatting and executes both unit and corpus tests on the research branch and pull requests. The source/configuration/fixture digests should be captured in a versioned benchmark receipt before comparing results across implementations.
