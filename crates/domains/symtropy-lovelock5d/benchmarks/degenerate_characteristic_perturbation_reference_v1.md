# Degenerate characteristic perturbation: executable polarization-mixing control

**Owner:** [#6354 BH-5D-EGB-ROTATING-SYMBOL-MIXING-001](https://github.com/Luminous-Dynamics/symtropy/issues/6354)

**Status:** executed symmetric-matrix algorithm control only. No rotating EGB principal symbol has been derived by this fixture.

## The methodological issue

A static spherical background can have degenerate physical polarization sectors. Rotation breaks part of the static symmetry and can mix modes that were degenerate. First-order characteristic shifts therefore cannot generally be read from diagonal entries in an arbitrarily chosen static polarization basis.

For a self-adjoint or symmetrized pencil
[
P(\epsilon)=P_0+\epsilon P_1+O(\epsilon^2),
]
let (Pi) project onto a degenerate eigenspace of (P_0). The first-order splittings are the eigenvalues of the complete projected block (Pi P_1\Pi), with the inner product or symmetrizer declared. The result must be invariant under a basis change inside that block.

A gravitational principal-symbol pencil may be non-symmetric before physical reduction or symmetrization. In that case use the appropriate left/right generalized-eigenvector perturbation problem; do not silently force a symmetric interpretation.

## Executed 3x3 control

The control uses (P_0=\operatorname{diag}(0,0,4)) and
[
P_1=\begin{pmatrix}0.2&0.6&0.25\\0.6&-0.1&-0.2\\0.25&-0.2&0.3\end{pmatrix}.
]
The two-dimensional degenerate block has first-order shifts ((-0.5684658438,0.6684658438)). Across four decreasing spin-like parameters, the maximum direct-spectrum prediction error divided by (epsilon^2) is (2.56\times10^{-2}). Basis rotations leave the shifts invariant to (1.11\times10^{-16}). The diagonal-only shortcut has error proportional to (epsilon), an explicit negative control.

## How this becomes an EGB calculation

1. Derive the full five-dimensional EGB principal symbol from the pinned action and curvature conventions. Do not differentiate the cohomogeneity-one stationary ODE residual and call it a spacetime principal symbol.
2. Fix a horizon-regular orthonormal frame and geometric covector. Construct the zero-spin and first-order-spin symbol.
3. Separate pure gauge/constraint modes from physical polarizations; prove the zero-spin symmetry decomposition.
4. Project into each degenerate physical block and diagonalize the complete block. If no positive symmetrizer is established, use a left/right generalized-eigenvalue formulation.
5. Check the exact Myers–Perry/GR limit, static Boulware–Deser reference, physical-subspace leakage, eigenpair backward error, eigenvalue gaps and eigenbasis conditioning.
6. Compare perturbative shifts with direct small-spin symbol evaluations and require the expected remainder scaling.

The static effective-metric sectors are an H1 control only. They are not rotating formulas. The 2026 black-string hyperbolicity result is a different background and symmetry reduction.

## Scorecard and claim ceiling

Measure prediction error divided by (epsilon^2), diagonal-only error divided by (epsilon), basis-covariance defect, eigenpair backward error, physical-subspace leakage, minimum spectral gap, maximum eigenbasis condition number and horizon-frame convergence.

This control demonstrates a numerical method and failure mode. It does not derive the EGB physical polarization subspace, rotating symbol, strong hyperbolicity, or any nonzero-spin classification.
