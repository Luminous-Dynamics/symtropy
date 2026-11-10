# GMD Reference Kernel

A small, dependency-free Rust research kernel for two analytical steps in a geomagnetically induced current pipeline:

1. integrate a locally represented horizontal geoelectric field along an oriented path;
2. solve the quasi-DC resistive nodal equations for a tiny network with explicit grounding paths.

This crate is an analytic reference oracle, not an operational grid solver.

## Scope

- Local East/North tangent-plane coordinates.
- Electric field in V/m; path displacement in m.
- Branch resistance in ohms; induced voltage in V; current in A.
- Piecewise-constant electric field along supplied path segments.
- Linear resistive branches with series impressed voltage.
- Explicit node-to-ground resistors.
- Dense nodal solver with partial pivoting.
- Deterministic standard-library-only implementation.

## Deliberate non-goals

No geomagnetic field reconstruction, SECS solver, 1-D/3-D Earth conductivity inversion, geographic CRS conversion, magnetic coordinate conversion, line-path interpolation from GIS, transformer winding/grounding topology importer, transformer saturation, harmonics, thermal/damage model, AC power flow, transient stability, GMD forecast, grid control, or actuation.

Do not feed it a current or voltage limit and infer transformer safety. That needs a qualified asset-specific specialist model.

## Equations and sign convention

For an oriented path:

    V_ij = integral from i to j of E(x) dot dl

For each resistive branch oriented i to j:

    I_ij = (phi_i - phi_j + V_ij) / R_ij

For each explicit ground path at node i:

    I_i_ground = phi_i / R_i_ground

Kirchhoff current balance is imposed at every node. Every connected component must contain at least one explicit ground connection. An ungrounded component is rejected rather than assigned an arbitrary reference potential.

## Quick checks

This package is a standalone nested Cargo workspace and has no third-party dependencies:

    cargo test --manifest-path crates/tools/gmd-reference-kernel/Cargo.toml --offline
    cargo fmt --manifest-path crates/tools/gmd-reference-kernel/Cargo.toml -- --check
    cargo clippy --manifest-path crates/tools/gmd-reference-kernel/Cargo.toml --offline --all-targets -- -D warnings

Those commands are instructions, not claimed results. Exact-head CI and a local Rust toolchain run are still required.

## Initial analytic fixtures

- oriented line integral and path reversal;
- piecewise-field additivity;
- two-node line with two ground returns and a hand-computed 2.5 A result;
- branch orientation reversal;
- parallel path current sharing;
- disconnected ungrounded component rejection;
- non-finite field rejection;
- non-positive resistance rejection.

The next physics increment is to compare this toy kernel against an independently authored analytic oracle and then introduce a versioned, synthetic multi-node test corpus. Do not connect it to the production renderer or operator-control surfaces.
