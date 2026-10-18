# Organ Transport Q0 Reference Kernel

A runtime-dependency-free Rust reference for the first analytic transport benchmark described in [ORGAN_ENGINEERING_TRANSPORT_BENCHMARKS_V0.md](../../docs/research/ORGAN_ENGINEERING_TRANSPORT_BENCHMARKS_V0.md). Test-only `serde_json` and `sha2` dependencies let the executable integration test parse the fixture corpora and verify their pinned byte digests without adding runtime dependencies to the numerical library.

## Scope

- Analytic Hagen–Poiseuille resistance/flow for a rigid circular tube under declared steady, laminar, Newtonian assumptions.
- Signed junction-flow residuals for a precomputed set of edge flows.
- Unit tests for a pinned analytic value, radius/viscosity/length scaling, flow direction, invalid inputs, numerical-range failure, and junction conservation.

This is **not** a network pressure solver, vascular growth model, oxygen transport model, biological simulator, or clinical tool. No cell-specific constants are present. The function does not independently establish the laminar-flow assumption; the caller/benchmark must declare whether the assumptions apply.

## Run

```sh
bash research/organ-transport-q0/verify-q0.sh \
  target/organ-transport-q0-receipt.json \
  "$(git rev-parse HEAD)"
```

The crate owns a tiny workspace so it is not implicitly added to the production Symtropy workspace. Review licensing and domain ownership before product integration.

## Shared evidence artifacts

- [`benchmark-receipt.schema.json`](benchmark-receipt.schema.json) defines versioned receipts with context of use, evidence tier, exact code/config/input identity, assumptions, metrics, uncertainty, validation split, and an explicit claim ceiling. `PASS`, `FAIL`, `BLOCKED`, and `NOT_RUN` are distinct.
- [`tube-reference-cases.v0.json`](tube-reference-cases.v0.json) pins baseline, pressure-reversal, zero-pressure, and negative-control examples.
- [`network-reference-cases.v0.json`](network-reference-cases.v0.json) pins six prescribed-pressure networks: equal-resistance series flow, a two-outlet bifurcation, a two-inlet merge, a 3:1 flow split, two-level branching with multiple junctions, and reversed edge orientation. The verifier checks analytical edge flows and mass conservation at each declared internal junction. Node pressures are specified inputs; Q0 does not solve for unknown network pressures.
- [`corpus-manifest.v0.json`](corpus-manifest.v0.json) pins SHA-256 digests, byte lengths, case counts, and ordered case IDs for both exact fixture files. The verifier fails closed on byte or case-identity drift until the change is reviewed and intentionally repinned.

These are intended to make benchmark results comparable without pretending unlike contexts (for example, flow-only and barrier-function assays) are interchangeable. Future receipt producers must bind the digest to exact input bytes and must never promote a synthetic result into physical or biological validation.

## Executable corpus verifier

Run the pinned tube and small-network specifications against the Rust reference kernel with:

```sh
cargo test --manifest-path research/organ-transport-q0/Cargo.toml
```

The `verify-q0.sh` entrypoint runs the format check and Cargo unit/integration tests, then writes a receipt shaped by `benchmark-receipt.schema.json`. The integration tests compare exact fixture bytes with manifest digests and byte lengths, enforce ordered case IDs, execute every tube fixture, compare network edge flows with the pinned expected values, and check junction mass balance at every declared internal junction. The negative controls must be rejected with the specified error. The receipt records the exact code commit, toolchain, fixture digests, command outcomes, and claim ceiling. `PASS`, `FAIL`, `BLOCKED`, and `NOT_RUN` remain distinct; every non-PASS status exits nonzero.

The dedicated `Organ Transport Q0` workflow checks out the exact PR head, pins Rust 1.96.0, runs the same receipt-producing entrypoint, and uploads the receipt artifact even if verification fails. A queued run or receipt from a previous source head is not a current pass.
