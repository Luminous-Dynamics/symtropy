# MAG-FINITE-VECTOR-001 — full-vector analytical cuboid field reference

**Prepared:** 2026-11-24  
**Status:** analytic reference implementation; no Rust test or CI result claimed yet  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/7376  
**Axial predecessor:** https://github.com/Luminous-Dynamics/symtropy/pull/7366

## 1. Purpose and novelty boundary

The axial cuboid reference checks only Bz on the symmetry axis. The next useful verification target is the complete vector field at off-axis interior and exterior points.

Magpylib and magpylib_jax already implement full analytical cuboid fields. This project does not claim that the cuboid formula is new or that an AI optimizer or FEA kernel is novel. Its purpose is to establish a separately derived, inspectable Rust reference based on six rectangular surface-charge integrations, with explicit region and boundary semantics.

## 2. Face-integral derivation

For homogeneous prescribed polarization J in tesla, each face with outward unit normal n has magnetic surface charge sigma = J dot n. The field contribution in free space is

    B_face(r) = sigma/(4*pi) * integral_face (r-r')/|r-r'|^3 dA.

Choose a right-handed local face basis (u,v,n), half-widths A,B and observer coordinates (U,V,W). Evaluating the two tangential components by integrating one coordinate first gives asinh boundary terms:

    Iu = g(U-A) - g(U+A)
    Iv = h(V-B) - h(V+B)

The normal component is the signed rectangular solid angle:

    Omega = atan((U-A)(V-B)/(W R--))
            - atan((U+A)(V-B)/(W R+-))
            - atan((U-A)(V+B)/(W R-+))
            + atan((U+A)(V+B)/(W R++))

where R are the four observer-to-corner distances and atan denotes the principal arctangent of the ratio, not an unadjusted atan2 branch. The face integral vector is Iu*u + Iv*v + Omega*n. Sum all six outward-oriented faces. Add J only when the observer is strictly inside the cuboid to obtain B from the H-derived surface-charge field.

The implementation uses a common geometry scale before evaluating normalized face integrals. Asinh ratios use a stable finite-ratio/log fallback. A zero face-plane offset is rejected in this version.

## 3. Boundary semantics

The field's tangential one-sided limit at an ideal magnet face depends on whether the observer is inside or outside when J has a tangential component. Edges and corners have direction-dependent singular limits. v0.1 therefore rejects exact cuboid coordinate planes rather than inventing a side convention. This is a conservative restriction and excludes a few exterior points lying on extensions of those planes as well.

A later API can add explicit inside/outside side selection for face-interior positions, while continuing to reject edges/corners unless a precise direction-dependent limit is specified.

## 4. Independent reference fixtures

The machine-readable fixture contains four known vector-field cases for a non-cubic rectangular prism with mixed polarization: centre, off-axis interior, off-axis exterior, and mixed-octant exterior.

Expected values were obtained by a separate 2D tensor-product Gauss-Legendre numerical integration (220 nodes per axis) over each of the six charged faces, summing sigma/(4*pi)*(r-r')/|r-r'|^3 dA, then adding J only for interior observers. The Rust implementation evaluates closed-form asinh and arctangent expressions. This is a different numerical route; it is still a synthetic mathematical reference, not a physical measurement.

A previous axial case is also included as a cross-check against PR #7366. The implementation also tests central inversion, polarization linearity/reversal, geometric scaling and far-field convergence to an equivalent point dipole.

## 5. External analytical comparators

The independent comparison targets are Magpylib 6.1, based on published cuboid analytical solutions, and magpylib_jax, whose documented cuboid field uses eight-corner arctangent/logarithm expressions and explicitly marks edge singularities.

The face-integral representation is deliberately different. Future parity cases should compare exact dimensions, polarization, observer coordinates, B/H/J convention, interior/exterior condition and boundary exclusions. Library output is an analytical cross-check, not experimental validation.

## 6. References

1. Magpylib 6.1, https://magpylib.readthedocs.io/en/6.1.0/_pages/user_guide/guide_resources_01_physics.html
2. magpylib_jax cuboid equation model, https://magpylib-jax.readthedocs.io/en/latest/equations.html#cuboid
3. Camacho & Sosa (2013), https://www.scielo.org.mx/scielo.php?script=sci_arttext&pid=S0035-001X2013000200005

## 7. Claim ceiling

This is an analytical reference for a homogeneous cuboid with prescribed uniform polarization. It is not a nonlinear B-H solver, multi-material interface solver, FEM/BEM solver, physical measurement, or component qualification.
