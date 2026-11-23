# Einstein-limit Frobenius series through cubic order

**Status:** exact result at alpha=0, r_H=1, derived by specializing the five-dimensional reduced Einstein action before Euler–Lagrange variation. The independent radial fields b,f,g,h,w are varied first; the gauge g=r^2 is imposed only after variation. The full E_f^(2) algebraic row reconstructs Fhat2; E_b^(2), E_h^(2) form the reduced differential system.

## First-order system

With y=rho-1 and U=(Bhat2,Bhat2',Hhat2,Hhat2')^T:
\[
yU'=(A_0+yA_1+y^2A_2+y^3A_3+O(y^4))U+
(g_0+yg_1+y^2g_2+y^3g_3+O(y^4)).
\]

The new coefficients are
\[
A_3=\begin{pmatrix}
0&0&0&0\\
0&-3&-97/4&-43/8\\
0&0&0&0\\
0&0&-51/4&23/8
\end{pmatrix},\quad
g_3=(0,2849/3,0,209)^T.
\]
The prior A0,A1,A2 and g0,g1,g2 are repeated in the fixture so the recurrence is self-contained.

## Analytic branch

For U(y)=sum U_n y^n, the recurrence is
\[
(nI-A_0)U_n=\sum_{k=1}^{n}A_kU_{n-k}+g_n.
\]
Since the residue spectrum is {0,-1}, the recurrence matrix is invertible for every positive integer n. Thus the analytic branch has no positive-order resonance.

After imposing Bhat2(1)=0, the sole leading compatibility remains
\[
Hhat2'(1)=6Hhat2(1)-8.
\]
The two independent data are b1=Bhat2'(1), h0=Hhat2(1). The local coefficients are
\[
U_0=(0,b_1,h_0,6h_0-8)^T,
\]
\[
U_1=(b_1,24-3b_1,6h_0-8,22h_0-16)^T,
\]
\[
U_2=(12-\tfrac32b_1,6b_1-132,11h_0-8,24h_0-36)^T,
\]
\[
\boxed{U_3=(2b_1-44,444-10b_1,8h_0-12,8h_0+12)^T.}
\]
These are recurrence consequences, not independent boundary conditions. In particular:
\[
Bhat2'''(1)=12b_1-264,\quad Hhat2'''(1)=48h_0-72,
\]
\[
Bhat2''''(1)=2664-60b_1,\quad Hhat2''''(1)=48h_0+72.
\]

## Excluded singular branch

The exponent -1 eigenvector is u=(0,1,0,3)^T. With P0=I+A0, the first obstruction remains
\[
P_0A_1u=(1,2,3,18)^T\ne0.
\]
The excluded branch contains a logarithm:
\[
U_{\rm sing}=y^{-1}u+(1,2,3,18)^T\log y+O(1).
\]
This does not add a further condition to the analytic branch.

## Myers–Perry exact control

At alpha=0 and r_H=1:
\[
Bhat2=-rho^{-2}+rho^{-6},\quad Fhat2=-rho^{-2}+rho^{-4},\quad Hhat2=rho^{-2}.
\]
Its coefficients are
\[
U_0=(0,-4,1,-2),\quad U_1=(-4,36,-2,6),
\]
\[
U_2=(18,-156,3,-12),\quad U_3=(-52,484,-4,20).
\]
The action-specialized reduced E_b/E_h residuals, E_f reconstruction error, and recurrences n=0,1,2,3 vanish exactly.

## Evidence boundary

The local action-specialized derivation source SHA-256 is 49fb0cb536a5f0a8acd588e4c36f94badca63585dc144203fb9d336c19b60fca. The generated fixture SHA-256 is 80ce3b7cdca5fb224551ba6f4f7bc8b8b3dd3f2b0fe103129e840faa4db1dcff. The exact-rational recurrence verifier checks the fixture, not the original variational derivation. This is not a finite-coupling result, boundary-map theorem, interval existence certificate, validated branch, or hyperbolicity result.
