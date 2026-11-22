# Einstein-limit horizon Frobenius recurrence through second order

**Status:** exact action-specialized result at \(\lambda=0\), \(r_H=1\). The derivation sets the Gauss–Bonnet coupling to zero before varying the five independent radial fields \(b,f,g,h,w\), then imposes \(g=r^2\). The algebraic \(E_f^{(2)}\) row reconstructs \(\widehat F_2\); \(E_b^{(2)}\) and \(E_h^{(2)}\) form the reduced differential system.

## 1. Local first-order system

With \(y=\rho-1\) and
\[
U=(\widehat B_2,\widehat B_2',\widehat H_2,\widehat H_2')^T,
\]
the exact Taylor expansion is
\[
yU'=(A_0+yA_1+y^2A_2+O(y^3))U
 +(g_0+yg_1+y^2g_2+O(y^3)).
\]

The coefficient matrices are
\[
A_0=\begin{pmatrix}
0&0&0&0\\
0&0&2&-1/3\\
0&0&0&0\\
0&0&6&-1
\end{pmatrix},
\]
\[
A_1=\begin{pmatrix}
0&1&0&0\\
0&-3&-23/3&1/2\\
0&0&0&1\\
0&0&-7&5/2
\end{pmatrix},
\qquad
A_2=\begin{pmatrix}
0&0&0&0\\
0&3&33/2&3/4\\
0&0&0&0\\
0&0&19/2&-11/4
\end{pmatrix}.
\]

The source vectors are
\[
g_0=(0,-8/3,0,-8)^T,
\quad
g_1=(0,116/3,0,36)^T,
\quad
g_2=(0,-706/3,0,-98)^T.
\]

The \(A_0,g_0,A_1,g_1\) values reproduce the previous Einstein-limit derivation. The new increment is the exact \(A_2,g_2\) extraction.

## 2. Analytic horizon data and recurrence

Write
\[
U(y)=U_0+U_1y+U_2y^2+O(y^3).
\]
Coefficient matching gives
\[
A_0U_0+g_0=0,
\]
\[
(I-A_0)U_1=A_1U_0+g_1,
\]
\[
(2I-A_0)U_2=A_1U_1+A_2U_0+g_2.
\]

Because \(\operatorname{spec}(A_0)=\{0,-1\}\), both \(I-A_0\) and \(2I-A_0\) are invertible. After the fixed-horizon normalization \(\widehat B_2(1)=0\), the leading compatibility equation remains
\[
\widehat H_2'(1)=6\widehat H_2(1)-8.
\]
The two free data are \(b_1=\widehat B_2'(1)\) and \(h_0=\widehat H_2(1)\).

The first three coefficient vectors are
\[
U_0=(0,b_1,h_0,6h_0-8)^T,
\]
\[
U_1=(b_1,24-3b_1,6h_0-8,22h_0-16)^T,
\]
\[
\boxed{U_2=(12-\tfrac32b_1,\;6b_1-132,\;11h_0-8,\;24h_0-36)^T.}
\]

Thus the implied higher horizon derivatives include
\[
\widehat B_2''(1)=24-3b_1,\quad
\widehat H_2''(1)=22h_0-16,
\]
\[
\widehat B_2'''(1)=12b_1-264,\quad
\widehat H_2'''(1)=48h_0-72.
\]
These are derived recurrence consequences, not independent boundary conditions.

## 3. The singular branch is logarithmically obstructed

The residue factorization gives \(P_0=I+A_0\), the projector onto the three-dimensional zero eigenspace. The exponent \(-1\) eigenvector is
\[
u=(0,1,0,3)^T.
\]
The first logarithmic obstruction is
\[
P_0A_1u=(1,2,3,18)^T\ne0.
\]
Therefore the excluded singular homogeneous branch has local form
\[
U_{\rm sing}(y)=y^{-1}u+(1,2,3,18)^T\log y+O(1).
\]
Analytic horizon regularity excludes this entire branch. It does not add a further constraint to the analytic branch.

## 4. Exact Myers–Perry control

For \(\lambda=0\), the exact control profiles are
\[
\widehat B_2=-\rho^{-2}+\rho^{-6},\qquad
\widehat F_2=-\rho^{-2}+\rho^{-4},\qquad
\widehat H_2=\rho^{-2}.
\]
Their horizon coefficient vectors are
\[
U_0^{\rm MP}=(0,-4,1,-2)^T,
\quad U_1^{\rm MP}=(-4,36,-2,6)^T,
\]
\[
U_2^{\rm MP}=(18,-156,3,-12)^T.
\]
The reduced \(E_b,E_h\) residuals, \(E_f\) reconstruction error, and all three recurrence defects vanish exactly.

## 5. Evidence boundary

The direct \(\alpha=0\) action-specialized development script completed locally with exact SymPy arithmetic. Its development-source SHA-256 is `1f40a5d4ba03a94c36df1c629575b7fe460921accc18cbbce9d1dcda63a9a33a`; the emitted fixture SHA-256 is `a5adc1a220ad67622a6c0cef2b32c06ea08f4826d7399b3057e67ca3e83c1274`.

This result does **not** determine the finite-coupling lower-order residue, infinity indicial modes, global boundary-data map, finite-coupling BVP, interval existence, validated continuation, or full PDE hyperbolicity. The public generic-coupling equation generator has not been replayed as part of this specialization.
