# Defective characteristic eigenvalue: Jordan/Puiseux negative control

**Owner:** [#6354 BH-5D-EGB-ROTATING-SYMBOL-MIXING-001](https://github.com/Luminous-Dynamics/symtropy/issues/6354)

**Status:** executed finite-dimensional failure control only. No defective eigenvalue is claimed for an EGB background.

## The failure mode

The semisimple left/right perturbation formula is not valid at a defective eigenvalue. A two-dimensional Jordan block can split with a Puiseux series rather than an ordinary Taylor series. The minimal control is
\[
A(\epsilon)=
\begin{pmatrix}0&1\\0&0\end{pmatrix}
+\epsilon
\begin{pmatrix}0&0\\1&0\end{pmatrix}
=
\begin{pmatrix}0&1\\\epsilon&0\end{pmatrix}.
\]
Its exact eigenvalues are \(\omega_\pm=\pm\sqrt{\epsilon}\). At zero perturbation the algebraic multiplicity is 2 but the geometric multiplicity is 1; the eigenvalue is defective.

## Executed scaling check

For \(\epsilon=10^{-2},10^{-4},10^{-6},10^{-8}\), the script reproduces the exact square-root scaling. The normalized quantity \(|\omega|/\sqrt{\epsilon}\) is exactly one, while \(|\omega|/\epsilon\) grows from 10 to 10,000. Thus there is no finite linear-in-\(\epsilon\) eigenvalue slope to use in the semisimple first-order method.

The correct response is not to force a linear shift: detect the defect and switch to Jordan-chain/Puiseux analysis, or return Unresolved if that analysis is not available.

## Consequence for the EGB symbol work

At every background/covector, the principal-symbol pipeline must check more than root reality:

- algebraic versus geometric multiplicities;
- smallest singular value / rank of the physical pencil at candidate roots;
- left/right eigenvector conditioning and spectral projectors;
- eigenvalue gap and sensitivity under perturbation;
- whether a semisimple expansion is justified;
- whether characteristic eigenvectors remain complete and uniformly conditioned.

A defective physical characteristic block also fails the completeness requirement for strong hyperbolicity. But this toy Jordan block does not imply such a block exists in the target black-hole family.

## Claim ceiling

This is an executable exceptional-point failure control. It demonstrates why a semisimple first-order perturbation must abstain at a Jordan point. It does not establish the physical EGB symbol, any Jordan point in the EGB family, or a hyperbolicity classification.
