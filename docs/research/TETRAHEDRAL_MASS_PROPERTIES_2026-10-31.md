# Tetrahedral volume mass properties

**Status:** analytic, piecewise-constant-density tetrahedral integration prototype.
**Date:** 2026-10-31
**Implementation branch:** `research/tetrahedral-mass-properties-2026-10-31`.

## Purpose

A mesh-derived rotational inertia should come from the volume and density model, not from a guessed primitive radius or a hand-entered tensor. This increment adds a direct integration path for tetrahedralized 3D solids.

It accepts tetrahedra as volume cells (four vertices plus one constant density per cell). It does not tetrahedralize a surface mesh, detect overlapping cells, repair non-manifold geometry, or infer material density.

## Mathematical formulation

For tetrahedron vertices (p_0,p_1,p_2,p_3), define:

[
V=\frac{1}{6}\left| (p_1-p_0)\cdot
((p_2-p_0)\times(p_3-p_0))\right|,
\qquad m=\rho V.
]

The tetrahedron center of mass is:

[
c=\frac{p_0+p_1+p_2+p_3}{4}.
]

For centered vertex offsets (q_i=p_i-c), the exact centered second moment of a uniform tetrahedron is:

[
S_{\mathrm{tet}}=\int (x-c)(x-c)^T\,dm
=\frac{m}{20}\sum_{i=0}^{3}q_iq_i^T.
]

For a set of non-overlapping tetrahedra, the global mass and center are:

[
M=\sum_k m_k,\qquad
C=\frac{1}{M}\sum_k m_kc_k.
]

The combined centered second moment follows by parallel-axis shifting:

[
S=\sum_k\left(S_k+m_k(c_k-C)(c_k-C)^T\right).
]

This formulation preserves off-diagonal moments for asymmetric solids and gives the full bivector plane-inertia operator without reducing it to a diagonal 3D approximation.

## Numerical treatment

- Coordinates are first shifted by the first input vertex. The local tetrahedron moments and the combined COM are accumulated relative to this reference, then translated back once at the end. This reduces loss of small CAD-scale offsets for geometry far from the world origin.
- Degeneracy is tested using a scale-relative volume threshold based on the longest of the tetrahedron's six edges.
- Negative/non-finite density, non-finite vertices, overflowed volume/moments, degenerate cells and empty input fail explicitly.
- Total mass is retained in the resulting `BivectorInertia<3>`, so the existing body-mass consistency check can reject mismatches.
- Tetrahedron orientation does not change the integrated volume or moments; absolute signed volume is used.

## API

```rust
let properties = BivectorInertia::<3>::from_tetrahedra(
    tetrahedra,                 // [four world-space vertices, density] per cell
    1e-12,                      // relative degeneracy tolerance
)?;
let mass = properties.total_mass;
let center_of_mass = properties.center_of_mass;
let rotational_inertia = properties.inertia;
```

The public `TetrahedralMassProperties` returns mass, global COM and the centered authoritative inertia operator.

## Regression coverage

Source tests cover:
- a unit right tetrahedron at density 6: mass 1, COM (1/4,1/4,1/4), diagonal second moments 3/80 and off-diagonal moments -1/80;
- large translation (coordinates around 10^12) while preserving centered moments;
- piecewise-constant densities across multiple tetrahedra;
- invariance to tetrahedron ordering;
- invariance to vertex orientation reversal;
- global rotation covariance of COM and second-moment matrix;
- degenerate-volume and invalid-density rejection.

The source has not been promoted until exact-head formatting, Clippy, compilation and tests pass.

## Scope limits and next work

1. Integrate/import a validated tetrahedral mesh from a CAD or mesh pipeline, preserving cell connectivity and checking that tetrahedra are non-overlapping and consistently represent the intended solid.
2. Record density units, material assignment, mesh hash, tetrahedralization algorithm/version, COM frame, source coordinates/units and numerical integration error as mass-property provenance.
3. Add variable density fields. For density varying within a tetrahedron, use exact polynomial integration or adaptive quadrature; do not silently treat the sampled density as constant.
4. Add surface-to-volume tetrahedralization quality checks, inverted/near-zero cell detection, boundary manifold validation and volume reconstruction comparisons.
5. Compare integrated mass/COM/inertia against CAD-kernel analytic primitives and at least one independent mesh mass-property implementation.
6. Add convergence studies as tetrahedralization is refined, separately tracking mass, COM and each second-moment component.

This is the exact volume integral for the supplied piecewise-uniform tetrahedral decomposition, not a claim that the decomposition itself matches the intended CAD solid. Geometry provenance and tetrahedralization correctness remain independent validation obligations.
