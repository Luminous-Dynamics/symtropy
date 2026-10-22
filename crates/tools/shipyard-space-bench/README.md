# Shipyard space-time benchmark (V0 verifier)

This is the first small, dependency-free Rust slice for [SHIPYARD-SPACE-001](https://github.com/Luminous-Dynamics/symtropy/issues/2050), a child of [SHIPBUILD-CO-DESIGN-001](https://github.com/Luminous-Dynamics/symtropy/issues/2048).

V0 independently checks a proposed synthetic plan for:

- simultaneous overlap between rectangular block footprints in the same area;
- occupancy of explicitly required access corridors;
- declared shared-resource capacity using half-open time intervals;
- operation precedence;
- invalid intervals/footprints;
- explicit unknowns for missing footprints and resource capability/capacity.

## Run

```sh
cargo test --locked --offline --manifest-path crates/tools/shipyard-space-bench/Cargo.toml
```

No third-party crates are required. This crate is deliberately not added to the top-level application build or a CI workflow in this initial slice.

## Interpretation

The geometry profile uses axis-aligned rectangles on an integer lattice and half-open intervals `[start, end)`. It is a small verification profile, **not** a shipyard planner, polygon/3D collision engine, crane or rigging model, floor-bearing analysis, structural solver, or real-world safety assessment.

A known hard violation yields `Infeasible`. Missing evidence yields `Indeterminate`, never implied free space or unlimited capacity. The result preserves violations and unknowns simultaneously.

The tests are intended as a starter oracle contract. This initial slice has not yet been run with a Rust toolchain in the current authoring environment; do not treat it as a passing exact-head result until the exact command above succeeds on the proposed commit.
