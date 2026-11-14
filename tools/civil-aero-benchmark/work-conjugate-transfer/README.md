# AER-01 work-conjugate surface transfer v0.1

**Parent:** [#4112 conservative 3-D interface transfer](https://github.com/Luminous-Dynamics/symtropy/issues/4112)  
**Aircraft evidence bridge:** [#4093](https://github.com/Luminous-Dynamics/symtropy/issues/4093)  
**MDO / solver orchestration:** [#3237](https://github.com/Luminous-Dynamics/symtropy/issues/3237)

## Paradigm: an interface operator pair, not two unrelated interpolators

For a source surface traction quadrature field f_q, a target structural finite-element displacement field u_i, and shape weights N_i(x_q), define:

\[
F_i = \sum_q N_i(x_q) f_q,\qquad
u_q = \sum_i N_i(x_q)u_i
\]

Then:

\[
\sum_q f_q \cdot u_q
= \sum_i F_i \cdot u_i
\]

The force-transfer operator is the transpose/adjoint of the displacement interpolation operator over the declared quadrature and target shape basis. This makes virtual-work equality a construction property for this interpolation, rather than an unrelated check applied after two independently chosen maps.

On exact geometric overlap, partition of unity preserves resultant force. Linear coordinate reproduction preserves the first moment and therefore the global resultant moment. Coverage is checked at every source quadrature point; unsupported points are rejected rather than extrapolated.

## Implemented v0.1 scope

The Rust crate implements a bounded planar, linear-triangular surface patch:

- vector traction in declared Pa (N/m²) at source mesh nodes;
- source triangular face integration with a three-point degree-2 barycentric quadrature rule;
- interpolation of source traction to quadrature points;
- projection of each source quadrature point onto an overlapping target triangle;
- target barycentric shape weights;
- mapped structural nodal forces using the consistent transpose operator;
- target nodal displacement interpolation to source quadrature points using the same weights;
- comparison of source and target virtual work;
- independent source/target resultant force and moment calculations about an explicit reference origin;
- mesh, traction, displacement and aircraft-state digest bindings;
- face winding, degeneracy, duplicate connectivity, and geometric coverage checks.

A deliberately mismatched nearest-node force-transfer baseline is retained. It preserves total force because each quadrature force is assigned once, but it generally loses moment and work conjugacy when compared with the consistent FE displacement interpolation. The benchmark must show those errors rather than hiding them.

## Independent oracle

The independent Python standard-library verifier is a separate implementation. It independently:
- integrates the source traction;
- locates target triangles and computes barycentric coordinates;
- maps nodal forces and displacement fields;
- recomputes force, moment and virtual-work residuals;
- verifies the nearest-node negative baseline;
- injects hostile geometry, winding, digest and coverage mutations;
- compares its complete summary against the Rust executable output.

It does not import Rust implementation code.

## Run

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/work-conjugate-transfer/Cargo.toml -- --check
cargo test --manifest-path tools/civil-aero-benchmark/work-conjugate-transfer/Cargo.toml
cargo run --quiet --manifest-path tools/civil-aero-benchmark/work-conjugate-transfer/Cargo.toml > /tmp/work-conjugate-transfer-result.json
python3 tools/civil-aero-benchmark/work-conjugate-transfer/verify_independent.py --rust-output /tmp/work-conjugate-transfer-result.json
~~~

A dedicated workflow retains the fixture, result JSON and SHA-256 receipts.

## Claim ceiling and next steps

The current fixture is a planar patch with linear triangular interpolation and exact overlap. It does not qualify curved/non-coplanar interfaces, non-matching high-order elements, mortar/dual-mortar projection, RBF interpolation, contact, gaps, shell offsets, pressure sign conventions from actual CFD, turbulence loads, or full FSI solver convergence.

The method is a candidate operator for the broader #4112 campaign, not a universal winner. Next comparisons should include consistent higher-order shape functions, RBF with conditioning/support diagnostics, mortar/dual-mortar projection, and constrained mapping under exactly matched surface/load/mesh budgets. Each must be evaluated on force, moment, patch reproduction, virtual work, coverage, conditioning and local load distribution.

This result is mechanics/software verification on a synthetic patch only. It is not CFD/FEM validation, CRM static-FSI reproduction, aircraft structural substantiation, flutter clearance, fatigue evidence, airworthiness or certification.
