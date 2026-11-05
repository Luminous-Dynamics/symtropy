# Exact coupled Coulomb stick/slip transition witness

**Research issue:** [PHYS-CONTACT-TRANSITION-ATLAS-001 #3196](https://github.com/Luminous-Dynamics/symtropy/issues/3196)  
**Related multi-contact work:** [#3187](https://github.com/Luminous-Dynamics/symtropy/issues/3187), [two-contact Schur reduction #3194](https://github.com/Luminous-Dynamics/symtropy/pull/3194)  
**Benchmark authority:** [#3183](https://github.com/Luminous-Dynamics/symtropy/issues/3183)  
**Evidence authority:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460)

## Purpose and claim boundary

This note freezes an analytic regime-transition witness for two simultaneously loaded 3D point contacts. It exercises a stick/slip transition in a symmetric positive-definite 6x6 Delassus matrix with a nonzero cross-contact block.

The Rust module is test-only. It does not modify production contact semantics, search for impulses, enumerate the complete solution set, or certify arbitrary parameter boxes. It verifies one known mixed sliding/sticking branch, the exact transition point, and a strictly sticking branch on the other side.

## Exact coupled fixture

Use contact ordering (n1,t11,t12,n2,t21,t22) and

\[
W=\begin{bmatrix}I&\frac14 I\\ \frac14 I&I\end{bmatrix},
\qquad
p^*=(10,3,4,10,3,4)^T,
\qquad
q=-Wp^*.
\]

In full,

\[
q=(-25/2,-15/4,-5,-25/2,-15/4,-5)^T.
\]

The cross-contact block has Frobenius norm \(\sqrt{3}/4\). The eigenvalues of the complete matrix are \(3/4\) and \(5/4\), each with multiplicity three. This is an SPD coupled island, not a block-diagonal pair.

Hold \(\mu_2=1\), vary \(\mu_1\), and use the impulse law

\[
u=q+Wp,\quad
\lambda_k\ge0,\quad u_{n,k}\ge0,\quad \lambda_k u_{n,k}=0,
\quad \|j_k\|\le\mu_k\lambda_k.
\]

For strict sticking, \(u_{t,k}=0\) and the squared disk margin is positive. For strict sliding, the disk boundary is active and \(u_{t,k}+\tau_k j_k=0\) with \(\tau_k>0\).

## The transition is exact

The all-sticking impulse is \(p^*\), independent of \(\mu_1\). Contact 1's tangential impulse has norm five and normal impulse ten, so its exact friction threshold is

\[
\mu_{1,*}=\frac{\sqrt{3^2+4^2}}{10}=\frac12.
\]

For \(\mu_1>1/2\), \(p=p^*\) has \(u=0\) and strict disk margins

\[
m_1=100\mu_1^2-25>0,\qquad m_2=100-25=75>0.
\]

At \(\mu_1=1/2\), the same impulse has \(u=0\), \(m_1=0\), \(m_2=75\), and the limiting sliding multiplier is \(\tau_1=0\). This is the transition stratum. It is neither strictly sticking nor strictly sliding.

For \(0<\mu_1<1/2\), the following mixed branch satisfies the original full-island equations:

\[
p_1=(10,6\mu_1,8\mu_1),
\qquad
p_2=(10,15/4-3\mu_1/2,5-2\mu_1).
\]

The post-contact velocities are

\[
u_{n,1}=u_{n,2}=0,\qquad u_{t,2}=0,
\]

\[
u_{t,1}=
\left(\frac{45}{16}(2\mu_1-1),
      \frac{15}{4}(2\mu_1-1)\right).
\]

Contact 1 lies on its Coulomb boundary because
\(\|j_1\|^2=(6\mu_1)^2+(8\mu_1)^2=100\mu_1^2\). Its sliding multiplier is

\[
\tau_1=\frac{15}{32\mu_1}(1-2\mu_1)>0,
\]

which gives \(u_{t,1}+\tau_1j_1=0\). Contact 2 remains strictly inside its \(\mu_2=1\) disk. The tests also recompute the full-island kinetic-energy change

\[
\Delta K=p^Tq+\frac12p^TWp<0.
\]

All fixture inputs and sampled \(\mu_1\) values (1/8, 1/4, 3/8, 1/2, 5/8, 3/4) are exactly representable dyadics. The tests use the full 6x6 matrix for every residual and energy check.

## What the test suite demonstrates

- full-matrix symmetry and SPD, with a nonzero cross-contact block;
- a mixed slide/stick branch on the open sliding side, with positive \(\tau_1\);
- exact zero stick margin and \(\tau_1=0\) at the boundary;
- a strict all-sticking branch on the other side;
- negative controls against falsely labeling an out-of-disk impulse as sticking or a negative-multiplier branch as sliding;
- a block-diagonal ablation that fails the reconstructed sticking-contact equations.

The tests establish that a known branch reaches a precisely located regime boundary in a genuinely coupled system. They do not prove that this is the only branch, that other branches do not coexist, or that every regime assignment has been searched.

## Why this matters next

The current interval branch work can certify a branch when its inequalities have strict sign margins. At a transition, those margins reach zero and a box can no longer be labeled uniformly sticking or sliding. The next step is to certify the boundary polynomial/inequality itself, then partition uncertain parameter boxes into cells that are strictly inside a regime, strictly outside it, intersect a certified transition stratum, or remain unresolved and explicitly inconclusive.

For sliding branches, the atlas must also track repeated direction-polynomial roots, leading-coefficient degree loss, and the projective direction at infinity. Multi-contact work must retain intersections of two or more contact boundary strata and must recheck every reconstructed candidate against the original unreduced island.

This fixture is the first exact target for that larger atlas, not a substitute for its interval/certified implementation.
