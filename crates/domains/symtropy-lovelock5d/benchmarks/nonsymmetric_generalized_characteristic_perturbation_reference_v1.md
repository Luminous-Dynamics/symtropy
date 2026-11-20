# Non-symmetric generalized characteristic perturbation control

**Owner:** [#6354 BH-5D-EGB-ROTATING-SYMBOL-MIXING-001](https://github.com/Luminous-Dynamics/symtropy/issues/6354)

**Status:** executed standard-library method control. This is not an EGB principal-symbol result.

## Why a second perturbation control is needed

The symmetric-matrix control tests a degenerate Hermitian block. A spacetime principal-symbol pencil need not be self-adjoint in the coordinate or physical quotient basis, and a positive symmetrizer cannot be assumed before it is constructed and bounded.

For a generalized eigenvalue problem
\[
A(\epsilon)r=\omega(\epsilon)B(\epsilon)r,
\]
with a semisimple degenerate eigenvalue \(\omega_0\), normalized left and right bases \(L^T R=I\), the first-order splitting is given by the eigenvalues of
\[
L^T(A_1-\omega_0B_1)R
\]
when \(B_0=I\) on the degenerate block. More generally the restricted \(B_0\) pairing must be included. This is biorthogonal perturbation theory, not diagonal inspection in an arbitrary basis.

## Executed control

The 2x2 pencil uses \(A_0=2I\), \(B_0=I\), with non-symmetric \(A_1,B_1\). The effective matrix is
\[
M=A_1-2B_1=
\begin{pmatrix}0.5&2.0\\0.1&-0.2\end{pmatrix}.
\]
A deliberately non-orthogonal basis change \(R\) and its left inverse \(L^T=R^{-1}\) preserves the two first-order shifts. Across \(\epsilon=0.1,0.05,0.025,0.0125\), the maximum direct generalized-eigenvalue error divided by \(\epsilon^2\) is 0.1011. The basis-covariance defect is \(1.23\times10^{-15}\).

The diagonal-only shortcut is basis-dependent: the canonical diagonal is \((-0.2,0.5)\), while the transformed diagonal is \((-4.2,4.5)\). Neither reproduces the invariant first-order shifts \((-0.41789,0.71789)\). The script rejects this shortcut.

## EGB application contract

When computing the rotating EGB symbol:

1. Declare the generalized pencil and its covector/frequency convention.
2. Establish the physical gauge/constraint quotient before selecting left/right vectors.
3. Verify the eigenvalue is semisimple; defective/Jordan cases require a different expansion and may show fractional-power (Puiseux) splitting.
4. Construct left and right nullspaces and their biorthogonal normalization; track conditioning.
5. Use a positive symmetrizer only if independently constructed and bounded over the domain.
6. Compare first-order predictions to direct small-spin spectrum calculations.
7. Return Unresolved when eigenvalue gaps close, the eigenspace becomes defective, or left/right conditioning cannot be bounded.

The semisimple assumption matters: near an exceptional point, ordinary linear-in-spin perturbation theory can fail. A future negative control should force a Jordan block and verify that the simple first-order method refuses the case.

## Claim ceiling

This control verifies the generalized-eigenvalue perturbation machinery and the basis dependence of a diagonal-only shortcut. It does not establish the EGB principal symbol, the physical polarization space, strong hyperbolicity, or the absence of defective modes in the target black-hole backgrounds.
