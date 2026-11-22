# Einstein-limit infinity indicial anchor for the reduced spin-2 system

**Status:** exact frozen-coefficient asymptotic result at \(\lambda=0\), \(r_H=1\). The result complements the horizon Frobenius coefficients but does not yet give the complete asymptotic recurrence or global boundary index.

## 1. Logarithmic-radius state

Starting from the exact Einstein-limit reduced system
\[
M(r)\begin{pmatrix}\widehat B_2''\\ \widehat H_2''\end{pmatrix}
+C(r)\begin{pmatrix}\widehat B_2\\\widehat B_2'\\\widehat H_2\\\widehat H_2'\end{pmatrix}+s(r)=0,
\]
define
\[
\tau=\log r,\qquad V=(\widehat B_2,r\widehat B_2',\widehat H_2,r\widehat H_2')^T.
\]
Then a power-law solution \(V\sim r^\sigma v\) is an eigenmode of the constant asymptotic matrix in
\[
\frac{dV}{d\tau}=A_\infty V+g_\infty+o(1).
\]

## 2. Exact matrix and indicial polynomial

The action-specialized reduction gives
\[
A_\infty=
\begin{pmatrix}
0&1&0&0\\
0&-2&0&0\\
0&0&0&1\\
0&0&8&2
\end{pmatrix},
\qquad
g_\infty=0.
\]

Therefore
\[
\boxed{p_\infty(\sigma)=\det(\sigma I-A_\infty)
=\sigma(\sigma-4)(\sigma+2)^2.}
\]

The exponents in the convention \(V\sim r^\sigma\) are:
- \(\sigma=-2\), algebraic and geometric multiplicity two;
- \(\sigma=0\), multiplicity one;
- \(\sigma=4\), multiplicity one.

A complete eigenvector basis is
\[
v_{-2}^{(B)}=(1,-2,0,0)^T,\quad
v_{-2}^{(H)}=(0,0,1,-2)^T,
\]
\[
v_{0}=(1,0,0,0)^T,\quad
v_{4}=(0,0,1,4)^T.
\]
Its determinant is \(-12\), so the leading infinity matrix is diagonalizable. The repeated \(-2\) exponent does not create a leading-order Jordan logarithm. Subleading asymptotic recurrence terms can still produce logarithms and have not yet been evaluated.

## 3. Boundary interpretation and limits

The two \(r^{-2}\) modes are candidate decaying asymptotic directions. The constant and \(r^4\) modes are candidate non-decaying/growing directions under asymptotically flat boundary conditions. Do not yet label these as the final admissible source/response subspaces: the remaining asymptotic recurrence, residual gauge/normalization conditions, and fixed-charge versus fixed-horizon ensemble must be included.

This gives an exact Einstein-limit starting mode count for the global boundary-data map. It does not determine its rank or Fredholm index. It says nothing about the finite-\(\lambda\) infinity spectrum.

## 4. Evidence boundary

The local action-specialized Einstein-limit reduction and exact limit calculation were executed with SymPy 1.14.0. The committed machine-readable fixture and verifier check the characteristic polynomial, eigenvectors, diagonalization, and zero leading source. Exact replay of the committed public action-specialized derivation source, clean-checkout equation generation, a finite-coupling infinity spectrum, full BVP, interval existence, and PDE hyperbolicity remain separate gates.
