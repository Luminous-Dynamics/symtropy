# AER-01 conservative 3-D surface-transfer oracle v0.1

**Parent:** [#4093 structural/aeroelastic bridge](https://github.com/Luminous-Dynamics/symtropy/issues/4093)  
**Implementation follow-up:** [#4112 conservative 3-D interface transfer](https://github.com/Luminous-Dynamics/symtropy/issues/4112)  
**Predecessor:** [#4110 bounded 1-D analytical bridge](https://github.com/Luminous-Dynamics/symtropy/pull/4110)  
**Status:** synthetic, coplanar, triangular-surface known-answer prototype.

## Purpose

This executable extends the beam-axis transfer fixture into a bounded 3-D surface case. It transfers integrated aerodynamic traction loads from a source triangular surface mesh to a non-matching target triangular mesh.

The source discretization has five nodes and four triangles (a centre-fan). The target has four nodes and two triangles (a diagonal split). The two meshes represent the same planar square but have different topology and node counts.

A linear traction vector is specified at source nodes. Each source triangle is integrated using three symmetric degree-two barycentric quadrature points. For a linear traction field, this quadrature exactly integrates resultant force and the first moment of the traction field over a planar triangle. Each integrated quadrature force is then distributed to a containing target triangle using barycentric weights.

Because the quadrature point is reconstructed by the target barycentric coordinates, distributing a point force F using the same weights preserves:
- resultant force: sum of mapped nodal forces equals the source force;
- moment about the declared origin: sum of x_i cross (lambda_i F) equals x_q cross F;
- virtual work for the target finite-element interpolation: F dot (sum lambda_i u_i) equals sum (lambda_i F) dot u_i;
- affine displacement reproduction over the target linear triangular elements.

A deterministic first-face tie-break is used for points on shared edges. Points outside target support or off the target surface are rejected. Degenerate target triangles are rejected.

## Run locally

From repository root:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/conservative-surface-transfer-oracle/Cargo.toml -- --check
cargo test --manifest-path tools/civil-aero-benchmark/conservative-surface-transfer-oracle/Cargo.toml
cargo run --quiet --manifest-path tools/civil-aero-benchmark/conservative-surface-transfer-oracle/Cargo.toml > /tmp/aero-surface-transfer.json
python3 tools/civil-aero-benchmark/conservative-surface-transfer-oracle/verify_independent.py \
  --rust-output /tmp/aero-surface-transfer.json
~~~

Running the Python verifier without --rust-output validates the Python implementation only and reports rust_python_boundary=not_run.

## Frozen known-answer case

| Quantity | Expected |
|---|---:|
| Source nodes / triangles | 5 / 4 |
| Target nodes / triangles | 4 / 2 |
| Quadrature load packets | 12 |
| Resultant force | [1085, -530, 120] N |
| Resultant moment about origin | [62.5, -60.8333333333, -810] N m |
| Virtual work | 2.0794166666666665 J |
| Force/moment residual | Floating-point roundoff only |
| Affine displacement interpolation error | Floating-point roundoff only |

The independent Python implementation re-integrates the source traction, repeats point location and barycentric distribution, computes force/moment resultants, evaluates virtual work, and checks a whole-interface rotation-plus-translation transformation.

## Hostile controls

- source quadrature point outside target support;
- source quadrature point off target surface;
- degenerate target triangle;
- source traction count mismatch;
- non-finite source traction;
- deterministic target facet assignment checked by the independent implementation.

## Claim boundary

This is **not** a general non-matching mesh transfer library. It currently supports only:
- planar, coplanar source and target triangular meshes;
- linear vector traction at source nodes;
- a three-point triangle quadrature rule;
- point-load distribution through target linear-triangle barycentric coordinates.

It does not yet support curved/non-coincident surfaces, pressure-to-normal construction, free couples, quadrilateral or higher-order elements, gaps/partial overlap, mortar/dual-mortar projection, RBF interpolation, displacement transfer from structure to CFD, large deformation, unsteady FSI, CFD/FEM solution validation, or aircraft-specific structural validity.

The method must not be generalized from this case by assertion. Next comparisons belong in #4112: nearest-neighbour baseline, compatible shape functions, RBF, conservative mortar/dual-mortar, and constrained minimum-correction mapping. Compare local distribution and virtual-work behavior in addition to global force/moment balance.

No aircraft feasibility, structural adequacy, flutter, fatigue, airworthiness or certification finding is emitted.
