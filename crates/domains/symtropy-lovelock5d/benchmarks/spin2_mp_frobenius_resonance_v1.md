# Einstein-limit Frobenius resonance: exact obstruction reduction

**Status:** exact algebraic/Frobenius consequence of the public Einstein-limit residue. The next Taylor coefficient \(A_1\) has not been extracted, so this document does **not** decide whether the singular solution contains a logarithm.

## 1. Inputs and conventions

Use the first-order state
\[
U=(\widehat B_2,\widehat B_2',\widehat H_2,\widehat H_2')^T,
\qquad y=\rho-1,
\]
and the already-derived Einstein-limit system
\[
y\,U'=A(y)U+g(y),\qquad
A(y)=A_0+yA_1+y^2A_2+\cdots,\quad
g(y)=g_0+yg_1+\cdots.
\]

The exact committed horizon data are
\[
A_0=\begin{pmatrix}
0&0&0&0\\
0&0&2&-\frac13\\
0&0&0&0\\
0&0&6&-1
\end{pmatrix},
\qquad
g_0=\begin{pmatrix}0\\-\frac83\\0\\-8\end{pmatrix}.
\]

No coefficient of \(A_1\) is supplied by the existing residue fixture.

## 2. A sharper structure than the indicial polynomial alone

Define
\[
u=\begin{pmatrix}0\\1\\0\\3\end{pmatrix},
\qquad
w^T=\begin{pmatrix}0&0&2&-\frac13\end{pmatrix}.
\]
Then
\[
A_0=u\,w^T,\qquad w^Tu=-1,\qquad A_0^2=-A_0.
\]

Consequently the spectral projectors are available without a numerical eigensolve:
\[
\boxed{P_-=-A_0},\qquad
\boxed{P_0=I+A_0}.
\]
They satisfy
\[
P_-^2=P_-,\quad P_0^2=P_0,\quad
P_-P_0=0,\quad P_-+P_0=I.
\]
The \(-1\) eigenspace is one-dimensional, spanned by \(u\); the zero eigenspace is three-dimensional. The image of \(A_0\) is exactly \(\operatorname{span}(u)\).

This also explains the inhomogeneous compatibility geometrically:
\[
g_0=-\frac83u,\qquad
A_0U_0+g_0=0
\iff
w^TU_0=\frac83
\iff
\widehat H_2'(1)=6\widehat H_2(1)-8.
\]
There is one scalar compatibility condition, not a second independent equation hidden in the duplicate rows.

### 2.1 Exact horizon-adapted coordinates

