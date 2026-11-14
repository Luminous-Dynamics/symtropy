# Exact duplicate-contact Delassus rank and multiplier witness

**Research parent:** [PHYS-CONTACT-RANK-001 #3198](https://github.com/Luminous-Dynamics/symtropy/issues/3198)  
**Transition atlas:** [#3196](https://github.com/Luminous-Dynamics/symtropy/issues/3196)  
**Multi-contact solution sets:** [#3187](https://github.com/Luminous-Dynamics/symtropy/issues/3187)  
**Benchmark authority:** [#3183](https://github.com/Luminous-Dynamics/symtropy/issues/3183)  
**Evidence authority:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460)

## Claim boundary

This is a test-only exact integer witness for a singular but valid contact operator. It does not implement a general singular contact solver, arbitrary frictional rank-deficient contact handling, or a production contact-manifold deduplicator.

It proves a deliberately small but important distinction: a continuum of locally admissible contact multiplier allocations can produce one unique generalized impulse, while another vector in the same algebraic nullspace class can violate the local friction law.

## Physical construction

Use three generalized translational degrees of freedom with \(M^{-1}=I_3\). There are two duplicate 3D contact frames, so

\[
J=\begin{bmatrix}I_3\\I_3\end{bmatrix},
\qquad
W=JM^{-1}J^T
=\begin{bmatrix}I_3&I_3\\I_3&I_3\end{bmatrix}.
\]

The matrix is positive semidefinite because for \(x=(x_1,x_2)\),

\[
x^T W x=\|x_1+x_2\|^2\ge0.
\]

The leading 3x3 minor is the identity and the last three rows duplicate the first three, so

\[
\operatorname{rank}(J)=\operatorname{rank}(W)=3<6.
\]

This is a physical redundancy, not an invalid mass matrix. The contact coordinates are duplicated.

Choose

\[
v_{\mathrm{free}}=(-1,0,0)^T,\qquad
q=Jv_{\mathrm{free}}=(-1,0,0,-1,0,0)^T.
\]

Both contacts are frictionless (\(\mu_1=\mu_2=0\)); each contact therefore requires its two tangential impulse coordinates to be zero.

## Continuum of multiplier allocations

For any real \(t\in[0,1]\), choose

\[
p(t)=(t,0,0,1-t,0,0)^T.
\]

Then

\[
Wp(t)=(1,0,0,1,0,0)^T,
\qquad q+Wp(t)=0.
\]

Both normal impulses are nonnegative, both normal post-contact velocities are zero, and all tangential impulses are zero. Thus every member of this segment satisfies the two frictionless Signorini contact laws.

But the generalized impulse is independent of \(t\):

\[
J^T p(t)=(1,0,0)^T.
\]

The post-impact generalized velocity is therefore uniquely

\[
v_{\mathrm{free}}+M^{-1}J^Tp=(0,0,0)^T.
\]

The two multiplier coordinates are not unique; the body-level impulse and velocity outcome are.

The exact kinetic-energy change is also independent of the allocation:

\[
\Delta K=p^Tq+\tfrac12p^TWp=-1+\tfrac12=-\tfrac12.
\]

The Rust test enumerates five dyadic points \(t\in\{0,1/4,1/2,3/4,1\}\) using integer numerators with common denominator four. The parametric identities above establish the full segment.

## Why quotienting by the body impulse is insufficient

The nullspace contains

\[
n_t=(0,1,0,0,-1,0)^T,
\quad J^Tn_t=0,\quad Wn_t=0.
\]

Therefore \(p\) and \(p+n_t/4\) have the same generalized impulse and the same contact velocity. Yet the shifted vector has nonzero tangential impulse at both contacts. Since \(\mu=0\), it violates both local Coulomb disks.

Consequently, body-level equivalence

\[
p\sim p'\iff J^T(p-p')=0
\]

is useful for identifying physical impulse outcomes, but it cannot be used to discard the contact-local admissibility constraints. A correct solver must preserve the admissible representatives in each equivalence class.

## Exact tests

The Rust module checks:

- exact Gram construction \(W=JJ^T\);
- rank three from duplicate rows and a nonzero identity minor;
- positive semidefiniteness through the exact Gram quadratic identity;
- three independent null directions and their zero generalized impulse;
- the five dyadic samples of the admissible normal-allocation segment;
- zero post-contact velocity, nonnegative normal impulses, local zero-friction disks, and common generalized impulse;
- exact energy identity;
- a nullspace shift that preserves body impulse but violates local friction;
- rejection of normal allocations outside the nonnegative segment.

All matrix and impulse arithmetic in the witness is integer arithmetic. It does not rely on floating singular-value thresholds, pseudoinverses, epsilon regularization, or an external numerical library.

## Next step

Generalize the fixture from duplicate rows to exact rational row dependencies, then to interval rank bounds for uncertain Jacobians. The general path must distinguish:

1. a physical body-level outcome;
2. the admissible contact-multiplier set representing that outcome;
3. rank-changing parameter strata;
4. local Coulomb-cone feasibility within a nullspace equivalence class.

A singular Schur block must be handled with verified range compatibility and nullspace evidence, or produce a typed inconclusive result. A pseudoinverse is not a substitute for those proofs.
