# Independent P1 triangle element oracle

This standalone Rust crate independently implements the constant-strain three-node
triangle (CST) element for small-strain, isotropic plane strain. It does **not** run
CalculiX and is not a general-purpose or production FEM solver.

## Verification scope

The element stiffness is built from the analytic shape-function gradients,
engineering-shear strain-displacement matrix, plane-strain constitutive matrix,
positive area, and declared thickness. This implementation is separate from
CalculiX's element implementation and is intended as a second line of algebraic
verification for the frozen PHYS-CCX-MMS-P1-001 case.

Tests cover:

- affine displacement patch on a skew triangle, including exact strain and stress;
- symmetric stiffness and the two translation plus one infinitesimal-rotation null modes;
- invariance under reversed triangle vertex order, with the matching DOF permutation;
- nonnegative strain energy for a deterministic displacement vector;
- relative matrix-versus-constitutive energy identity;
- a deliberately corrupted stiffness entry that must fail the energy identity control;
- fail-closed rejection of degenerate geometry, non-finite coordinates/displacements,
  and invalid thickness;
- global assembly of a 4-by-4 structured patch: the affine strain is exact element by
  element and interior assembled forces cancel to roundoff;
- an irregular four-triangle star patch, checking constant strain and cancellation at
  its interior node;
- metamorphic checks that uniform geometry scaling preserves CST stiffness and that
  stiffness scales linearly with thickness;
- rotation covariance of recovered strain and stress for isotropic plane strain;
- a scale-invariant triangle mean-ratio quality measure and a sliver-geometry sweep;
- explicit rejection immediately below the frozen relative floating-point degeneracy
  floor, and acceptance of a represented triangle above that floor;
- equilateral-triangle mean-ratio normalization to one, with only few-ulp overshoot
  clamped after a finite/upper-bound sanity check.

## Geometry conditioning receipt

`triangle-conditioning-probe` emits the versioned `ccx-mms-p1-geometry-conditioning-v2`
deterministic JSON receipt over triangles whose height decreases from 1 to 1e-4. It
reports the mean-ratio quality

`q = 2 * sqrt(3) * abs(det) / sum(edge_length_squared)`

the dimensionless geometry-gradient amplification proxy

`a = longest_edge * max_i(norm(grad(N_i)))`,

and the local element spectral condition number. A symmetric Jacobi eigensolver computes
the six-by-six CST stiffness eigenvalues; three rigid-body modes are classified using a
scale-relative floating-point threshold, and the ratio of largest to smallest positive
deformational eigenvalue is reported. This is a local element metric, **not** the
condition number of the globally assembled, boundary-constrained FEM system.

The frozen sweep checks that quality decreases strictly while gradient amplification and
local non-rigid spectral condition increase. Unit tests check three near-zero rigid modes,
three positive deformational modes, a monotonic spectral-condition trend for slivers, and
a final local element condition number above 1e7. Quality is invariant under rigid rotation
and uniform scaling. Tests also verify spectral trace and trace-of-square invariants,
three rigid modes, three positive modes, and invariance of the non-rigid condition number
under rigid rotation and vertex winding reversal. The spectral estimate is useful precisely
because it comes from the element stiffness matrix rather than being only a geometry-
derived proxy, but its rigid-mode classification threshold and eigensolver accuracy still
require independent tests. The element constructor rejects a triangle when
`abs(det) <= 64 * f64::EPSILON * longest_edge_squared`; that is a floating-point
degeneracy floor, not a general mesh-quality recommendation. Sliver elements above the
floor are measured rather than automatically declared adequate for engineering analysis.

## Frozen probe gates

`triangle-patch-probe` emits deterministic JSON and source/checkout Git SHAs. The
probe requires:

- absolute strain-component error <= 1e-12;
- maximum absolute stiffness-symmetry defect <= 1e-12;
- maximum relative matrix-versus-constitutive energy defect <= 1e-12;
- maximum absolute assembled interior-force component <= 1e-12;
- maximum absolute rigid-mode internal force component <= 1e-12.

The output explicitly contains `solver_executed: false` and
`qualification_pass: false`. Even a passing element oracle is **not** a CalculiX run,
not an MMS convergence result, not physical validation, and not proof of superiority
to another solver. The values and tolerances are nondimensional for the declared
unit-square patch and E=1 material.

## Why this benchmark

The patch test verifies that a complete linear displacement field generates constant
strain and stress exactly and that the discretized internal forces balance at interior
nodes. It is complementary to the non-affine manufactured-solution mesh-convergence
test: the patch test checks formulation completeness and algebraic consistency, while
the MMS test checks observed asymptotic error rates from a known smooth solution.

## References

- NAFEMS, *Code Verification Exemplars in Computational Solid Mechanics* (R0135,
  3 March 2026): https://www.nafems.org/publications/resource_center/r0135/
- Sandia Sierra/SM theory manual, element patch test and completeness condition:
  https://www.sandia.gov/files/sierra/SM_Theory_5_30/main/element_basics.html
- Sorgente et al., *A Survey of Indicators for Mesh Quality Assessment* (2023):
  https://doi.org/10.1111/cgf.14779
- Sandia CUBIT, triangular element quality metrics:
  https://cubit.sandia.gov/files/cubit/15.8/help_manual/WebHelp/mesh_generation/mesh_quality_assessment/triangular_metrics.htm
- Abaqus Verification Guide, element verification tests and rigid-body modes:
  https://ceae-server.colorado.edu/v2016/books/ver/ch01s01abo01.html
