# Organ Transport Q0 Reference Kernel

A dependency-free Rust reference for the first analytic transport benchmark described in [ORGAN_ENGINEERING_TRANSPORT_BENCHMARKS_V0.md](../../docs/research/ORGAN_ENGINEERING_TRANSPORT_BENCHMARKS_V0.md).

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
