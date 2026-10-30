# Matrix-free 2D active-phase operator and Jacobi-PCG

This standalone Rust utility implements the fixed-flag 2D linear block operator used by the implicit enthalpy flag-update method. It is a **linear-algebra kernel**, not a complete phase-change time integrator.

## Operator semantics

- Cell-centred rectangular grid with nonuniform dx/dy permitted.
- West cold Dirichlet boundary uses the half-cell conductance 2*dt/dx².
- East, north and south outer faces are insulated.
- Internal faces use dt/dx² or dt/dy².
- gamma>0 represents an active solid/liquid node with gamma=1/alpha_phase.
- gamma=0 represents a mixed node, which imposes Kirchhoff potential v=0.
- Couplings to mixed nodes vanish; the face conductance remains in the active-node diagonal.
- Mixed rows are identity rows, so the full masked operator is symmetric positive definite for the admitted positive gamma and boundary contract.

The production operator uses a stencil apply over flat SoA fields. It does not build or retain CSR/COO sparse storage. Jacobi-preconditioned conjugate gradients use preallocated vectors for the current prototype; workspace reuse and fused kernels are future optimisation tasks.

## Independent checks

The test-only dense builder independently enumerates the 5-point stencil and Gaussian elimination solves tiny systems. Tests compare:
- matrix-free action vs dense matrix-vector products;
- symmetry and positive energy;
- PCG solution vs a known vector and tiny dense solve;
- exact zero potential for mixed nodes;
- invalid geometry/masks/nonfinite input and iteration-budget rejection.

## Build and run

    rustc --edition=2024 -D warnings --test tools/casting-phase-reference/matrix_free_2d.rs -o /tmp/matrix-free-2d-tests
    /tmp/matrix-free-2d-tests

    rustc --edition=2024 -D warnings tools/casting-phase-reference/matrix_free_2d.rs -o /tmp/matrix-free-2d
    /tmp/matrix-free-2d --verify
    /tmp/matrix-free-2d --csv

The CSV storage estimate includes eight f64 fields and one byte per cell. It is a transparent working-array estimate, not measured peak RSS.

## Next steps

1. Integrate the fixed-flag solver with latent-component recovery and consistency updates.
2. Add phase-aware geometric multigrid preconditioning and compare iteration growth with mesh size.
3. Verify 2D planar extrusion against the 1D exact Stefan reference, then use the arbitrary-dimensional radial Stefan benchmark for genuine curvature.
4. Add finite-domain materials and heterogeneous conductivity only after exact boundary and flux semantics are separately verified.
5. Add GPU kernels only after the CPU operator is independently qualified.

## Claim ceiling

No real casting, 2D phase change, curved front, GPU performance or production benefit is demonstrated by this linear-operator kernel. The current grid fixtures only test the algebraic operator and PCG.
