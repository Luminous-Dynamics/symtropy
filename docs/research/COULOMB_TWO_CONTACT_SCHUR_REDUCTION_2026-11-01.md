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