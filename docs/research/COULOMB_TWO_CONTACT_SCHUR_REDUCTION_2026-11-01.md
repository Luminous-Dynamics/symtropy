# Two-contact Coulomb Schur-complement regime reduction

**Parent witness:** [PR #3188](https://github.com/Luminous-Dynamics/symtropy/pull/3188)  
**Research issue:** [PHYS-MULTICONTACT-COULOMB-001 #3187](https://github.com/Luminous-Dynamics/symtropy/issues/3187)  
**Single-contact reference:** [PR #3168](https://github.com/Luminous-Dynamics/symtropy/pull/3168)  
**Evidence authority:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460)

## Paradigm shift: solve a regime-conditioned island, not six unrelated unknowns

For the two-contact block system

\[
\begin{bmatrix}u_1\\u_2\end{bmatrix}
=
\begin{bmatrix}q_1\\q_2\end{bmatrix}
+
\begin{bmatrix}A&B\\B^T&C\end{bmatrix}
\begin{bmatrix}p_1\\p_2\end{bmatrix},
\]

assume contact 2 is sticking, so its complete three-coordinate post-contact velocity is zero:

\[
u_2=q_2+B^Tp_1+Cp_2=0.
\]

When the principal block C is SPD, eliminate its impulse:

\[
p_2=-C^{-1}(q_2+B^Tp_1).
\]

The remaining contact then obeys a reduced 3D Signorini–Coulomb problem with

\[
W_{\mathrm{eff}}=A-BC^{-1}B^T,\qquad
q_{\mathrm{eff}}=q_1-BC^{-1}q_2.
\]

This is the Schur complement of the sticking contact block. It preserves the full cross-contact coupling; it is not equivalent to solving contacts independently.

## Implemented reference path

A test-only reducer:
1. checks finite input, symmetry and SPD of the full 6x6 W;
2. factors the sticking principal block C and refuses if it is not SPD;
3. evaluates the exact Schur block formulas in binary64 for this numerical candidate-generation lane;
4. calls the existing 3x3 single-contact reference for contact 1;
5. reconstructs contact 2's impulse from the eliminated equation;
6. rechecks each candidate against the original, unreduced 6x6 normal complementarity, contact-2 strict sticking disk, contact-1 declared regime (including the sliding KKT residual rather than incorrectly requiring zero tangential velocity), and full island kinetic energy;
7. deduplicates only at the numerical comparison tolerance.

## Frozen result target

The implementation is exercised against the existing 6x6 SPD witness with nonzero cross-contact blocks and three distinct solutions:
- contact 1 sticking / contact 2 sticking;
- contact 1 sliding / contact 2 sticking, first sliding branch;
- contact 1 sliding / contact 2 sticking, second sliding branch.

The regression compares reconstructed 6D impulses against all three frozen witness impulses, requires one sticking and two sliding contact-1 branches, checks full 6D post-velocity residual, and requires strictly dissipative island energy.

## Critical limitations

This is a **regime-restricted numerical candidate generator**, not a certified two-contact solution-set solver:
- it only considers the regime in which contact 2 is strictly sticking;
- the reduced single-contact solver is floating point and is not a formal root-completeness proof;
- the full 6x6 rechecks validate candidates but do not prove omitted branches do not exist;
- it does not yet handle two sliding contacts, contact 2 separating, stick/slip boundary continuation, uncertainty families, or more than two contacts.

The Schur complement is a decomposition tool, not a license to discard cross-contact blocks. For larger islands, regime-conditioned block elimination, sparse factorization and interval branch certification should be composed with explicit rank and conditioning guards.

## Benchmark implications

Compare the reduced path against direct full-island nonlinear search on the same fixed-regime corpus. Report:
- recovered known branches and false candidate rate;
- full 6D residual and energy error;
- reduction/factorization time versus direct solve time;
- sensitivity to Schur-block condition number;
- abstention when the full operator or C block is singular/ill-conditioned;
- strict rejection when a reconstructed eliminated-contact impulse lies outside its sticking disk;
- deterministic replay and contact-block permutation invariance.

No engine-speed or SOTA claim is made. All benchmark/evidence receipts remain subject to #460.


## Additional regime reduction: contact 2 strictly separating

A second regime-conditioned elimination now covers the case

\[
p_2=0,\qquad u_{n,2}=q_{n,2}+B^T_{n,:}p_1>0.
\]

There is no inverse of C in this branch. Solve the principal contact-1 problem using (A,q_1), reconstruct p=(p_1,0), then evaluate u=q+Wp with the original full 6x6 operator. The candidate is retained only when contact 2's full-island normal velocity is strictly positive and contact 1's declared separating/sticking/sliding conditions, full energy and residuals pass.

This is not equivalent to ignoring contact 2: its normal separating inequality depends on the cross-contact block B^T and the reconstructed p_1. The negative control makes q_{n,2}+B^T_{n,:}p_1 negative and requires the regime-restricted path to return no candidate / INCONCLUSIVE.

The frozen synthetic SPD fixture uses W=[[I,0.1I],[0.1I,I]], which has nonzero cross-contact coupling and remains positive definite. With q_2,n=0.25, it recovers contact-1 separating, sticking and sliding cases while contact 2 remains strictly separating. With q_2,n=-0.2, contact 2 cannot remain separating after the contact-1 impulse and the candidate is rejected.

### Coverage accounting

Together, the strict contact-2 sticking path in this note and the strict contact-2 separating path cover six of the nine two-contact regime assignments:

- contact 2 sticking + contact 1 separating/sticking/sliding;
- contact 2 separating + contact 1 separating/sticking/sliding.

The remaining three assignments require contact 2 to slide. They are not solved by these reductions. Neither path proves complete solution-set enumeration, and all candidates continue to be rechecked against the original unreduced system.

## Symmetric regime reductions: contact 1 separating or sticking

The same algebra applies after permuting the two contact blocks. The implementation now includes two mirrored test-only paths:

- **Contact 1 strictly separating:** permute contact blocks, solve the original contact-2-separating reduction, unpermute the impulse, then recheck \(p_1=0\) and \(u_{n,1}>0\) against the original full 6x6 system. A coupled fixture recovers contact 2 sliding while contact 1 separates; an inward post-contact normal velocity is rejected.
- **Contact 1 strictly sticking:** permute contact blocks and use the contact-2-sticking Schur complement to eliminate original contact 1. Unpermute and recheck the strict contact-1 disk interior, its zero post-contact velocity, contact-2 sliding KKT, and the original full-island energy. A coupled fixture recovers contact 1 sticking / contact 2 sliding.

Both fixtures use a nonzero \(0.1I\) cross-contact block. They are synthetic known-branch checks, not complete regime solvers or interval certificates.

### Updated two-contact regime coverage

The combined test-only candidate paths now cover **eight of the nine** basic assignments:

1. contact 2 sticking + contact 1 separating/sticking/sliding (#3194);
2. contact 2 separating + contact 1 separating/sticking/sliding (#3209);
3. contact 1 separating + contact 2 sliding (symmetric reduction);
4. contact 1 sticking + contact 2 sliding (symmetric Schur reduction).

The remaining assignment is **contact 1 sliding / contact 2 sliding**. It is the genuinely coupled two-sliding branch and is still open. No global completeness or no-solution claim follows from the eight restricted paths.


## Both-sliding coupled witness: the ninth regime assignment

A final frozen witness exercises the simultaneous sliding/sliding assignment without claiming a general double-sliding solver.

Choose

\[
W=\begin{bmatrix}I&\rho I\\\rho I&I\end{bmatrix},\quad
\rho=\frac14,\quad
\mu_1=\mu_2=\frac12,
\]

\[
q=\left(-\frac54,\frac54,0,-\frac54,\frac54,0\right)^T,\qquad
p=\left(1,-\frac12,0,1,-\frac12,0\right)^T.
\]

The full operator is SPD with eigenvalues \(3/4\) and \(5/4\), each of multiplicity three, and its cross-contact blocks are nonzero.

The original full-island equations give

\[
u=q+Wp=\left(0,\frac58,0,0,\frac58,0\right)^T.
\]

Both normal impulses are positive and both normal post-velocities are zero. Both tangential impulses are exactly on their circular Coulomb boundaries:

\[
\mu_k^2\lambda_{n,k}^2-\|j_{t,k}\|^2=0.
\]

Both sliding KKT multipliers are \(\tau_1=\tau_2=5/4>0\), so \(u_{t,k}+\tau_kj_{t,k}=0\) at each contact. The full-island energy change is exactly

\[
\Delta K=p^Tq+\tfrac12p^TWp=-\frac{35}{16}? 
\]

The Rust test is the authoritative executable arithmetic check; its frozen expected value is \(-2.1875=-35/16\).

This adds a known branch witness for the only previously uncovered regime assignment. It does not enumerate all two-sliding branches, prove uniqueness, or replace the bivariate direction-root isolation problem. It completes **known-witness coverage across all nine basic regime assignments**, not complete solution-set coverage.