Choose the columns of
\[
T=\begin{pmatrix}
1&0&0&0\\
0&1&0&1\\
0&0&1&0\\
0&0&6&3
\end{pmatrix},\qquad \det T=3,
\]
as a basis of three zero-eigenvalue vectors and the \(-1\)-eigenvector. The inverse state transformation is
\[
\widetilde U=T^{-1}U=
\begin{pmatrix}
\widehat B_2\\
\widehat B_2'+2\widehat H_2-\widehat H_2'/3\\
\widehat H_2\\
\widehat H_2'/3-2\widehat H_2
\end{pmatrix}.
\]
Direct exact multiplication gives
\[
\boxed{T^{-1}A_0T=\operatorname{diag}(0,0,0,-1)},\qquad
\boxed{T^{-1}g_0=(0,0,0,-8/3)^T}.
\]
Thus the fourth transformed coordinate is precisely the singular-mode amplitude, while the first three coordinates span the analytic eigenspace. The leading inhomogeneous horizon equation fixes that fourth coordinate to
\[
\widetilde U_4(0)=\widehat H_2'(1)/3-2\widehat H_2(1)=-8/3.
\]
With the existing normalization \(\widehat B_2(1)=0\), the horizon data become
\[
\boxed{\widetilde U(0)=(0,\widehat B_2'(1)+8/3,\widehat H_2(1),-8/3)^T.}
\]
The two unconstrained local data are therefore \(\widehat B_2'(1)\) and \(\widehat H_2(1)\). This basis is a practical solver coordinate system: it isolates the excluded singular direction and imposes the inhomogeneous regularity condition as a fixed endpoint value, rather than mixing it with the regular variables.

## 3. The analytic branch has no positive-order resonance

Write an analytic solution and analytic source as
\[
U(y)=\sum_{n\ge0}u_ny^n,\qquad
g(y)=\sum_{n\ge0}g_ny^n.
\]
Coefficient matching gives
\[
A_0u_0+g_0=0
\]
and, for \(n\ge1\),
\[
\boxed{
(nI-A_0)u_n
=\sum_{k=1}^{n}A_ku_{n-k}+g_n.
}
\]

Since \(\operatorname{spec}(A_0)=\{0,-1\}\), the eigenvalues of \(nI-A_0\) are \(n\) and \(n+1\). Thus this matrix is invertible for every positive integer \(n\).

**Consequence:** once the order-zero compatibility condition is met, the analytic Taylor coefficients are recursively determined by the three free components of \(u_0\). The exponent gap of one does not obstruct the analytic branch. After imposing the existing fixed-horizon normalization \(\widehat B_2(1)=0\), this leaves the two local data already identified in the PR.

## 4. The resonance test belongs to the excluded singular branch

The homogeneous singular branch starts with exponent \(-1\). Use
\[
U_{\rm sing}(y)
=y^{-1}(v_0+v_1y+\cdots)+r_0\log y+\cdots,
\qquad v_0\in\ker(A_0+I).
\]
Choose \(v_0=u\) to fix its normalization.

At the first resonant order, a log-free Laurent ansatz would require
\[
-A_0v_1=A_1v_0.
\]
This equation is solvable exactly when \(A_1v_0\) belongs to the image of \(A_0\), namely \(\operatorname{span}(u)\). With the displayed \(+r_0\log y\) convention, the required logarithmic coefficient is
\[
\boxed{r_0=P_0A_1v_0.}
\]
Therefore
\[
\boxed{
\text{the exponent }-1\text{ branch is log-free}
\iff P_0A_1u=0
\iff A_1u\in\operatorname{span}(u).
}
\]

This is three exact scalar conditions because \(P_0\) has rank three. In components, if \(q=A_1u\), they are
\[
q_1=0,\qquad q_3=0,\qquad q_4-3q_2=0
\]
using one-based mathematical component labels \(q_1,\ldots,q_4\).

For orders \(n\ge2\) in the exponent-\(-1\) recurrence, the matrices \((n-1)I-A_0\) have eigenvalues \(n-1\) and \(n\), so they are invertible. Hence the sole integer-gap obstruction is this first \(n=1\) projection test.

The singular branch is excluded by analytic horizon regularity either way. Resolving its logarithm is nevertheless useful: it completes the local fundamental-system classification and makes the endpoint singularity type explicit.

## 5. How to compute \(A_1\) from the reduced second-order system

Write the reduced equations in the fixed state order as
\[
M(y)\begin{pmatrix}\widehat B_2''\\\widehat H_2''\end{pmatrix}
+C(y)U+s(y)=0,
\]
with Taylor **coefficients** (not derivatives)
\[
M=M_0+yM_1+y^2M_2+\cdots,\quad
C=C_0+yC_1+\cdots,\quad
s=s_0+ys_1+\cdots.
\]
The established principal result gives
\[
\det M=d_1y+d_2y^2+\cdots,\qquad d_1=\frac12.
\]

Define \(K(y)=yM(y)^{-1}\). Since the determinant has a simple zero,
\(K(y)\) is analytic at the horizon. For a \(2\times2\) matrix,
\[
d_2=\det M_1+\operatorname{tr}(\operatorname{adj}(M_0)M_2),
\]
\[
K_0=\frac{\operatorname{adj}(M_0)}{d_1},\qquad
K_1=\frac{\operatorname{adj}(M_1)}{d_1}
-\frac{d_2}{d_1^2}\operatorname{adj}(M_0).
\]
The lower two rows of the first-order coefficient matrices are then
\[
\boxed{A_{1,\rm lower}=-(K_1C_0+K_0C_1),}
\]
\[
\boxed{g_{1,\rm lower}=-(K_1s_0+K_0s_1).}
\]
The upper kinematic rows of \(A_1\) are fixed by \(B_2'=U_2\) and \(H_2'=U_4\):
\[
(A_1)_{1,:}=(0,1,0,0),\qquad
(A_1)_{3,:}=(0,0,0,1).
\]

Thus the log test requires \(M_2\) and \(C_1\); the inhomogeneous analytic recurrence at order one additionally requires \(s_1\). This is more targeted than expanding the entire reduced rational system to unnecessarily high order.

## 6. Next exact computation

Extend the existing affine-jet / series-division extractor at \(\lambda=0\) by one additional order:

1. Extract \(M_2,C_1,s_1\) from the same action-derived equation strings and the same \(E_f\) Schur reconstruction.
2. Assemble \(A_1\) and \(g_1\) with the formulas above.
3. Evaluate \(P_0A_1u\) in exact rational arithmetic.
4. If it vanishes, verify the Laurent recurrence at \(n=1\) directly and then all higher recurrences; if it does not, verify the logarithmic ansatz and its coefficient \(r_0\).
5. Preserve the exact source equation digests, the public generator digest, and an exact-byte replay receipt.

The current artifact proves the reduction of the resonance question to one exact projection. It does **not** claim the value of that projection or the logarithmic status.

## 7. Reproduction

The companion standard-library verifier reads the committed indicial JSON, uses rational arithmetic, and checks the rank-one factorization, exact eigenspace projectors, left-nullspace basis, and compatibility-source alignment. Its output JSON records the semantic input digest and the source Git blob SHA-1.

It does not calculate \(A_1\); the next Taylor-jet extraction remains an explicit open gate.