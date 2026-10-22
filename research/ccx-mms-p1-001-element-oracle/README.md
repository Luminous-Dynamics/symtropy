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

## Constrained global stiffness conditioning

`triangle-global-condition-probe` emits the deterministic
`ccx-mms-p1-global-condition-v1` receipt for uniform triangular meshes with 2, 3, 4,
5, and 6 cells per side. It assembles the global stiffness matrix from this crate's
independent CST element kernel, removes both displacement DOFs for every boundary node,
and computes the spectrum of the resulting small positive-definite matrix using a dense
symmetric Jacobi iteration. The recorded free-DOF counts are 2, 8, 18, 32, and 50.

An independent NumPy calculation gives constrained global condition numbers of roughly
1.76923, 5.06740, 9.77110, 15.8533, and 23.3044 for this frozen mesh family. These are
cross-check values, not Rust output. The probe's gate is qualitative: all minimum
eigenvalues must be positive, and the global condition number must increase strictly with
refinement and exceed 20 on the final small mesh. The five coarse points illustrate the
trend; they are not enough to establish a precise asymptotic rate.

Unlike the local element spectrum, this is a condition number for a **globally assembled,
boundary-constrained stiffness matrix**. However, it is assembled from this oracle's
own element kernel rather than from CalculiX output, and no displacement solve is
performed. It therefore checks global assembly/conditioning behavior but is not a
CalculiX matrix, a solver benchmark, or a qualification result. The spectral growth is
consistent with the mesh-quality literature's observation that the condition number of
elliptic finite-element systems worsens with mesh refinement; performance must still be
measured with an actual solve and matched accuracy.

## Global CG and Jacobi-preconditioned CG diagnostic

`triangle-global-pcg-probe` emits a deterministic
`ccx-mms-p1-global-pcg-v1` receipt comparing two independent dense linear solves on
the same oracle-assembled, boundary-constrained CST stiffness matrices:

- standard Conjugate Gradient (CG);
- Jacobi-preconditioned Conjugate Gradient (PCG), using the positive diagonal of the
  assembled matrix.

For each mesh with 2, 3, 4, 5, and 6 cells per side, the probe records both the
unpreconditioned spectral condition number and the spectrum of the **symmetric**
Jacobi-scaled matrix (D^{-1/2} K D^{-1/2}), along with CG/PCG iteration counts and
the independently recomputed true relative residual (|b-Kx|_2 / |b|_2).
Both solvers use the same deterministic, nonzero synthetic right-hand side and frozen
(10^{-10}) true-residual tolerance. The reported condition numbers are properties of
the reduced matrix assembled by this research oracle, not a CalculiX matrix.

The probe fails closed for invalid/non-finite matrices, non-positive diagonals, invalid
search curvature, eigensolver exhaustion, or a residual above tolerance. Unit tests
include a known small SPD system, a non-positive-diagonal negative control, and repeatable
mesh-family diagnostics. CI executes the binary twice and requires byte-identical receipts
plus the exact schema, five resolutions, expected free-DOF counts, finite condition
estimates, bounded true residuals, and bounded iteration counts.

**Interpretation boundary:** the probe does not assume Jacobi PCG must outperform CG
on every right-hand side or mesh; it measures both under equal stopping criteria so
iteration/conditioning trends are evidence, not a baked-in victory condition. A dense
Jacobi eigensolver and small matrices are appropriate for this independent diagnostic,
not a scalable solver benchmark. It performs no physical FEM solve, measures no
discretization error, and keeps `solver_executed=false` and
`qualification_pass=false`. Later performance comparisons must use an actual pinned
solver, matched accuracy, recorded hardware/build provenance, repeated wall time, and
peak memory.

This diagnostic follows the distinction made in mature Krylov-solver documentation:
preconditioning effectiveness depends on spectral behavior, and CG requires symmetric
positive-definite system and preconditioner. Reference: PETSc, [KSPCG documentation](https://petsc.gitlab.io/petsc/main/manualpages/KSP/KSPCG/).

## Near-incompressibility material sweep

`triangle-material-conditioning-probe` emits a deterministic
`ccx-mms-p1-material-conditioning-v1` receipt over Poisson ratios 0.3, 0.49, 0.4999,
and 0.49999, with Young's modulus fixed to 1. Each record includes `lambda/mu`, the
smallest and largest positive element-stiffness eigenvalues, and the local non-rigid
spectral condition number. The receipt requires the Lamé ratio and local condition
number to increase strictly across the frozen parameter sequence and explicitly records
`volumetric_locking_measured: false`.

An independent NumPy calculation for the current skew triangle produced local condition
numbers of approximately 2.76347, 52.3931, 5,230.02, and 52,299.4 for those four values.
These figures are mathematical cross-checks, not Rust output. This is a local spectral
conditioning study: **it does not measure volumetric locking**. To measure locking, the
next protocol must solve a global near-incompressible displacement benchmark and track
response/displacement error as `nu -> 0.5`, ideally against a stable mixed formulation or
a trusted reference solution. A single element's eigenvalue ratio cannot establish that
global approximation pathology.

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
